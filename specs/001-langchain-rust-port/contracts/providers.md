# Provider Contracts: All Partners

**Date**: 2026-03-09
**Branch**: `001-langchain-rust-port`

## Provider Architecture

### OpenAI-Compatible Base

Providers using OpenAI's `/v1/chat/completions` API format share a common
base type in `langchain-openai`. This avoids duplicating HTTP client, SSE
parsing, tool-call accumulation, and error handling across 7+ crates.

```rust
// In langchain-openai/src/base.rs
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
Provider-specific types wrap it and add provider-specific configuration.

### Provider Classification

| Category | Providers | Shared Code |
|---|---|---|
| OpenAI-native | OpenAI | Full implementation in langchain-openai |
| OpenAI-compatible | Groq, Fireworks, DeepSeek, xAI, OpenRouter | Thin wrappers around `BaseChatOpenAI` |
| OpenAI-partial | Mistral AI, Perplexity | Use OpenAI format with extensions |
| Native API | Anthropic, Ollama, HuggingFace | Own HTTP client and parsing |
| Vector Store | Chroma, Qdrant | Own client SDKs |
| Specialized | Exa, Nomic | Retriever/embeddings-only |

---

## OpenAI-Compatible Providers

These providers use OpenAI's API format with a different `api_base` and
`api_key` env var. Each is a thin wrapper around `BaseChatOpenAI`.

### langchain-groq

```rust
pub struct ChatGroq {
    base: BaseChatOpenAI,
    // Groq-specific
    reasoning_format: Option<ReasoningFormat>,  // "parsed", "raw", "hidden"
    reasoning_effort: Option<String>,
    service_tier: Option<ServiceTier>,          // "on_demand", "flex", "auto"
}

pub enum ReasoningFormat { Parsed, Raw, Hidden }
pub enum ServiceTier { OnDemand, Flex, Auto }
```

- **Default api_base**: `https://api.groq.com/openai/v1`
- **API key env**: `GROQ_API_KEY`
- **Traits**: `BaseChatModel`, `Runnable<Vec<Message>, ChatResult>`, `Send + Sync + Debug`
- **Models**: llama-3.3-70b-versatile, llama-3.1-8b-instant, gemma2-9b-it, etc.

### langchain-fireworks

```rust
pub struct ChatFireworks {
    base: BaseChatOpenAI,
}

pub struct FireworksEmbeddings {
    model: String,
    api_key: String,
    api_base: Option<String>,
    timeout: Option<Duration>,
    max_retries: u32,
    client: reqwest::Client,
}
```

- **Default api_base**: `https://api.fireworks.ai/inference/v1`
- **API key env**: `FIREWORKS_API_KEY`
- **Traits**: ChatFireworks: `BaseChatModel`; FireworksEmbeddings: `Embeddings`
- **Models**: accounts/fireworks/models/llama-v3p3-70b-instruct, etc.

### langchain-deepseek

```rust
pub struct ChatDeepSeek {
    base: BaseChatOpenAI,
}
```

- **Default api_base**: `https://api.deepseek.com/v1`
- **API key env**: `DEEPSEEK_API_KEY`
- **Traits**: `BaseChatModel`, `Runnable<Vec<Message>, ChatResult>`
- **Models**: deepseek-chat, deepseek-reasoner

### langchain-xai

```rust
pub struct ChatXAI {
    base: BaseChatOpenAI,
}
```

- **Default api_base**: `https://api.x.ai/v1`
- **API key env**: `XAI_API_KEY`
- **Traits**: `BaseChatModel`, `Runnable<Vec<Message>, ChatResult>`
- **Models**: grok-4, grok-3-mini, grok-3

### langchain-openrouter

```rust
pub struct ChatOpenRouter {
    base: BaseChatOpenAI,
}
```

- **Default api_base**: `https://openrouter.ai/api/v1`
- **API key env**: `OPENROUTER_API_KEY`
- **Traits**: `BaseChatModel`, `Runnable<Vec<Message>, ChatResult>`
- **Models**: provider-prefixed (e.g. `anthropic/claude-3.5-sonnet`, `openai/gpt-4o`)

---

## OpenAI-Partial Providers

These use an OpenAI-like format but have provider-specific extensions.

### langchain-mistralai

```rust
pub struct ChatMistralAI {
    base: BaseChatOpenAI,
    // Mistral-specific
    max_concurrent_requests: Option<usize>,
}

pub struct MistralAIEmbeddings {
    model: String,              // e.g. "mistral-embed"
    api_key: String,
    api_base: Option<String>,
    timeout: Option<Duration>,
    max_retries: u32,
    client: reqwest::Client,
}
```

- **Default api_base**: `https://api.mistral.ai/v1`
- **API key env**: `MISTRAL_API_KEY`
- **Traits**: ChatMistralAI: `BaseChatModel`; MistralAIEmbeddings: `Embeddings`
- **Models**: mistral-large-latest, mistral-small-latest, codestral-latest
- **Note**: Mistral's API is OpenAI-compatible for chat completions but
  uses Mistral-specific endpoints for embeddings

