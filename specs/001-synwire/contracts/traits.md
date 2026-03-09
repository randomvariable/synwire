# Public Trait Contracts: synwire-core — M1

**Date**: 2026-03-09
**Branch**: `001-synwire`
**Scope**: M1 traits only. Agent-layer traits (M2) and protocol traits (M3) are deferred.

> Architecture review fixes applied:
> - §1.3: `Arc<dyn CallbackHandler>` in `RunnableConfig` (cheaply cloneable)
> - §2.1: Layered error types (`ModelError`, `GraphError`, etc.)
> - §2.2: Split `Runnable` into `RunnableCore` + `ObservableRunnable`; dropped `stream_log`
> - §3.4: RPITIT candidates identified (static dispatch where dyn not needed)
> - §4.1: Builder pattern for `filter_messages`
> - §5.5: `#[non_exhaustive]` on all enums

All trait signatures use manual BoxFuture desugaring for dyn-compatibility
unless noted as RPITIT candidates (§3.4).

---

## MessageLike

Static dispatch candidate (§3.4) — no dyn needed.

```rust
/// Trait for types that can be converted into a Message.
pub trait MessageLike: Send + Sync {
    fn to_message(&self) -> Message;
}

// Blanket implementations:
// impl MessageLike for Message { identity }
// impl MessageLike for &str { wraps in Human message }
// impl MessageLike for String { wraps in Human message }
// impl MessageLike for (MessageRole, &str) { message of given role }
```

## Message Utility Functions

> §4.1: `filter_messages` uses builder pattern instead of 7 positional params.

```rust
/// Builder for filtering messages by type, name, or ID.
pub struct MessageFilter {
    include_types: Option<Vec<MessageType>>,
    exclude_types: Option<Vec<MessageType>>,
    include_names: Option<Vec<String>>,
    exclude_names: Option<Vec<String>>,
    include_ids: Option<Vec<String>>,
    exclude_ids: Option<Vec<String>>,
}

impl MessageFilter {
    pub fn new() -> Self;
    pub fn include_types(self, types: &[MessageType]) -> Self;
    pub fn exclude_types(self, types: &[MessageType]) -> Self;
    pub fn include_names(self, names: &[impl AsRef<str>]) -> Self;
    pub fn exclude_names(self, names: &[impl AsRef<str>]) -> Self;
    pub fn include_ids(self, ids: &[impl AsRef<str>]) -> Self;
    pub fn exclude_ids(self, ids: &[impl AsRef<str>]) -> Self;
    pub fn apply(&self, messages: &[Message]) -> Vec<Message>;
}

/// Trim messages to fit within a token budget.
pub fn trim_messages(
    messages: &[Message],
    max_tokens: usize,
    token_counter: &dyn Fn(&Message) -> usize,
    strategy: TrimStrategy,
    allow_partial: bool,
    start_on: Option<MessageType>,
    include_system: bool,
) -> Vec<Message>;

/// Merge consecutive messages of the same type.
pub fn merge_message_runs(messages: &[Message]) -> Vec<Message>;

/// Emit a custom event for stream_events.
pub fn dispatch_custom_event(
    name: &str,
    data: Value,
    config: &RunnableConfig,
) -> Result<(), SynwireError>;
```

## BaseChatModel

```rust
pub trait BaseChatModel: Send + Sync + Debug {
    /// Invoke the model with a list of messages.
    fn invoke<'a>(
        &'a self,
        messages: &'a [Message],
        config: Option<&'a RunnableConfig>,
        stop: Option<&'a [String]>,
    ) -> BoxFuture<'a, Result<ChatResult, ModelError>>;

    /// Invoke for multiple inputs concurrently.
    fn batch<'a>(
        &'a self,
        inputs: &'a [Vec<Message>],
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Vec<ChatResult>, ModelError>>;

    /// Stream model output as chunks.
    fn stream<'a>(
        &'a self,
        messages: &'a [Message],
        config: Option<&'a RunnableConfig>,
        stop: Option<&'a [String]>,
    ) -> BoxFuture<'a, Result<BoxStream<'a, Result<ChatChunk, ModelError>>, ModelError>>;

    /// Return the model identifier (e.g. "gpt-4o", "claude-3").
    fn model_type(&self) -> &str;

    /// Return a new model instance with tools pre-configured.
    fn bind_tools<'a>(
        &'a self,
        tools: Vec<ToolSchema>,
    ) -> BoxFuture<'a, Result<Box<dyn BaseChatModel>, ModelError>>;

    /// Return a Runnable that parses output into type T.
    fn with_structured_output(
        &self,
        schema: ToolSchema,
    ) -> Box<dyn RunnableCore<Vec<Message>, Value>>;
}
```

