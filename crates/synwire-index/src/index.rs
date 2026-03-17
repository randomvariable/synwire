//! [`SemanticIndex`] — the primary entry point for semantic indexing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use synwire_core::embeddings::Embeddings;
use synwire_core::rerankers::Reranker;
use synwire_core::vectorstores::VectorStore;
use synwire_core::vfs::{
    IndexEvent, IndexHandle, IndexOptions, IndexResult, IndexStatus, SemanticSearchOptions,
    SemanticSearchResult, VfsError,
};

use crate::cache;
use crate::config::IndexConfig;
use crate::hashes;
use crate::pipeline;
use crate::watcher::WatcherHandle;

/// Internal state of a single indexing job.
struct IndexJob {
    path: PathBuf,
    status: IndexStatus,
    watcher: Option<WatcherHandle>,
}

/// Factory closure that creates a [`VectorStore`] for a given cache directory.
///
/// Receives the path to the per-index cache directory.  The factory is
/// responsible for opening or creating the store at that location.
pub type StoreFactory = Box<
    dyn Fn(&Path) -> Result<Arc<dyn VectorStore>, Box<dyn std::error::Error + Send + Sync>>
        + Send
        + Sync,
>;

/// The semantic indexing pipeline.
///
/// Orchestrates directory walking, AST-aware chunking, embedding, and
/// vector storage.  VFS providers hold an instance of this struct and
/// delegate `index`, `status`, and `search` calls to it.
///
/// # Thread safety
///
/// `SemanticIndex` is `Send + Sync` and may be shared across async tasks via
/// `Arc<SemanticIndex>`.
pub struct SemanticIndex {
    embeddings: Arc<dyn Embeddings>,
    reranker: Option<Arc<dyn Reranker>>,
    store_factory: StoreFactory,
    config: IndexConfig,
    jobs: Arc<RwLock<HashMap<String, IndexJob>>>,
    event_tx: Option<mpsc::Sender<IndexEvent>>,
}

impl SemanticIndex {
    /// Create a new `SemanticIndex` with the given dependencies.
    ///
    /// - `embeddings` — embedding model used to vectorise chunks.
    /// - `reranker` — optional cross-encoder for result reranking.
    /// - `store_factory` — factory that produces a [`VectorStore`] for a cache path.
    /// - `config` — pipeline configuration (chunk sizes, cache directory).
    /// - `event_tx` — optional channel for streaming [`IndexEvent`]s to a caller.
    pub fn new(
        embeddings: Arc<dyn Embeddings>,
        reranker: Option<Arc<dyn Reranker>>,
        store_factory: StoreFactory,
        config: IndexConfig,
        event_tx: Option<mpsc::Sender<IndexEvent>>,
    ) -> Self {
        Self {
            embeddings,
            reranker,
            store_factory,
            config,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        }
    }

