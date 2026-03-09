# Research: Synwire Port

**Date**: 2026-03-09
**Branch**: `001-synwire`

## 1. Async Trait Pattern

**Decision**: Use manual `BoxFuture` desugaring for all public async traits.

**Rationale**: Native `async fn` in traits (stable since Rust 1.75) does not
support trait objects (`dyn Trait`). Since the library requires runtime
polymorphism (swapping providers), we need dyn-compatible async methods.
Manual `BoxFuture` desugaring is the idiomatic pattern used by tower, tonic,
and hyper — it avoids a proc-macro dependency while providing full dyn support.

**Pattern**:
```rust
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait BaseChatModel: Send + Sync {
    fn invoke<'a>(
        &'a self,
        messages: &'a [Message],
    ) -> BoxFuture<'a, Result<ChatResult, SynwireError>>;
}
```

**Alternatives considered**:
- `async-trait` crate: Acceptable but adds proc-macro dependency for the same
  heap allocation. Rejected in favour of manual control.
- Native `async fn in trait`: Cannot support `dyn Trait`. Rejected.
- `trait_variant` crate: Solves Send bounds but not dyn dispatch. Rejected.

## 2. Streaming Pattern

**Decision**: Use `Pin<Box<dyn Stream<Item = Result<T, E>> + Send>>` as the
stream return type in traits. Depend on `futures-core` for the `Stream` trait
in the public API.

**Rationale**: Boxed streams are dyn-compatible and the single heap allocation
is negligible compared to network I/O latency. `futures-core` is a minimal
dependency (just the trait), avoiding coupling to tokio-stream or the full
futures facade.

**Pattern**:
```rust
pub type BoxStream<'a, T> = Pin<Box<dyn Stream<Item = T> + Send + 'a>>;

pub trait BaseChatModel: Send + Sync {
    fn stream<'a>(
        &'a self,
        messages: &'a [Message],
    ) -> BoxFuture<'a, Result<BoxStream<'a, Result<ChatChunk, SynwireError>>, SynwireError>>;
}
```

**SSE parsing**: Use `eventsource-stream` crate for W3C-compliant SSE parsing
in provider crates, rather than rolling a custom parser.

**Cancellation**: Rely on Rust's `Drop` semantics — dropping the stream cancels
the HTTP body read. Store `AbortHandle` if background tasks are involved.

**Alternatives considered**:
- Associated type `type ChatStream: Stream<...>`: Zero-cost but not
  dyn-compatible. Rejected for public traits.
- `tokio-stream` dependency: Couples library to tokio. Rejected for core crate.

## 3. Python API Mapping

**Decision**: Port the 10 core modules from `synwire_core` with idiomatic
Rust names and patterns, maintaining 1:1 conceptual mapping.

**Key mappings** (Python → Rust):

| Python Class | Rust Trait / Type | Key Methods |
|-------------|-------------------|-------------|
| `BaseChatModel` | `trait BaseChatModel` | `invoke`, `batch`, `stream` |
| `BaseLLM` | `trait BaseLLM` | `invoke`, `batch`, `stream` |
| `Embeddings` | `trait Embeddings` | `embed_documents`, `embed_query` |
| `VectorStore` | `trait VectorStore` | `add_documents`, `similarity_search`, `similarity_search_with_score` |
| `Runnable[I, O]` | `trait Runnable<I, O>` | `invoke`, `batch`, `stream` |
| `BaseTool` | `trait Tool` | `invoke`, `schema` |
| `BaseRetriever` | `trait Retriever` | `get_relevant_documents` |
| `BaseOutputParser` | `trait OutputParser<T>` | `parse`, `parse_result` |
| `BaseCallbackHandler` | `trait CallbackHandler` | `on_llm_start`, `on_llm_end`, `on_llm_error`, etc. |
| `BaseMessage` (+ variants) | `enum Message` | `content()`, `message_type()` |
| `Document` | `struct Document` | `page_content`, `metadata`, `id` |

**Key design decisions**:

- Python's `BaseMessage` hierarchy (HumanMessage, AIMessage, SystemMessage,
  ToolMessage) maps to a Rust `enum Message` with variants, rather than a
  trait hierarchy. Enums are more idiomatic for a closed set of known types.
- Python's `Runnable.__or__` (pipe operator) maps to a `pipe()` free function
  in Rust, since Rust's `BitOr` trait is less ergonomic for this purpose.
