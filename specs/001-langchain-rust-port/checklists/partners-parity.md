# Partners Parity Checklist: LangChain Rust Port

**Purpose**: Validate that spec, contracts, and data model adequately document parity with Python `langchain-partners` provider integrations
**Created**: 2026-03-09
**Updated**: 2026-03-09 (scope expanded — all 16 Python partner providers now in scope)
**Feature**: [spec.md](../spec.md) | [contracts/traits.md](../contracts/traits.md) | [contracts/openai.md](../contracts/openai.md) | [contracts/providers.md](../contracts/providers.md)
**Depth**: Rigorous | **Scope**: Provider integration API surface for all 16 partners
**Audience**: Reviewer (PR) | **Timing**: Pre-implementation gate
**Source**: `/langchain/libs/partners/`

## OpenAI Provider — Chat Models

- [x] CHK170 Is `ChatOpenAI` documented with its full configuration surface (model, temperature, max_tokens, top_p, frequency_penalty, presence_penalty, stop, seed, response_format, streaming)? [Completeness, Contracts §openai.md]
- [x] CHK171 Is `AzureChatOpenAI` (Azure-hosted OpenAI) documented as excluded with rationale? Python has a distinct class with azure_endpoint, api_version, azure_deployment parameters [Completeness, Contracts §openai.md §Intentional Exclusions]
- [x] CHK172 Are OpenAI-specific configuration parameters (`organization`, `api_base`, `api_key`, `timeout`, `max_retries`) documented for the Rust `ChatOpenAI` implementation? [Completeness, Contracts §openai.md]
- [x] CHK173 Is OpenAI function-calling / tool-use support specified for `ChatOpenAI`? Must implement `bind_tools` from `BaseChatModel` trait — are provider-specific details documented? [Completeness, Contracts §openai.md §Tool Calling]
- [x] CHK174 Is OpenAI structured output mode (`response_format: { type: "json_schema" }`) documented for the `with_structured_output` implementation? [Completeness, Contracts §openai.md §Structured Output]
- [x] CHK175 Is OpenAI streaming with tool calls specified? The SSE stream must accumulate `ToolCallChunk` fragments — is this provider-specific logic documented? [Clarity, Contracts §openai.md §Streaming with Tool Calls]

## OpenAI Provider — Embeddings

- [x] CHK176 Is `OpenAIEmbeddings` documented with its configuration (model, dimensions, encoding_format, chunk_size)? [Completeness, Contracts §openai.md §OpenAIEmbeddings]
- [x] CHK177 Is `AzureOpenAIEmbeddings` documented as excluded with rationale? [Completeness, Contracts §openai.md §Intentional Exclusions]
- [x] CHK178 Is embedding model dimension validation (e.g. text-embedding-3-small returns 1536 by default, configurable) documented? [Clarity, Contracts §openai.md §Dimension Validation]

## OpenAI Provider — LLM (Completions)

- [x] CHK179 Is `BaseOpenAI` / `OpenAI` (legacy completions API) documented as excluded? The completions endpoint is deprecated in favour of chat — is this decision documented? [Completeness, Contracts §openai.md §Intentional Exclusions]

## OpenAI Provider — Error Handling

- [x] CHK180 Is the mapping from OpenAI HTTP errors (401 Unauthorized, 429 Rate Limited, 500 Internal Server Error) to `LangChainError` variants documented? [Completeness, Contracts §openai.md §Error Handling]
- [x] CHK181 Is OpenAI API response validation (malformed JSON, unexpected schema) documented as mapping to `OpenAIError::ParseError`? [Clarity, Contracts §openai.md §Response Validation]
- [x] CHK182 Are transient vs permanent OpenAI error classifications documented for retry behaviour? HTTP 429 and 5xx are retryable; 400/401/403 are not [Completeness, Contracts §openai.md §Transient vs Permanent]

## OpenAI Provider — HTTP Resilience

- [x] CHK183 Is the `reqwest-retry` + `reqwest-middleware` integration documented with specific configuration (retry count, backoff policy, retryable status codes)? [Clarity, Contracts §openai.md §reqwest-middleware Configuration]
- [x] CHK184 Is request timeout configuration documented? Both connect timeout and read timeout for streaming [Completeness, Contracts §openai.md §Timeout Configuration]

## OpenAI Provider — Moderation

- [x] CHK185 Is `OpenAIModerationMiddleware` documented as a reference implementation in the `langchain` crate? [Completeness, Contracts §openai.md §Intentional Exclusions]

## Provider Scope — Overall

- [x] CHK186 Is the scope boundary clearly stated as "all 16 Python partner providers"? [Completeness, Spec §Assumptions + §FR-028]
- [x] CHK187 Are all 16 Python partner packages documented with contracts? Anthropic, Chroma, Qdrant, HuggingFace, Ollama, MistralAI, Fireworks, Groq, Nomic, Exa, DeepSeek, XAI, OpenRouter, Perplexity (+ OpenAI) [Completeness, Contracts §providers.md]
- [x] CHK188 Is a provider integration guide or template documented for third-party crate authors? The design should show how to implement `BaseChatModel` and `Embeddings` for a new provider [Completeness, Contracts §openai.md §Provider Integration Guide]