    /// Start indexing `path` asynchronously.  Returns immediately with an [`IndexHandle`].
    ///
    /// If `opts.force` is `false` and a valid cache exists for this path, the
    /// index is considered fresh and no work is performed.
    ///
    /// # Errors
    ///
    /// - [`VfsError::IndexDenied`] — `path` resolves to the filesystem root.
    /// - [`VfsError::Io`] — `path` cannot be canonicalised or the store factory fails.
    #[allow(clippy::too_many_lines)]
    pub async fn index(&self, path: &Path, opts: &IndexOptions) -> Result<IndexHandle, VfsError> {
        let canonical = std::fs::canonicalize(path).map_err(VfsError::Io)?;

        if canonical == Path::new("/") {
            return Err(VfsError::IndexDenied {
                reason: "Indexing the root filesystem is not permitted.".into(),
            });
        }

        let index_id = Uuid::new_v4().to_string();
        let handle = IndexHandle {
            index_id: index_id.clone(),
            path: canonical.to_string_lossy().to_string(),
        };

        {
            let mut jobs = self.jobs.write().await;
            let _ = jobs.insert(
                index_id.clone(),
                IndexJob {
                    path: canonical.clone(),
                    status: IndexStatus::Pending,
                    watcher: None,
                },
            );
        }

        let cache_dir = cache::cache_dir(&self.config, &canonical);
        let store = (self.store_factory)(&cache_dir)
            .map_err(|e| VfsError::Io(std::io::Error::other(e.to_string())))?;

        // Use cached result if fresh and force=false.
        if !opts.force
            && let Some(meta) = cache::read_meta(&cache_dir)
        {
            let result = IndexResult {
                path: canonical.to_string_lossy().to_string(),
                files_indexed: meta.files_indexed,
                chunks_produced: meta.chunks_produced,
                was_cached: true,
            };
            {
                let mut jobs = self.jobs.write().await;
                if let Some(job) = jobs.get_mut(&index_id) {
                    job.status = IndexStatus::Ready(result.clone());
                }
            }
            if let Some(tx) = &self.event_tx {
                let _ = tx
                    .send(IndexEvent::Complete {
                        index_id: index_id.clone(),
                        result,
                    })
                    .await;
            }
            return Ok(handle);
        }

        // Spawn the background indexing task.
        let jobs_ref = Arc::clone(&self.jobs);
        let embeddings = Arc::clone(&self.embeddings);
        let store_for_watch = Arc::clone(&store);
        let event_tx = self.event_tx.clone();
        let opts_clone = opts.clone();
        let config = self.config.clone();
        let id_clone = index_id.clone();
        let canonical_clone = canonical.clone();

        let _task = tokio::spawn(async move {
            // Transition to Indexing.
            {
                let mut jobs = jobs_ref.write().await;
                if let Some(job) = jobs.get_mut(&id_clone) {
                    job.status = IndexStatus::Indexing { progress: 0.0 };
                }
            }
            if let Some(tx) = &event_tx {
                let _ = tx
                    .send(IndexEvent::Progress {
                        index_id: id_clone.clone(),
                        progress: 0.0,
                    })
                    .await;
            }

            let idx_cache_dir = cache::cache_dir(&config, &canonical_clone);
            let mut hash_registry = hashes::read_hashes(&idx_cache_dir);

            match pipeline::run(
                &canonical_clone,
                &opts_clone,
                &embeddings,
                &store,
                config.chunk_size,
                config.chunk_overlap,
                &mut hash_registry,
            )
            .await
            {
                Ok((files_indexed, chunks_produced)) => {
                    // Persist the updated content hashes alongside the meta.
                    if let Err(e) = hashes::write_hashes(&idx_cache_dir, &hash_registry) {
                        tracing::warn!("Failed to write hash registry: {e}");
                    }

                    let meta = cache::IndexMeta {
                        path: canonical_clone.to_string_lossy().to_string(),
                        indexed_at: chrono::Utc::now().to_rfc3339(),
                        files_indexed,
                        chunks_produced,
                        version: 1,
                    };
                    if let Err(e) = cache::write_meta(&idx_cache_dir, &meta) {
                        tracing::warn!("Failed to write index meta: {e}");
                    }

                    let result = IndexResult {
                        path: canonical_clone.to_string_lossy().to_string(),
                        files_indexed,
                        chunks_produced,
                        was_cached: false,
                    };

                    let watcher = crate::watcher::start(
                        canonical_clone,
                        embeddings,
                        store_for_watch,
                        config.chunk_size,
                        config.chunk_overlap,
                        hash_registry.files,
                    );

                    {
                        let mut jobs = jobs_ref.write().await;
                        if let Some(job) = jobs.get_mut(&id_clone) {
                            job.status = IndexStatus::Ready(result.clone());
                            job.watcher = Some(watcher);
                        }
                    }
                    if let Some(tx) = &event_tx {
                        let _ = tx
                            .send(IndexEvent::Complete {
                                index_id: id_clone,
                                result,
                            })
                            .await;
                    }
                }
                Err(e) => {
                    let err_str = e.to_string();
                    {
                        let mut jobs = jobs_ref.write().await;
                        if let Some(job) = jobs.get_mut(&id_clone) {
                            job.status = IndexStatus::Failed(err_str.clone());
                        }
                    }
                    if let Some(tx) = &event_tx {
                        let _ = tx
                            .send(IndexEvent::Failed {
                                index_id: id_clone,
                                error: err_str,
                            })
                            .await;
                    }
                }
            }
        });

        Ok(handle)
    }

    /// Check the status of an indexing operation by its `index_id`.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::NotFound`] if `index_id` is unknown.
    pub async fn status(&self, index_id: &str) -> Result<IndexStatus, VfsError> {
        let jobs = self.jobs.read().await;
        jobs.get(index_id).map_or_else(
            || Err(VfsError::NotFound(format!("No index with id {index_id}"))),
            |job| Ok(job.status.clone()),
        )
    }