- Python's async/sync duality (`invoke`/`ainvoke`) collapses to a single async
  method in Rust, with sync wrappers provided via a `blocking` module (see §9).
- Python's `RunnableConfig` (dict with callbacks, tags, metadata) maps to a
  Rust struct with `Option` fields.
- Python's `**kwargs` patterns are replaced with typed config structs or
  builder patterns in Rust.

**Per-trait async/sync mapping**:

| Python Methods | Rust Method | Notes |
|---|---|---|
| `BaseChatModel.invoke` / `ainvoke` | `BaseChatModel::invoke` (async) | Single async method |
| `BaseChatModel.batch` / `abatch` | `BaseChatModel::batch` (async) | Single async method |
| `BaseChatModel.stream` / `astream` | `BaseChatModel::stream` (async) | Returns BoxStream |
| `BaseLLM.invoke` / `ainvoke` | `BaseLLM::invoke` (async) | Single async method |
| `BaseLLM.batch` / `abatch` | `BaseLLM::batch` (async) | Single async method |
| `BaseLLM.stream` / `astream` | `BaseLLM::stream` (async) | Returns BoxStream |
| `Embeddings.embed_documents` / `aembed_documents` | `Embeddings::embed_documents` (async) | Single async method |
| `Embeddings.embed_query` / `aembed_query` | `Embeddings::embed_query` (async) | Single async method |
| `VectorStore.add_documents` / `aadd_documents` | `VectorStore::add_documents` (async) | Single async method |
| `VectorStore.similarity_search` / `asimilarity_search` | `VectorStore::similarity_search` (async) | Single async method |
| `Runnable.invoke` / `ainvoke` | `Runnable::invoke` (async) | Single async method |
| `Runnable.batch` / `abatch` | `Runnable::batch` (async) | Single async method |
| `Runnable.stream` / `astream` | `Runnable::stream` (async) | Returns BoxStream |
| `BaseRetriever.invoke` / `ainvoke` | `Retriever::get_relevant_documents` (async) | Single async method |
| `BaseTool.invoke` / `ainvoke` | `Tool::invoke` (async) | Single async method |
| `BaseOutputParser.parse` / `aparse` | `OutputParser::parse` (async) | Single async method |

All sync variants in Rust are provided via the `blocking` module (see §9).

## 4. Error Handling

**Decision**: Single `#[non_exhaustive]` error enum in `synwire-core` with
`thiserror`. Provider crates define their own error types that convert into
the core error via `From`.

**Pattern**:
```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SynwireError {
    #[error("model invocation failed: {0}")]
    ModelError(String),
    #[error("prompt formatting failed: missing variable '{variable}'")]
    PromptError { variable: String },
    #[error("output parsing failed: {0}")]
    ParseError(String),
    #[error("embedding failed: {0}")]
    EmbeddingError(String),
    #[error("vector store error: {0}")]
    VectorStoreError(String),
    #[error("tool invocation failed: {0}")]
    ToolError(String),
    #[error("agent error: {0}")]
    AgentError(String),
    #[error("retry exhausted after {attempts} attempts: {source}")]
    RetryExhausted {
        attempts: u32,
        #[source]
        source: Box<SynwireError>,
    },
    #[error(transparent)]
    SerializationError(#[from] serde_json::Error),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}
```

## 5. Serialisation

**Decision**: Use `serde` with `serde_json` as the default wire format.
All public types that cross serialisation boundaries derive `Serialize` and
`Deserialize`.

**Rationale**: Matches Python LangChain's JSON-based serialisation. serde is
the de facto standard in the Rust ecosystem.

**Python method mapping**:
- `to_json()` / `dict()` → `serde_json::to_value(&msg)` or
  `serde_json::to_string(&msg)`
- `from_json()` / `parse_obj()` → `serde_json::from_value::<Message>(val)`
  or `serde_json::from_str::<Message>(s)`
- No explicit `to_json` / `from_json` methods on types — serde derives
  provide this automatically for all `Serialize` / `Deserialize` types.

**`lc_serializable` / `get_lc_namespace`**: Python's type-discriminated
deserialisation pattern (for JSON round-trips with type tags). Not applicable
to Rust — serde's `#[serde(tag = "type")]` provides equivalent functionality
when needed. The `lc_*` namespace system is Python packaging infrastructure.

## 6. Dependencies Summary

