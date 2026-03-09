# Provider Contract: langchain-openai

**Date**: 2026-03-09
**Branch**: `001-langchain-rust-port`

## ChatOpenAI

### Configuration

```rust
pub struct ChatOpenAI {
    // Required
    model: String,              // e.g. "gpt-4o", "gpt-4o-mini", "gpt-3.5-turbo"
    api_key: String,            // OpenAI API key (from env OPENAI_API_KEY or explicit)

    // Model parameters
    temperature: Option<f32>,         // 0.0-2.0, default provider default
    max_tokens: Option<u32>,          // max output tokens
    top_p: Option<f32>,               // nucleus sampling
    frequency_penalty: Option<f32>,   // -2.0 to 2.0
    presence_penalty: Option<f32>,    // -2.0 to 2.0
    stop: Option<Vec<String>>,        // stop sequences
    seed: Option<u64>,                // for deterministic output
    response_format: Option<ResponseFormat>,  // JSON mode / JSON schema

    // Connection parameters
    api_base: Option<String>,         // custom base URL (default: https://api.openai.com/v1)
    organization: Option<String>,     // OpenAI org ID
    timeout: Option<Duration>,        // request timeout (default: 60s)
    max_retries: u32,                 // HTTP-level retries (default: 2)

    // Internal
    client: reqwest::Client,          // configured with reqwest-middleware
}

pub enum ResponseFormat {
    Text,
    JsonObject,
    JsonSchema { schema: Value, name: String, strict: bool },
}
```

### Builder Pattern

```rust
impl ChatOpenAI {
    pub fn builder() -> ChatOpenAIBuilder;
}

impl ChatOpenAIBuilder {
    pub fn model(self, model: impl Into<String>) -> Self;
    pub fn api_key(self, key: impl Into<String>) -> Self;
    pub fn temperature(self, temp: f32) -> Self;
    pub fn max_tokens(self, tokens: u32) -> Self;
    pub fn top_p(self, p: f32) -> Self;
    pub fn frequency_penalty(self, penalty: f32) -> Self;
    pub fn presence_penalty(self, penalty: f32) -> Self;
    pub fn stop(self, stop: Vec<String>) -> Self;
    pub fn seed(self, seed: u64) -> Self;
    pub fn response_format(self, format: ResponseFormat) -> Self;
    pub fn api_base(self, url: impl Into<String>) -> Self;
    pub fn organization(self, org: impl Into<String>) -> Self;
    pub fn timeout(self, timeout: Duration) -> Self;
    pub fn max_retries(self, retries: u32) -> Self;
    pub fn build(self) -> Result<ChatOpenAI, LangChainError>;
}
```

### Trait Implementations

`ChatOpenAI` implements:
- `BaseChatModel` — invoke via POST to `/v1/chat/completions`
- `Runnable<Vec<Message>, ChatResult>` — for chain composition
- `Send + Sync + Debug`

### Tool Calling / Function Calling

`ChatOpenAI::bind_tools` configures the model to include tool definitions
in every request. Implementation:
1. Converts `Vec<ToolSchema>` to OpenAI's `tools` parameter format
2. Returns a new `ChatOpenAI` clone with tools pre-configured
3. Tool calls in the response are parsed into `Message::AI { tool_calls }`

### Structured Output

`ChatOpenAI::with_structured_output` uses OpenAI's `response_format` with
`type: "json_schema"` when available (GPT-4o+), falling back to tool-calling
mode for older models:
1. Wraps the model with a `StructuredOutputParser<T>` in a `RunnableSequence`
2. Returns `Box<dyn Runnable<Vec<Message>, Value>>`

### Streaming with Tool Calls

SSE stream parsing for tool calls:
1. Parse SSE events using `eventsource-stream`
2. Each `data:` line is a `ChatCompletionChunk` JSON
3. Tool call deltas arrive as `ToolCallChunk` with `index`, partial `id`,
   partial `name`, and partial `arguments` (JSON string fragment)
4. Accumulate by `index` across chunks — concatenate `arguments` strings
5. On `finish_reason: "tool_calls"`, parse accumulated arguments as JSON
6. Emit complete `ToolCall` objects in the final `ChatChunk`

## OpenAIEmbeddings

### Configuration

```rust
pub struct OpenAIEmbeddings {
    model: String,              // e.g. "text-embedding-3-small", "text-embedding-3-large"
    api_key: String,
    dimensions: Option<u32>,    // output dimensions (model-specific default if None)
    encoding_format: Option<EncodingFormat>,  // float or base64
    chunk_size: usize,          // max texts per API call (default: 2048)
    api_base: Option<String>,
    organization: Option<String>,
    timeout: Option<Duration>,
    max_retries: u32,
    client: reqwest::Client,
}

pub enum EncodingFormat {
    Float,
    Base64,
}
```

### Trait Implementations

`OpenAIEmbeddings` implements:
- `Embeddings` — embed via POST to `/v1/embeddings`
- `Send + Sync + Debug`

### Dimension Validation