## OpenAI-Compatible Providers (BaseChatOpenAI pattern)

- [x] CHK199 Is `BaseChatOpenAI` documented as a shared base type for OpenAI-compatible providers? Must include `api_base`, `api_key`, `model`, and all shared parameters [Completeness, Contracts §providers.md §BaseChatOpenAI]
- [x] CHK200 Is `ChatGroq` documented as a thin wrapper around `BaseChatOpenAI` with Groq-specific `api_base` and `api_key` env var (`GROQ_API_KEY`)? [Completeness, Contracts §providers.md §Groq]
- [x] CHK201 Is `ChatFireworks` documented as a thin wrapper with `FIREWORKS_API_KEY` and Fireworks-specific model names? [Completeness, Contracts §providers.md §Fireworks]
- [x] CHK202 Is `ChatDeepSeek` documented with `DEEPSEEK_API_KEY` and DeepSeek model names (deepseek-chat, deepseek-coder)? [Completeness, Contracts §providers.md §DeepSeek]
- [x] CHK203 Is `ChatXAI` documented with `XAI_API_KEY` and xAI model names (grok-2, grok-2-mini)? [Completeness, Contracts §providers.md §xAI]
- [x] CHK204 Is `ChatOpenRouter` documented with `OPENROUTER_API_KEY`, HTTP-Referer, and X-Title headers? [Completeness, Contracts §providers.md §OpenRouter]

## OpenAI-Partial Providers

- [x] CHK205 Is `ChatMistralAI` documented with Mistral-specific differences from OpenAI API (tool format, safe_prompt parameter)? [Completeness, Contracts §providers.md §MistralAI]
- [x] CHK206 Is `MistralAIEmbeddings` documented with Mistral embedding models and configuration? [Completeness, Contracts §providers.md §MistralAI]
- [x] CHK207 Is `ChatPerplexity` documented with `search_domain_filter`, `search_recency_filter`, and `citations` parsing? [Completeness, Contracts §providers.md §Perplexity]
- [x] CHK208 Is `PerplexitySearchRetriever` documented implementing the `Retriever` trait? [Completeness, Contracts §providers.md §Perplexity]

## Native API Providers — Anthropic

- [x] CHK209 Is `ChatAnthropic` documented with its full configuration surface (model, max_tokens, temperature, top_p, top_k, system, stop_sequences)? [Completeness, Contracts §providers.md §Anthropic]
- [x] CHK210 Is the Anthropic message format mapping (LangChain Message ↔ Anthropic message) documented? System prompt is a top-level parameter, not a message [Clarity, Contracts §providers.md §Anthropic Message Format]
- [x] CHK211 Is `AnthropicError` enum documented with Anthropic-specific HTTP status mappings (529 = overloaded)? [Completeness, Contracts §providers.md §Anthropic Error]
- [x] CHK212 Is Anthropic streaming documented as SSE with `content_block_delta` events (different from OpenAI chunk format)? [Clarity, Contracts §providers.md §Anthropic]
- [x] CHK213 Is Anthropic tool use documented with `tool_use` content blocks and `tool_result` response format? [Completeness, Contracts §providers.md §Anthropic]

## Native API Providers — Ollama

- [x] CHK214 Is `ChatOllama` documented with its configuration (model, base_url defaulting to localhost:11434, no API key)? [Completeness, Contracts §providers.md §Ollama]
- [x] CHK215 Is `OllamaLLM` documented as implementing `BaseLLM` for the `/api/generate` endpoint? [Completeness, Contracts §providers.md §Ollama]
- [x] CHK216 Is `OllamaEmbeddings` documented implementing `Embeddings` for the `/api/embed` endpoint? [Completeness, Contracts §providers.md §Ollama]
- [x] CHK217 Is Ollama NDJSON streaming (not SSE) documented? Each line is a JSON object with `done: bool` [Clarity, Contracts §providers.md §Ollama]
- [x] CHK218 Is the Ollama no-auto-pull decision documented? Models must be pre-installed [Completeness, Research §20]

## Native API Providers — HuggingFace

- [x] CHK219 Is `ChatHuggingFace` documented with Inference API configuration (model, api_key, task)? [Completeness, Contracts §providers.md §HuggingFace]
- [x] CHK220 Is `HuggingFaceEmbeddings` documented with model configuration and batching? [Completeness, Contracts §providers.md §HuggingFace]
- [x] CHK221 Is `HuggingFacePipeline` documented as implementing `BaseLLM` for local pipeline inference? [Completeness, Contracts §providers.md §HuggingFace]
- [x] CHK222 Is the HF local inference exclusion (candle/ort) documented with rationale? API-only initially [Completeness, Contracts §providers.md §HuggingFace Intentional Exclusions]

## Vector Store Providers