| Crate | Purpose | Where Used |
|-------|---------|------------|
| `futures-core` | Stream trait (public API) | synwire-core |
| `futures-util` | StreamExt combinators (internal) | synwire-core |
| `pin-project-lite` | Safe pin projections | synwire-core |
| `thiserror` | Error derive macros | synwire-core |
| `serde` | Serialization framework | synwire-core |
| `serde_json` | JSON serialization | synwire-core |
| `uuid` | Run IDs for callbacks | synwire-core |
| `reqwest` (rustls) | HTTP client | synwire-llm-openai |
| `eventsource-stream` | SSE parsing | synwire-llm-openai |
| `backoff` | Exponential backoff retry policies | synwire-core (with_retry) |
| `json-patch` | RFC 6902 JSON Patch operations | synwire-core (stream_log) |
| `tracing` | Structured diagnostics framework | synwire-core (optional) |
| `tracing-opentelemetry` | Bridge tracing spans to OTel | synwire-core (optional) |
| `opentelemetry` | OTel SDK/API | synwire-core (optional) |
| `opentelemetry-otlp` | OTLP exporter (Jaeger, etc.) | synwire (optional feature) |
| `reqwest-retry` | HTTP client retry middleware | synwire-llm-openai |
| `reqwest-middleware` | reqwest middleware infrastructure | synwire-llm-openai |
| `tokio` | Async runtime (dev/test) | tests, examples |
| `mockall` | Mocking framework | tests |

## 7. Scope Exclusions

**Decision**: Document all Python API elements intentionally excluded from
the initial Rust port, organized by category and trait.

### Globally Excluded

| Category | Python Elements | Rationale |
|---|---|---|
| Legacy Chains | `LLMChain`, `SequentialChain`, `RetrievalQA` | Legacy chain API; use Runnable composition |
| RunnableSerializable | `lc_serializable`, `get_lc_namespace`, `RunnableSerializable` | Pydantic packaging infrastructure; not applicable to Rust |
| LangSmith Introspection | `with_listeners`, `get_graph`, `get_prompts` | LangSmith-specific internal introspection; not part of the core API |
| LangGraph Middleware | `AgentMiddleware`, decorators, execution policies | LangGraph is a separate framework; the Rust port covers classic ReAct agents via AgentExecutor |
| Pipeline Templates | `PipelinePromptTemplate`, `DictPromptTemplate` | Permanently excluded — Runnable composition (pipe) replaces pipeline templates; DictPromptTemplate is Python dict-specific |
| Callback Manager | `CallbackManager`, `AsyncCallbackManager`, run-specific managers | Permanently excluded — Rust uses `RunnableConfig.callbacks` (Vec) with parent/child run IDs; no manager hierarchy needed |
| Model Capabilities | `ModelProfile`, `rate_limiter` | Permanently excluded — compile-time trait bounds replace runtime capability detection; rate limiting uses tower/reqwest-middleware |
| Document (Binary) | `Blob`, `BaseDocumentCompressor` | Permanently excluded — binary content processing and document compression are document loader concerns |
| Deprecated Messages | `FunctionMessage`, `FunctionMessageChunk` | Permanently excluded — deprecated; superseded by ToolMessage |
| LangGraph Messages | `RemoveMessage` | Permanently excluded — LangGraph state management primitive; not a core Synwire type |

### Excluded from Core — Reference Implementations in `synwire` Crate

These items are excluded from `synwire-core` trait hierarchy but provided
as concrete reference implementations in the `synwire` convenience crate,
mirroring Python's `synwire` package layering.

| Category | Items | Location in `synwire` crate |
|---|---|---|
| FewShot Templates | `FewShotPromptTemplate`, `FewShotChatMessagePromptTemplate`, `SemanticSimilarityExampleSelector` | `synwire::prompts::few_shot`, `synwire::prompts::example_selector` |
| History Management | `RunnableWithMessageHistory`, `ChatMessageHistory` trait, `InMemoryChatMessageHistory` | `synwire::chat_history` |
| Embedding Cache | `CacheBackedEmbeddings` (wraps Embeddings + moka cache) | `synwire::cache::embeddings` |
| Additional Parsers | `CommaSeparatedListOutputParser`, `EnumOutputParser`, `XMLOutputParser`, `RegexParser`, `RetryOutputParser`, `CombiningOutputParser` | `synwire::output_parsers` |
| Text Splitters | `CharacterTextSplitter`, `RecursiveCharacterTextSplitter` | `synwire::text_splitters` |
| Model Caching | Model response caching via `moka` | `synwire::cache` (future — not in initial reference impls) |
| Moderation | `OpenAIModerationMiddleware` (RunnableLambda wrapper) | `synwire_openai::moderation` |

