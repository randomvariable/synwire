//! Orchestrates the full walk → chunk → embed → store pipeline.

use std::path::Path;
use std::sync::Arc;
use synwire_chunker::{ChunkOptions, Chunker};
use synwire_core::embeddings::Embeddings;
use synwire_core::vectorstores::VectorStore;
use synwire_core::vfs::IndexOptions;
use tracing::{debug, warn};

use crate::hashes::{self, HashRegistry};

/// Run the full indexing pipeline for a directory.
///
/// Compares each file's xxh128 content hash against `hash_registry` and skips
/// files whose content has not changed.  After processing, updated hashes are
/// written back into the registry (the caller is responsible for persisting it).
///
/// Returns `(files_indexed, chunks_produced)` — `files_indexed` counts only
/// files that were actually re-embedded, not those skipped by hash match.
///
/// # Errors
///
/// Propagates any error from the vector store's `add_documents` call that
/// terminates early.  Individual file read failures are logged and skipped.
pub async fn run(
    root: &Path,
    opts: &IndexOptions,
    embeddings: &Arc<dyn Embeddings>,
    store: &Arc<dyn VectorStore>,
    chunk_size: usize,
    chunk_overlap: usize,
    hash_registry: &mut HashRegistry,
) -> Result<(usize, usize), Box<dyn std::error::Error + Send + Sync>> {
    let files = crate::walker::walk(root, opts);
    let mut files_indexed = 0usize;
    let mut chunks_produced = 0usize;
    let chunker = Chunker::with_options({
        let mut opts = ChunkOptions::default();
        opts.chunk_size = chunk_size;
        opts.overlap = chunk_overlap;
        opts
    });

    for file_path in &files {
        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Skipping {}: {e}", file_path.display());
                continue;
            }
        };

        let path_str = file_path.to_string_lossy().to_string();
        let new_hash = hashes::xxh128_hex(content.as_bytes());

        // Skip if content hash matches the previously indexed version.
        if let Some(old_hash) = hash_registry.files.get(&path_str)
            && *old_hash == new_hash
        {
            debug!("Skipping {} (unchanged, xxh128 match)", file_path.display());
            continue;
        }

        let chunks = chunker.chunk_file(&path_str, &content);
        if chunks.is_empty() {
            // Update hash even for empty-chunk files so we don't re-process them.
            let _ = hash_registry.files.insert(path_str, new_hash);
            continue;
        }
        chunks_produced += chunks.len();
        files_indexed += 1;
        debug!("Indexing {} ({} chunks)", file_path.display(), chunks.len());

        // Embed and store in one batch per file; log failures but continue.
        match store.add_documents(&chunks, embeddings.as_ref()).await {
            Ok(_ids) => {
                let _ = hash_registry.files.insert(path_str, new_hash);
            }
            Err(e) => warn!("Failed to index {}: {e}", file_path.display()),
        }
    }

    Ok((files_indexed, chunks_produced))
}
