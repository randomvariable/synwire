# Provider Contracts: All Partners

**Date**: 2026-03-09
**Branch**: `001-synwire`

## Provider Architecture

### OpenAI-Compatible Base

`BaseChatOpenAI` in `synwire-llm-openai` provides the shared foundation
for OpenAI's `/v1/chat/completions` API format, including HTTP client, SSE
parsing, tool-call accumulation, and error handling. `ChatOpenAI` wraps
this base type directly.

```rust
// In synwire-llm-openai/src/base.rs
pub struct BaseChatOpenAI {
    pub(crate) model: String,
    pub(crate) api_key: String,
    pub(crate) api_base: String,          // provider-specific default
    pub(crate) api_key_env: &'static str, // env var name for auto-detection
    pub(crate) temperature: Option<f32>,
    pub(crate) max_tokens: Option<u32>,
    pub(crate) top_p: Option<f32>,
    pub(crate) stop: Option<Vec<String>>,
    pub(crate) timeout: Option<Duration>,
    pub(crate) max_retries: u32,
    pub(crate) model_kwargs: HashMap<String, Value>,
    pub(crate) client: reqwest::Client,
}
```

`BaseChatOpenAI` implements `BaseChatModel` and `Runnable<Vec<Message>, ChatResult>`.

### Provider Classification

| Category | Providers | Shared Code |
|---|---|---|
| OpenAI-native | OpenAI | Full implementation in synwire-llm-openai |
| Native API | Ollama | Own HTTP client and parsing |
| Vector Store | Qdrant, PGVector | Own client SDKs |
| Graph Store | Neo4j | neo4rs Bolt client |
| Search | SerpAPI, SearXNG, NCBI, arXiv | REST/XML API clients |
| Workflow | Temporal | temporal-sdk-core |

---

## OpenAI-Native Provider

### synwire-llm-openai

```rust
pub struct ChatOpenAI {
    base: BaseChatOpenAI,
}
```

See [openai.md](openai.md) for full contract details. `ChatOpenAI` wraps
`BaseChatOpenAI` and provides the standard OpenAI chat completions interface.

- **Default api_base**: `https://api.openai.com/v1`
- **API key env**: `OPENAI_API_KEY`
- **Traits**: `BaseChatModel`, `Runnable<Vec<Message>, ChatResult>`, `Send + Sync + Debug`

---

## Native API Providers

### synwire-llm-ollama

```rust
pub struct ChatOllama {
    model: String,
    base_url: String,            // default: http://localhost:11434
    // Model parameters
    temperature: Option<f32>,
    top_k: Option<u32>,
    top_p: Option<f32>,
    num_predict: Option<u32>,    // max tokens
    repeat_penalty: Option<f32>,
    num_thread: Option<u32>,
    // Connection
    timeout: Option<Duration>,
    model_kwargs: HashMap<String, Value>,
    client: reqwest::Client,
}

pub struct OllamaLLM {
    model: String,
    base_url: String,
    temperature: Option<f32>,
    // ... same model params as ChatOllama
    client: reqwest::Client,
}

pub struct OllamaEmbeddings {
    model: String,
    base_url: String,
    timeout: Option<Duration>,
    client: reqwest::Client,
}
```

- **API key env**: None (local server)
- **API endpoints**: POST `/api/chat` (chat), POST `/api/generate` (LLM),
  POST `/api/embed` (embeddings)
- **Traits**: ChatOllama: `BaseChatModel`; OllamaLLM: `BaseLLM`;
  OllamaEmbeddings: `Embeddings`
- **Models**: llama3.3, mistral, gemma2, phi3, etc.
- **Ollama-specific**: Streaming uses NDJSON (newline-delimited JSON),
  not SSE. Each line is a JSON object with `message`, `done`, etc.

---

## Vector Store Providers

### synwire-vectorstore-qdrant