### Per-Trait Exclusions

| Trait | Excluded Method/Type | Rationale |
|---|---|---|
| BaseChatModel | `generate`, `generate_prompt` | Internal dispatch; Rust uses invoke/batch directly |
| BaseChatModel | `LanguageModelInput` union | Rust uses `&[Message]` with `Into` conversions |
| BaseChatModel | `SimpleChatModel`, `LLM` base | Implement traits directly; no intermediate base needed |
| BaseLLM | `generate`, `generate_prompt` | Same as BaseChatModel |
| BaseLLM | `dict`, `save` | Serde derives replace explicit serialisation methods |
| VectorStore | `similarity_search_with_relevance_scores` | Unified into `similarity_search_with_score` |
| VectorStore | `from_documents`, `from_texts` | Rust uses constructors + `add_documents` |
| Runnable | `bind`, `RunnableBinding` | Python kwargs currying; Rust uses typed configs |
| Runnable | `pick`, `assign`, `RunnablePick`, `RunnableAssign` | Python dict manipulation; use `RunnableParallel` for combining outputs |
| Runnable | Schema introspection methods | Pydantic-specific; Rust uses compile-time generics |
| Runnable | `RunnableGenerator` | Use `RunnableLambda` with closure returning BoxStream |
| Runnable | `RouterRunnable` | `RunnableBranch` covers conditional routing |
| Runnable | `ConfigurableField*` | Pydantic runtime config; Rust uses builder patterns + RunnableConfig |
| Runnable | `@chain` decorator | Use `RunnableLambda` with closures |
| Tool | `_run`, `_arun` | Python internal pattern; trait methods are extension point |
| Tool | `is_single_input` | JSON Schema defines arity |
| Tool | `run`, `arun` | Deprecated legacy API |
| Tool | `Tool` (simple class) | Covered by `StructuredTool` with single-field schema |
| CallbackHandler | `on_text` | General-purpose; not useful in typed Rust API |
| CallbackHandler | `collect_runs` | Use tracing spans for run data collection |
| CallbackHandler | `tracing_v2_enabled` | Use tracing crate's standard dispatcher checks |
| OutputParser | Niche parsers (XML, list, enum, retry, combining) | Excluded from core; reference impls in `synwire` crate |
| Message | `pretty_repr`, `pretty_print` | Rust uses Debug/Display traits |
| Message | `convert_to_messages` | Rust uses From/Into trait implementations |
| Message | `messages_to_dict`, `messages_from_dict` | Covered by serde serialisation |
| Message | `AgentActionMessageLog` | Standard AgentAction with log field covers the use case |
| Document | `type` discriminator | Rust uses struct typing; serde tag if needed |
| ContentBlock | `GuardContent`, `RefusalContent`, `CitationContent`, `CacheControl` | Provider-specific; mapped to response_metadata or additional_kwargs |
| OpenAI | `AzureChatOpenAI`, `AzureOpenAIEmbeddings` | Separate provider crate (synwire-azure-openai) |
| OpenAI | `BaseOpenAI` (completions) | Deprecated API; use ChatOpenAI |
| OpenAI | `OpenAIModerationMiddleware` | Excluded from core; reference impl in `synwire_openai::moderation` |

## 8. Error Mapping

**Decision**: Map Python exception types to `SynwireError` enum variants.

| Python Exception | Rust `SynwireError` Variant | When Raised |
|---|---|---|
| `OutputParserException` | `ParseError(String)` | Output parser fails to parse model response |
| `ToolException` | `ToolError(String)` | Tool invocation fails |
| `ValueError` (prompt) | `PromptError { variable: String }` | Missing template variable during formatting |
| `APIError` (provider) | `ModelError(String)` | Provider API returns error (HTTP 4xx/5xx) |
| `ValidationError` (Pydantic) | `Other(Box<dyn Error>)` | Input validation fails (rare in Rust) |
| `NotImplementedError` | Compile-time via traits | Unimplemented trait methods are compile errors |
| `serde_json::Error` | `SerializationError` | JSON parse/format errors |

**`return_exceptions` on batch**: When `return_exceptions: true` is passed
to `Runnable::batch`, individual item failures are returned as
`Err(SynwireError)` in the result vector rather than failing the entire
batch. When `false` (default), the batch fails atomically on the first error.

## 9. Sync Wrappers

