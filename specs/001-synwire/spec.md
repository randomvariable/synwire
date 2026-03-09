# Synwire Specification -- M1: Core + Orchestrator

**Feature Branch**: `001-synwire`
**Created**: 2026-03-09
**Status**: Draft
**Milestone**: M1 (Core + Orchestrator + Providers + Derive)

## Overview

Synwire is a Rust port of LangChain Python and LangGraph Python into a unified Cargo workspace. This specification covers **M1 only**: the core abstractions, graph orchestration engine, SQLite checkpointing, OpenAI and Ollama provider integrations, and the `synwire-derive` proc-macro crate.

### M1 Crates

| Crate | Purpose |
|-------|---------|
| `synwire-core` | Foundational traits, types, errors |
| `synwire-orchestrator` | StateGraph, Pregel, channels, checkpointing traits |
| `synwire-checkpoint` | Checkpoint traits, serialisation, in-memory store |
| `synwire-checkpoint-sqlite` | SQLite checkpoint saver |
| `synwire-llm-openai` | OpenAI ChatModel + Embeddings |
| `synwire-llm-ollama` | Ollama ChatModel + Embeddings |
| `synwire` | Convenience crate re-exporting common types |
| `synwire-derive` | Proc macros (`#[tool]`, `#[derive(State)]`) |
| `synwire-test-utils` | Shared test utilities, proptest strategies, fakes |
| `synwire-checkpoint-conformance` | Checkpoint backend conformance test suite |

### Roadmap

M2 (Agents + MCP) and M3 (Protocols + DSPy + Evals) are out of scope for this document. See [roadmap](../../docs/roadmap.md) for M2/M3 scope.

---

## 1. Foundational Requirements

Foundational requirements define vocabulary, developer experience, observability infrastructure, testing strategy, and documentation standards. They are placed first because all subsequent FRs depend on them.

### 1.1 Terminology and Naming

- **FR-001**: The spec documentation MUST include a "Terminology Glossary" distinguishing:
  **Runnable** = stateless composable data transformer (invoke, batch, stream).
  **CallbackHandler** = observability-only event listener with no control flow.
  **EventBus** = typed pub/sub system for decoupled analytics and metrics (feature-gated).
  **Provider** = a crate implementing `ChatModel`, `Embeddings`, or `VectorStore` for a specific backend (OpenAI, Ollama, etc.).
  **Channel** = a state management primitive within a graph that accumulates updates and applies reducers.
  **Checkpoint** = a serialised snapshot of graph state at a point in execution.
  **State** = typed graph state struct implementing the `State` trait.
  **Node** = a function or closure registered in a `StateGraph` that receives state and returns a state update.

- **FR-002**: The spec MUST document that `Agent<D,O>` and `AgentExecutor` are M2 concepts. For M1, only `create_react_agent` exists as the prebuilt agent entry point. The term "agent" in M1 refers to a `CompiledGraph` built via `create_react_agent` or manual `StateGraph` construction.

- **FR-003**: The spec MUST document that `RunnableConfig` propagates configuration (callbacks, metadata, tags, tenant_id) through the execution chain. All execution entry points (`invoke`, `batch`, `stream`) accept `&RunnableConfig`.

- **FR-004**: The spec MUST document the interop between `OutputMode<T>` and `TypedValue`: when a graph node produces a structured result, the value is wrapped as `TypedValue::Json(serde_json::to_value(&result))` before being written to graph state. When retrieved, `TypedValue::Json` can be deserialized back to `T`.

- **FR-005**: The spec MUST document the relationship between `RetryPolicy` (per-node graph-level retry with backoff), `RunnableRetry` (per-runnable chain-level retry via `with_retry()`), and `ToolResult::Retry` (tool-initiated retry requesting model self-correction). They are composable: a tool returning `Retry` triggers the model to re-invoke; if the model's retry also fails, `RetryPolicy` (if configured) retries the entire node.

- **FR-006**: The library documentation MUST include a decision tree for the callback and hook systems, clarifying when to use each:
  (1) `CallbackHandler` -- chain/model/tool observability. Passive, read-only.
  (2) `EventBus` -- decoupled pub/sub for analytics, metrics, external systems.
  M2 will add agent-level callbacks, plugins, and MCP callbacks. The decision tree MUST be extensible.

- **FR-007**: The spec MUST use a single canonical retry abstraction in M1. `RetryPolicy` is the graph-level retry. `with_retry()` is the chain-level retry. One canonical output parsing approach: `OutputParser<T>` for template-based flows. Overlapping abstractions from Python MUST be reduced to one canonical form per concept in Rust.

### 1.2 Developer Experience

- **FR-008**: `CompiledGraph` MUST provide a `to_mermaid() -> String` method generating a Mermaid diagram of the graph structure including node names, edge labels, and conditional routing.

- **FR-009**: All error types MUST include actionable error messages. When a prompt template references a missing variable, the error MUST name the variable. When a provider returns malformed JSON, the error MUST include the raw response body. When `Model::from_str` receives an unknown provider prefix, the error MUST list known providers.

- **FR-010**: `synwire-derive` MUST be an optional dependency for `synwire-core`. Core traits MUST be usable without proc macros. Gate test utilities behind a `test-utils` feature flag.

### 1.3 Observability Architecture

All observability FRs are consolidated here. The observability stack has three layers in M1:
1. **CallbackHandler** -- per-chain event listeners propagated via `RunnableConfig`
2. **EventBus** -- global typed pub/sub (feature-gated behind `event-bus`)
3. **`tracing` crate integration** -- OTel spans and metrics (feature-gated behind `tracing`)

#### 1.3.1 Core Observability

- **FR-011**: When the `tracing` feature is enabled, all tracing spans MUST include structured context fields: `tenant_id` (from RunnableConfig), `run_id`, `graph_id` (if in a graph), `node_id` (if in a node), `tool_name` (if in a tool call). These enable filtering and correlation in observability backends.

- **FR-012**: Per-node execution metrics MUST be emitted as tracing span attributes: `node.duration_ms`, `node.input_tokens`, `node.output_tokens`, `node.retries`, `node.status`.

- **FR-013**: Library MUST define an `EventBus` type in `synwire-core` with `subscribe<E: Event>(listener: Box<dyn EventListener<E>>)` and `publish<E: Event>(event: E)` methods. Typed events include: `ModelCallEvent`, `ToolCallEvent`. The event bus is optional (feature-gated) and complements `CallbackHandler`.

- **FR-014**: Each `EventBus` event type MUST be specified with its payload fields:
  `ModelCallEvent { model_id, provider, run_id, input_messages, output_message, usage, duration, cost, timestamp }`,
  `ToolCallEvent { tool_name, tool_call_id, arguments, result, duration, timestamp }`.

- **FR-015**: The `EventBus` and `CallbackHandler` are independent, complementary systems. Both fire for the same underlying operations. `CallbackHandler` is per-chain (propagated via `RunnableConfig`); `EventBus` is global (feature-gated singleton). Users may use one or both.

#### 1.3.2 Tracing Configuration