### langchain-perplexity

```rust
pub struct ChatPerplexity {
    base: BaseChatOpenAI,
    // Perplexity-specific
    search_mode: Option<SearchMode>,           // "academic", "sec", "web"
    reasoning_effort: Option<ReasoningEffort>, // "low", "medium", "high"
    search_domain_filter: Option<Vec<String>>, // max 20 domains
    return_images: bool,
    return_related_questions: bool,
    search_recency_filter: Option<SearchRecency>, // "day", "week", "month", "year"
}

pub struct PerplexitySearchRetriever {
    client: ChatPerplexity,
    k: usize,
}

pub enum SearchMode { Academic, Sec, Web }
pub enum ReasoningEffort { Low, Medium, High }
pub enum SearchRecency { Day, Week, Month, Year }
```

- **Default api_base**: `https://api.perplexity.ai`
- **API key env**: `PPLX_API_KEY`
- **Traits**: ChatPerplexity: `BaseChatModel`; PerplexitySearchRetriever: `Retriever`
- **Models**: sonar, sonar-pro, sonar-reasoning, sonar-reasoning-pro

---

## Native API Providers

These use their own API formats and need dedicated HTTP client logic.

### langchain-anthropic

```rust
pub struct ChatAnthropic {
    model: String,              // e.g. "claude-sonnet-4-20250514"
    api_key: String,
    // Model parameters
    temperature: Option<f32>,
    max_tokens: u32,            // required by Anthropic API
    top_k: Option<u32>,
    top_p: Option<f32>,
    stop_sequences: Option<Vec<String>>,
    // Connection
    api_base: Option<String>,   // default: https://api.anthropic.com
    timeout: Option<Duration>,
    max_retries: u32,           // default: 2
    model_kwargs: HashMap<String, Value>,
    client: reqwest::Client,
}
```

- **API key env**: `ANTHROPIC_API_KEY`
- **API endpoint**: POST `/v1/messages`
- **Traits**: `BaseChatModel`, `Runnable<Vec<Message>, ChatResult>`, `Send + Sync + Debug`
- **Models**: claude-sonnet-4-20250514, claude-haiku-4-5-20251001, claude-opus-4-20250514
- **Anthropic-specific features**:
  - `max_tokens` is required (not optional like OpenAI)
  - Streaming uses SSE with `event: content_block_delta` and `event: message_stop`
  - Tool use via `tools` parameter (similar to OpenAI but different JSON schema)
  - Extended thinking via `thinking` content blocks
  - System prompt is a top-level parameter, not a message

#### Anthropic Message Format Mapping

| LangChain Message | Anthropic Role | Notes |
|---|---|---|
| Human | `user` | Direct mapping |
| AI | `assistant` | Direct mapping |
| System | top-level `system` param | NOT a message — extracted and set separately |
| Tool | `user` with `tool_result` block | Wraps tool output in `tool_result` content |
| Chat(role) | varies | Map to closest Anthropic role |

#### Anthropic Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum AnthropicError {
    #[error("authentication failed: {0}")]
    AuthenticationError(String),          // HTTP 401
    #[error("permission denied: {0}")]
    PermissionError(String),              // HTTP 403
    #[error("resource not found: {0}")]
    NotFoundError(String),                // HTTP 404
    #[error("rate limited: retry after {retry_after:?}")]
    RateLimitError { retry_after: Option<Duration> },  // HTTP 429
    #[error("invalid request: {0}")]
    BadRequestError(String),              // HTTP 400
    #[error("overloaded: {0}")]
    OverloadedError(String),              // HTTP 529
    #[error("server error: {status} {body}")]
    ServerError { status: u16, body: String },
    #[error(transparent)]
    HttpError(#[from] reqwest::Error),
}
```

### langchain-ollama

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

### langchain-huggingface

```rust
pub struct ChatHuggingFace {
    model_id: String,
    api_token: Option<String>,
    temperature: Option<f32>,
    max_new_tokens: Option<u32>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    // Endpoint config
    endpoint_url: Option<String>,  // for HF Inference Endpoints
    timeout: Option<Duration>,
    model_kwargs: HashMap<String, Value>,
    client: reqwest::Client,
}

pub struct HuggingFaceEmbeddings {
    model_id: String,             // e.g. "sentence-transformers/all-MiniLM-L6-v2"
    api_token: Option<String>,
    endpoint_url: Option<String>,
    timeout: Option<Duration>,
    client: reqwest::Client,
}

pub struct HuggingFacePipeline {
    model_id: String,
    task: String,                 // "text-generation", etc.
    // Note: local pipeline — requires a Rust ML runtime
    // Initial implementation: API-only via Inference Endpoints
    client: reqwest::Client,
}
```

- **API key env**: `HUGGINGFACEHUB_API_TOKEN`
- **API endpoints**: POST `https://api-inference.huggingface.co/models/{model_id}`
  or custom Inference Endpoint URL
