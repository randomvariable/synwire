# Partners Parity Checklist: Synwire Port

**Purpose**: Validate that spec, contracts, and data model adequately document parity with the in-scope provider integrations
**Created**: 2026-03-09
**Updated**: 2026-03-09 (scope revised — focused provider set for Rust port)
**Feature**: [spec.md](../spec.md) | [contracts/traits.md](../contracts/traits.md) | [contracts/openai.md](../contracts/openai.md) | [contracts/providers.md](../contracts/providers.md)
**Depth**: Rigorous | **Scope**: Provider integration API surface for in-scope providers
**Audience**: Reviewer (PR) | **Timing**: Pre-implementation gate
**Source**: Provider API documentation

## synwire-llm-openai — Chat Models

- [x] CHK170 Is `ChatOpenAI` documented with its full configuration surface (model, temperature, max_tokens, top_p, frequency_penalty, presence_penalty, stop, seed, response_format, streaming)? [Completeness, Contracts §openai.md]
- [x] CHK171 Is `AzureChatOpenAI` (Azure-hosted OpenAI) documented as excluded with rationale? Python has a distinct class with azure_endpoint, api_version, azure_deployment parameters [Completeness, Contracts §openai.md §Intentional Exclusions]
- [x] CHK172 Are OpenAI-specific configuration parameters (`organization`, `api_base`, `api_key`, `timeout`, `max_retries`) documented for the Rust `ChatOpenAI` implementation? [Completeness, Contracts §openai.md]
- [x] CHK173 Is OpenAI function-calling / tool-use support specified for `ChatOpenAI`? Must implement `bind_tools` from `BaseChatModel` trait — are provider-specific details documented? [Completeness, Contracts §openai.md §Tool Calling]
- [x] CHK174 Is OpenAI structured output mode (`response_format: { type: "json_schema" }`) documented for the `with_structured_output` implementation? [Completeness, Contracts §openai.md §Structured Output]
- [x] CHK175 Is OpenAI streaming with tool calls specified? The SSE stream must accumulate `ToolCallChunk` fragments — is this provider-specific logic documented? [Clarity, Contracts §openai.md §Streaming with Tool Calls]

## synwire-llm-openai — Embeddings

- [x] CHK176 Is `OpenAIEmbeddings` documented with its configuration (model, dimensions, encoding_format, chunk_size)? [Completeness, Contracts §openai.md §OpenAIEmbeddings]
- [x] CHK177 Is `AzureOpenAIEmbeddings` documented as excluded with rationale? [Completeness, Contracts §openai.md §Intentional Exclusions]
- [x] CHK178 Is embedding model dimension validation (e.g. text-embedding-3-small returns 1536 by default, configurable) documented? [Clarity, Contracts §openai.md §Dimension Validation]

## synwire-llm-openai — LLM (Completions)

- [x] CHK179 Is `BaseOpenAI` / `OpenAI` (legacy completions API) documented as excluded? The completions endpoint is deprecated in favour of chat — is this decision documented? [Completeness, Contracts §openai.md §Intentional Exclusions]

## synwire-llm-openai — Error Handling

- [x] CHK180 Is the mapping from OpenAI HTTP errors (401 Unauthorized, 429 Rate Limited, 500 Internal Server Error) to `SynwireError` variants documented? [Completeness, Contracts §openai.md §Error Handling]
- [x] CHK181 Is OpenAI API response validation (malformed JSON, unexpected schema) documented as mapping to `OpenAIError::ParseError`? [Clarity, Contracts §openai.md §Response Validation]
- [x] CHK182 Are transient vs permanent OpenAI error classifications documented for retry behaviour? HTTP 429 and 5xx are retryable; 400/401/403 are not [Completeness, Contracts §openai.md §Transient vs Permanent]

## synwire-llm-openai — HTTP Resilience

- [x] CHK183 Is the `reqwest-retry` + `reqwest-middleware` integration documented with specific configuration (retry count, backoff policy, retryable status codes)? [Clarity, Contracts §openai.md §reqwest-middleware Configuration]
- [x] CHK184 Is request timeout configuration documented? Both connect timeout and read timeout for streaming [Completeness, Contracts §openai.md §Timeout Configuration]