> **Error type**: Returns `ModelError` (layered, §2.1) not `SynwireError`.
> Callers can convert via `SynwireError::from(ModelError)`.

### Intentional Exclusions

- **`generate` / `generate_prompt`**: Internal dispatch in Python. Rust has `invoke` + `batch`.
- **`LanguageModelInput` union**: Rust requires `&[Message]`; callers convert via `Into<Vec<Message>>`.

## BaseLLM

```rust
pub trait BaseLLM: Send + Sync + Debug {
    fn invoke<'a>(
        &'a self,
        prompt: &'a str,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<String, ModelError>>;

    fn batch<'a>(
        &'a self,
        prompts: &'a [String],
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Vec<String>, ModelError>>;

    fn stream<'a>(
        &'a self,
        prompt: &'a str,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<BoxStream<'a, Result<String, ModelError>>, ModelError>>;

    fn model_type(&self) -> &str;
}
```

## Embeddings

Static dispatch candidate (§3.4) — often used monomorphically.

```rust
pub trait Embeddings: Send + Sync + Debug {
    fn embed_documents<'a>(
        &'a self,
        texts: &'a [String],
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Vec<Vec<f32>>, EmbeddingError>>;

    fn embed_query<'a>(
        &'a self,
        text: &'a str,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Vec<f32>, EmbeddingError>>;
}
```

## VectorStore

> §4.1: Consider splitting into `VectorStoreCore` (add/search/delete) +
> `VectorStoreMmr` extension trait. For M1, kept as single trait.

```rust
pub trait VectorStore: Send + Sync + Debug {
    fn add_documents<'a>(
        &'a self,
        documents: &'a [Document],
        ids: Option<&'a [String]>,
    ) -> BoxFuture<'a, Result<Vec<String>, VectorStoreError>>;

    fn add_texts<'a>(
        &'a self,
        texts: &'a [String],
        metadatas: Option<&'a [HashMap<String, Value>]>,
        ids: Option<&'a [String]>,
    ) -> BoxFuture<'a, Result<Vec<String>, VectorStoreError>>;

    fn similarity_search<'a>(
        &'a self,
        query: &'a str,
        k: usize,
        filter: Option<&'a MetadataFilter>,
    ) -> BoxFuture<'a, Result<Vec<Document>, VectorStoreError>>;

    fn similarity_search_with_score<'a>(
        &'a self,
        query: &'a str,
        k: usize,
        filter: Option<&'a MetadataFilter>,
    ) -> BoxFuture<'a, Result<Vec<(Document, f32)>, VectorStoreError>>;

    fn similarity_search_by_vector<'a>(
        &'a self,
        embedding: &'a [f32],
        k: usize,
        filter: Option<&'a MetadataFilter>,
    ) -> BoxFuture<'a, Result<Vec<Document>, VectorStoreError>>;

    fn get_by_ids<'a>(
        &'a self,
        ids: &'a [String],
    ) -> BoxFuture<'a, Result<Vec<Document>, VectorStoreError>>;

    fn delete<'a>(
        &'a self,
        ids: &'a [String],
    ) -> BoxFuture<'a, Result<(), VectorStoreError>>;

    fn max_marginal_relevance_search<'a>(
        &'a self,
        query: &'a str,
        k: usize,
        fetch_k: usize,
        lambda_mult: f32,
    ) -> BoxFuture<'a, Result<Vec<Document>, VectorStoreError>>;

    fn max_marginal_relevance_search_by_vector<'a>(
        &'a self,
        embedding: &'a [f32],
        k: usize,
        fetch_k: usize,
        lambda_mult: f32,
    ) -> BoxFuture<'a, Result<Vec<Document>, VectorStoreError>>;

    fn embeddings(&self) -> Option<&dyn Embeddings>;
    fn as_retriever(&self, k: usize) -> Box<dyn Retriever>;
}
```