- **FR-016**: When the `tracing` feature is enabled, all tracing spans MUST respect a `trace_include_sensitive_data: bool` configuration (default `false`). When `false`, LLM input/output content, tool arguments, and tool results MUST be omitted from span attributes. Span structure (timing, node names, error types) MUST still be recorded.

  **Architecture review fix (P2 3.7)**: Default is `false`, not `true`.

- **FR-017**: The `trace_include_sensitive_data: bool` MUST be augmented with a `TraceContentFilter` configuration providing per-attribute granularity: `include_input_messages: bool`, `include_output_messages: bool`, `include_system_instructions: bool`, `include_tool_arguments: bool`, `include_tool_results: bool`, `include_retrieval_queries: bool`, `max_content_length: Option<usize>` (truncation limit, default None). When `trace_include_sensitive_data` is false, ALL content attributes are omitted regardless of per-attribute settings.

- **FR-018**: `SecretValue` instances MUST be automatically redacted in tracing spans. When a `SecretValue` appears in span attributes, it MUST be serialised as `"***"`.

#### 1.3.3 OTel GenAI Semantic Conventions

- **FR-019**: When the `tracing` feature is enabled, all LLM invocation spans MUST include OTel GenAI semantic convention attributes: `gen_ai.operation.name` (required, value: `chat` | `text_completion` | `embeddings` | `retrieval` | `execute_tool`), `gen_ai.provider.name` (required), `gen_ai.request.model` (conditionally required), `gen_ai.response.model` (recommended). Span names MUST follow `{gen_ai.operation.name} {gen_ai.request.model}`.

- **FR-020**: LLM invocation spans MUST include request parameter attributes when available: `gen_ai.request.max_tokens`, `gen_ai.request.temperature`, `gen_ai.request.top_p`, `gen_ai.request.top_k`, `gen_ai.request.stop_sequences`, `gen_ai.request.frequency_penalty`, `gen_ai.request.presence_penalty`, `gen_ai.request.seed`.

- **FR-021**: LLM invocation spans MUST include response attributes: `gen_ai.response.id`, `gen_ai.response.finish_reasons`, `gen_ai.usage.input_tokens`, `gen_ai.usage.output_tokens`, `gen_ai.usage.cache_read.input_tokens`, `gen_ai.usage.cache_creation.input_tokens`.

- **FR-022**: LLM invocation spans MUST include opt-in content attributes governed by the sensitive data configuration (FR-016): `gen_ai.input.messages` (opt-in), `gen_ai.output.messages` (opt-in), `gen_ai.system_instructions` (opt-in), `gen_ai.tool.definitions` (opt-in). Content attributes MUST follow the OTel GenAI JSON schema for messages.

- **FR-023**: Tool execution spans MUST include: `gen_ai.operation.name` (value: `execute_tool`), `gen_ai.tool.name`, `gen_ai.tool.call.id`, `gen_ai.tool.description`, `gen_ai.tool.type` (value: `function`), `gen_ai.tool.call.arguments` (opt-in), `gen_ai.tool.call.result` (opt-in), `error.type` (on error). Span name MUST be `execute_tool {gen_ai.tool.name}`. Span kind MUST be `INTERNAL`.

- **FR-024**: Embedding spans MUST include: `gen_ai.operation.name` (value: `embeddings`), `gen_ai.request.encoding_formats`, `gen_ai.usage.input_tokens`, `gen_ai.embeddings.dimension.count`. Span name MUST be `embeddings {gen_ai.request.model}`.

- **FR-025**: Retrieval spans MUST include: `gen_ai.operation.name` (value: `retrieval`), `gen_ai.retrieval.query.text` (opt-in), `gen_ai.retrieval.documents` (opt-in), `gen_ai.data_source.id`, `gen_ai.request.top_k`. Span name MUST be `retrieval {gen_ai.data_source.id}`.

- **FR-026**: All GenAI client spans MUST include `server.address` and `server.port` (when address is set). All spans that end in error MUST include `error.type` with a low-cardinality error identifier.

- **FR-027**: Library MUST define an `ObservabilitySpanKind` enum in `synwire-core` with M1 variants: `Llm`, `Chain`, `Tool`, `Embedding`, `Retriever`, `Graph`. Each variant maps to a `gen_ai.operation.name` value.

- **FR-028**: `gen_ai.output.type` MUST be set on spans where the client requests a specific output modality: `text`, `json` (for structured output), `image`, `speech`.

#### 1.3.4 OTel Metrics

- **FR-029**: When the `tracing` feature is enabled, the library MUST emit OTel GenAI metrics: `gen_ai.client.token.usage` (histogram, unit: `{token}`), `gen_ai.client.operation.duration` (histogram, unit: `s`), `gen_ai.client.operation.time_to_first_chunk` (histogram, unit: `s`, streaming only). Metrics use the `opentelemetry` metrics API, behind the `tracing` feature flag.

#### 1.3.5 OTLP Export

- **FR-030**: Library MUST provide generic OTLP export support via `opentelemetry-otlp`. When the `tracing` feature is enabled and an OTel `TracerProvider` is configured by the user, spans and metrics are exported. The library does NOT bundle a default exporter.

- **FR-031**: OTLP export MUST be feature-gated behind `otlp`. Core functionality MUST work without the exporter.

#### 1.3.6 Span Lifecycle

- **FR-032**: For streaming operations, a single parent span MUST be created for the entire stream lifecycle. `on_llm_new_token` fires for each chunk. `on_llm_end` fires when the stream completes with the aggregated result.

- **FR-033**: For batch operations, a parent span MUST be created for the batch. Each item gets a child span. The parent aggregates total token usage.

- **FR-034**: For retry operations (`with_retry`), a parent span MUST be created for the retry sequence. Each attempt gets a child span. The parent's `error.type` is set only if ALL attempts fail.

- **FR-035**: For fallback chains (`with_fallbacks`), a parent span MUST be created. Each provider attempt gets a child span. `gen_ai.provider.name` on child spans reflects the actual provider.

#### 1.3.7 Observability Edge Cases and NFRs

- **FR-036**: When both `CallbackHandler` and `tracing` feature are active, they operate independently. No deduplication. Callbacks serve user-defined logic; tracing serves backend export.

- **FR-037**: When tracing export fails, the `BatchSpanProcessor` drops spans after buffer is full. This MUST NOT affect application execution. Export failures are logged via `tracing::warn!`.

- **FR-038**: For long-running sessions, span exporters SHOULD use `BatchSpanProcessor`. The library MUST NOT accumulate spans in memory.

- **FR-039**: Nested graph execution MUST produce correctly nested span hierarchies. Inner graph spans are children of the tool execution span.

- **FR-040**: Concurrent superstep node execution MUST produce parallel sibling spans under the superstep parent.

- **FR-041**: Tracing overhead MUST be < 50 microseconds per span creation (excluding export). When `tracing` is disabled, zero runtime overhead via feature flags.

- **FR-042**: `CallbackHandler` implementations MUST be safe to call concurrently (`Send + Sync`). Parallel node execution produces concurrent callback calls.

- **FR-043**: Span attribute memory MUST be bounded. Content attributes MUST respect `TraceContentFilter.max_content_length`.

