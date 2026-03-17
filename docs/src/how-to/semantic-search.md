# Semantic Search

Task-focused recipes for common semantic search operations.

## Enable semantic search

Add the `semantic-search` feature to your `synwire-agent` dependency:

```toml
[dependencies]
synwire-agent = { version = "0.1", features = ["semantic-search"] }
```

This is an opt-in feature because it adds heavyweight dependencies (fastembed,
LanceDB, tree-sitter grammars). When disabled, `LocalProvider` omits the
`INDEX` and `SEMANTIC_SEARCH` capabilities and the corresponding VFS tools are
not offered to the LLM.

## Index a directory

```rust,ignore
use synwire_agent::vfs::local::LocalProvider;
use synwire_core::vfs::protocol::Vfs;
use synwire_core::vfs::types::IndexOptions;
use std::path::PathBuf;

let vfs = LocalProvider::new(PathBuf::from("/path/to/project"))?;

let handle = vfs.index("src", IndexOptions {
    force: false,
    include: vec!["**/*.rs".into(), "**/*.py".into()],
    exclude: vec!["target/**".into(), "**/node_modules/**".into()],
    max_file_size: Some(1_048_576),
}).await?;
```

The `include` and `exclude` fields accept glob patterns. If `include` is empty,
all files are included. `exclude` patterns are always applied.

## Wait for indexing to complete

```rust,ignore
use synwire_core::vfs::types::IndexStatus;

loop {
    match vfs.index_status(&handle.index_id).await? {
        IndexStatus::Ready(result) => {
            println!("{} files, {} chunks", result.files_indexed, result.chunks_produced);
            break;
        }
        IndexStatus::Failed(e) => return Err(e.into()),
        _ => tokio::time::sleep(std::time::Duration::from_millis(500)).await,
    }
}
```

## Search by meaning

```rust,ignore
use synwire_core::vfs::types::SemanticSearchOptions;

let results = vfs.semantic_search("authentication flow", SemanticSearchOptions {
    top_k: Some(10),
    min_score: None,
    file_filter: vec![],
    rerank: Some(true),
}).await?;
```

## Search within specific files

Use `file_filter` to restrict search to a subset of indexed files:

```rust,ignore
let results = vfs.semantic_search("error handling", SemanticSearchOptions {
    top_k: Some(5),
    file_filter: vec!["src/auth/**".into(), "src/middleware/**".into()],
    ..Default::default()
}).await?;
```

## Disable reranking for faster results

Cross-encoder reranking improves accuracy but adds latency. Disable it when
speed matters more than precision:

```rust,ignore
let results = vfs.semantic_search("database queries", SemanticSearchOptions {
    rerank: Some(false),
    ..Default::default()
}).await?;
```

## Force a full re-index

By default, `index()` reuses cached results if available. To force a fresh index
(e.g. after a large merge or branch switch):

```rust,ignore
let handle = vfs.index("src", IndexOptions {
    force: true,
    ..Default::default()
}).await?;
```

## Configure chunk sizes

The default chunk size (1 500 bytes with 200-byte overlap) works well for most
codebases. To adjust for very large or very small files, construct the
`SemanticIndex` directly:

```rust,ignore
use synwire_index::{SemanticIndex, IndexConfig};

let config = IndexConfig {
    cache_base: None,       // use OS default
    chunk_size: 2000,       // larger chunks for long functions
    chunk_overlap: 300,     // more context between chunks
};
```

> **Note**: Chunk size only affects the text splitter fallback. AST-chunked code
> files always use one chunk per definition regardless of size.

## Use a custom cache directory

By default, index data is stored under the OS cache directory
(`$XDG_CACHE_HOME/synwire/indices/` on Linux). To use a project-local cache:

```rust,ignore
use synwire_index::IndexConfig;
use std::path::PathBuf;

let config = IndexConfig {
    cache_base: Some(PathBuf::from(".synwire-cache")),
    ..Default::default()
};
```

## Stop the file watcher

The background file watcher starts automatically after indexing completes. To
stop it (e.g. before shutting down):

```rust,ignore
// If using SemanticIndex directly:
index.unwatch(&path).await;

// If using LocalProvider, the watcher stops when the provider is dropped.
```

## Combine semantic search with grep

Semantic search and grep serve different purposes. Use both in a complementary
workflow:

```rust,ignore
// Step 1: Find the concept
let semantic_results = vfs.semantic_search(
    "rate limiting middleware",
    SemanticSearchOptions::default(),
).await?;

// Step 2: Find exact usages of the function you discovered
let grep_results = vfs.grep(
    "apply_rate_limit",
    synwire_core::vfs::types::GrepOptions::default(),
).await?;
```

## Handle indexing errors

Individual file failures during indexing are logged and skipped — the pipeline
continues with remaining files. To detect these:

- Check `IndexResult::files_indexed` against expected file count.
- Enable `tracing` at `WARN` level to see per-file errors:

```rust,ignore
// In your application setup:
tracing_subscriber::fmt()
    .with_env_filter("synwire_index=warn")
    .init();
```

## Agentic ignore files

`LocalProvider` automatically discovers and respects agentic ignore files —
`.cursorignore`, `.aiignore`, `.claudeignore`, `.aiderignore`, `.copilotignore`,
`.codeiumignore`, `.tabbyignore`, and `.gitignore` — by searching upward from
the provider's root directory to the filesystem root. Files matching any
discovered pattern are excluded from `ls`, `grep`, `glob`, and semantic indexing.

All ignore files use gitignore syntax, including negation (`!` prefix) and
directory-only patterns (trailing `/`):

```text
# .cursorignore — exclude secrets and build artifacts
.env*
secret/
target/
node_modules/
!.env.example    # but keep the example
```

To check which ignore files are in effect, create an `AgenticIgnore` directly:

```rust,ignore
use synwire_core::vfs::agentic_ignore::AgenticIgnore;
use std::path::Path;

let ai = AgenticIgnore::discover(Path::new("/path/to/project"));
assert!(ai.is_ignored(Path::new("/path/to/project/.env"), false));
```

## Prevent indexing the root filesystem

`index("/", ..)` is always rejected with `VfsError::IndexDenied`. This is a
safety measure — indexing the entire filesystem would be extremely slow and
produce unusable results. Always index specific project directories.

## Hybrid search (BM25 + vector)

When `synwire-index` is built with the `hybrid-search` feature, you can combine BM25 lexical scoring with vector semantic scoring:

```rust,ignore
#[cfg(feature = "hybrid-search")]
use synwire_index::{HybridSearchConfig, hybrid_search};

let config = HybridSearchConfig {
    alpha: 0.5,   // 0.0 = pure vector, 1.0 = pure BM25
    top_k: 10,
};

let results = hybrid_search(&bm25_index, &vector_store, &embeddings, "auth", config).await?;
```

### Tuning alpha

| alpha | Best for |
|-------|---------|
| `0.0` | Conceptual queries ("authentication logic") |
| `0.5` | General use — balanced (default) |
| `1.0` | Exact identifier queries ("MyStruct::authenticate") |

Start with `alpha = 0.5` and adjust based on your query patterns. If you are searching for exact function names, increase alpha toward 1.0. If you are searching conceptually, decrease it toward 0.0.

## See also

- [Semantic Search Tutorial](../tutorials/09-semantic-search.md) — step-by-step walkthrough
- [Semantic Search Architecture](../explanation/semantic-search-architecture.md) — design rationale
- [VFS Providers](./vfs.md) — general VFS operations
- [Advanced Search (grep)](./grep-search.md) — text pattern search