**Decision**: Provide synchronous wrappers via a `synwire_core::blocking`
module for use in non-async contexts.

**Pattern**:
```rust
pub mod blocking {
    pub struct BlockingChatModel<T: BaseChatModel> {
        inner: T,
        runtime: tokio::runtime::Runtime,
    }

    impl<T: BaseChatModel> BlockingChatModel<T> {
        pub fn new(inner: T) -> Self { ... }

        pub fn invoke(
            &self,
            messages: &[Message],
            config: Option<&RunnableConfig>,
            stop: Option<&[String]>,
        ) -> Result<ChatResult, SynwireError> {
            self.runtime.block_on(self.inner.invoke(messages, config, stop))
        }

        // batch, stream (returns iterator instead of Stream) ...
    }
}
```

**Design decisions**:
- Each async trait gets a corresponding `Blocking*` wrapper struct
- Wrapper owns a `tokio::runtime::Runtime` (single-threaded) or accepts a
  `Handle` if the caller already has a runtime
- `stream` returns an iterator (`impl Iterator<Item = Result<T, E>>`)
  instead of a `Stream`, using `block_on` per item
- Lives in `synwire_core::blocking` module
- The blocking wrapper is provided as a convenience; the async API is primary

## 10. Retry and Resilience

**Decision**: Use `backoff` crate for retry policies in synwire-core.
Use `reqwest-retry` + `reqwest-middleware` in provider crates for HTTP-level
retry.

**Rationale**: `backoff` provides composable iterator-based backoff policies
without proc-macro overhead. `tower` middleware was considered but rejected for
the initial port — the `Runnable` trait uses `&self` (not the Clone-based
`Service` pattern), making tower `Layer` composition less natural. Migration to
tower is possible later if needed.

**Pattern**:

`RunnableRetry<I, O>` wraps `Box<dyn Runnable<I, O>>` and applies retry logic
in `invoke()`:

```rust
pub struct RunnableRetry<I, O> {
    inner: Box<dyn Runnable<I, O>>,
    config: RetryConfig,
}

impl<I, O> Runnable<I, O> for RunnableRetry<I, O>
where
    I: Send + Sync + Clone + 'static,
    O: Send + 'static,
{
    fn invoke<'a>(
        &'a self,
        input: I,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<O, SynwireError>> {
        // Uses backoff::ExponentialBackoff with jitter
        // Retries only on error kinds matching self.config.retry_on
    }
}
```

`RunnableWithFallbacks<I, O>` tries primary then each fallback in order:

```rust
pub struct RunnableWithFallbacks<I, O> {
    primary: Box<dyn Runnable<I, O>>,
    fallbacks: Vec<Box<dyn Runnable<I, O>>>,
    exceptions_to_handle: Vec<SynwireErrorKind>,
}
```

**`SynwireErrorKind`**: A discriminant enum (no payload) used by retry and
fallback configuration to specify which error variants to handle. Maps 1:1 to
`SynwireError` variants. See data-model.md §Retry & Resilience Types.

**Alternatives considered**:
- `tower::retry`: Requires `Service` (Clone + poll_ready pattern). Not a
  natural fit for `Runnable` trait. Rejected for initial port.
- `retry` crate: Less maintained than `backoff`, fewer features. Rejected.
- `reqwest-retry` at application level: Only handles HTTP errors, not
  parse/tool/embedding errors. Used in provider crates only, not for core
  Runnable retry.

## 11. Observability and Event Streaming

**Decision**: Use `tracing` + `tracing-opentelemetry` for instrumentation,
behind an optional `tracing` feature flag in synwire-core. Implement
`stream_events` as a method on Runnable that emits `StreamEvent` enum variants.
Implement `stream_log` using `json-patch` crate for RFC 6902 patch operations.

**Feature flag**:

```toml
[features]
default = []
tracing = ["dep:tracing", "dep:tracing-opentelemetry", "dep:opentelemetry"]
```

`tracing` and OTel crates are optional dependencies gated behind
`features = ["tracing"]` in synwire-core's Cargo.toml. Core crate stays
lightweight by default. When enabled, trait method implementations use
`#[cfg(feature = "tracing")]` to emit tracing spans. The `stream_events` and
`stream_log` methods are always available (they use the CallbackHandler system,
not tracing directly), but OTel export requires the feature flag.

**Rationale**: `tracing` is the de facto Rust standard for structured
diagnostics, maintained by the Tokio project. The OTel bridge
(`tracing-opentelemetry`) provides vendor-neutral export to LangSmith,
Helicone, Jaeger, etc. Making it optional avoids adding ~15 transitive
dependencies for users who don't need observability.