- **FR-044**: The `tracing` feature flag enables ALL tracing functionality. When disabled, all tracing code is compiled out via `#[cfg(feature = "tracing")]`. Dependencies `tracing`, `tracing-opentelemetry`, and `opentelemetry` are pulled only when enabled. Consider `tracing` as a default feature (negligible overhead when no subscriber attached).

  **Architecture review fix (P3 4.2)**: Consider making `tracing` a default feature.

- **FR-045**: Add structured log correlation: spans MUST include `trace_id`/`span_id` accessible for log-to-trace jumping.

  **Architecture review fix (P2 3.6)**.

#### 1.3.8 Streaming

- **FR-046**: When multiple nodes stream concurrently within the same superstep, their stream events MUST be interleaved. Each event MUST carry the source `node_id` for demultiplexing.

- **FR-047**: Graph streaming MUST support backpressure. When the consumer is slower than the producer, the graph runtime MUST buffer events up to `stream_buffer_size: usize` (default: 1024). When buffer is full, producer nodes MUST be suspended. Buffer overflow after timeout returns `StreamError::BackpressureTimeout`.

  **Architecture review fix (P1 2.10)**: Define lossless vs lossy semantics per stream mode. `values`, `updates`, `checkpoints` are lossless (backpressure suspends producers). `debug`, `messages` are lossy (events may be dropped for lagging subscribers). `custom` and `tasks` default to lossy.

### 1.4 Testing Strategy

- **FR-048**: The project MUST use nextest as the primary test runner. `.config/nextest.toml` MUST define `default` and `ci` profiles (JUnit XML, stricter timeouts, retry config).

- **FR-049**: Nextest test partitioning via `--partition hash:m/n` for CI parallelism.

- **FR-050**: Nextest test groups: `api-tests` (max concurrency 4 for external API keys).

- **FR-051**: Integration tests with external services retry up to 2 times with exponential backoff. Unit and property tests MUST NOT retry.

- **FR-052**: Slow-timeout thresholds: unit tests 10s (warn 5s), property tests 60s (warn 30s), integration tests 120s (warn 60s).

- **FR-053**: Filterset expressions: `test(~prop_)` for property tests, `test(~integration_)` for integration tests.

- **FR-054**: Property-based tests using proptest for synwire-core: (a) Message serialisation round-trips, (b) Document construction with arbitrary metadata, (c) PromptTemplate variable substitution completeness, (d) ChatPromptTemplate message list generation, (e) tool schema validation, (f) SynwireError Display/Debug non-panicking, (g) embedding dimension invariants, (h) InMemoryVectorStore similarity_search result count and ordering.

- **FR-055**: Property-based tests for synwire-orchestrator: (a) channel merge semantics, (b) valid graph topology compilation, (c) Pregel superstep determinism, (d) checkpoint serialisation round-trips, (e) conditional edge routing, (f) Send() fan-out exactly-once.

- **FR-056**: Reusable proptest Strategy implementations for core domain types: Message, Document, ToolInput, ChatResult, CheckpointData. In a shared test utilities module.

- **FR-057**: Proptest config: minimum 256 cases, max shrink 4096, regression files committed, fork mode for timeout tests.

- **FR-058**: A shared test utilities crate (`synwire-test-utils`) providing: FakeChatModel, FakeEmbeddings, proptest strategies, test fixture builders. Gate behind `test-utils` feature.

- **FR-059**: CI workflows (GitHub Actions): (a) `lint` -- fmt, clippy, (b) `unit-tests` -- nextest with unit + property, (c) `integration-tests` -- feature-gated, (d) `docs` -- doc build + link check, (e) `coverage` -- cargo-llvm-cov with nextest.

- **FR-060**: E2E tests using Tilt with Ollama (small model). Cover: chat invoke, streaming, RAG pipeline, tool-using agent, graph checkpoint+resume.

- **FR-061**: CI triggers: PR = lint + unit (fast), merge = full suite + coverage + docs, nightly = extended property (1024 cases).

- **FR-062**: CI MUST produce JUnit XML from nextest, uploaded as GitHub Actions artifacts.

- **FR-063**: Conformance test suites for: (a) checkpoint backends (round-trip, ordering, concurrent access), (b) ChatModel provider conformance (invoke/stream/batch/error).

- **FR-064**: CI MUST include `cargo-geiger` or equivalent audit to enforce zero-unsafe in `synwire-core` and `synwire-orchestrator`.

  **Architecture review fix (P3 4.8)**: Scope `#![forbid(unsafe_code)]` to core and orchestrator only.

- **FR-065**: Property tests for resource cleanup: dropped streams, cancelled futures, interrupted graph executions MUST NOT leak file handles, connections, or memory.

- **FR-066**: Property tests for checkpoint backwards compatibility: checkpoint written by version N readable by version N+1.

- **FR-067**: Document cancellation safety per public async method.

  **Architecture review fix (P2 3.1)**.

### 1.5 Documentation Requirements

Documentation follows the Diataxis framework: tutorials (learning), how-to guides (goals), explanation (understanding), reference (information).

- **FR-068**: The project MUST define a documentation architecture mapping every artefact to exactly one Diataxis quadrant. Cross-type content MUST be linked, not inlined.

- **FR-069**: The project MUST provide a documentation site (mdbook or equivalent) on GitHub Pages with versioned content tracking crate versions. GitHub Actions builds and deploys on merge to main.

- **FR-070**: Getting-started tutorial: zero to working synwire application. Diataxis tutorial conventions.

- **FR-071**: M1 tutorials: (a) chat model invocation, (b) prompt templates and chains, (c) streaming, (d) RAG with vector stores, (e) tool-using agents with `create_react_agent`, (f) graph-based agents with synwire-orchestrator, (g) structured output extraction, (h) proc-macro usage with synwire-derive.

- **FR-072**: Tutorials MUST specify prerequisites and progression order. Tutorial code MUST be tested in CI using FakeChatModel or environment-gated providers.

- **FR-073**: M1 how-to guides: adding a custom tool, switching LLM providers, adding checkpointing, writing custom channels, using interrupts, writing custom ChatModel/VectorStore providers, enabling tracing, redacting sensitive data, SecretValue credential management, error handling with retry/fallback.

- **FR-074**: How-to guides: action-oriented titles, assume competence, link to explanation docs.

- **FR-075**: Architecture explanation documents: (a) trait-based abstraction design, (b) Pregel execution model, (c) channel system design, (d) crate organisation rationale, (e) synwire-core vs synwire-orchestrator trade-offs.

- **FR-076**: The Hook/Callback decision tree MUST be a standalone explanation document.

- **FR-077**: The terminology glossary (FR-001) MUST be a standalone reference document linked from relevant pages.

- **FR-078**: LangChain-to-Synwire migration guide mapping Python concepts to Rust equivalents with intentional divergences.

- **FR-079**: All public types MUST have `#[doc]` comments: summary, description, `# Examples` with compilable doc-tests, `# Errors`, `# Panics`. Traits MUST include usage examples.

- **FR-080**: Every crate MUST have `//!` module-level docs explaining purpose, usage, and relationship to other crates.

- **FR-081**: Every error enum variant MUST document when it occurs and how to handle it.