- [x] CHK189 Is `InMemoryVectorStore` documented as the core reference implementation? [Completeness, Contracts §VectorStore]
- [x] CHK190 Are `Chroma` and `Qdrant` documented as in-scope vector store provider crates? [Completeness, Contracts §providers.md §Vector Store Providers]
- [x] CHK191 Does `InMemoryVectorStore` implement all `VectorStore` trait methods including MMR? Is this specified? [Completeness, Contracts §VectorStore]
- [x] CHK192 Is `SparseEmbeddings` documented as excluded with rationale (Qdrant-specific, niche use case)? [Completeness, Contracts §providers.md §Intentional Exclusions]
- [x] CHK223 Is `Chroma` documented with REST-only client configuration (host, port, collection_name, embedding_function)? [Completeness, Contracts §providers.md §Chroma]
- [x] CHK224 Is `QdrantVectorStore` documented with REST default + optional gRPC (via tonic) configuration? [Completeness, Contracts §providers.md §Qdrant]
- [x] CHK225 Are Pinecone, Weaviate, and Milvus documented as excluded (not Python partner packages)? [Completeness, Spec §Assumptions]

## Specialized Providers

- [x] CHK226 Is `NomicEmbeddings` documented with Nomic API configuration and model selection? [Completeness, Contracts §providers.md §Nomic]
- [x] CHK227 Is Nomic local inference exclusion documented with rationale? [Completeness, Contracts §providers.md §Nomic Intentional Exclusions]
- [x] CHK228 Is `ExaSearchRetriever` documented implementing `Retriever` with `ExaSearchResults` return type? [Completeness, Contracts §providers.md §Exa]
- [x] CHK229 Is Exa documented with both `search` and `find_similar` modes? [Completeness, Contracts §providers.md §Exa]

## Provider Architecture

- [x] CHK230 Is the provider categorisation (OpenAI-native, OpenAI-compatible, OpenAI-partial, Native API, Vector Store, Specialized) documented? [Completeness, Research §20]
- [x] CHK231 Is each provider category's shared base type or pattern documented (e.g. BaseChatOpenAI for compatible providers)? [Clarity, Contracts §providers.md §Provider Architecture]
- [x] CHK232 Are provider-specific error types and their mapping to `LangChainError` documented for each native API provider (Anthropic, Ollama, HuggingFace)? [Completeness, Contracts §providers.md]

## Provider Testing Strategy

- [x] CHK193 Are integration tests documented with feature-gating strategy (`integration-tests` feature flag) for all provider crates? [Completeness, Contracts §openai.md §Provider Testing Strategy]
- [x] CHK194 Are mock/fake provider implementations documented for unit testing? `FakeChatModel` and `FakeEmbeddings` from langchain-core [Completeness, Contracts §openai.md §Provider Testing Strategy]
- [x] CHK195 Is API key configuration documented for integration tests (environment variables per provider)? [Completeness, Contracts §openai.md §Provider Testing Strategy]
- [x] CHK233 Is the test scope for integration tests documented (invoke, stream, batch, embeddings, tool calling)? [Completeness, Contracts §openai.md §Provider Testing Strategy]

## Provider Trait Compliance

- [x] CHK196 Is the requirement that `ChatOpenAI` implements both `BaseChatModel` AND `Runnable<Vec<Message>, ChatResult>` documented? [Clarity, Contracts §openai.md §Trait Implementations]
- [x] CHK197 Is the requirement that `OpenAIEmbeddings` implements `Embeddings` with specific output dimensions documented? [Clarity, Contracts §openai.md §Dimension Validation]
- [x] CHK198 Is the requirement that all provider types implement `Send + Sync + Debug` documented as a provider author obligation? [Completeness, Contracts §providers.md §Common Patterns]
- [x] CHK234 Is the trait compliance requirement documented for all non-OpenAI providers? Each chat model must implement `BaseChatModel`, each embedding model `Embeddings`, each vector store `VectorStore`, each retriever `Retriever` [Completeness, Contracts §providers.md §Trait Compliance]

## Provider Implementation Tasks

- [x] CHK235 Are implementation tasks documented for all 16 provider crates in tasks.md Phase 12? [Completeness, tasks.md §Phase 12]
- [x] CHK236 Are provider task dependencies documented (BaseChatOpenAI refactor before compatible wrappers, Phase 3+6 before providers)? [Completeness, tasks.md §Dependencies]
- [x] CHK237 Are workspace integration tasks documented (Cargo.toml member addition, langchain re-export crate updates)? [Completeness, tasks.md T225-T227]

## Notes

- Check items off as completed: `[x]`
- Items marked [Gap] indicate missing requirements that need spec/contract updates
- Items marked [Clarity] indicate existing requirements needing more precision
- Items marked [Completeness] indicate partially specified requirements
- All 16 Python partner providers are in scope (expanded from OpenAI-only)
- New items CHK199-CHK237 added to cover expanded provider scope
- Reference: Python API audited from `/langchain/libs/partners/`
