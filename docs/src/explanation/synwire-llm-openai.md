# synwire-llm-openai: OpenAI LLM Provider

`synwire-llm-openai` provides `ChatOpenAI`, `OpenAIEmbeddings`, and `OpenAIModerationMiddleware`, connecting Synwire to the OpenAI API (and any OpenAI-compatible endpoint) for chat completions, embeddings, and content moderation.

> For a comparison of all LLM providers, see [LLM Providers](./synwire-llm-providers.md). This document focuses specifically on the OpenAI implementation.

## Architecture: `BaseChatOpenAI`

The crate is structured around a shared base type, `BaseChatOpenAI`, which holds all configuration common to OpenAI-compatible providers: model name, API base URL, API key, temperature, max tokens, timeout, retry settings, and HTTP client. `ChatOpenAI` wraps this base and adds the `tools` vector for function calling.

This separation exists because several third-party services (Azure OpenAI, Together AI, Groq, local vLLM) expose OpenAI-compatible APIs. By extracting the common configuration into `BaseChatOpenAI`, future provider crates can reuse it without duplicating HTTP, retry, and credential logic.

## `ChatOpenAI`

Implements `BaseChatModel` from `synwire-core`.

```rust,no_run
use synwire_llm_openai::ChatOpenAI;

let model = ChatOpenAI::builder()
    .model("gpt-4o")
    .api_key("sk-...")
    .build()
    .unwrap();
```

### Builder options

| Option | Default | Description |
|---|---|---|
| `model` | `"gpt-4o"` | Model identifier |
| `api_key` | `""` | OpenAI API key (or set `OPENAI_API_KEY` env var) |
| `api_base` | `"https://api.openai.com/v1"` | API base URL (override for compatible endpoints) |
| `temperature` | `None` | Sampling temperature |
| `max_tokens` | `None` | Maximum tokens to generate |
| `top_p` | `None` | Nucleus sampling parameter |
| `stop` | `None` | Stop sequences |
| `timeout` | 30 seconds | Request timeout |
| `max_retries` | 3 | Automatic retries on transient errors |
| `model_kwargs` | `{}` | Additional JSON parameters passed through to the API |
| `credential_provider` | `None` | Dynamic credential refresh for rotating keys |

### Streaming

Streaming uses Server-Sent Events (SSE) via the `eventsource-stream` crate. Each SSE event is parsed into a `ChatChunk` and yielded through a `BoxStream`. The stream handles `[DONE]` sentinel events and partial tool call assembly across chunks.

### Tool calling

When tools are bound via `bind_tools`, `ChatOpenAI` includes `tools` and `tool_choice` in the API request. Tool call responses are parsed from the `tool_calls` array in the response message, with `ToolCallChunk` types handling the streaming case where a single tool call spans multiple SSE events.

### Retry middleware

The crate uses `reqwest-middleware` with `reqwest-retry` for automatic retries on transient HTTP errors (429, 500, 502, 503, 504). The retry policy respects `Retry-After` headers from the OpenAI API, which is important for rate limit compliance.

## `OpenAIEmbeddings`

Implements `Embeddings` from `synwire-core`.

```rust,no_run
use synwire_llm_openai::OpenAIEmbeddings;

let embeddings = OpenAIEmbeddings::builder()
    .model("text-embedding-3-small")
    .api_key("sk-...")
    .build()
    .unwrap();
```

Supports both `embed_documents` (batch) and `embed_query` (single). Batching is handled transparently by the OpenAI API.

## `OpenAIModerationMiddleware`

A `RunnableCore` implementation that checks input text against the OpenAI Moderation API before passing it downstream. Rejects content flagged as harmful, preventing it from reaching the chat model.

```rust,no_run
use synwire_llm_openai::moderation::OpenAIModerationMiddleware;

let middleware = OpenAIModerationMiddleware::new(
    "https://api.openai.com/v1",
    "sk-...",
);
```

## Error handling

`OpenAIError` covers HTTP errors, deserialization failures, rate limits, authentication failures, and configuration errors. It converts to `SynwireError` via `From`.

## Dependencies

| Crate | Role |
|---|---|
| `synwire-core` | `BaseChatModel`, `Embeddings`, `RunnableCore` traits (with `http` feature) |
| `reqwest` | HTTP client (rustls backend) |
| `reqwest-middleware` / `reqwest-retry` | Automatic retry on transient errors |
| `eventsource-stream` | SSE parsing for streaming responses |
| `futures-core` / `futures-util` | Stream processing |
| `serde` / `serde_json` | Request/response serialization |
| `thiserror` | Error type derivation |

## Ecosystem position

`synwire-llm-openai` is a leaf crate. It implements traits from `synwire-core` and is optionally re-exported by the `synwire` umbrella crate behind the `openai` feature flag. The `BaseChatOpenAI` base type is designed for reuse by future OpenAI-compatible provider crates.

## See also

- [First Chat](../getting-started/first-chat.md) --- getting started with OpenAI
- [LLM Providers](./synwire-llm-providers.md) --- comparison of all providers
- [Credentials](../how-to/credentials.md) --- credential management
- [Retry and Fallback](../how-to/retry-fallback.md) --- retry configuration