**`k` parameter**: Explicit on every call (no default). Callers define constants.

### Intentional Exclusions

- **`similarity_search_with_relevance_scores`**: Covered by `similarity_search_with_score`.
- **`from_documents` / `from_texts`**: Factory methods belong on concrete implementations.

## RunnableCore (was: Runnable)

> **Architecture review fix §2.2**: Split into `RunnableCore` (invoke, batch, stream)
> and `ObservableRunnable` (stream_events). Dropped `stream_log`, `transform`,
> `batch_as_completed` from the core trait.

```rust
/// Core execution trait. All runnables implement this.
pub trait RunnableCore<I, O>: Send + Sync
where
    I: Send + Sync,
    O: Send + Sync,
{
    /// Transform a single input into an output.
    fn invoke<'a>(
        &'a self,
        input: I,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<O, SynwireError>>
    where
        I: 'a;

    /// Transform multiple inputs concurrently.
    fn batch<'a>(
        &'a self,
        inputs: Vec<I>,
        config: Option<&'a RunnableConfig>,
        return_exceptions: bool,
    ) -> BoxFuture<'a, Result<Vec<Result<O, SynwireError>>, SynwireError>>
    where
        I: 'a;

    /// Stream the output for a single input.
    fn stream<'a>(
        &'a self,
        input: I,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<BoxStream<'a, Result<O, SynwireError>>, SynwireError>>
    where
        I: 'a;
}
```

## ObservableRunnable (extension trait)

> **Architecture review fix §2.2**: Observability methods separated from core.

```rust
/// Extension trait for observable execution. Opt-in for runnables
/// that support structured event streaming.
pub trait ObservableRunnable<I, O>: RunnableCore<I, O>
where
    I: Send + Sync,
    O: Send + Sync,
{
    /// Stream structured events as the runnable executes.
    fn stream_events<'a>(
        &'a self,
        input: I,
        config: Option<&'a RunnableConfig>,
        include_names: Option<&'a [String]>,
        include_types: Option<&'a [String]>,
        include_tags: Option<&'a [String]>,
        exclude_names: Option<&'a [String]>,
        exclude_types: Option<&'a [String]>,
        exclude_tags: Option<&'a [String]>,
    ) -> BoxFuture<'a, Result<BoxStream<'a, Result<StreamEvent, SynwireError>>, SynwireError>>
    where
        I: 'a;

    /// Stream-to-stream transformation.
    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, Result<I, SynwireError>>,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<BoxStream<'a, Result<O, SynwireError>>, SynwireError>>
    where
        I: 'a;

    /// Yield (index, result) as each batch item completes (completion order).
    fn batch_as_completed<'a>(
        &'a self,
        inputs: Vec<I>,
        config: Option<&'a RunnableConfig>,
        return_exceptions: bool,
    ) -> BoxFuture<'a, Result<BoxStream<'a, (usize, Result<O, SynwireError>)>, SynwireError>>
    where
        I: 'a;
}
```

### Dropped from Runnable

- **`stream_log`**: Python/LangSmith legacy. No Rust equivalent need. Use OTel tracing spans instead.

### Composition Functions

Free functions, not trait methods (preserves dyn-compatibility):