**Event names** follow Python convention:
`on_chain_start`, `on_chain_stream`, `on_chain_end`,
`on_llm_start`, `on_llm_stream`, `on_llm_end`,
`on_chat_model_start`, `on_chat_model_stream`, `on_chat_model_end`,
`on_tool_start`, `on_tool_end`,
`on_retriever_start`, `on_retriever_end`,
`on_prompt_start`, `on_prompt_end`,
`on_custom_event`.

**`stream_events` design**: Emits `StreamEvent::Standard` for lifecycle events
and `StreamEvent::Custom` for user-dispatched events (via `on_custom_event`
callback). Filtering by name/type/tag is handled at the emission layer to avoid
unnecessary allocation.

**`stream_log` design**: Emits `RunLogPatch` containing `Vec<JsonPatchOp>`
(RFC 6902). Uses the `json-patch` crate for patch construction. Each runnable
step emits add/replace operations against a virtual run log document. Useful
for UIs that reconstruct execution state incrementally.

**Alternatives considered**:
- `log` crate: No structured spans, no OTel bridge. Rejected.
- Custom event system without tracing: Would duplicate existing ecosystem
  infrastructure. Rejected.
- Required (non-optional) tracing: Adds ~15 transitive deps for all users.
  Rejected — feature flag is more ergonomic.

## 12. Content Block Types

**Decision**: Support `Text`, `Image`, `Audio`, `Video`, `File`, `Reasoning`,
and `Thinking` content block types. Exclude provider-specific content blocks.

**Rationale**: Multimodal content (images, audio, video, files) is becoming
standard across major LLM providers. `Reasoning` and `Thinking` blocks are
essential for Claude's extended thinking and similar features in other models.
Provider-specific blocks (`GuardContent`, `RefusalContent`, `CitationContent`,
`CacheControl`) are mapped to `response_metadata` or `additional_kwargs` on
the message rather than dedicated content block types, since they are not
universally supported and their semantics vary by provider.

## 13. Message Utility Functions

**Decision**: Include `filter_messages`, `trim_messages`, and
`merge_message_runs` as standalone functions in `synwire-core`. Exclude
`convert_to_messages` and serialisation helpers.

**Rationale**: These three functions are essential for production chat
applications — `trim_messages` for context window management,
`filter_messages` for selecting relevant messages, and `merge_message_runs`
for cleaning up conversation history. They operate on `Vec<Message>` and are
pure functions with no external dependencies.

`convert_to_messages` is replaced by Rust's `From`/`Into` trait
implementations. Serialisation helpers are replaced by serde.

## 14. Runnable Concrete Types

**Decision**: Include `RunnableLambda`, `RunnableParallel`, `RunnablePassthrough`,
and `RunnableBranch` as concrete types in `synwire-core`. Exclude
`RunnableGenerator`, `RunnableBinding`, `RunnableWithMessageHistory`,
`RouterRunnable`, and `ConfigurableField`.

**Rationale**: The included types represent the minimal set needed for
practical chain composition:
- `RunnableLambda`: Ad-hoc transformations (most commonly used Runnable type)
- `RunnableParallel`: Fan-out pattern (run multiple chains on same input)
- `RunnablePassthrough`: Identity (forward input unchanged; essential for RAG)
- `RunnableBranch`: Conditional routing (if-else chains)

Excluded types are either redundant (`RunnableGenerator` = `RunnableLambda`
with stream, `RouterRunnable` = `RunnableBranch`), or Python-specific
(`RunnableBinding`, `ConfigurableField`). `RunnableWithMessageHistory` is
excluded from core but provided as a reference implementation in the
`synwire` crate with a pluggable `ChatMessageHistory` trait.

## 15. Output Parser Concrete Implementations

**Decision**: Ship `StrOutputParser`, `JsonOutputParser`,
`StructuredOutputParser<T>`, and `ToolsOutputParser` with `synwire-core`.
Additional parsers (`CommaSeparatedListOutputParser`, `EnumOutputParser`,
`XMLOutputParser`, `RegexParser`, `RetryOutputParser`,
`CombiningOutputParser`) are provided as reference implementations in the
`synwire` convenience crate.

**Rationale**: These four parsers cover the essential workflows:
- `StrOutputParser`: Every text chain
- `JsonOutputParser`: Structured output without function-calling
- `StructuredOutputParser<T>`: Type-safe structured output (Rust equivalent of PydanticOutputParser)
- `ToolsOutputParser`: Function-calling chains