- **FR-082**: Every feature flag MUST be documented in crate-level docs.

- **FR-083**: Examples in `examples/` MUST include file-level doc comments and compile in CI.

- **FR-084**: Contributor docs: dev setup, test execution, PR process, code review standards.

- **FR-085**: Provider author how-to guide for implementing ChatModel, Embeddings, VectorStore.

- **FR-086**: Documentation style guide: terminology consistency, code example formatting, Rust doc conventions, Mermaid usage.

- **FR-087**: Documentation MUST be searchable. Doc build part of CI. Broken links and doc-test failures MUST fail CI.

- **FR-088**: Error scenario guidance per crate: common errors, causes, resolution.

- **FR-089**: Offline/no-API-key usage guidance: which features work without network, FakeChatModel usage.

---

## 2. Core Traits

### 2.1 Trait Definitions

- **FR-090**: Library MUST define a `BaseChatModel` trait with `invoke()`, `batch()`, and `stream()` methods for interacting with language models.

- **FR-091**: Library MUST define a `PromptTemplate` type supporting named variable placeholders and runtime formatting.

- **FR-092**: Library MUST define a `RunnableCore` trait with `invoke()`, `batch()`, and `stream()` methods as the universal composition interface. A separate `ObservableRunnable` extension trait provides `stream_events()`. `stream_log` is dropped.

  **Architecture review fix (P1 2.2)**: Split `Runnable` into `RunnableCore` (invoke, batch, stream) + `ObservableRunnable` (stream_events). Drop `stream_log`.

- **FR-093**: Library MUST define an `Embeddings` trait with `embed_documents()` and `embed_query()` methods.

- **FR-094**: Library MUST define a `VectorStore` trait with `add_documents()`, `similarity_search()`, and `similarity_search_with_score()`. MMR methods SHOULD be in a separate `MmrVectorStore` extension trait.

  **Architecture review fix (P3 4.1)**: Split VectorStore into core + MMR extension.

- **FR-095**: Library MUST define a `Tool` trait with `invoke()` and `schema()` methods. Use `schemars` for JSON Schema generation in `#[tool]`.

  **Architecture review fix (P3 4.7)**: Use `schemars` for schema generation.

- **FR-096**: Library MUST define message types (`HumanMessage`, `AIMessage`, `SystemMessage`, `ToolMessage`) as an enum with shared metadata.

- **FR-097**: Library MUST define a `Document` type with content and arbitrary metadata fields.

- **FR-098**: Library MUST define a `CallbackHandler` trait for observability hooks (on_llm_start, on_llm_end, on_llm_error, etc.). Callbacks use `Arc<dyn CallbackHandler>` throughout. `RunnableConfig` MUST be cheaply cloneable -- pass by value or `Arc<RunnableConfig>`.

  **Architecture review fix (P0 1.3)**: Switch to `Arc<dyn CallbackHandler>`. Make `RunnableConfig` cloneable.

- **FR-099**: Library MUST define an `OutputParser` trait for converting raw model output into structured types.

- **FR-100**: Library MUST define a `Retriever` trait with `get_relevant_documents()`. Use RPITIT (`async fn` in traits) for `Retriever` and `OutputParser<T>` where dyn-dispatch is not needed.

  **Architecture review fix (P2 3.4)**: Use RPITIT for static-dispatch traits.

- **FR-101**: All fallible operations MUST return `Result<T, E>` with typed error enums. Panics in library code are forbidden.

- **FR-102**: All I/O-bound operations MUST be async-compatible.

- **FR-103**: All public types MUST be serialisable/deserialisable where semantically meaningful.

- **FR-104**: The library MUST be organised as a Cargo workspace with `synwire-core` as the foundational crate and provider integrations as separate member crates.

### 2.2 Error Types

- **FR-105**: Library MUST use layered error types per domain: `ModelError`, `GraphError`, `ToolError`, `CheckpointError`, `ProviderError`. A top-level `SynwireError` wraps them via `#[from]`. Each crate defines its own error type. Use `#[non_exhaustive]` from day one on all enums, error types, and config structs.

  **Architecture review fix (P1 2.1)**: Layered errors with `#[non_exhaustive]`.

### 2.3 Concrete Runnables

- **FR-106**: Library MUST define concrete runnable types: `RunnableSequence`, `RunnableParallel`, `RunnablePassthrough`, `RunnableLambda`, `RunnableBranch`.

- **FR-107**: Library MUST define concrete output parsers: `StrOutputParser`, `JsonOutputParser`, `StructuredOutputParser<T>`, `ToolsOutputParser`.

- **FR-108**: Library MUST define a `StructuredTool` type with builder pattern. The `#[tool]` attribute macro generates a `Tool` impl from an async function. Special parameters `config: &RunnableConfig` are injected, not included in schema.

  **Architecture review fix (P3 4.7)**: `#[tool]` schema generation uses `schemars`.

- **FR-109**: Library MUST define agent types (`AgentAction`, `AgentFinish`, `AgentStep`, `AgentDecision`). In M1, the ReAct-style tool-calling loop is provided by `create_react_agent()` (FR-148), which builds a `CompiledGraph`. The full `AgentExecutor` abstraction is deferred to M2.

- **FR-110**: Library MUST define message utility functions: `filter_messages` (use builder pattern), `trim_messages`, `merge_message_runs`.

  **Architecture review fix (P3 4.1)**: `filter_messages` uses a builder pattern instead of 7 parameters.

- **FR-111**: `RunnableCore` trait MUST include `with_retry` and `with_fallbacks` composition for resilience. `ObservableRunnable` provides `stream_events`.

- **FR-112**: Library MUST define `dispatch_custom_event` for emitting custom events to the callback system.

- **FR-113**: The `synwire` convenience crate MUST provide: CacheBackedEmbeddings, RunnableWithMessageHistory, additional output parsers, few-shot prompt templates with ExampleSelector, text splitters.

- **FR-114**: `synwire-llm-openai` MUST provide OpenAIModerationMiddleware as a RunnableLambda wrapper.

- **FR-115**: M1 provider crates: OpenAI (LLM + Embeddings) and Ollama (LLM + Embeddings). OpenAI-compatible providers share `BaseChatOpenAI`. Additional providers (Qdrant, pgvector, etc.) deferred to M2/M3.

- **FR-116**: Consider `&I` or `Cow<I>` for `RunnableCore` input to avoid clones for reuse.

  **Architecture review fix (P3 4.1)**.

### 2.4 Callback Enrichments

- **FR-117**: `CallbackHandler` MUST include embedding hooks: `on_embeddings_start`, `on_embeddings_end`, `on_embeddings_error` with `ignore_embeddings()` filter.

- **FR-118**: `LLMResult` passed to `on_llm_end` MUST include token usage metadata and model identifier.

- **FR-119**: `CallbackHandler` MUST include `on_parse_start`, `on_parse_error`, `on_parse_success` for structured output lifecycle events.

---

## 3. Graph Orchestration

### 3.1 StateGraph and Compilation