```rust
/// Chain two runnables: output of first feeds into second.
pub fn pipe<I, M, O>(
    first: Box<dyn RunnableCore<I, M>>,
    second: Box<dyn RunnableCore<M, O>>,
) -> RunnableSequence<I, O>;

/// Return a runnable with pre-applied config.
pub fn with_config<I, O>(
    runnable: Box<dyn RunnableCore<I, O>>,
    config: RunnableConfig,
) -> RunnableWithConfig<I, O>;

/// Wrap with exponential backoff retry.
pub fn with_retry<I, O>(
    runnable: Box<dyn RunnableCore<I, O>>,
    config: RetryConfig,
) -> RunnableRetry<I, O>;

/// Try primary; on failure, try fallbacks in order.
pub fn with_fallbacks<I, O>(
    primary: Box<dyn RunnableCore<I, O>>,
    fallbacks: Vec<Box<dyn RunnableCore<I, O>>>,
    exceptions_to_handle: Vec<SynwireErrorKind>,
) -> RunnableWithFallbacks<I, O>;

/// Convert a Runnable into a Tool.
pub fn as_tool<I, O>(
    runnable: Box<dyn RunnableCore<I, O>>,
    name: Option<String>,
    description: Option<String>,
    schema: Option<ToolSchema>,
) -> RunnableTool<I, O>;
```

### Concrete Runnable Types

**RunnableSequence** (pipe):
```rust
pub struct RunnableSequence<I, O> {
    first: Box<dyn RunnableCore<I, M>>,
    second: Box<dyn RunnableCore<M, O>>,
}
// Implements RunnableCore<I, O>
```

**RunnableParallel** (concurrent named steps):
```rust
pub struct RunnableParallel<I> {
    steps: HashMap<String, Box<dyn RunnableCore<I, Value>>>,
}
// Implements RunnableCore<I, HashMap<String, Value>>
```

**RunnablePassthrough** (forward input unchanged):
```rust
pub struct RunnablePassthrough;
// Implements RunnableCore<I, I> where I: Clone
```

**RunnableLambda** (closure wrapper):
```rust
pub struct RunnableLambda<I, O> {
    func: Box<dyn Fn(I) -> BoxFuture<'static, Result<O, SynwireError>> + Send + Sync>,
    name: Option<String>,
}
// Implements RunnableCore<I, O>

impl<I, O> RunnableLambda<I, O> {
    pub fn new<F, Fut>(func: F) -> Self
    where
        F: Fn(I) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<O, SynwireError>> + Send + 'static;
    pub fn with_name(self, name: impl Into<String>) -> Self;
}
```

**RunnableBranch** (conditional routing):
```rust
pub struct RunnableBranch<I, O> {
    branches: Vec<(
        Box<dyn Fn(&I) -> bool + Send + Sync>,
        Box<dyn RunnableCore<I, O>>,
    )>,
    default: Box<dyn RunnableCore<I, O>>,
}
// Implements RunnableCore<I, O>
```

### Runnable — Intentional Exclusions

- **`bind` / `RunnableBinding`**: Python kwargs currying. Rust uses typed config structs.
- **Schema introspection**: Rust generics provide compile-time type info.
- **`pick` / `assign`**: Python dict manipulation. Rust uses typed composition.
- **`RunnableSerializable`**: Pydantic-specific. Serde handles serialisation.
- **`RunnableGenerator`**: Use `RunnableLambda` with `BoxStream` return.
- **`RunnableWithMessageHistory`**: Reference impl in `synwire` crate, not core.
- **`RouterRunnable`**: `RunnableBranch` covers conditional routing.
- **`ConfigurableField`**: Rust uses builders and `RunnableConfig.configurable`.

## Tool

```rust
pub trait Tool: Send + Sync + Debug {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> ToolSchema;

    fn invoke<'a>(
        &'a self,
        input: Value,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Value, ToolError>>;
}
```

> Returns `ToolError` (layered, §2.1).

### StructuredTool (concrete)

```rust
pub struct StructuredTool {
    name: String,
    description: String,
    schema: ToolSchema,
    func: Box<dyn Fn(Value) -> BoxFuture<'static, Result<ToolOutput, ToolError>> + Send + Sync>,
}

impl StructuredTool {
    pub fn builder() -> StructuredToolBuilder;
}

pub struct StructuredToolBuilder { /* ... */ }
impl StructuredToolBuilder {
    pub fn name(self, name: impl Into<String>) -> Self;
    pub fn description(self, desc: impl Into<String>) -> Self;
    pub fn schema(self, schema: ToolSchema) -> Self;
    pub fn func<F, Fut>(self, func: F) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<ToolOutput, ToolError>> + Send + 'static;
    pub fn build(self) -> Result<StructuredTool, ToolError>;
}
```

