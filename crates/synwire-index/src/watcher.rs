//! Background file watcher for incremental index updates.

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use synwire_chunker::{ChunkOptions, Chunker};
use synwire_core::embeddings::Embeddings;
use synwire_core::vectorstores::VectorStore;
use tokio::sync::{Mutex, mpsc};
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use crate::hashes;

/// Handle to a running background file watcher.
pub struct WatcherHandle {
    /// Token used to stop the watcher background task.
    cancel: CancellationToken,
}

impl WatcherHandle {
    /// Stop the background watcher.
    pub fn stop(&self) {
        self.cancel.cancel();
    }
}

/// Start a background file watcher for `root`.
///
/// When files are created, modified, or removed the watcher re-chunks the
/// affected file and updates the vector store — but only if the file's xxh128
/// content hash has changed since the last index.
pub fn start(
    root: PathBuf,
    embeddings: Arc<dyn Embeddings>,
    store: Arc<dyn VectorStore>,
    chunk_size: usize,
    chunk_overlap: usize,
    known_hashes: HashMap<String, String>,
) -> WatcherHandle {
    let cancel = CancellationToken::new();
    let cancel_for_thread = cancel.clone();
    let cancel_for_task = cancel.clone();

    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<PathBuf>();

    // The notify watcher requires a synchronous callback; run it in a dedicated
    // OS thread.  The tokio task processes the events asynchronously.
    let _thread = std::thread::spawn(move || {
        let tx = event_tx.clone();
        let mut watcher: RecommendedWatcher =
            match notify::recommended_watcher(move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                            for path in event.paths {
                                if path.is_file() && event_tx.send(path).is_err() {
                                    // Receiver dropped; watcher should shut down.
                                    return;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }) {
                Ok(w) => w,
                Err(e) => {
                    warn!("Failed to create file watcher: {e}");
                    return;
                }
            };

        if let Err(e) = watcher.watch(&root, RecursiveMode::Recursive) {
            warn!("Failed to watch {}: {e}", root.display());
            return;
        }

        // Keep the watcher alive until the cancellation token fires.
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            if cancel_for_thread.is_cancelled() {
                break;
            }
        }

        // Dropping `watcher` here unregisters the watch.
        drop(watcher);
        // Also drop `tx` so the mpsc receiver observes closure.
        drop(tx);
    });

    let file_hashes = Arc::new(Mutex::new(known_hashes));

    let _task = tokio::spawn(async move {
        let chunker = Chunker::with_options({
            let mut co = ChunkOptions::default();
            co.chunk_size = chunk_size;
            co.overlap = chunk_overlap;
            co
        });
        loop {
            tokio::select! {
                () = cancel_for_task.cancelled() => break,
                path = event_rx.recv() => {
                    match path {
                        Some(p) => handle_change(&p, &chunker, &embeddings, &store, &file_hashes).await,
                        None => break,
                    }
                }
            }
        }
    });

    WatcherHandle { cancel }
}

async fn handle_change(
    path: &Path,
    chunker: &Chunker,
    embeddings: &Arc<dyn Embeddings>,
    store: &Arc<dyn VectorStore>,
    file_hashes: &Arc<Mutex<HashMap<String, String>>>,
) {
    let path_str = path.to_string_lossy().to_string();
    debug!("File changed: {path_str}");

    match std::fs::read_to_string(path) {
        Ok(content) => {
            let new_hash = hashes::xxh128_hex(content.as_bytes());

            // Skip re-indexing if the content hash is unchanged.
            {
                let hashes = file_hashes.lock().await;
                if let Some(old_hash) = hashes.get(&path_str)
                    && *old_hash == new_hash
                {
                    debug!("Skipping {path_str} (unchanged, xxh128 match)");
                    return;
                }
            }

            let chunks = chunker.chunk_file(&path_str, &content);
            if !chunks.is_empty() {
                match store.add_documents(&chunks, embeddings.as_ref()).await {
                    Ok(_ids) => {
                        let mut hashes = file_hashes.lock().await;
                        let _ = hashes.insert(path_str, new_hash);
                    }
                    Err(e) => warn!("Failed to re-index {path_str}: {e}"),
                }
            }
        }
        Err(_) => {
            // File deleted or unreadable — deletion from the vector store
            // requires IDs which are not tracked here; a full re-index is
            // needed to remove stale chunks.
            debug!("File deleted or unreadable: {path_str}");
        }
    }
}
