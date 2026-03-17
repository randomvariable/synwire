# synwire-llm-ollama: Ollama LLM Provider

`synwire-llm-ollama` provides `ChatOllama` and `OllamaEmbeddings`, connecting Synwire to a local (or remote) [Ollama](https://ollama.com) server for LLM inference and text embeddings without requiring cloud API keys.

> For a comparison of all LLM providers, see [LLM Providers](./synwire-llm-providers.md). This document focuses specifically on the Ollama implementation.

## Why Ollama?

Ollama wraps `llama.cpp` and other inference backends behind a simple HTTP API, handling model downloading, quantization selection, and GPU scheduling automatically. For Synwire users, this means:

- **Zero cloud dependencies.** Run agents entirely on local hardware.
- **Model flexibility.** Switch between Llama, Mistral, Gemma, Phi, and other model families by changing a string.
- **Privacy.** No data leaves the machine.

The trade-off is that local inference requires sufficient hardware (GPU recommended) and model quality depends on the chosen model and quantization level.

## `ChatOllama`

Implements `BaseChatModel` from `synwire-core`. Communicates with Ollama's `/api/chat` endpoint.

```rust,no_run
use synwire_llm_ollama::ChatOllama;

let model = ChatOllama::builder()
    .model("llama3.2")
    .base_url("http://localhost:11434")
    .temperature(0.7)
    .build()
    .unwrap();
```

### Builder options

| Option | Default | Description |
|---|---|---|
| `model` | `"llama3.2"` | Ollama model name |
| `base_url` | `"http://localhost:11434"` | Ollama server URL |
| `temperature` | `None` | Sampling temperature |
| `top_k` | `None` | Top-k sampling parameter |
| `top_p` | `None` | Nucleus sampling parameter |
| `num_predict` | `None` | Maximum tokens to generate |
| `timeout` | 120 seconds | Request timeout |
| `credential_provider` | `None` | Dynamic credential refresh (for authenticated Ollama proxies) |

### Streaming

`ChatOllama` supports both non-streaming (`invoke`) and streaming (`stream`) modes. Streaming uses Ollama's NDJSON (newline-delimited JSON) response format, where each line is a partial response object. The stream is parsed incrementally via `futures-util`.

### Tool calling

When tools are bound via `bind_tools`, `ChatOllama` includes tool definitions in the request payload. Ollama models that support function calling (e.g. Llama 3.2, Mistral) return `ToolCall` objects in the response, which the agent runtime can dispatch to registered tools.

## `OllamaEmbeddings`

Implements `Embeddings` from `synwire-core`. Communicates with Ollama's `/api/embed` endpoint.

```rust,no_run
use synwire_llm_ollama::OllamaEmbeddings;

let embeddings = OllamaEmbeddings::builder()
    .model("nomic-embed-text")
    .build()
    .unwrap();
```

Supports both `embed_documents` (batch) and `embed_query` (single document). The embedding model must be pulled separately in Ollama (`ollama pull nomic-embed-text`).

## Error handling

All errors are surfaced as `OllamaError`, which wraps:

- **HTTP errors** --- connection refused, timeout, non-2xx status
- **Deserialization errors** --- unexpected response format from the Ollama API
- **Configuration errors** --- invalid builder parameters

`OllamaError` converts to `SynwireError` via `From`, so it integrates cleanly with the rest of the Synwire error hierarchy.

## Dependencies

| Crate | Role |
|---|---|
| `synwire-core` | `BaseChatModel`, `Embeddings` traits (with `http` feature) |
| `reqwest` | HTTP client (rustls backend) |
| `futures-core` / `futures-util` | Stream processing for NDJSON responses |
| `serde` / `serde_json` | Request/response serialization |
| `thiserror` | Error type derivation |

## Ecosystem position

`synwire-llm-ollama` is a leaf crate --- nothing else in the workspace depends on it. It implements traits from `synwire-core` and is optionally re-exported by the `synwire` umbrella crate behind the `ollama` feature flag.

## See also

- [Local Inference with Ollama](../getting-started/ollama.md) --- getting started guide
- [LLM Providers](./synwire-llm-providers.md) --- comparison of all providers
- [Switch Provider](../how-to/switch-provider.md) --- how to swap between OpenAI and Ollama