- **Traits**: ChatHuggingFace: `BaseChatModel`; HuggingFaceEmbeddings: `Embeddings`;
  HuggingFacePipeline: `BaseLLM`
- **Note**: `HuggingFacePipeline` in Python runs models locally via
  `transformers`. The Rust port initially supports API-only mode via
  HuggingFace Inference Endpoints. Local inference via `candle` or
  `ort` (ONNX Runtime) is a future extension.

---

## Vector Store Providers

### langchain-chroma

```rust
pub struct Chroma {
    collection_name: String,
    embedding_function: Box<dyn Embeddings>,
    client: ChromaClient,        // HTTP client to Chroma server
    // Connection
    host: String,                // default: "localhost"
    port: u16,                   // default: 8000
    ssl: bool,
    api_key: Option<String>,     // for Chroma Cloud
    tenant: Option<String>,
    database: Option<String>,
}

struct ChromaClient {
    base_url: String,
    client: reqwest::Client,
}
```

- **API endpoints**: REST API on Chroma server
  - POST `/api/v1/collections` (create)
  - POST `/api/v1/collections/{id}/add` (add documents)
  - POST `/api/v1/collections/{id}/query` (search)
- **Traits**: `VectorStore`, `Send + Sync + Debug`
- **Dependencies**: `reqwest` (HTTP client to Chroma server)
- **Note**: No native Rust Chroma SDK — uses REST API directly

### langchain-qdrant

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

---

## Specialized Providers

### langchain-nomic

```rust
pub struct NomicEmbeddings {
    model: String,               // e.g. "nomic-embed-text-v1.5"
    api_key: Option<String>,
    dimensionality: Option<u32>,
    inference_mode: InferenceMode,
    client: reqwest::Client,
}

pub enum InferenceMode {
    Remote,     // Nomic API
    Local,      // future: local model
    Dynamic,    // auto-select
}
```

- **API key env**: `NOMIC_API_KEY`
- **Traits**: `Embeddings`, `Send + Sync + Debug`
- **Initial implementation**: Remote mode only (API calls to Nomic)

### langchain-exa

```rust
pub struct ExaSearchRetriever {
    api_key: String,
    k: usize,                    // default: 10
    search_type: ExaSearchType,  // neural, keyword, auto
    include_domains: Option<Vec<String>>,
    exclude_domains: Option<Vec<String>>,
    use_autoprompt: Option<bool>,
    highlights: bool,
    text_contents: bool,
    client: reqwest::Client,
}

pub struct ExaSearchResults {
    api_key: String,
    client: reqwest::Client,
}

pub enum ExaSearchType { Neural, Keyword, Auto }
```

- **API key env**: `EXA_API_KEY`
- **API endpoint**: POST `https://api.exa.ai/search`
- **Traits**: ExaSearchRetriever: `Retriever`;
  ExaSearchResults: `Tool`
- **Note**: Exa is a search engine, not a language model.
  ExaSearchRetriever returns `Vec<Document>` from web search results.

---

## Common Provider Patterns

### Builder Pattern (all providers)

Every provider type uses the builder pattern:

```rust
impl ChatAnthropic {
    pub fn builder() -> ChatAnthropicBuilder;
}

impl ChatAnthropicBuilder {
    pub fn model(self, model: impl Into<String>) -> Self;
    pub fn api_key(self, key: impl Into<String>) -> Self;
    pub fn temperature(self, temp: f32) -> Self;
    // ... provider-specific fields ...
    pub fn build(self) -> Result<ChatAnthropic, LangChainError>;
}
```

`build()` auto-reads API key from the provider's env var if not set
explicitly. Returns `LangChainError::ConfigError` if required fields
are missing.

### Error Pattern (all providers)

Each provider defines its own error enum with `From<ProviderError> for
LangChainError`. The error classification (transient vs permanent) follows
the same pattern as OpenAI (see [openai.md](openai.md) §Error Handling).

### Trait Compliance (all providers)

All provider types MUST implement:
- `Send + Sync` (required by trait bounds)
- `Debug` (for error reporting)
- The relevant core trait(s): `BaseChatModel`, `Embeddings`, `VectorStore`,
  `Retriever`, `Tool`, and/or `BaseLLM`
- `Runnable<I, O>` for the appropriate input/output types (enables chain
  composition via `pipe`)

### Integration Testing (all providers)

All provider crates feature-gate integration tests:

```toml
[features]
integration-tests = []
```

Integration tests read API keys from environment variables and skip
with a warning if not set.

---

## Intentional Exclusions

- **AzureChatOpenAI / AzureOpenAIEmbeddings**: Azure AD authentication
  and deployment-based routing are architecturally different from standard
  API key auth. Would be a `langchain-azure-openai` crate.
- **HuggingFacePipeline local inference**: Running models locally via
  `candle` or `ort` is a future extension. Initial port is API-only.
- **SparseEmbeddings**: Qdrant-specific sparse vectors are not part of
  the core `Embeddings` trait. May be added as a Qdrant-specific extension.
- **Nomic local/dynamic inference modes**: Initial port is remote API only.