- **FR-120**: Library MUST define `StateGraph<S: State>` and `CompiledGraph<S: State>` keeping type safety through compilation. Erase to `serde_json::Value` only at serialisation boundaries (checkpoint save/load, HTTP API). Consider `Arc<Value>` with structural sharing for checkpoint copies.

  **Architecture review fix (P0 1.2)**: Generic state, not `Value` everywhere.

- **FR-121**: `CompiledGraph<S>` MUST implement `RunnableCore`.

- **FR-122**: Library MUST define a channel system with `BaseChannel` trait and concrete types: `LastValue`, `Topic`, `BinaryOperatorAggregate`, `AnyValue`, `EphemeralValue`, `NamedBarrierValue`.

- **FR-123**: Library MUST define `Pregel` as the internal execution engine for superstep-based synchronous execution.

- **FR-124**: Library MUST define control flow primitives: `Send`, `Command` (goto, update state, resume from interrupt), `Overwrite` (bypass channel reducers).

- **FR-125**: Library MUST define `interrupt()` for pausing graph execution. Interrupts persist via checkpoints and support `Command::resume`.

- **FR-126**: Library MUST define `StreamMode`: `values`, `updates`, `debug`, `messages`, `custom`, `tasks`, `checkpoints`.

- **FR-127**: Library MUST define `StateSnapshot` with current state, next nodes, config, metadata, timestamp, parent config, tasks, interrupts.

- **FR-128**: `CompiledGraph` MUST provide `get_state()`, `get_state_history()`, `update_state()` for state inspection and time-travel debugging.

- **FR-129**: Library MUST define `START` and `END` constants.

- **FR-130**: Library MUST define graph error types: `GraphRecursionError`, `InvalidUpdateError`, `GraphInterrupt`, `EmptyInputError`, `TaskNotFound`, `EmptyChannelError`. Integrate with layered error system (FR-105).

- **FR-131**: Library MUST define `RetryPolicy` for per-node retry: initial_interval, backoff_factor, max_interval, max_attempts, jitter, retry predicate, `idempotent: bool` (default true) gating retry behaviour.

- **FR-132**: Library MUST define `CachePolicy` for per-node result caching with key function and TTL.

- **FR-133**: Library MUST define `MessagesState` convenience type with `messages` field using `add_messages` reducer.

### 3.2 Checkpointing

- **FR-134**: Library MUST define a `BaseCheckpointSaver` trait with async methods: `get_tuple`, `list`, `put`, `put_writes`, `delete_thread`, `copy_thread`, `prune`, `get_next_version`.

- **FR-135**: Library MUST define `Checkpoint` with: version, id, timestamp, channel_values, channel_versions, versions_seen, updated_channels.

- **FR-136**: Library MUST define `CheckpointMetadata` with source (input/loop/update/fork), step number, parent IDs, run_id.

- **FR-137**: Library MUST define `CheckpointTuple` bundling checkpoint, metadata, config, parent_config, pending_writes.

- **FR-138**: Library MUST define `SerializerProtocol` trait with `JsonPlusSerializer` default. `SecretValue` in checkpoints serialises as a sentinel reference. Secrets are re-fetched from `CredentialProvider` on restore.

  **Architecture review fix (P1 2.4)**: Specify SecretValue checkpoint serialisation as sentinel.

- **FR-139**: Library MUST provide SQLite checkpoint implementation (`synwire-checkpoint-sqlite`). SQLite file permissions MUST be `0600`.

- **FR-140**: Library MUST provide in-memory checkpoint implementation for testing.

- **FR-141**: Library MUST provide a checkpoint conformance test suite.

### 3.3 Store

- **FR-142**: Library MUST define `BaseStore` trait for persistent key-value storage with namespace hierarchy: `get`, `search`, `put`, `delete`, `list_namespaces`, `batch`.

- **FR-143**: Library MUST define store types: `Item`, `SearchItem`, operation types.

- **FR-144**: Store MUST support TTL via `TTLConfig`.

- **FR-145**: Store MUST support optional semantic search via `IndexConfig`.

- **FR-146**: In-memory store for testing.

### 3.4 Cache

- **FR-147**: Library MUST define `BaseCache` trait for node result memoisation with `get`, `set`, `clear`, TTL per entry.

### 3.5 Prebuilt Agents

- **FR-148**: Library MUST provide `create_react_agent()` factory constructing a `StateGraph`-based ReAct agent from LLM and tools.

- **FR-149**: Library MUST provide `ToolNode` for parallel tool execution with error handling and state/store injection.

- **FR-150**: Library MUST provide `tools_condition` routing function.

- **FR-151**: Library MUST provide `ValidationNode` for validating tool inputs before execution.

- **FR-152**: Library MUST provide `AgentState` as standard prebuilt agent state.

### 3.6 Functional API

- **FR-153**: Library MUST define a `task` macro/builder for parallelisable, retryable, cacheable task units.

- **FR-154**: Library MUST define `entrypoint` for composing tasks into workflows with checkpointing, store, cache.

- **FR-155**: `entrypoint::final` MUST allow decoupling return value from checkpointed state.

### 3.7 Runtime

- **FR-156**: Library MUST provide runtime context accessors: `get_config()`, `get_store()`, `get_stream_writer()`.

- **FR-157**: Library MUST define managed values: `IsLastStep` (bool), `RemainingSteps` (usize).

### 3.8 Workspace Structure

- **FR-158**: Cargo workspace with both synwire and orchestrator crates under `crates/`. `synwire-orchestrator` depends on `synwire-core`. Share workspace-level `reqwest` dependency to reduce compile times.

  **Architecture review fix (P1 2.9)**: Share workspace dependencies.

---

## 4. Prebuilt Nodes and Graph Extensions

### 4.1 Prebuilt Workflow Nodes

- **FR-159**: Library MUST provide prebuilt control-flow node types: `IfElseNode` (conditional branching), `LoopNode` (repeating with termination predicate and max_iterations), `IterationNode` (iterate over collection with per-item execution). Convenience wrappers over conditional edges and Send.

- **FR-160**: Library MUST provide prebuilt data-transform node types: `TemplateTransformNode`, `ListOperatorNode` (sort, filter, slice, deduplicate), `VariableAggregatorNode`.

- **FR-161**: Library MUST provide `HttpRequestNode` for outbound HTTP as a first-class graph operation. All HTTP requests MUST use the SSRF-protected client (FR-191).

- **FR-162**: Library MUST provide `QuestionClassifierNode` using LLM classification to route to downstream nodes.

### 4.2 Graph-as-Tool

- **FR-163**: `CompiledGraph` MUST implement `as_tool(name, description) -> Box<dyn Tool>` for graph-in-graph composition.

- **FR-164**: `CompiledGraph` MUST be usable as a node within another `StateGraph` via `add_node`. Inner graph maintains own checkpoint history nested under outer graph.

### 4.3 Per-Node Error Strategies

- **FR-165**: Library MUST define `NodeErrorStrategy` enum: `FailWorkflow` (default), `FailBranch` (mark downstream SKIPPED, continue other branches), `Continue` (ignore error). Configurable per node.

- **FR-166**: When both `RetryPolicy` and `NodeErrorStrategy` are configured, retry exhausts all attempts before error strategy applies.