Remaining Python parsers (XML, list, enum, retry, combining) are provided
as reference implementations in the `synwire` crate rather than core.
They are straightforward but useful — the `synwire` crate serves as a
batteries-included convenience layer, mirroring Python's `synwire`
package on top of `synwire-core`.

## 16. Tool Creation Pattern

**Decision**: Provide `StructuredTool` with a builder pattern as the primary
tool creation API. No proc-macro `#[tool]` attribute in the initial port.

**Rationale**: Python's `@tool` decorator auto-generates tool definitions
from function signatures and docstrings. Rust does not have equivalent
decorator patterns. A builder pattern (`StructuredToolBuilder`) provides the
same functionality without proc-macro compile-time overhead. A `#[tool]`
proc-macro could be added as a separate crate later, but the builder is
sufficient and avoids the dependency.

## 17. Agents/Middleware Architecture

**Decision**: The Python `agents/middleware` system (LangGraph-based) is
excluded from the Rust port. The Rust port implements classic ReAct agents
via `AgentExecutor`.

**Rationale**: The Python `agents/middleware` package is part of the
LangGraph framework, which is a higher-level orchestration system built on
top of synwire-core. It uses:
- `AgentMiddleware` base class with decorator-based hooks
- TypedDict-based mutable `AgentState`
- Execution policies (`Host`, `CodexSandbox`, `Docker`)
- 14 concrete middleware classes for retry, fallback, limits, PII, etc.
  - `ModelCallLimitMiddleware` / `ToolCallLimitMiddleware` are covered by
    `AgentExecutor.max_iterations` (total agent steps) — per-tool invocation
    limits are not provided as a separate mechanism.

This is architecturally distinct from the core Synwire abstractions.
The Rust port covers `synwire-core` + a classic ReAct `AgentExecutor`
(the loop: LLM → parse → tool → observe → repeat). LangGraph could be
ported as a separate Rust crate that depends on `synwire-core`.

The middleware functionality that IS in scope (retry, fallback) is provided
at the Runnable level via `with_retry` and `with_fallbacks`, which are more
composable than the middleware pattern.

## 18. Test Utilities

**Decision**: Ship `FakeChatModel` and `FakeEmbeddings` with `synwire-core`
for testing chains and agents without real API calls.

**Rationale**: Users need deterministic, no-network test doubles for unit
testing their chains. Python provides `FakeListChatModel`,
`FakeStreamingListLLM`, etc. Rust provides:
- `FakeChatModel`: Returns pre-configured responses in order; supports
  streaming (one char at a time)
- `FakeEmbeddings`: Returns deterministic vectors based on content hash;
  configurable dimensionality

These are in the main library (not behind a feature flag) because they are
essential for users writing tests, not just for testing synwire-core itself.

## 19. Reference Implementations (`synwire` crate)

**Decision**: Items previously excluded as "application-level concerns"
are provided as concrete reference implementations in the `synwire`
convenience crate, mirroring Python's `synwire` package layering on
top of `synwire-core`.

**Rationale**: Python's ecosystem has three layers:
1. `synwire-core` — abstract traits/base classes
2. `synwire` — convenience implementations (text splitters, few-shot,
   caching, history management, additional parsers)
3. `synwire-{provider}` — provider integrations

The Rust port should mirror this. Excluding application-level patterns
from `synwire-core` is correct (keeps core lean), but users expect
ready-to-use implementations for common patterns without building them
from scratch.

**What goes in the `synwire` crate:**

| Category | Items | Dependencies |
|---|---|---|
| Embedding cache | `CacheBackedEmbeddings`, `EmbeddingCache` trait, `InMemoryEmbeddingCache` | `moka` |
| Chat history | `ChatMessageHistory` trait, `InMemoryChatMessageHistory`, `RunnableWithMessageHistory` | none (core types) |
| Few-shot prompts | `FewShotPromptTemplate`, `FewShotChatMessagePromptTemplate`, `ExampleSelector` trait, `SemanticSimilarityExampleSelector` | core VectorStore |
| Text splitters | `CharacterTextSplitter`, `RecursiveCharacterTextSplitter` | none |
| Additional parsers | `CommaSeparatedListOutputParser`, `EnumOutputParser`, `XMLOutputParser`, `RegexParser`, `RetryOutputParser`, `CombiningOutputParser` | `quick-xml`, `regex` |

