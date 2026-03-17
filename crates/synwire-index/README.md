# synwire-index

Semantic indexing pipeline for Synwire VFS providers. Orchestrates directory walking, AST-aware chunking, embedding, vector storage, and background file watching into a single `SemanticIndex` entry point.

## Pipeline

```
walk(path) → chunk_file() → embed_documents() → vector_store::add() → meta.json
```

1. **Walk**: Collect files matching include/exclude globs, up to `max_file_size` (default 1 MiB).
2. **Hash check**: Skip files whose xxHash128 content hash matches the stored hash — no re-embedding on unchanged files.
3. **Chunk**: Each changed file is split into `Document`s by `synwire-chunker` (AST or text splitter).
4. **Embed**: Document texts are batch-embedded via `synwire-embeddings-local`.
5. **Store**: Vectors are written to `synwire-vectorstore-lancedb`.
6. **Cache**: `meta.json` and `hashes.json` are written to the index cache directory.

## Quick start

```rust,no_run
use synwire_index::{SemanticIndex, IndexConfig, StoreFactory};
use synwire_chunker::Chunker;
use synwire_embeddings_local::{LocalEmbeddings, LocalReranker};
use synwire_vectorstore_lancedb::LanceDbVectorStore;
use std::sync::Arc;
use std::path::Path;

let embeddings = Arc::new(LocalEmbeddings::new()?);
let reranker = Some(Arc::new(LocalReranker::new()?));

let store_factory: StoreFactory = Box::new(|path: &Path| {
    let handle = tokio::runtime::Handle::current();
    handle.block_on(LanceDbVectorStore::open(
        path.join("lance").to_string_lossy(),
        "chunks",
        384,
    ))
});

let index = SemanticIndex::new(
    Chunker::new(),
    embeddings,
    reranker,
    store_factory,
    IndexConfig::default(),
    None, // optional event sender
);

// Start indexing (non-blocking — returns a handle)
let handle = index.index(Path::new("/path/to/project"), Default::default()).await?;

// Poll for completion
use synwire_index::IndexStatus;
loop {
    match index.status(&handle.index_id).await {
        IndexStatus::Ready(_) => break,
        IndexStatus::Failed(e) => return Err(e.into()),
        _ => tokio::time::sleep(std::time::Duration::from_millis(500)).await,
    }
}

// Search
let results = index.search(
    Path::new("/path/to/project"),
    "authentication logic",
    Default::default(),
).await?;
```

## Configuration

```rust,no_run
use synwire_index::IndexConfig;
use std::path::PathBuf;

let config = IndexConfig {
    cache_base: Some(PathBuf::from(".myapp-cache")),  // default: OS cache dir
    chunk_size: 2000,    // default: 1500
    chunk_overlap: 300,  // default: 200
};
```

`chunk_size` and `chunk_overlap` apply only to the text splitter path. AST-chunked files always produce one chunk per definition.

## Feature flags

| Feature | Default | Description |
|---------|---------|-------------|
| `hybrid-search` | No | BM25 (tantivy) + vector hybrid search |
| `code-graph` | No | Cross-file call/import/inherit dependency graph |
| `community-detection` | No | HIT-Leiden clustering over code graph |

### Hybrid search (`hybrid-search`)

Combines BM25 lexical scoring with vector semantic scoring using a configurable alpha parameter:

```
score = alpha * bm25_score + (1 - alpha) * vector_score
```

- `alpha = 1.0`: pure BM25 (exact keyword match)
- `alpha = 0.0`: pure vector (semantic match)
- `alpha = 0.5`: balanced hybrid (default)

```rust,no_run
#[cfg(feature = "hybrid-search")]
use synwire_index::{HybridSearchConfig, hybrid_search};

let config = HybridSearchConfig { alpha: 0.5, top_k: 10 };
let results = hybrid_search(&bm25_index, &vector_store, &embeddings, "auth", config).await?;
```

### Code graph (`code-graph`)

Builds a cross-file dependency graph from tree-sitter ASTs. Node types: `(file, symbol)`. Edge types: `calls`, `imports`, `contains`, `inherits`.

```rust,no_run
#[cfg(feature = "code-graph")]
use synwire_index::{XrefGraph, xref_query, XrefDirection};

let xrefs = xref_query(&graph, "MyStruct::authenticate", 2, XrefDirection::Incoming).await?;
```

### Community detection (`community-detection`)

Applies HIT-Leiden clustering to the code graph to identify cohesive modules. Community state is persisted via `StorageLayout::communities_dir()`.

## File watcher

After a successful index, a background file watcher starts automatically:

- Platform-native: `inotify` (Linux), `FSEvents` (macOS), `ReadDirectoryChangesW` (Windows)
- Events within a 300 ms window are coalesced (debounced)
- Only files with changed content (by xxHash128) trigger re-indexing
- The watcher stops when `SemanticIndex` is dropped or `unwatch(path)` is called

## Incremental updates

Only changed files are re-processed on subsequent `index()` calls or watcher events. The hash table (`hashes.json`) tracks content hashes per file; unchanged files are skipped entirely.

## See also

- [synwire-chunker](../synwire-chunker/README.md) — chunking strategies
- [synwire-embeddings-local](../synwire-embeddings-local/README.md) — local embedding models
- [synwire-vectorstore-lancedb](../synwire-vectorstore-lancedb/README.md) — vector storage backend
- [synwire-storage](../synwire-storage/README.md) — cache path management