> §4.7: `#[tool]` proc-macro in `synwire-derive` generates `StructuredTool` from
> annotated functions. Uses `schemars` for JSON Schema generation.

## OutputParser

Static dispatch candidate (§3.4).

```rust
pub trait OutputParser<T>: Send + Sync
where
    T: Send,
{
    fn parse<'a>(
        &'a self,
        text: &'a str,
    ) -> BoxFuture<'a, Result<T, ParseError>>;

    fn parse_result<'a>(
        &'a self,
        result: &'a [Generation],
    ) -> BoxFuture<'a, Result<T, ParseError>> {
        // Default: parse(result[0].text)
    }

    fn parse_with_prompt<'a>(
        &'a self,
        text: &'a str,
        _prompt: &'a PromptValue,
    ) -> BoxFuture<'a, Result<T, ParseError>> {
        self.parse(text)
    }

    fn get_format_instructions(&self) -> Option<String> { None }
}
```

**Concrete parsers** (in synwire-core):
- `StrOutputParser` — identity, returns raw text
- `JsonOutputParser` — `serde_json::from_str`
- `StructuredOutputParser<T: DeserializeOwned>` — typed JSON parsing
- `ToolsOutputParser` — extracts `Vec<ToolCall>` from AI message

**Reference impls** (in `synwire` crate): XML, CSV list, enum, regex, retry, combining.

## Retriever

Static dispatch candidate (§3.4).

```rust
pub trait Retriever: Send + Sync + Debug {
    fn get_relevant_documents<'a>(
        &'a self,
        query: &'a str,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Vec<Document>, SynwireError>>;
}
```

**RetrieverRunnable** adapter:
```rust
pub struct RetrieverRunnable<R: Retriever> { inner: R }
impl<R: Retriever> RunnableCore<String, Vec<Document>> for RetrieverRunnable<R> { /* delegates */ }
```

**VectorStoreRetriever**:
```rust
pub struct VectorStoreRetriever {
    store: Box<dyn VectorStore>,
    k: usize,
    search_type: SearchType,
}

#[non_exhaustive]
pub enum SearchType {
    Similarity,
    Mmr { fetch_k: usize, lambda_mult: f32 },
}
```

**InMemoryVectorStore**: Full `VectorStore` impl for testing. Uses brute-force cosine similarity.

## CallbackHandler

> §1.3: Referenced via `Arc<dyn CallbackHandler>` in `RunnableConfig`.
> All hooks have default no-op implementations. Failures must not interrupt execution.