- `text-embedding-3-small`: default 1536 dimensions, configurable 256-1536
- `text-embedding-3-large`: default 3072 dimensions, configurable 256-3072
- `text-embedding-ada-002`: fixed 1536 dimensions (dimensions param ignored)
- If `dimensions` is specified and not supported by the model, return
  `LangChainError::EmbeddingError`

### Batching

`embed_documents` splits input texts into batches of `chunk_size` and sends
them concurrently. Results are concatenated in input order.

## Error Handling

### OpenAI Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum OpenAIError {
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
    #[error("server error: {status} {body}")]
    ServerError { status: u16, body: String },  // HTTP 5xx
    #[error("request timed out")]
    TimeoutError,
    #[error("response parse error: {0}")]
    ParseError(String),                   // malformed JSON
    #[error(transparent)]
    HttpError(#[from] reqwest::Error),
}

impl From<OpenAIError> for LangChainError {
    fn from(e: OpenAIError) -> Self {
        match e {
            OpenAIError::ParseError(msg) => LangChainError::ParseError(msg),
            _ => LangChainError::ModelError(e.to_string()),
        }
    }
}
```

### Transient vs Permanent Error Classification

| HTTP Status | Error Type | Retryable? | Action |
|---|---|---|---|
| 400 | BadRequestError | No | Return error immediately |
| 401 | AuthenticationError | No | Return error immediately |
| 403 | PermissionError | No | Return error immediately |
| 404 | NotFoundError | No | Return error immediately |
| 429 | RateLimitError | Yes | Retry with backoff; respect Retry-After header |
| 500 | ServerError | Yes | Retry with backoff |
| 502 | ServerError | Yes | Retry with backoff |
| 503 | ServerError | Yes | Retry with backoff |
| 504 | ServerError | Yes | Retry with backoff |
| Timeout | TimeoutError | Yes | Retry with backoff |

### Response Validation

- All API responses are parsed as JSON. Malformed JSON returns
  `OpenAIError::ParseError` with the raw response body for debugging.
- Missing expected fields (e.g. `choices` array) return `ParseError`.
- The `error` field in the response body is parsed into the appropriate
  `OpenAIError` variant based on `type` and HTTP status.

## HTTP Resilience

### reqwest-middleware Configuration

```rust
// Applied to the reqwest::Client in ChatOpenAI and OpenAIEmbeddings
let retry_policy = ExponentialBackoff::builder()
    .retry_bounds(Duration::from_millis(500), Duration::from_secs(30))
    .build_with_max_retries(max_retries);

let client = ClientBuilder::new(reqwest::Client::new())
    .with(RetryTransientMiddleware::new_with_policy(retry_policy))
    .build();
```

### Timeout Configuration

- **Connect timeout**: 10 seconds (default)
- **Read timeout**: 60 seconds for non-streaming; 300 seconds for streaming
- Configurable via builder: `.timeout(Duration::from_secs(120))`
- Streaming requests use a longer timeout to accommodate slow generation

## Intentional Exclusions

- **`AzureChatOpenAI` / `AzureOpenAIEmbeddings`**: Azure-hosted OpenAI
  requires different authentication (Azure AD tokens, API versions,
  deployment names). This is a separate provider crate (`langchain-azure-openai`),
  not part of the initial `langchain-openai` crate.
- **`BaseOpenAI` / `OpenAI`** (legacy completions API): The `/v1/completions`
  endpoint is deprecated by OpenAI. All usage should migrate to
  `/v1/chat/completions` via `ChatOpenAI`. Not ported.
- **`OpenAIModerationMiddleware`**: Content moderation via OpenAI's
  `/v1/moderations` endpoint. Provided as a reference implementation in
  `langchain-openai::moderation` — wraps as a `RunnableLambda` that
  checks content before passing through. See contracts/traits.md
  §Reference Implementations for the API contract.

## Provider Scope

All 16 Python partner providers are in scope. See [providers.md](providers.md)
for contracts for all non-OpenAI providers.

### Provider Integration Guide

To create a new provider crate:
1. Add `langchain-core` as a dependency
2. Implement `BaseChatModel` for your chat model type (requires `Send + Sync + Debug`)
3. Implement `Embeddings` for your embeddings type (if applicable)
4. Define provider-specific error types with `From<YourError> for LangChainError`
5. Use builder pattern for configuration
6. Feature-gate integration tests behind `integration-tests`
7. See `langchain-openai` as the reference implementation

### Provider Testing Strategy

- **Unit tests**: Use `FakeChatModel` and `FakeEmbeddings` from `langchain-core`
  for testing chains without real API calls
- **Integration tests**: Feature-gated behind `integration-tests` feature flag
  ```toml
  [features]
  integration-tests = []
  ```
- **API key configuration**: Via environment variables (`OPENAI_API_KEY`)
  Read at runtime, not compile time. Integration tests skip with a warning
  if the key is not set.
- **Test scope**: Integration tests cover invoke, stream, batch, embeddings,
  and tool calling against the real API