- **FR-167**: `FailBranch` in parallel branches: sibling branches complete, only failed path skipped. `FailWorkflow` cancels all in-progress parallel nodes.

---

## 5. Type System and Security

### 5.1 Typed Graph Values

- **FR-168**: `synwire-orchestrator` MUST define `TypedValue` enum for runtime type safety: `String`, `Integer(i64)`, `Float(f64)`, `Boolean`, `Secret(SecretValue)`, `List(Vec<TypedValue>)`, `Map(HashMap<String, TypedValue>)`, `Json(Value)`, `None`. Opt-in -- channels can continue using `serde_json::Value`.

- **FR-169**: Iteration/loop nodes MUST create nested variable scopes. Inner scope shadows outer. After iteration, inner scope discarded.

- **FR-170**: Nodes MUST reference other nodes' output via structured paths (`node_id.key`). Unresolved references return descriptive errors.

- **FR-171**: `synwire-orchestrator` MUST inject system variables: `sys.run_id`, `sys.thread_id`, `sys.created_at`, `sys.step_count`.

### 5.2 Credentials and Secret Management

- **FR-172**: `synwire-core` MUST define `SecretValue` wrapping sensitive data. MUST: (a) `Debug` as `"SecretValue(***)"`, (b) `Display` as `"***"`, (c) `Serialize` as `null` by default (opt-in via `expose()`), (d) `Clone + Send + Sync`. Uses the `secrecy` crate (or `zeroize`) for memory zeroisation on drop.

  **Architecture review fix (P1 2.5)**: Use `secrecy`/`zeroize` for SecretValue.

- **FR-173**: `synwire-core` MUST define `CredentialProvider` trait: `get_credential(key) -> Result<SecretValue>`, `refresh_credential(key) -> Result<SecretValue>`. Implementations: `EnvCredentialProvider`, `StaticCredentialProvider`.

- **FR-174**: For long-running execution, `CredentialProvider` MUST support refresh. On 401/403, provider crates SHOULD call `refresh_credential()` and retry once.

### 5.3 Token Tracking

- **FR-175**: `synwire-orchestrator` MUST track aggregate execution metrics: `total_input_tokens`, `total_output_tokens`, `total_tokens`, `model_invocations`, `execution_duration`. Accessible via `get_config().metrics()` and in `StateSnapshot` metadata.

- **FR-176**: Per-node metrics: `duration`, `input_tokens`, `output_tokens`, `retries`. Via `StateSnapshot.tasks[].metrics`.

- **FR-177**: Library MUST define `QuotaEnforcer` trait: `check_quota(metrics) -> Result<(), QuotaExceededError>`. Called before each LLM invocation. `NoOpQuotaEnforcer` default.

### 5.4 Checkpoint Versioning

- **FR-178**: Checkpoint serialisation MUST include `format_version: String` (initial `"1.0"`). Loaders validate version, return `CheckpointError::IncompatibleVersion`.

- **FR-179**: Library MUST define `CheckpointMigration` trait for migrating between format versions. Opt-in.

### 5.5 SSRF Protection

- **FR-180**: `synwire-core` MUST provide `SsrfProtectedClient` wrapping `reqwest::Client` that rejects private/internal addresses. Resolve DNS once, pin the resolved IP, connect to that exact IP. Block IPv4-mapped IPv6 addresses (`::ffff:10.0.0.1`) and tunnelling schemes. Configurable allow-list.

  **Architecture review fix (P1 2.6)**: DNS pinning for SSRF.

- **FR-181**: Library MUST define `HttpClientFactory` trait. All outbound HTTP in tools and HTTP nodes MUST use factory-provided client. Default returns `SsrfProtectedClient`.

### 5.6 Input Sanitisation

- **FR-182**: Tool arguments from LLM output MUST be validated against the tool's JSON Schema before invocation. Failing validation returns descriptive error to the model.

- **FR-183**: Path traversal protection MUST apply across all backend types operating in virtual/scoped mode. Reject `..`, null bytes, absolute paths.

---

## 6. Infrastructure

### 6.1 Node Registry

- **FR-184**: `synwire-orchestrator` MUST provide `NodeRegistry` for extensible node type registration. `register("type_name", constructor_fn)`.

- **FR-185**: Versioned registration: `register_versioned("type_name", "1.0", constructor_fn)`. Unregistered versions return `NodeRegistryError::VersionNotFound`.

- **FR-186**: `synwire-orchestrator` MUST define `NodeState` enum: `Pending`, `Running`, `Succeeded`, `Failed { error }`, `Skipped { reason }`, `Paused { interrupt }`. Recorded in `StateSnapshot.tasks[]`.

- **FR-187**: `step_count: u64` incremented at each superstep. Accessible via `sys.step_count` and `StateSnapshot`.

### 6.2 RAG Core Traits

- **FR-188**: `synwire-core` MUST define `DocumentLoader` trait: `load(source) -> Result<Vec<Document>>`. `synwire` crate provides `TextLoader`, `JsonLoader`, `CsvLoader`.

- **FR-189**: `synwire-core` MUST define `Reranker` trait: `rerank(query, documents, top_k) -> Result<Vec<(Document, f64)>>`.

- **FR-190**: `Retriever` trait MUST support `retrieval_mode`: `Dense`, `Sparse`, `Hybrid { alpha }`. Unsupported modes return `UnsupportedRetrievalMode`.

- **FR-191**: `VectorStore` MUST support optional `MetadataFilter` on search methods: `Eq`, `Ne`, `In`, `Gt/Lt/Gte/Lte`, `And`, `Or`. Unsupported returns `UnsupportedFilter`.

### 6.3 Streaming Coordination

- **FR-192**: Streaming content MUST distinguish primary (text response, structured data) from secondary (tool calls, reasoning traces, usage metrics) via `ContentCategory` enum.

### 6.4 Execution Resilience

- **FR-193**: Graph execution MUST support partial recovery from the last checkpoint. Nodes that completed successfully in the failed superstep are not re-executed if checkpointed.

- **FR-194**: Library documentation MUST specify node idempotency expectations. `RetryPolicy` MUST include `idempotent: bool` (default true) gating retry behaviour.

### 6.5 Result Size Management

- **FR-195**: `ToolNode` MUST support `max_result_size: usize` (default: 100KB). Exceeding results are truncated with `[truncated -- {original_size} bytes]` suffix. `truncated: bool` flag on `ToolMessage`.

- **FR-196**: Checkpoint savers MUST handle oversized state. `max_checkpoint_size: usize` (default: 10MB) returns `CheckpointError::StateTooLarge`.

### 6.6 Security

- **FR-197**: Tool names MUST conform to `^[a-zA-Z0-9_-]{1,64}$`. Validated at construction time.

---

## 7. Provider Integration

### 7.1 Test Utilities

- **FR-198**: Library MUST provide `FakeChatModel` and `FakeEmbeddings` test utilities for unit testing without real API calls.

- **FR-199**: Library MUST define `BatchProcessor<T>` for provider batch APIs (e.g. OpenAI Batch API). Provider-specific, behind feature flags.

- **FR-200**: `GraphExecutionMetrics` MUST include token usage from ALL LLM invocations including failed retries. `total_attempts: u32` and `successful_attempt: u32` fields.