```rust
pub trait CallbackHandler: Send + Sync {
    // --- Selective filtering ---
    fn ignore_llm(&self) -> bool { false }
    fn ignore_chain(&self) -> bool { false }
    fn ignore_tool(&self) -> bool { false }
    fn ignore_retriever(&self) -> bool { false }
    fn ignore_agent(&self) -> bool { false }
    fn ignore_graph(&self) -> bool { false }
    fn ignore_embeddings(&self) -> bool { false }

    // --- LLM hooks ---
    fn on_llm_start<'a>(&'a self, run_id: Uuid, prompts: &'a [String],
        parent_run_id: Option<Uuid>, tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    fn on_chat_model_start<'a>(&'a self, run_id: Uuid, messages: &'a [Message],
        parent_run_id: Option<Uuid>, tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    fn on_llm_new_token<'a>(&'a self, run_id: Uuid, token: &'a str,
        parent_run_id: Option<Uuid>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    fn on_llm_end<'a>(&'a self, run_id: Uuid, response: &'a LLMResult,
        parent_run_id: Option<Uuid>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    fn on_llm_error<'a>(&'a self, run_id: Uuid, error: &'a ModelError,
        parent_run_id: Option<Uuid>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    // --- Chain hooks ---
    fn on_chain_start<'a>(&'a self, run_id: Uuid, inputs: &'a Value,
        parent_run_id: Option<Uuid>, tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    fn on_chain_end<'a>(&'a self, run_id: Uuid, outputs: &'a Value,
        parent_run_id: Option<Uuid>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    fn on_chain_error<'a>(&'a self, run_id: Uuid, error: &'a SynwireError,
        parent_run_id: Option<Uuid>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    // --- Tool hooks ---
    fn on_tool_start<'a>(&'a self, run_id: Uuid, input: &'a Value,
        parent_run_id: Option<Uuid>, tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    fn on_tool_end<'a>(&'a self, run_id: Uuid, output: &'a Value,
        parent_run_id: Option<Uuid>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    fn on_tool_error<'a>(&'a self, run_id: Uuid, error: &'a ToolError,
        parent_run_id: Option<Uuid>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    // --- Retriever hooks ---
    fn on_retriever_start<'a>(&'a self, run_id: Uuid, query: &'a str,
        parent_run_id: Option<Uuid>, tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    fn on_retriever_end<'a>(&'a self, run_id: Uuid, documents: &'a [Document],
        parent_run_id: Option<Uuid>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    fn on_retriever_error<'a>(&'a self, run_id: Uuid, error: &'a SynwireError,
        parent_run_id: Option<Uuid>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    // --- Embedding hooks ---
    fn on_embeddings_start<'a>(&'a self, run_id: Uuid, model: &'a str,
        texts: &'a [String], parent_run_id: Option<Uuid>,
        tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    fn on_embeddings_end<'a>(&'a self, run_id: Uuid, embeddings_count: usize,
        usage: Option<&'a UsageMetadata>,
        parent_run_id: Option<Uuid>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    fn on_embeddings_error<'a>(&'a self, run_id: Uuid, error: &'a EmbeddingError,
        parent_run_id: Option<Uuid>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    // --- Streaming completion hook ---
    fn on_completion_start<'a>(&'a self, run_id: Uuid,
        parent_run_id: Option<Uuid>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    // --- Agent hooks ---
    fn on_agent_action<'a>(&'a self, run_id: Uuid, action: &'a AgentAction,
        parent_run_id: Option<Uuid>, tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    fn on_agent_finish<'a>(&'a self, run_id: Uuid, finish: &'a AgentFinish,
        parent_run_id: Option<Uuid>, tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    // --- Retry hook ---
    fn on_retry<'a>(&'a self, run_id: Uuid, retry_state: &'a RetryState,
        parent_run_id: Option<Uuid>, tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    // --- Graph hooks ---
    fn on_graph_node_start<'a>(&'a self, run_id: Uuid, node_name: &'a str,
        input: &'a Value, parent_run_id: Option<Uuid>,
        tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    fn on_graph_node_end<'a>(&'a self, run_id: Uuid, node_name: &'a str,
        output: &'a Value, parent_run_id: Option<Uuid>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    fn on_graph_node_error<'a>(&'a self, run_id: Uuid, node_name: &'a str,
        error: &'a SynwireGraphError, parent_run_id: Option<Uuid>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    fn on_graph_interrupt<'a>(&'a self, run_id: Uuid, interrupts: &'a [Interrupt],
        parent_run_id: Option<Uuid>, tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    fn on_checkpoint<'a>(&'a self, run_id: Uuid, checkpoint: &'a Checkpoint,
        metadata: &'a CheckpointMetadata,
        parent_run_id: Option<Uuid>) -> BoxFuture<'a, ()> { Box::pin(async {}) }

    // --- Custom event hook ---
    fn on_custom_event<'a>(&'a self, name: &'a str, data: &'a Value,
        run_id: Uuid, tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>) -> BoxFuture<'a, ()> { Box::pin(async {}) }
}
```

### Failure Semantics

All hooks return `()`. Failures must not interrupt execution. Implementations
catch errors internally and log via `tracing::warn!`. Panics are caught and logged.

### Intentional Exclusions

- **`on_text`**: Not useful in typed Rust API.
- **`CallbackManager` hierarchy**: Handled by `RunnableConfig.callbacks` + `run_id`/`parent_run_id`.
- **`collect_runs()`**: Use `tracing` spans behind the `tracing` feature flag.

---

## Checkpoint Traits (synwire-orchestrator / synwire-checkpoint)