## synwire-llm-openai — Moderation

- [x] CHK185 Is `OpenAIModerationMiddleware` documented as a reference implementation in the `synwire` crate? [Completeness, Contracts §openai.md §Intentional Exclusions]

## synwire-llm-ollama

- [x] CHK214 Is `ChatOllama` documented with its configuration (model, base_url defaulting to localhost:11434, no API key)? [Completeness, Contracts §providers.md §Ollama]
- [x] CHK215 Is `OllamaLLM` documented as implementing `BaseLLM` for the `/api/generate` endpoint? [Completeness, Contracts §providers.md §Ollama]
- [x] CHK216 Is `OllamaEmbeddings` documented implementing `Embeddings` for the `/api/embed` endpoint? [Completeness, Contracts §providers.md §Ollama]
- [x] CHK217 Is Ollama NDJSON streaming (not SSE) documented? Each line is a JSON object with `done: bool` [Clarity, Contracts §providers.md §Ollama]
- [x] CHK218 Is the Ollama no-auto-pull decision documented? Models must be pre-installed [Completeness, Research §20]

## synwire-vectorstore-qdrant

- [x] CHK224 Is `QdrantVectorStore` documented with REST default + optional gRPC (via tonic) configuration? [Completeness, Contracts §providers.md §Qdrant]
- [x] CHK192 Is `SparseEmbeddings` documented as excluded with rationale (Qdrant-specific, niche use case)? [Completeness, Contracts §providers.md §Intentional Exclusions]

## synwire-vectorstore-pgvector

- [ ] CHK238 Is `PgVectorStore` documented with connection configuration (connection string, pool size, table name, embedding dimensions)? [Completeness, Contracts §providers.md §PgVector]
- [ ] CHK239 Is the pgvector extension requirement (CREATE EXTENSION vector) documented? [Completeness, Contracts §providers.md §PgVector]
- [ ] CHK240 Is the `sqlx` dependency for PostgreSQL connectivity documented? [Completeness, Contracts §providers.md §PgVector]

## synwire-graphstore-neo4j

- [ ] CHK241 Is `Neo4jGraphStore` documented with connection configuration (URI, auth, database name)? [Completeness, Contracts §providers.md §Neo4j]
- [ ] CHK242 Is the Cypher query interface for graph retrieval documented? [Completeness, Contracts §providers.md §Neo4j]
- [ ] CHK243 Is the `neo4rs` Rust driver dependency documented? [Completeness, Contracts §providers.md §Neo4j]

## synwire-search-serpapi

- [ ] CHK244 Is `SerpApiSearchRetriever` documented implementing `Retriever` with SerpApi configuration (api_key, engine, params)? [Completeness, Contracts §providers.md §SerpApi]
- [ ] CHK245 Is the SerpApi search result to `Document` mapping documented? [Completeness, Contracts §providers.md §SerpApi]

## synwire-search-searxng

- [ ] CHK246 Is `SearxNGSearchRetriever` documented with self-hosted SearxNG instance configuration (base_url, categories, engines)? [Completeness, Contracts §providers.md §SearxNG]
- [ ] CHK247 Is the SearxNG JSON API response to `Document` mapping documented? [Completeness, Contracts §providers.md §SearxNG]

## synwire-search-ncbi

- [ ] CHK248 Is `NCBISearchRetriever` documented for PubMed/NCBI E-utilities search with API key and database selection? [Completeness, Contracts §providers.md §NCBI]

## synwire-search-arxiv

- [ ] CHK249 Is `ArxivSearchRetriever` documented for arXiv API search with query parameters and result parsing? [Completeness, Contracts §providers.md §Arxiv]

## synwire-workflow-temporal

- [ ] CHK250 Is the Temporal workflow integration documented with activity and workflow type definitions? [Completeness, Contracts §providers.md §Temporal]
- [ ] CHK251 Is the `temporal-sdk` Rust dependency documented? [Completeness, Contracts §providers.md §Temporal]

## Vector Store — Core

- [x] CHK189 Is `InMemoryVectorStore` documented as the core reference implementation? [Completeness, Contracts §VectorStore]
- [x] CHK191 Does `InMemoryVectorStore` implement all `VectorStore` trait methods including MMR? Is this specified? [Completeness, Contracts §VectorStore]