```rust
pub struct QdrantVectorStore {
    collection_name: String,
    embedding: Box<dyn Embeddings>,
    client: QdrantClient,
    // Search configuration
    search_type: SearchType,
    score_threshold: Option<f32>,
}

pub enum SearchType {
    Similarity,
    Mmr { fetch_k: usize, lambda_mult: f32 },
}

struct QdrantClient {
    url: String,
    api_key: Option<String>,
    grpc_port: Option<u16>,
    prefer_grpc: bool,
    timeout: Option<Duration>,
    client: reqwest::Client,     // REST client (gRPC via tonic optional)
}
```

- **API key env**: `QDRANT_API_KEY` (optional for local)
- **API endpoints**: REST or gRPC to Qdrant server
  - PUT `/collections/{name}/points` (upsert)
  - POST `/collections/{name}/points/query` (search)
- **Traits**: `VectorStore`, `Send + Sync + Debug`
- **Dependencies**: `reqwest` for REST; optionally `tonic` + `qdrant-client` for gRPC
- **Note**: `SparseEmbeddings` for hybrid search is NOT part of the core
  `Embeddings` trait. A Qdrant-specific `SparseEmbeddings` trait may be
  added in this crate as an extension.

### synwire-vectorstore-pgvector

```rust
pub struct PgVectorStore {
    pool: deadpool_postgres::Pool,
    table_name: String,
    embedding: Box<dyn Embeddings>,
    vector_dimensions: u32,
    distance_strategy: DistanceStrategy,
}

pub enum DistanceStrategy {
    Cosine,
    Euclidean,
    InnerProduct,
}
```

- **API**: Direct PostgreSQL connection via tokio-postgres + pgvector extension
- **Traits**: `VectorStore`, `Send + Sync + Debug`
- **Dependencies**: `tokio-postgres`, `pgvector`, `deadpool-postgres`
- **Setup requires**: PostgreSQL with `CREATE EXTENSION vector`

---

## Graph Store Providers

### synwire-graphstore-neo4j

```rust
pub struct Neo4jGraphStore {
    graph: neo4rs::Graph,
    database: Option<String>,
    embedding: Option<Box<dyn Embeddings>>,
}

pub struct Neo4jRetriever {
    graph_store: Neo4jGraphStore,
    query_template: String,
    k: usize,
}
```

- **Connection**: Bolt protocol via `neo4rs`
- **Traits**: `GraphStore` (new trait), `Send + Sync + Debug`; Neo4jRetriever: `Retriever`
- **Dependencies**: `neo4rs`
- **Env**: `NEO4J_URI`, `NEO4J_USERNAME`, `NEO4J_PASSWORD`

---

## Search Providers

### synwire-search-serpapi

```rust
pub struct SerpApiSearch {
    api_key: String,
    engine: String,          // default: "google"
    params: HashMap<String, String>,
    client: reqwest::Client,
}

pub struct SerpApiRetriever {
    search: SerpApiSearch,
    k: usize,
}
```

- **API key env**: `SERPAPI_API_KEY`
- **API endpoint**: GET `https://serpapi.com/search`
- **Traits**: SerpApiSearch: `Tool`; SerpApiRetriever: `Retriever`

### synwire-search-searxng

```rust
pub struct SearxngSearch {
    base_url: String,        // self-hosted SearXNG instance
    categories: Option<Vec<String>>,
    engines: Option<Vec<String>>,
    language: Option<String>,
    client: reqwest::Client,
}

pub struct SearxngRetriever {
    search: SearxngSearch,
    k: usize,
}
```

- **API key env**: None (self-hosted)
- **API endpoint**: GET `{base_url}/search?format=json`
- **Traits**: SearxngSearch: `Tool`; SearxngRetriever: `Retriever`

### synwire-search-ncbi

```rust
pub struct NcbiSearch {
    api_key: Option<String>,
    database: String,         // default: "pubmed"
    max_results: usize,       // default: 10
    client: reqwest::Client,
}

pub struct NcbiRetriever {
    search: NcbiSearch,
    k: usize,
}
```