    /// Semantic search across indexed content for `path`.
    ///
    /// # Errors
    ///
    /// - [`VfsError::IndexNotReady`] — the index is still building or has never been started.
    /// - [`VfsError::Io`] — store factory or similarity search fails.
    #[allow(clippy::too_many_lines)]
    pub async fn search(
        &self,
        path: &Path,
        query: &str,
        opts: &SemanticSearchOptions,
    ) -> Result<Vec<SemanticSearchResult>, VfsError> {
        let canonical = std::fs::canonicalize(path).map_err(VfsError::Io)?;
        let cache_dir = cache::cache_dir(&self.config, &canonical);

        {
            let jobs = self.jobs.read().await;
            let any_indexing = jobs.values().filter(|j| j.path == canonical).any(|j| {
                matches!(
                    j.status,
                    IndexStatus::Indexing { .. } | IndexStatus::Pending
                )
            });
            let any_ready = jobs
                .values()
                .filter(|j| j.path == canonical)
                .any(|j| matches!(j.status, IndexStatus::Ready(_)));
            drop(jobs);

            if any_indexing && !any_ready {
                return Err(VfsError::IndexNotReady(
                    canonical.to_string_lossy().to_string(),
                ));
            }
            if !any_ready && cache::read_meta(&cache_dir).is_none() {
                return Err(VfsError::IndexNotReady(
                    canonical.to_string_lossy().to_string(),
                ));
            }
        }

        let vector_store = (self.store_factory)(&cache_dir)
            .map_err(|e| VfsError::Io(std::io::Error::other(e.to_string())))?;

        let top_k = opts.top_k.unwrap_or(10);
        let search_results = vector_store
            .similarity_search_with_score(query, top_k, self.embeddings.as_ref())
            .await
            .map_err(|e| VfsError::Io(std::io::Error::other(e.to_string())))?;

        let min_score = opts.min_score.unwrap_or(0.0);
        let use_reranker = opts.rerank.unwrap_or(true);

        // Optionally rerank the candidate documents.
        let docs_for_rerank: Vec<_> = search_results.iter().map(|(d, _)| d.clone()).collect();
        let reranked = if use_reranker {
            if let Some(reranker) = &self.reranker {
                reranker
                    .rerank(query, &docs_for_rerank, top_k)
                    .await
                    .map_err(|e| VfsError::Io(std::io::Error::other(e.to_string())))?
            } else {
                docs_for_rerank
            }
        } else {
            docs_for_rerank
        };

        // Pair reranked docs with scores from the original similarity search by
        // position.  Reranking may reorder results so scores are approximate.
        let raw_scores: Vec<f32> = search_results.iter().map(|(_, s)| *s).collect();

        let file_filter_globs: Vec<globset::GlobMatcher> = opts
            .file_filter
            .iter()
            .filter_map(|pat| {
                globset::Glob::new(pat)
                    .map_err(|e| tracing::warn!("Invalid file_filter glob {pat:?}: {e}"))
                    .ok()
                    .map(|g| g.compile_matcher())
            })
            .collect();

        let mut output = Vec::new();
        for (i, doc) in reranked.into_iter().enumerate() {
            let hit_score = raw_scores.get(i).copied().unwrap_or(0.0);
            if hit_score < min_score {
                continue;
            }

            let file = doc
                .metadata
                .get("file")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();

            if !file_filter_globs.is_empty() && !file_filter_globs.iter().any(|m| m.is_match(&file))
            {
                continue;
            }

            #[allow(clippy::cast_possible_truncation)]
            let line_start = doc
                .metadata
                .get("line_start")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(1) as usize;

            #[allow(clippy::cast_possible_truncation)]
            let line_end = doc
                .metadata
                .get("line_end")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(1) as usize;

            output.push(SemanticSearchResult {
                file,
                line_start,
                line_end,
                content: doc.page_content.clone(),
                score: hit_score,
                symbol: doc
                    .metadata
                    .get("symbol")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                language: doc
                    .metadata
                    .get("language")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
            });
        }

        Ok(output)
    }

    /// Stop the background watcher for `path`.
    ///
    /// Does nothing if `path` cannot be canonicalised or has no active watcher.
    pub async fn unwatch(&self, path: &Path) {
        let Ok(canonical) = std::fs::canonicalize(path) else {
            return;
        };
        let mut jobs = self.jobs.write().await;
        for job in jobs.values_mut().filter(|j| j.path == canonical) {
            if let Some(w) = job.watcher.take() {
                w.stop();
            }
        }
    }
}