**What goes in provider crates:**
- `OpenAIModerationMiddleware` → `synwire-llm-openai::moderation`

**Alternatives considered:**
- Separate `synwire-extras` crate: Rejected — adds another crate to
  manage; `synwire` is the natural home matching Python's structure.
- Feature-gating each module: Considered but rejected for initial port —
  the dependency cost is minimal (`moka`, `quick-xml`, `regex` are small).
  Can add feature flags later if `synwire` crate grows too large.

## 20. Provider Architecture

**Decision**: A focused set of providers is in scope. The `synwire-llm-openai`
crate provides the primary LLM integration with `BaseChatOpenAI` as a shared
base type. `synwire-llm-ollama` provides local LLM access. Additional
providers cover vector stores, graph stores, search, and workflow orchestration.

**Rationale**: A focused provider set avoids spreading effort across too many
thin wrappers. Users needing additional LLM providers can use OpenAI-compatible
endpoints via `BaseChatOpenAI` with custom `api_base`. The provider set covers
the key categories: LLM (OpenAI, Ollama), vector store (Qdrant, pgvector),
graph store (Neo4j), search (SerpApi, SearxNG, NCBI, arXiv), and workflow
(Temporal).

### Provider Categories

| Category | Crate | Code Sharing |
|---|---|---|
| LLM — OpenAI | synwire-llm-openai | Full impl with BaseChatOpenAI |
| LLM — Ollama | synwire-llm-ollama | Own HTTP client (NDJSON streaming) |
| Vector Store | synwire-vectorstore-qdrant | Own client SDK, implement VectorStore trait |
| Vector Store | synwire-vectorstore-pgvector | sqlx + pgvector extension |
| Graph Store | synwire-graphstore-neo4j | neo4rs driver, implement GraphStore trait |
| Search | synwire-search-serpapi | Own HTTP client, implement Retriever trait |
| Search | synwire-search-searxng | Own HTTP client, implement Retriever trait |
| Search | synwire-search-ncbi | Own HTTP client, implement Retriever trait |
| Search | synwire-search-arxiv | Own HTTP client, implement Retriever trait |
| Workflow | synwire-workflow-temporal | temporal-sdk, workflow/activity definitions |

### BaseChatOpenAI Pattern

```rust
// synwire-llm-openai/src/base.rs — pub(crate) shared base for OpenAI-compatible providers
pub struct BaseChatOpenAI {
    pub(crate) model: String,
    pub(crate) api_key: String,
    pub(crate) api_base: String,
    pub(crate) api_key_env: &'static str,
    pub(crate) temperature: Option<f32>,
    pub(crate) max_tokens: Option<u32>,
    pub(crate) top_p: Option<f32>,
    pub(crate) stop: Option<Vec<String>>,
    pub(crate) timeout: Option<Duration>,
    pub(crate) max_retries: u32,
    pub(crate) model_kwargs: HashMap<String, Value>,
    pub(crate) client: reqwest_middleware::ClientWithMiddleware,
}
```

Users needing OpenAI-compatible providers (e.g. Groq, DeepSeek, xAI) can
instantiate `BaseChatOpenAI` directly with a custom `api_base` and
`api_key_env` rather than requiring dedicated crates.

### Ollama-Specific Decisions

- **No API key**: Ollama runs locally; no authentication needed.
- **NDJSON streaming**: Ollama uses newline-delimited JSON for streaming,
  not Server-Sent Events. Each line is a JSON object.
- **Model management**: Ollama requires models to be pulled first. The
  crate does NOT auto-pull models — returns an error if model not found.

### Vector Store Decisions

- **Qdrant**: REST by default; optional gRPC via `tonic` + `qdrant-client`
  behind a feature flag. REST is simpler to implement initially.
- **pgvector**: Requires PostgreSQL with the `vector` extension. Uses `sqlx`
  for connection pooling and query execution.

### Graph Store Decisions

- **Neo4j**: Uses `neo4rs` Rust driver. Supports Cypher queries for graph
  retrieval and knowledge graph construction.

**Alternatives considered**:
- Single `synwire-providers` mega-crate: Rejected — forces all provider
  dependencies on every user. Per-provider crates allow selective inclusion.
- tower middleware for provider HTTP clients: Rejected — `reqwest-middleware`
  is simpler and already used in synwire-llm-openai. tower's `Service` pattern
  adds unnecessary complexity for straightforward HTTP clients.