- **FR-201**: `Runner` and `Agent` (when added in M2) MUST maintain `Vec<FailedAttempt>` during execution. `FailedAttempt`: `attempt_number`, `error`, `response`, `token_usage`, `duration`.

### 7.2 Provider Abstraction

- **FR-202**: Library MUST define `ModelProfileRegistry` trait: `register(profile)`, `get(model_id)`, `supports(model_id, capability)`. `InMemoryModelProfileRegistry` provided. Provider crates register profiles during model construction.

- **FR-203**: Runtime provider registration via `ModelProfileRegistry`. Custom providers register profile metadata without dynamic library loading.

### 7.3 Multimodal Content

- **FR-204**: Structured extraction MUST support multimodal inputs: messages with `ContentBlock::Image`, `ContentBlock::Audio`, `ContentBlock::File`.

- **FR-205**: When a model produces reasoning/thinking content alongside structured output, both MUST be preserved: thinking in `thinking: Option<Vec<String>>` and structured output in typed result.

---

## 8. Structured Output

- **FR-206**: When structured output extraction fails validation, the retry loop MUST include the validation error message in the next LLM prompt. Configurable `validation_error_formatter`.

- **FR-207**: `OutputMode<T>` MUST define a fallback chain: `Native` -> `Tool` -> `Prompt`. The chain is configurable via `OutputMode::Custom(Vec<OutputMode>)`. Distinguish JSON parse failures (trigger mode fallback) from schema validation failures (trigger reask within same mode).

- **FR-208**: `OutputMode` MUST validate mode/provider compatibility at construction time. Mode fallback is for runtime extraction failures only.

---

## 9. Edge Cases

### M1-Relevant Edge Cases

- Prompt template references missing variable -> descriptive error naming the variable.
- LLM returns malformed JSON -> parse error with raw response body.
- Async stream dropped before completion -> resources cleaned up without blocking or leaking.
- Embedding dimension mismatch -> vector store rejects with clear error at insertion.
- Empty message list to chat model -> validation error, not empty request.
- Graph cycle with no conditional exit -> recursion limit + `GraphRecursionError` with current state.
- Multiple nodes write to same channel in one superstep -> reducer applied; `LastValue` rejects multiple writers with `InvalidUpdateError`.
- Checkpoint saver fails mid-write -> error without corrupting existing checkpoints; partial writes rolled back.
- Resume with invalid/expired thread_id -> clear error (checkpoint not found).
- `Command` targeting non-existent node -> `InvalidUpdateError`.
- `interrupt()` outside graph context -> error, not panic.
- Graph compiled without checkpointer but `interrupt_before` specified -> compile-time error.
- Node with `FailBranch` in parallel branch -> only that branch's downstream skipped.
- Token quota exceeded mid-execution -> `QuotaExceededError` at next LLM invocation.
- Tool result exceeds `max_result_size` -> truncated, flag set.
- `SecretValue` in tracing span -> serialises as `"***"`.
- `CompiledGraph` as tool exceeds recursion limit -> tool error, not panic.
- `Agent::builder()` without model -> compile-time error (typestate) or runtime error at `build()`. (M2 concern, noted here for completeness.)
- Tool returns `ToolResult::Retry` beyond `max_retries` -> `ToolMessage` with `status=Error`.
- `OutputMode::Native` but model lacks support -> fall back to `OutputMode::Tool`.
- `Model::from_str` unknown provider prefix -> descriptive error listing known providers.
- All providers in a fallback chain fail -> return last provider's error.

---

## 10. Key Entities (M1)

- **Message**: Conversation unit -- role (human, ai, system, tool), content (text or structured), optional metadata.
- **PromptTemplate**: Parameterised template with named variable slots and format method.
- **Document**: Retrievable content with page content and metadata map.
- **Embedding**: Dense vector representation of text.
- **ToolCall**: Structured request from model to invoke a tool -- name, arguments, optional call ID.
- **ChatResult**: Model invocation output -- generations, token usage, model metadata.
- **AgentAction/AgentFinish**: Structured requests and terminal output from agent decision loops.
- **StreamEvent**: Structured observability event during runnable execution.
- **RetryConfig**: Retry composition config -- error kinds, max attempts, backoff.
- **ContentBlock**: Typed content element -- text, image URL, audio URL, reasoning, thinking.
- **StateGraph\<S\>**: Graph builder with typed state, nodes, edges. Compiles to `CompiledGraph<S>`.
- **Channel**: State management primitive accumulating updates and applying reducers per superstep.
- **Checkpoint**: Serialised snapshot of graph state -- channel values, versions, metadata.
- **CheckpointTuple**: Checkpoint bundled with metadata, config, parent reference, pending writes.
- **Send**: Control flow primitive for dynamic fan-out to a specific node.
- **Command**: Control flow for goto, state updates, interrupt resumption.
- **Interrupt**: Pause requesting user input with value and unique ID.
- **StateSnapshot**: Observable graph state -- values, next nodes, tasks, interrupts.
- **Store Item**: Persistent key-value entry with namespace hierarchy and optional TTL.
- **PregelTask**: Unit of work within a superstep -- task ID, node name, path.
- **SecretValue**: Wrapper preventing accidental logging/serialisation. Displays as `"***"`. Memory-zeroised on drop via `secrecy`/`zeroize`.
- **TypedValue**: Runtime type-safe enum for graph state beyond raw JSON.
- **NodeErrorStrategy**: Per-node error handling policy.
- **NodeState**: Node execution lifecycle state.
- **MetadataFilter**: Composable filter for vector store queries.
- **QuotaEnforcer**: Pluggable token/cost quota check before LLM invocations.
- **DocumentLoader**: Trait for extracting documents from source formats.
- **Reranker**: Trait for post-retrieval document reordering.
- **GraphExecutionMetrics**: Aggregate metrics -- tokens, cost, duration, invocation count.
- **NodeRegistry**: Extensible registry for node types and versions.
- **ContentCategory**: Distinguishes primary from secondary streaming content.
- **ObservabilitySpanKind**: Enum mapping operation types to OTel span attributes.
- **FailedAttempt**: Record of a failed retry attempt with error, response, tokens, duration.

---

## 11. Success Criteria (M1)