## Provider Scope — Overall

- [x] CHK186 Is the scope boundary clearly stated for in-scope providers? [Completeness, Spec §Assumptions + §FR-028]
- [x] CHK187 Are all in-scope providers documented with contracts? synwire-llm-openai, synwire-llm-ollama, synwire-vectorstore-qdrant, synwire-vectorstore-pgvector, synwire-graphstore-neo4j, synwire-search-serpapi, synwire-search-searxng, synwire-search-ncbi, synwire-search-arxiv, synwire-workflow-temporal [Completeness, Contracts §providers.md]
- [x] CHK188 Is a provider integration guide or template documented for third-party crate authors? The design should show how to implement `BaseChatModel` and `Embeddings` for a new provider [Completeness, Contracts §openai.md §Provider Integration Guide]

## Provider Architecture

- [x] CHK230 Is the provider categorisation (LLM, Vector Store, Graph Store, Search, Workflow) documented? [Completeness, Research §20]
- [x] CHK231 Is each provider category's shared base type or pattern documented (e.g. BaseChatOpenAI for OpenAI-compatible providers)? [Clarity, Contracts §providers.md §Provider Architecture]
- [x] CHK232 Are provider-specific error types and their mapping to `SynwireError` documented for each provider? [Completeness, Contracts §providers.md]

## Provider Testing Strategy

- [x] CHK193 Are integration tests documented with feature-gating strategy (`integration-tests` feature flag) for all provider crates? [Completeness, Contracts §openai.md §Provider Testing Strategy]
- [x] CHK194 Are mock/fake provider implementations documented for unit testing? `FakeChatModel` and `FakeEmbeddings` from synwire-core [Completeness, Contracts §openai.md §Provider Testing Strategy]
- [x] CHK195 Is API key configuration documented for integration tests (environment variables per provider)? [Completeness, Contracts §openai.md §Provider Testing Strategy]
- [x] CHK233 Is the test scope for integration tests documented (invoke, stream, batch, embeddings, tool calling)? [Completeness, Contracts §openai.md §Provider Testing Strategy]

## Provider Trait Compliance

- [x] CHK196 Is the requirement that `ChatOpenAI` implements both `BaseChatModel` AND `Runnable<Vec<Message>, ChatResult>` documented? [Clarity, Contracts §openai.md §Trait Implementations]
- [x] CHK197 Is the requirement that `OpenAIEmbeddings` implements `Embeddings` with specific output dimensions documented? [Clarity, Contracts §openai.md §Dimension Validation]
- [x] CHK198 Is the requirement that all provider types implement `Send + Sync + Debug` documented as a provider author obligation? [Completeness, Contracts §providers.md §Common Patterns]
- [x] CHK234 Is the trait compliance requirement documented for all providers? Each chat model must implement `BaseChatModel`, each embedding model `Embeddings`, each vector store `VectorStore`, each retriever `Retriever` [Completeness, Contracts §providers.md §Trait Compliance]

## Provider Implementation Tasks

- [x] CHK235 Are implementation tasks documented for all provider crates? [Completeness, tasks.md]
- [x] CHK236 Are provider task dependencies documented (BaseChatOpenAI refactor before compatible wrappers, Phase 3+6 before providers)? [Completeness, tasks.md §Dependencies]
- [x] CHK237 Are workspace integration tasks documented (Cargo.toml member addition, synwire re-export crate updates)? [Completeness, tasks.md]

## Notes

- Check items off as completed: `[x]`
- Items marked [Gap] indicate missing requirements that need spec/contract updates
- Items marked [Clarity] indicate existing requirements needing more precision
- Items marked [Completeness] indicate partially specified requirements
- In-scope providers: synwire-llm-openai, synwire-llm-ollama, synwire-vectorstore-qdrant, synwire-vectorstore-pgvector, synwire-graphstore-neo4j, synwire-search-serpapi, synwire-search-searxng, synwire-search-ncbi, synwire-search-arxiv, synwire-workflow-temporal
- New items CHK238-CHK251 added for new provider scope (pgvector, neo4j, search providers, temporal)