### BaseCheckpointSaver

```rust
pub trait BaseCheckpointSaver: Send + Sync {
    fn get_tuple<'a>(
        &'a self,
        config: &'a RunnableConfig,
    ) -> BoxFuture<'a, Result<Option<CheckpointTuple>, SynwireGraphError>>;

    fn list<'a>(
        &'a self,
        config: Option<&'a RunnableConfig>,
        limit: Option<usize>,
        before: Option<&'a RunnableConfig>,
        filter: Option<&'a HashMap<String, Value>>,
    ) -> BoxFuture<'a, Result<BoxStream<'a, CheckpointTuple>, SynwireGraphError>>;

    fn put<'a>(
        &'a self,
        config: &'a RunnableConfig,
        checkpoint: &'a Checkpoint,
        metadata: &'a CheckpointMetadata,
        new_versions: &'a HashMap<String, ChannelVersion>,
    ) -> BoxFuture<'a, Result<RunnableConfig, SynwireGraphError>>;

    fn put_writes<'a>(
        &'a self,
        config: &'a RunnableConfig,
        writes: &'a [(String, String, Value)],
        task_id: &'a str,
    ) -> BoxFuture<'a, Result<(), SynwireGraphError>>;

    fn get_next_version(
        &self,
        current: Option<&ChannelVersion>,
        channel: &str,
    ) -> ChannelVersion;
}
```

### BaseStore

```rust
pub trait BaseStore: Send + Sync {
    fn get<'a>(
        &'a self,
        namespace: &'a [String],
        key: &'a str,
    ) -> BoxFuture<'a, Result<Option<Item>, SynwireGraphError>>;

    fn search<'a>(
        &'a self,
        namespace_prefix: &'a [String],
        filter: Option<&'a HashMap<String, Value>>,
        limit: usize,
        offset: usize,
        query: Option<&'a str>,
    ) -> BoxFuture<'a, Result<Vec<SearchItem>, SynwireGraphError>>;

    fn put<'a>(
        &'a self,
        namespace: &'a [String],
        key: &'a str,
        value: Option<&'a HashMap<String, Value>>,
        index: Option<&'a IndexDirective>,
    ) -> BoxFuture<'a, Result<(), SynwireGraphError>>;

    fn list_namespaces<'a>(
        &'a self,
        match_conditions: Option<&'a [MatchCondition]>,
        max_depth: Option<usize>,
        limit: usize,
        offset: usize,
    ) -> BoxFuture<'a, Result<Vec<Vec<String>>, SynwireGraphError>>;

    fn batch<'a>(
        &'a self,
        ops: &'a [StoreOp],
    ) -> BoxFuture<'a, Result<Vec<StoreResult>, SynwireGraphError>>;
}
```

---

## Additional Traits (synwire-core)

### DocumentLoader

```rust
pub trait DocumentLoader: Send + Sync {
    fn load<'a>(&'a self) -> BoxFuture<'a, Result<Vec<Document>, SynwireError>>;
    fn load_lazy<'a>(&'a self) -> BoxFuture<'a, Result<BoxStream<'a, Result<Document, SynwireError>>, SynwireError>>;
}
```

### Reranker

```rust
pub trait Reranker: Send + Sync {
    fn rerank<'a>(
        &'a self,
        query: &'a str,
        documents: &'a [Document],
        top_n: usize,
    ) -> BoxFuture<'a, Result<Vec<(Document, f32)>, SynwireError>>;
}
```

### CredentialProvider

```rust
pub trait CredentialProvider: Send + Sync {
    fn get_credential<'a>(
        &'a self,
        key: &'a str,
    ) -> BoxFuture<'a, Result<SecretValue, SynwireError>>;
}
```

Built-in: `EnvCredentialProvider`, `StaticCredentialProvider`.

### HttpClientFactory

```rust
pub trait HttpClientFactory: Send + Sync {
    fn create_client(
        &self,
        config: &HttpClientConfig,
    ) -> Result<reqwest::Client, SynwireError>;
}
```

Default: `DefaultHttpClientFactory` with SSRF protection (§2.6: DNS pinning).