- **SC-001**: A developer adds `synwire-core` + a provider crate, implements a mock provider, and completes a round-trip model invocation in under 30 minutes using docs only.
- **SC-002**: Core trait definitions compile and pass tests with >= 90% line coverage on synwire-core.
- **SC-003**: OpenAI and Ollama provider crates demonstrate end-to-end: prompt -> model -> response, including streaming.
- **SC-004**: Switching providers requires changing only the concrete type instantiation.
- **SC-005**: Zero `unsafe` blocks in `synwire-core` and `synwire-orchestrator`.
- **SC-006**: All async operations support cancellation -- dropping a future/stream releases resources.
- **SC-007**: A developer defines a 3-node StateGraph, compiles with in-memory checkpointer, invokes, interrupts, resumes, and verifies state continuity.
- **SC-008**: Checkpoint round-trip: save to SQLite, restart, resume, same final state.
- **SC-009**: `create_react_agent` produces working agent with tool calling, streaming, checkpointing in under 10 lines.
- **SC-010**: >= 80% line coverage on synwire-orchestrator.
- **SC-011**: All orchestrator public types are `Send + Sync`.
- **SC-012**: Prebuilt nodes (IfElse, Loop, HttpRequest) produce correct results in unit tests.
- **SC-013**: `SecretValue` never appears in Debug, Display, or serialised JSON.
- **SC-014**: Aggregate token tracking correctly sums across multiple LLM invocations in a graph.
- **SC-015**: MetadataFilter correctly filters vector store results.
- **SC-016**: SSRF protection rejects private IPs, allows public IPs.
- **SC-017**: `#[tool]` macro generates a working tool implementation from annotated async function.
- **SC-018**: `CompiledGraph::to_mermaid()` produces valid Mermaid syntax.
- **SC-019**: Sensitive data redaction omits LLM I/O from traces when configured.
- **SC-020**: OTel GenAI span includes `gen_ai.operation.name`, `gen_ai.provider.name`, `gen_ai.request.model`, `gen_ai.usage.input_tokens`, `gen_ai.usage.output_tokens`.
- **SC-021**: OTel metrics emitted for token usage and operation duration.
- **SC-022**: `trace_include_sensitive_data: false` excludes all content attributes.
- **SC-023**: Documentation site builds without errors, doc-tests pass in CI.
- **SC-024**: Every public trait in synwire-core has a compilable doc-test.
- **SC-025**: Every crate has `//!` module-level docs.
- **SC-026**: All examples compile in CI without API keys (FakeChatModel).
- **SC-027**: Documentation provides Diataxis quadrant navigation.
- **SC-028**: Nextest CI with JUnit XML output. `default` and `ci` profiles defined.
- **SC-029**: Property tests achieve >= 256 cases per property.
- **SC-030**: Conformance suites for checkpoint backends and ChatModel providers.
- **SC-031**: E2E via Tilt+Ollama exercises chat invoke, streaming, RAG, tool agent, checkpoint.
- **SC-032**: PR checks complete in < 10 minutes (Tier 1). Full suite < 30 minutes.
- **SC-033**: Zero-unsafe enforced by CI audit.
- **SC-034**: Property-based path traversal tests reject all escape attempts.
- **SC-035**: `RunnableConfig` is `Clone` and callbacks use `Arc<dyn CallbackHandler>`.

---

## 12. Assumptions

- Primary audience: Rust developers familiar with LangChain/LangGraph concepts.
- M1 covers synwire-core abstractions and LangGraph graph orchestration in a unified workspace.
- M1 providers: OpenAI, Ollama (LLM). Additional providers in M2/M3.
- `tokio` is the primary async runtime.
- Serialisation uses `serde` with JSON as default wire format.
- `synwire-orchestrator` depends on `synwire-core`.
- Checkpoint storage: SQLite (M1), PostgreSQL (M2).
- `MessageGraph` (deprecated in Python) is excluded.
- `serde_json::Value` is the default dynamic state representation. `TypedValue` is opt-in.
- `synwire-derive` depends on `syn`, `quote`, `proc-macro2`.
- `EventBus` is feature-gated behind `event-bus`.
- `#[tool]` generates at compile time; runtime schema generation not required.
- All provider crates use `reqwest` with `rustls`.
- The `LangGraph SDK client` and `synwire-cli` are deferred to M2.
- `Agent<D,O>` builder, `Runner`, `AgentNode`, middleware, plugins, subagents, workflow agents -- all M2.
- A2A, AG-UI, Agent Spec, MCP adapters -- all M2/M3.
- DSPy (Signature, Module, Adapter, Teleprompters) -- M3.
- Evaluation framework (scorers, datasets, experiments) -- M3.
- Sandbox crates (local, k8s) -- M2.

---

## 13. Intentional Exclusions

### Permanently Excluded

- **Legacy chains** (LLMChain, SequentialChain, etc.) -- deprecated in Python.
- **MessageGraph** -- use `StateGraph` with `MessagesState`.
- **RunnableSerializable** -- Python-specific JSON serialisation.
- **LangSmith introspection** -- platform-coupled.
- **PipelinePromptTemplate** -- use RunnableSequence.
- **SimpleChatModel, LLM base class** -- Python implementation helpers.
- **CallbackManager hierarchy** -- replaced by flat `Arc<dyn CallbackHandler>` in `RunnableConfig`.
- **Blob, BaseDocumentCompressor** -- beyond text splitters.
- **RunnableBinding, RunnableGenerator, RunnableAssign, RunnablePick, RouterRunnable** -- Python-specific.
- **ConfigurableField** -- Rust uses generics and builders.
- **FunctionMessage, AgentActionMessageLog** -- deprecated.
- **NodeInterrupt** -- replaced by `interrupt()`.
- **GuardContent, RefusalContent, CitationContent, CacheControl content blocks** -- provider-specific.
- **AzureChatOpenAI, AzureOpenAIEmbeddings** -- separate crate.
- **BaseOpenAI / OpenAI legacy completions** -- deprecated.
- **EncryptedSerializer** -- future crate.
- **LangGraph Platform** -- not ported.
- **stream_log** -- Python/LangSmith legacy with no Rust equivalent.

### Deferred to M2

- `Agent<D,O>` builder, `Runner`, `AgentNode` trait, agent-level callbacks
- Middleware framework (TodoList, Filesystem, Memory, Skills, SubAgent, etc.)
- Plugin system
- Workflow agents (Sequential, Parallel, Loop)
- Sandbox crates (local filesystem, shell, k8s)
- MCP adapters
- CLI (`synwire-cli`)
- LangGraph SDK client
- PostgreSQL checkpoint
- Guardrails (InputGuardrail, OutputGuardrail)
- Approval system (ApprovalRequest, ApprovalResponse, ledger)
- Headless mode and approval policies
- Multi-tenancy (tenant_id isolation) -- tenant_id field present in RunnableConfig but isolation enforcement deferred
- SessionProvider
- MemoryService, ArtifactService
- ConversationManager
- KnowledgeBase composite type
- HandoffHistoryFilter, agent transfer
- ToolProvider trait
- ToolCategory enum
- Long-running tool tracking
- Dynamic instruction providers

### Deferred to M3

- A2A protocol
- AG-UI protocol
- Oracle Agent Spec
- DSPy (Signature, Module, Adapter, Teleprompters)
- Evaluation framework (Scorer, Dataset, Experiment, Harbor)
- BootstrapFinetune, MIPRO, COPRO optimisers

### Dify Cross-Reference

- **Sandboxed code execution node** -- shell execution + WASM sandboxes separate scope.
- **Ready queue / scheduling** -- Pregel superstep suffices for library.
- **Tool parameter rendering hints** -- UI concern for AG-UI.
- **Dynamic graph modification at runtime** -- static compilation by design.
- **Multi-root graphs** -- single START with conditional edges.

### Agents Cross-Reference

- **DevUI** -- observability via tracing/OTel.
- **Hot-reload tool directories** -- compiled types in Rust.
- **LLM-based task planning** -- application pattern via graph construction.
