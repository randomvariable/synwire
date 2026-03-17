# synwire-vectorstore-lancedb

LanceDB-backed vector store for Synwire semantic search. Implements the `VectorStore` trait for persistent vector storage and k-NN similarity search.

## Quick start

```rust,no_run
use synwire_vectorstore_lancedb::LanceDbVectorStore;
use synwire_core::vectorstores::VectorStore;
use synwire_core::documents::Document;

// Open (or create) a LanceDB table
let store = LanceDbVectorStore::open("/tmp/my-db", "chunks", 384).await?;

// Add documents with precomputed embeddings
let docs = vec![
    Document::new("fn authenticate(user: &str) -> Result<Token>"),
];
let embeddings = vec![vec![0.1_f32; 384]];
store.add_documents(docs, embeddings).await?;

// Similarity search
let query_vec = vec![0.1_f32; 384];
let results = store.similarity_search_with_score(&query_vec, 10).await?;
for (doc, score) in results {
    println!("{score:.3}  {}", doc.page_content);
}
```

## Schema

Each row in the LanceDB table stores:

| Column | Type | Description |
|--------|------|-------------|
| `id` | `Utf8` | UUID document identifier |
| `content` | `Utf8` | Document text (`page_content`) |
| `vector` | `FixedSizeList<Float32>` | Embedding vector (`embedding_dim` floats) |
| `metadata` | `Utf8` | JSON-serialised metadata map |

## Opening a store

```rust,no_run
use synwire_vectorstore_lancedb::LanceDbVectorStore;

// Create table if it does not exist; open it if it does.
// embedding_dim must match the model producing your vectors.
let store = LanceDbVectorStore::open(
    "/path/to/lancedb-dir",   // directory; created if absent
    "chunks",                  // table name
    384,                       // embedding dimension (bge-small-en-v1.5 → 384)
).await?;
```

## Upsert semantics

`add_documents` uses `overwrite: false` by default — documents are appended. Documents with the same `id` create duplicates. Use `upsert_documents` when you need to replace existing entries (e.g. during incremental re-indexing of changed files).

## Concurrent access

LanceDB provides native concurrent access with MVCC (multi-version concurrency control). Multiple readers and writers can operate on the same directory simultaneously. No external locking is required.

## VectorStore trait

`LanceDbVectorStore` implements `synwire_core::vectorstores::VectorStore`:

| Method | Description |
|--------|-------------|
| `add_documents(docs, embeddings)` | Append documents with precomputed vectors |
| `similarity_search_with_score(query_vec, k)` | Return top-k `(Document, f32)` by cosine similarity |
| `delete_where(filter)` | Remove documents matching a SQL-like filter expression |

## Error handling

All methods return `Result<T, LanceDbError>`. The error type wraps `lancedb` library errors and I/O failures.

## Feature flags

No optional features. Always depends on `lancedb`, `arrow`, and `tokio`.
