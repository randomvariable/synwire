# synwire-embeddings-local

Local text embedding and cross-encoder reranking for Synwire, backed by [fastembed-rs](https://github.com/Anush008/fastembed-rs) and ONNX Runtime. No API keys required. No data leaves the machine.

## Quick start

```rust,no_run
use synwire_embeddings_local::{LocalEmbeddings, LocalReranker};
use synwire_core::embeddings::Embeddings;

// Embedding (BAAI/bge-small-en-v1.5, 384 dimensions)
let embedder = LocalEmbeddings::new()?;
let query_vec = embedder.embed_query("authentication logic").await?;
assert_eq!(query_vec.len(), 384);

let doc_vecs = embedder.embed_documents(&[
    "fn authenticate(user: &str) -> Result<Token>".to_owned(),
    "fn logout(session_id: &str)".to_owned(),
]).await?;

// Reranking (BAAI/bge-reranker-base)
let reranker = LocalReranker::new()?;
let reranked = reranker.rerank("authentication", &docs, 5).await?;
```

## Models

| Type | Model | Params | Output | Purpose |
|------|-------|--------|--------|---------|
| Bi-encoder | BAAI/bge-small-en-v1.5 | 33M | 384-dim f32 | Fast similarity search |
| Cross-encoder | BAAI/bge-reranker-base | 110M | Relevance score | Accurate re-scoring |

Both models are downloaded from Hugging Face Hub on first use and cached locally by fastembed. Subsequent instantiations load from cache with no network access.

### Model cache location

Fastembed stores models under `~/.cache/huggingface/hub/` by default. Use `HUGGINGFACE_HUB_CACHE` to override.

## Two-stage retrieval

`LocalEmbeddings` and `LocalReranker` are designed to work together in a two-stage pipeline:

```
Query → embed_query() → similarity_search(top_k=50) → rerank(top_n=10) → results
```

**Stage 1 (bi-encoder)**: Embed query and retrieve top-k candidates by cosine similarity. Fast because document vectors are precomputed.

**Stage 2 (cross-encoder)**: Rerank candidates by jointly attending to (query, document) pairs. Slower but significantly more accurate. Applied only to the top-k from stage 1 to bound latency.

## Async safety

fastembed inference is synchronous and CPU-bound. Both types wrap inference in `tokio::task::spawn_blocking` to avoid blocking the async runtime. Both implement `Send + Sync` and are safe to share via `Arc`.

```rust,no_run
use std::sync::Arc;
use synwire_embeddings_local::LocalEmbeddings;

let embedder = Arc::new(LocalEmbeddings::new()?);

// Safe to clone and use across tasks
let e1 = Arc::clone(&embedder);
let e2 = Arc::clone(&embedder);

tokio::join!(
    async move { e1.embed_query("query one").await },
    async move { e2.embed_query("query two").await },
);
```

## Traits implemented

`LocalEmbeddings` implements `synwire_core::embeddings::Embeddings`:

| Method | Input | Output |
|--------|-------|--------|
| `embed_documents` | `&[String]` | `Vec<Vec<f32>>` |
| `embed_query` | `&str` | `Vec<f32>` |

`LocalReranker` implements `synwire_core::rerankers::Reranker`:

| Method | Input | Output |
|--------|-------|--------|
| `rerank` | query, `&[Document]`, top_n | `Vec<Document>` (re-ordered) |

## Error handling

| Error | Cause |
|-------|-------|
| `LocalEmbeddingsError::Init` | Model download failure or ONNX load error |
| `LocalRerankerError::Init` | Same, for the reranker |
| `EmbeddingError::Failed` | Inference panic or no results returned |

Construction errors (`::new()`) are distinct from inference errors. Construction can fail on first use if the network is unavailable for model download. Subsequent constructions load from cache and should not fail under normal conditions.

## Performance

| Operation | Typical latency (CPU) | Notes |
|-----------|-----------------------|-------|
| `LocalEmbeddings::new()` | 50–200 ms (cached) | First ever: ~30 MB download |
| `LocalReranker::new()` | 100–400 ms (cached) | First ever: ~110 MB download |
| `embed_query` | 1–5 ms | Single text, 384-dim |
| `embed_documents` (batch) | ~2 ms/doc | Batching amortises overhead |
| `rerank` | 5–20 ms/candidate | Cross-encoder is heavier |

Figures are order-of-magnitude on a modern x86 CPU.

## Feature flags

No optional features. Always depends on `fastembed` and `tokio`.