- **API key env**: `NCBI_API_KEY` (optional — increases rate limit)
- **API endpoints**: NCBI E-utilities (esearch, efetch, einfo) — XML responses
- **Traits**: NcbiSearch: `Tool`; NcbiRetriever: `Retriever`
- **Dependencies**: `quick-xml` for XML parsing

### synwire-search-arxiv

```rust
pub struct ArxivSearch {
    max_results: usize,       // default: 10
    sort_by: SortCriterion,
    sort_order: SortOrder,
    client: reqwest::Client,
}

pub struct ArxivRetriever {
    search: ArxivSearch,
    k: usize,
}

pub enum SortCriterion { Relevance, LastUpdatedDate, SubmittedDate }
pub enum SortOrder { Ascending, Descending }
```

- **API key env**: None (public API)
- **API endpoint**: GET `http://export.arxiv.org/api/query`
- **Traits**: ArxivSearch: `Tool`; ArxivRetriever: `Retriever`
- **Dependencies**: `quick-xml` for Atom feed parsing

---

## Workflow Providers

### synwire-workflow-temporal

```rust
pub struct TemporalWorkflow {
    config: TemporalConfig,
    client: temporal_sdk_core::Client,
}

pub struct TemporalConfig {
    namespace: String,
    task_queue: String,
    server_url: String,       // default: "localhost:7233"
    retry_policy: Option<RetryPolicy>,
}
```

- **Connection**: gRPC to Temporal server
- **Traits**: Custom `WorkflowRuntime` trait
- **Dependencies**: `temporal-sdk-core`, `synwire-orchestrator`
- **Env**: `TEMPORAL_ADDRESS`, `TEMPORAL_NAMESPACE`

---

## Common Provider Patterns

### Builder Pattern (all providers)

Every provider type uses the builder pattern:

```rust
impl ChatOpenAI {
    pub fn builder() -> ChatOpenAIBuilder;
}

impl ChatOpenAIBuilder {
    pub fn model(self, model: impl Into<String>) -> Self;
    pub fn api_key(self, key: impl Into<String>) -> Self;
    pub fn temperature(self, temp: f32) -> Self;
    // ... provider-specific fields ...
    pub fn build(self) -> Result<ChatOpenAI, SynwireError>;
}
```

`build()` auto-reads API key from the provider's env var if not set
explicitly. Returns `SynwireError::ConfigError` if required fields
are missing.

### Error Pattern (all providers)

Each provider defines its own error enum with `From<ProviderError> for
SynwireError`. The error classification (transient vs permanent) follows
the same pattern as OpenAI (see [openai.md](openai.md) §Error Handling).

### Trait Compliance (all providers)

All provider types MUST implement:
- `Send + Sync` (required by trait bounds)
- `Debug` (for error reporting)
- The relevant core trait(s): `BaseChatModel`, `Embeddings`, `VectorStore`,
  `Retriever`, `Tool`, `GraphStore`, `WorkflowRuntime`, and/or `BaseLLM`
- `Runnable<I, O>` for the appropriate input/output types (enables chain
  composition via `pipe`)

### Integration Testing (all providers)

All provider crates feature-gate integration tests:

```toml
[features]
integration-tests = []
```

Integration tests read API keys from environment variables and skip
with a warning if not set. For example:

```rust
#[cfg(feature = "integration-tests")]
#[tokio::test]
async fn test_serpapi_search() {
    let api_key = std::env::var("SERPAPI_API_KEY")
        .expect("SERPAPI_API_KEY must be set for integration tests");
    let search = SerpApiSearch::builder()
        .api_key(api_key)
        .build()
        .unwrap();
    // ...
}
```

---

## Intentional Exclusions

- **AzureChatOpenAI / AzureOpenAIEmbeddings**: Azure AD authentication
  and deployment-based routing are architecturally different from standard
  API key auth. Would be a `synwire-llm-azure-openai` crate.
- **SparseEmbeddings**: Qdrant-specific sparse vectors are not part of
  the core `Embeddings` trait. May be added as a Qdrant-specific extension.
