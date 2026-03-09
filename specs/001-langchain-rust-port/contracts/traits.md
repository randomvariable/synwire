# Public Trait Contracts: langchain-core

**Date**: 2026-03-09
**Branch**: `001-langchain-rust-port`

All trait signatures use manual BoxFuture desugaring for dyn-compatibility.
Lifetimes tie futures to `&self` and input references.

## MessageLike

```rust
/// Trait for types that can be converted into a Message.
/// Enables ergonomic APIs that accept various input types.
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

```rust
/// Filter messages by type, name, or ID.
pub fn filter_messages(
    messages: &[Message],
    include_types: Option<&[MessageType]>,
    exclude_types: Option<&[MessageType]>,
    include_names: Option<&[String]>,
    exclude_names: Option<&[String]>,
    include_ids: Option<&[String]>,
    exclude_ids: Option<&[String]>,
) -> Vec<Message>;

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

/// Emit a custom event that will be picked up by stream_events.
/// Must be called within a Runnable execution context.
pub fn dispatch_custom_event(
    name: &str,
    data: Value,
    config: &RunnableConfig,
) -> Result<(), LangChainError>;
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
    ) -> BoxFuture<'a, Result<ChatResult, LangChainError>>;

    /// Invoke the model for multiple inputs concurrently.
    fn batch<'a>(
        &'a self,
        inputs: &'a [Vec<Message>],
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Vec<ChatResult>, LangChainError>>;

    /// Stream model output as chunks.
    fn stream<'a>(
        &'a self,
        messages: &'a [Message],
        config: Option<&'a RunnableConfig>,
        stop: Option<&'a [String]>,
    ) -> BoxFuture<'a, Result<BoxStream<'a, Result<ChatChunk, LangChainError>>, LangChainError>>;

    /// Return the model identifier (e.g. "gpt-4o", "claude-3").
    fn model_type(&self) -> &str;

    /// Return a new model instance with the given tools pre-configured.
    /// The returned model will include tool definitions in every request.
    fn bind_tools<'a>(
        &'a self,
        tools: Vec<ToolSchema>,
    ) -> BoxFuture<'a, Result<Box<dyn BaseChatModel>, LangChainError>>;

    /// Return a Runnable that invokes this model and parses output into type T.
    /// Uses tool-calling or JSON mode depending on provider capabilities.
    fn with_structured_output(
        &self,
        schema: ToolSchema,
    ) -> Box<dyn Runnable<Vec<Message>, Value>>;
}
```

### BaseChatModel — Return Type Mapping

Python `invoke` returns `AIMessage` directly. Rust returns `ChatResult`
wrapping the message plus `generation_info`. This provides a uniform place
for provider metadata without overloading the `Message` type. Callers
access the message via `chat_result.message`.

### BaseChatModel — Intentional Exclusions

- **`generate` / `generate_prompt`**: Internal dispatch layer in Python.
  Rust exposes `invoke` (single) and `batch` (multiple) directly. No
  user-facing value in separate methods.
- **`LanguageModelInput` union** (str | list[BaseMessage] | PromptValue):
  Python accepts multiple input types. Rust requires `&[Message]`; callers
  convert via `Into<Vec<Message>>` implementations on `PromptValue` and
  `&str` (wraps in a HumanMessage). See research.md §3.

## BaseLLM

```rust
pub trait BaseLLM: Send + Sync + Debug {
    /// Invoke the model with a text prompt.
    fn invoke<'a>(
        &'a self,
        prompt: &'a str,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<String, LangChainError>>;

    /// Invoke the model for multiple prompts concurrently.
    fn batch<'a>(
        &'a self,
        prompts: &'a [String],
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Vec<String>, LangChainError>>;

    /// Stream model output as text chunks.
    fn stream<'a>(
        &'a self,
        prompt: &'a str,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<BoxStream<'a, Result<String, LangChainError>>, LangChainError>>;

    fn model_type(&self) -> &str;
}
```

### BaseLLM — Intentional Exclusions

- **`generate` / `generate_prompt`**: Same rationale as BaseChatModel —
  Rust uses `invoke` (single) and `batch` (multiple) directly.
- **`dict` / `save`**: Serialisation handled by serde `Serialize` /
  `Deserialize` derives on concrete implementations, not trait methods.

## Embeddings

```rust
pub trait Embeddings: Send + Sync + Debug {
    /// Embed a list of texts, returning one vector per text.
    fn embed_documents<'a>(
        &'a self,
        texts: &'a [String],
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Vec<Vec<f32>>, LangChainError>>;

    /// Embed a single query text.
    fn embed_query<'a>(
        &'a self,
        text: &'a str,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Vec<f32>, LangChainError>>;
}
```

### Embeddings — Design Note

Python has separate `embed_documents` / `aembed_documents` and
`embed_query` / `aembed_query`. Rust collapses async/sync duality to a
single async method per operation. See research.md §3 and §7 for the
per-trait async mapping and §9 for sync wrappers via the `blocking` module.

`config` parameter added for consistency with all other traits that accept
`RunnableConfig` — enables callback propagation and metadata passing through
embedding operations.

## VectorStore

```rust
pub trait VectorStore: Send + Sync + Debug {
    /// Add documents to the store, returning their IDs.
    fn add_documents<'a>(
        &'a self,
        documents: &'a [Document],
        ids: Option<&'a [String]>,
    ) -> BoxFuture<'a, Result<Vec<String>, LangChainError>>;

    /// Add raw text strings with optional metadata, returning their IDs.
    /// Convenience method — implementations typically convert to Documents
    /// and delegate to add_documents.
    fn add_texts<'a>(
        &'a self,
        texts: &'a [String],
        metadatas: Option<&'a [HashMap<String, Value>]>,
        ids: Option<&'a [String]>,
    ) -> BoxFuture<'a, Result<Vec<String>, LangChainError>>;

    /// Search for documents similar to a query string.
    /// `k` must be specified explicitly (no default; see design note below).
    fn similarity_search<'a>(
        &'a self,
        query: &'a str,
        k: usize,
    ) -> BoxFuture<'a, Result<Vec<Document>, LangChainError>>;

    /// Search with relevance scores.
    fn similarity_search_with_score<'a>(
        &'a self,
        query: &'a str,
        k: usize,
    ) -> BoxFuture<'a, Result<Vec<(Document, f32)>, LangChainError>>;

    /// Search by pre-computed embedding vector.
    fn similarity_search_by_vector<'a>(
        &'a self,
        embedding: &'a [f32],
        k: usize,
    ) -> BoxFuture<'a, Result<Vec<Document>, LangChainError>>;

    /// Retrieve documents by their IDs (no similarity search).
    fn get_by_ids<'a>(
        &'a self,
        ids: &'a [String],
    ) -> BoxFuture<'a, Result<Vec<Document>, LangChainError>>;

    /// Delete documents by ID.
    fn delete<'a>(
        &'a self,
        ids: &'a [String],
    ) -> BoxFuture<'a, Result<(), LangChainError>>;

    /// Return docs selected using maximal marginal relevance.
    /// Optimizes for similarity to query AND diversity among selected docs.
    /// `fetch_k` is the number of candidates to fetch before MMR re-ranking.
    /// `lambda_mult` controls diversity: 0.0 = max diversity, 1.0 = min.
    fn max_marginal_relevance_search<'a>(
        &'a self,
        query: &'a str,
        k: usize,
        fetch_k: usize,
        lambda_mult: f32,
    ) -> BoxFuture<'a, Result<Vec<Document>, LangChainError>>;

    /// MMR search by pre-computed embedding vector.
    fn max_marginal_relevance_search_by_vector<'a>(
        &'a self,
        embedding: &'a [f32],
        k: usize,
        fetch_k: usize,
        lambda_mult: f32,
    ) -> BoxFuture<'a, Result<Vec<Document>, LangChainError>>;

    /// Return the underlying Embeddings instance, if available.
    fn embeddings(&self) -> Option<&dyn Embeddings>;

    /// Return a Retriever backed by this vector store with the given k.
    fn as_retriever(&self, k: usize) -> Box<dyn Retriever>;
}
```

### VectorStore — Design Notes

**`k` parameter**: Python defaults to `k=4`. Rust requires explicit `k` on
every call — explicit is better than implicit in Rust APIs. Callers define
their own constants if a default is desired.

**MMR algorithm**: The core MMR scoring function (cosine similarity + diversity
penalty) is provided as a standalone utility in `vectorstores/mmr.rs` so
concrete implementations can share it.

### VectorStore — Intentional Exclusions

- **`similarity_search_with_relevance_scores`**: Covered by
  `similarity_search_with_score`. Python has both for historical reasons;
  Rust unifies them into a single method.
- **`from_documents` / `from_texts`**: Factory class methods in Python.
  Rust uses constructors + `add_documents`. Factory methods don't map well
  to trait definitions; they belong on concrete implementations.

## Runnable

```rust
pub trait Runnable<I, O>: Send + Sync
where
    I: Send + Sync,
    O: Send + Sync,
{
    /// Transform a single input into an output.
    fn invoke<'a>(
        &'a self,
        input: I,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<O, LangChainError>>
    where
        I: 'a;

    /// Transform multiple inputs concurrently.
    /// When `return_exceptions` is true, individual failures are returned
    /// inline as Err values rather than failing the entire batch.
    fn batch<'a>(
        &'a self,
        inputs: Vec<I>,
        config: Option<&'a RunnableConfig>,
        return_exceptions: bool,
    ) -> BoxFuture<'a, Result<Vec<Result<O, LangChainError>>, LangChainError>>
    where
        I: 'a;

    /// Stream the output for a single input.
    fn stream<'a>(
        &'a self,
        input: I,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<BoxStream<'a, Result<O, LangChainError>>, LangChainError>>
    where
        I: 'a;

    /// Transform an input stream into an output stream (stream-to-stream).
    /// Default implementation buffers input then calls stream().
    /// Override for true streaming transformations.
    fn transform<'a>(
        &'a self,
        input: BoxStream<'a, Result<I, LangChainError>>,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<BoxStream<'a, Result<O, LangChainError>>, LangChainError>>
    where
        I: 'a;

    /// Run invoke on multiple inputs, yielding (index, result) as each completes.
    /// Results are returned in completion order, not input order.
    fn batch_as_completed<'a>(
        &'a self,
        inputs: Vec<I>,
        config: Option<&'a RunnableConfig>,
        return_exceptions: bool,
    ) -> BoxFuture<'a, Result<BoxStream<'a, (usize, Result<O, LangChainError>)>, LangChainError>>
    where
        I: 'a;

    /// Stream structured events as the runnable executes.
    /// Emits on_*_start, on_*_stream, on_*_end, and on_custom_event events.
    /// Filters select which sub-runnables' events to include/exclude.
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
    ) -> BoxFuture<'a, Result<BoxStream<'a, Result<StreamEvent, LangChainError>>, LangChainError>>
    where
        I: 'a;

    /// Stream JSON-Patch style log diffs as the runnable executes.
    /// Each RunLogPatch contains RFC 6902 operations that can be applied
    /// to reconstruct the full execution log.
    fn stream_log<'a>(
        &'a self,
        input: I,
        config: Option<&'a RunnableConfig>,
        include_names: Option<&'a [String]>,
        include_types: Option<&'a [String]>,
        include_tags: Option<&'a [String]>,
        exclude_names: Option<&'a [String]>,
        exclude_types: Option<&'a [String]>,
        exclude_tags: Option<&'a [String]>,
    ) -> BoxFuture<'a, Result<BoxStream<'a, Result<RunLogPatch, LangChainError>>, LangChainError>>
    where
        I: 'a;
}
```

### Runnable — Composition Methods

These are free functions or extension methods, not trait methods, to avoid
making the trait non-dyn-compatible:

```rust
/// Chain two runnables: output of `first` feeds into `second`.
/// Equivalent to Python's `first | second` pipe operator.
pub fn pipe<I, M, O>(
    first: Box<dyn Runnable<I, M>>,
    second: Box<dyn Runnable<M, O>>,
) -> RunnableSequence<I, O>;

/// Return a runnable with a pre-applied config.
pub fn with_config<I, O>(
    runnable: Box<dyn Runnable<I, O>>,
    config: RunnableConfig,
) -> RunnableWithConfig<I, O>;

/// Wrap a runnable with exponential backoff retry on failure.
/// Uses `backoff` crate internally. Retries only on error kinds
/// specified in config.retry_on.
pub fn with_retry<I, O>(
    runnable: Box<dyn Runnable<I, O>>,
    config: RetryConfig,
) -> RunnableRetry<I, O>;

/// Try the primary runnable; on failure, try each fallback in order.
/// Only handles error kinds specified in exceptions_to_handle.
pub fn with_fallbacks<I, O>(
    primary: Box<dyn Runnable<I, O>>,
    fallbacks: Vec<Box<dyn Runnable<I, O>>>,
    exceptions_to_handle: Vec<LangChainErrorKind>,
) -> RunnableWithFallbacks<I, O>;

/// Convert a Runnable into a Tool, wrapping invoke() as the tool's
/// execution method. Name, description, and input schema can be
/// provided explicitly or inferred from the Runnable's type.
/// Equivalent to Python's Runnable.as_tool().
pub fn as_tool<I, O>(
    runnable: Box<dyn Runnable<I, O>>,
    name: Option<String>,
    description: Option<String>,
    schema: Option<ToolSchema>,
) -> RunnableTool<I, O>;
```

### Runnable — Concrete Types

**RunnableSequence** (created by `pipe`):
```rust
pub struct RunnableSequence<I, O> {
    first: Box<dyn Runnable<I, M>>,
    second: Box<dyn Runnable<M, O>>,
}
// Implements Runnable<I, O> — invoke chains first then second
```

**RunnableParallel** (run multiple runnables on the same input, collect results):
```rust
pub struct RunnableParallel<I> {
    steps: HashMap<String, Box<dyn Runnable<I, Value>>>,
}
// Implements Runnable<I, HashMap<String, Value>>
// Runs all steps concurrently, collects named outputs into a map
```

**RunnablePassthrough** (forward input unchanged, optionally assigning new keys):
```rust
pub struct RunnablePassthrough;
// Implements Runnable<I, I> where I: Clone — forwards input as output
```

**RunnableLambda** (wrap a closure as a Runnable):
```rust
pub struct RunnableLambda<I, O> {
    func: Box<dyn Fn(I) -> BoxFuture<'static, Result<O, LangChainError>> + Send + Sync>,
    name: Option<String>,
}
// Implements Runnable<I, O>
// stream() default: wraps invoke result as single-item stream
// The most common way to create ad-hoc transformations in chains
```

Constructor:
```rust
impl<I, O> RunnableLambda<I, O> {
    pub fn new<F, Fut>(func: F) -> Self
    where
        F: Fn(I) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<O, LangChainError>> + Send + 'static;

    pub fn with_name(self, name: impl Into<String>) -> Self;
}
```

**RunnableBranch** (conditional routing based on input):
```rust
pub struct RunnableBranch<I, O> {
    branches: Vec<(
        Box<dyn Fn(&I) -> bool + Send + Sync>,
        Box<dyn Runnable<I, O>>,
    )>,
    default: Box<dyn Runnable<I, O>>,
}
// Implements Runnable<I, O>
// Evaluates conditions in order; runs the first matching branch
// Falls through to default if no condition matches
```

Constructor:
```rust
impl<I, O> RunnableBranch<I, O> {
    pub fn new(
        branches: Vec<(
            Box<dyn Fn(&I) -> bool + Send + Sync>,
            Box<dyn Runnable<I, O>>,
        )>,
        default: Box<dyn Runnable<I, O>>,
    ) -> Self;
}
```

### Runnable — Intentional Exclusions

- **`bind`**: Python uses kwargs currying. Rust uses typed config structs;
  no kwargs equivalent needed.
- **`RunnableBinding`**: Concrete type for `bind` — excluded alongside `bind`.
- **Schema introspection** (`get_input_schema`, `get_output_schema`,
  `InputType`, `OutputType`): Python/Pydantic specific. Rust uses generics
  with compile-time type information.
- **`pick` / `assign` / `RunnableAssign` / `RunnablePick`**: Python dict
  output manipulation. Rust uses typed composition with struct fields.
  `RunnableParallel` covers the primary use case of combining multiple
  outputs into a map.
- **`RunnableSerializable`**: Pydantic-based Runnable for JSON round-trips.
  Not applicable to Rust — serialisation is handled via serde on concrete
  types.
- **`RunnableGenerator`**: Python wraps async generators as Runnables.
  In Rust, use `RunnableLambda` with a closure that returns a `BoxStream`,
  or implement the `Runnable` trait directly with a custom `stream()`.
- **`RunnableWithMessageHistory`**: Automatic conversation history injection.
  Excluded from `langchain-core` — reference implementation provided in the
  `langchain` crate with a pluggable `ChatMessageHistory` trait and
  `InMemoryChatMessageHistory` store.
- **`RouterRunnable`**: Route input to one of several runnables by key.
  `RunnableBranch` covers conditional routing; for key-based dispatch,
  use `RunnableLambda` with a match expression.
- **`ConfigurableField` / `ConfigurableFieldSingleOption` /
  `ConfigurableFieldMultiOption`**: Runtime-configurable runnable parameters.
  Python-specific pattern built on Pydantic validators. Rust uses typed
  builder patterns and `RunnableConfig.configurable` for runtime overrides.
- **`@chain` decorator**: Python converts generator functions to
  `RunnableGenerator`. In Rust, use `RunnableLambda` with closures.

## Tool

```rust
pub trait Tool: Send + Sync + Debug {
    /// The tool's name (used by models to select it).
    fn name(&self) -> &str;

    /// Human-readable description of what the tool does.
    fn description(&self) -> &str;

    /// JSON Schema describing the tool's input parameters.
    fn schema(&self) -> ToolSchema;

    /// Invoke the tool with the given arguments.
    fn invoke<'a>(
        &'a self,
        input: Value,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Value, LangChainError>>;
}
```

### Tool — Intentional Exclusions

- **`_run` / `_arun`**: Python internal pattern separating public `invoke`
  from implementor's `_run`. Rust uses `invoke` directly — trait methods
  ARE the extension point; no public/private split needed.
- **`args_schema`**: Covered by `schema() -> ToolSchema` which returns the
  JSON Schema for the tool's input parameters. Pydantic BaseModel
  validation is replaced by JSON Schema validation.
- **`is_single_input`**: Not applicable. Rust tools accept `Value` (JSON);
  input arity is determined by the JSON Schema, not a runtime property.
- **`run` / `arun`**: Deprecated legacy execution methods in Python. Not
  ported — use `invoke` instead.

## StructuredTool (concrete type)

A convenience type for creating tools from functions without manually
implementing the `Tool` trait. Equivalent to Python's `StructuredTool`.

```rust
pub struct StructuredTool {
    name: String,
    description: String,
    schema: ToolSchema,
    func: Box<dyn Fn(Value) -> BoxFuture<'static, Result<ToolOutput, LangChainError>> + Send + Sync>,
}

impl StructuredTool {
    /// Create a new StructuredTool with a builder pattern.
    pub fn builder() -> StructuredToolBuilder;
}

pub struct StructuredToolBuilder {
    // ...
}

impl StructuredToolBuilder {
    pub fn name(self, name: impl Into<String>) -> Self;
    pub fn description(self, desc: impl Into<String>) -> Self;
    pub fn schema(self, schema: ToolSchema) -> Self;
    pub fn func<F, Fut>(self, func: F) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<ToolOutput, LangChainError>> + Send + 'static;
    pub fn build(self) -> Result<StructuredTool, LangChainError>;
}

// StructuredTool implements Tool trait
```

### Tool Builder — Design Note

Python's `@tool` decorator auto-generates tool definitions from function
signatures and docstrings. Rust does not have an equivalent decorator pattern.
Instead, `StructuredToolBuilder` provides a fluent API for the same purpose.
A proc-macro (`#[tool]`) could be added later but is not in scope for the
initial port — the builder pattern is sufficient and avoids proc-macro
compile-time overhead.

### Tool — Additional Exclusions

- **`Tool` (Python simple class)**: Python's `Tool` class (distinct from
  `BaseTool`) accepts a single string input. Covered by `StructuredTool`
  with a single-field schema. No separate type needed.

## OutputParser

```rust
pub trait OutputParser<T>: Send + Sync
where
    T: Send,
{
    /// Parse raw model output text into a structured type.
    fn parse<'a>(
        &'a self,
        text: &'a str,
    ) -> BoxFuture<'a, Result<T, LangChainError>>;

    /// Parse from a list of Generation objects. Default delegates to
    /// parse() using the first generation's text.
    fn parse_result<'a>(
        &'a self,
        result: &'a [Generation],
    ) -> BoxFuture<'a, Result<T, LangChainError>> {
        // Default: parse(result[0].text)
        Box::pin(async move {
            let text = result.first()
                .map(|g| g.text.as_str())
                .unwrap_or("");
            self.parse(text).await
        })
    }

    /// Parse with access to the original prompt for context.
    /// Default delegates to parse() ignoring the prompt.
    fn parse_with_prompt<'a>(
        &'a self,
        text: &'a str,
        _prompt: &'a PromptValue,
    ) -> BoxFuture<'a, Result<T, LangChainError>> {
        self.parse(text)
    }

    /// Optional: instructions to include in the prompt for this parser.
    fn get_format_instructions(&self) -> Option<String> {
        None
    }
}
```

### Concrete Output Parsers

The following parsers ship with `langchain-core`. Users can implement
`OutputParser<T>` for custom parsing logic.

**StrOutputParser** (identity — returns raw model output as String):
```rust
pub struct StrOutputParser;
// Implements OutputParser<String>
// parse(text) -> Ok(text.to_string())
```

**JsonOutputParser** (parse JSON from model output):
```rust
pub struct JsonOutputParser;
// Implements OutputParser<Value>
// parse(text) -> serde_json::from_str(text)
// get_format_instructions() returns JSON formatting guidance
```

**StructuredOutputParser<T>** (parse into a typed struct via JSON):
```rust
pub struct StructuredOutputParser<T: DeserializeOwned> {
    _phantom: PhantomData<T>,
}
// Implements OutputParser<T>
// parse(text) -> serde_json::from_str::<T>(text)
// Equivalent to Python's PydanticOutputParser
```

**ToolsOutputParser** (extract tool calls from model response):
```rust
pub struct ToolsOutputParser;
// Implements OutputParser<Vec<ToolCall>>
// Parses tool_calls from AIMessage, returning structured ToolCall list
```

### Output Parser — Scope Boundary

These parsers are included because they are required for common workflows:
- `StrOutputParser`: Every chain that produces text output
- `JsonOutputParser`: Structured output without function-calling
- `StructuredOutputParser<T>`: Type-safe structured output
- `ToolsOutputParser`: Function-calling chains

### Output Parser — Excluded from Core (Reference Impls in `langchain` crate)

The following parsers are excluded from `langchain-core` but provided as
reference implementations in the `langchain` convenience crate:

- **`XMLOutputParser`**: Parses XML-formatted model output. Uses `quick-xml`.
- **`CommaSeparatedListOutputParser`**: Splits comma-delimited output into
  `Vec<String>`.
- **`EnumOutputParser`**: Constrains output to one of a set of string
  variants. (Note: Rust enums with `serde::Deserialize` can also use
  `StructuredOutputParser<MyEnum>` directly.)
- **`RegexParser`**: Extracts named groups from model output via regex.
- **`RetryOutputParser`**: Wraps a parser and an LLM — on parse failure,
  sends the error back to the LLM to fix the output.
- **`CombiningOutputParser`**: Merges results from multiple parsers.

### Output Parser — Permanently Excluded

- **`NumberedListOutputParser`**, **`MarkdownListOutputParser`**: Trivial
  string splitting; `CommaSeparatedListOutputParser` covers the pattern.
- **`PydanticOutputParser`**: Python-specific. Rust equivalent is
  `StructuredOutputParser<T: DeserializeOwned>` already in core.

## Retriever

```rust
pub trait Retriever: Send + Sync + Debug {
    /// Retrieve documents relevant to a query.
    fn get_relevant_documents<'a>(
        &'a self,
        query: &'a str,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Vec<Document>, LangChainError>>;
}
```

### Retriever — Design Notes

**Runnable interface**: Retriever implementations SHOULD also implement
`Runnable<String, Vec<Document>>` where `invoke` delegates to
`get_relevant_documents`. This enables composing retrievers in runnable
chains (e.g. `retriever | prompt | model`). A blanket implementation or
adapter struct is provided in `langchain-core`.

**`_get_relevant_documents` pattern**: Python separates the public
`invoke`/`get_relevant_documents` from the implementor's
`_get_relevant_documents` (underscore prefix). Rust does not need this
split — trait methods are the extension point. Implementors override
`get_relevant_documents` directly.

### Retriever — Runnable Adapter

A blanket adapter struct enables any `Retriever` to be used as a
`Runnable<String, Vec<Document>>` in chains:

```rust
pub struct RetrieverRunnable<R: Retriever> {
    inner: R,
}

impl<R: Retriever> Runnable<String, Vec<Document>> for RetrieverRunnable<R> {
    fn invoke<'a>(
        &'a self,
        input: String,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Vec<Document>, LangChainError>> {
        self.inner.get_relevant_documents(&input, config)
    }
    // batch: parallel invoke per query
    // stream: wraps invoke as single-item stream
}
```

### VectorStoreRetriever (concrete type)

The concrete `Retriever` returned by `VectorStore::as_retriever()`.

```rust
pub struct VectorStoreRetriever {
    store: Box<dyn VectorStore>,
    k: usize,
    search_type: SearchType,
}

pub enum SearchType {
    Similarity,
    Mmr { fetch_k: usize, lambda_mult: f32 },
}

impl VectorStoreRetriever {
    pub fn new(store: Box<dyn VectorStore>, k: usize) -> Self;
    pub fn with_mmr(store: Box<dyn VectorStore>, k: usize, fetch_k: usize, lambda_mult: f32) -> Self;
}

// Implements Retriever: delegates to store.similarity_search or store.max_marginal_relevance_search
// Implements Runnable<String, Vec<Document>> via RetrieverRunnable adapter
```

### InMemoryVectorStore (concrete type)

In-memory vector store for testing and prototyping. Implements all
`VectorStore` trait methods including MMR.

```rust
pub struct InMemoryVectorStore {
    embeddings: Box<dyn Embeddings>,
    documents: Vec<(String, Document, Vec<f32>)>,  // (id, doc, embedding)
}

impl InMemoryVectorStore {
    pub fn new(embeddings: Box<dyn Embeddings>) -> Self;
}

// Implements VectorStore:
// - add_documents: embeds via self.embeddings, stores in memory
// - add_texts: converts to Documents, delegates to add_documents
// - similarity_search: brute-force cosine similarity
// - similarity_search_with_score: cosine sim with scores
// - similarity_search_by_vector: search by pre-computed embedding
// - max_marginal_relevance_search: uses mmr.rs utility
// - max_marginal_relevance_search_by_vector: uses mmr.rs utility
// - get_by_ids: O(n) scan by ID
// - delete: removes by ID
// - embeddings(): returns reference to self.embeddings
// - as_retriever(k): returns VectorStoreRetriever wrapping self
```

## CallbackHandler

All callback hooks receive `tags` and `metadata` from `RunnableConfig` for
tracing and filtering. All hooks have default no-op implementations.

```rust
pub trait CallbackHandler: Send + Sync {
    // --- Selective filtering (default: process all events) ---

    fn ignore_llm(&self) -> bool { false }
    fn ignore_chain(&self) -> bool { false }
    fn ignore_tool(&self) -> bool { false }
    fn ignore_retriever(&self) -> bool { false }
    fn ignore_agent(&self) -> bool { false }

    // --- LLM hooks ---

    fn on_llm_start<'a>(
        &'a self,
        run_id: Uuid,
        prompts: &'a [String],
        parent_run_id: Option<Uuid>,
        tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    /// Called when a chat model starts (receives messages instead of prompts).
    fn on_chat_model_start<'a>(
        &'a self,
        run_id: Uuid,
        messages: &'a [Message],
        parent_run_id: Option<Uuid>,
        tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    fn on_llm_new_token<'a>(
        &'a self,
        run_id: Uuid,
        token: &'a str,
        parent_run_id: Option<Uuid>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    fn on_llm_end<'a>(
        &'a self,
        run_id: Uuid,
        response: &'a LLMResult,
        parent_run_id: Option<Uuid>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    fn on_llm_error<'a>(
        &'a self,
        run_id: Uuid,
        error: &'a LangChainError,
        parent_run_id: Option<Uuid>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    // --- Chain hooks ---

    fn on_chain_start<'a>(
        &'a self,
        run_id: Uuid,
        inputs: &'a Value,
        parent_run_id: Option<Uuid>,
        tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    fn on_chain_end<'a>(
        &'a self,
        run_id: Uuid,
        outputs: &'a Value,
        parent_run_id: Option<Uuid>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    fn on_chain_error<'a>(
        &'a self,
        run_id: Uuid,
        error: &'a LangChainError,
        parent_run_id: Option<Uuid>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    // --- Tool hooks ---

    fn on_tool_start<'a>(
        &'a self,
        run_id: Uuid,
        input: &'a Value,
        parent_run_id: Option<Uuid>,
        tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    fn on_tool_end<'a>(
        &'a self,
        run_id: Uuid,
        output: &'a Value,
        parent_run_id: Option<Uuid>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    fn on_tool_error<'a>(
        &'a self,
        run_id: Uuid,
        error: &'a LangChainError,
        parent_run_id: Option<Uuid>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    // --- Retriever hooks ---

    fn on_retriever_start<'a>(
        &'a self,
        run_id: Uuid,
        query: &'a str,
        parent_run_id: Option<Uuid>,
        tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    fn on_retriever_end<'a>(
        &'a self,
        run_id: Uuid,
        documents: &'a [Document],
        parent_run_id: Option<Uuid>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    fn on_retriever_error<'a>(
        &'a self,
        run_id: Uuid,
        error: &'a LangChainError,
        parent_run_id: Option<Uuid>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    // --- Agent hooks ---

    fn on_agent_action<'a>(
        &'a self,
        run_id: Uuid,
        action: &'a AgentAction,
        parent_run_id: Option<Uuid>,
        tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    fn on_agent_finish<'a>(
        &'a self,
        run_id: Uuid,
        finish: &'a AgentFinish,
        parent_run_id: Option<Uuid>,
        tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    // --- Retry hook ---

    /// Called when a retry attempt occurs during RunnableRetry execution.
    /// retry_state contains the attempt number and the error that triggered retry.
    fn on_retry<'a>(
        &'a self,
        run_id: Uuid,
        retry_state: &'a RetryState,
        parent_run_id: Option<Uuid>,
        tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    // --- Custom event hook ---

    /// User-defined event hook for custom events emitted via stream_events.
    fn on_custom_event<'a>(
        &'a self,
        name: &'a str,
        data: &'a Value,
        run_id: Uuid,
        tags: Option<&'a [String]>,
        metadata: Option<&'a HashMap<String, Value>>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }
}
```

### CallbackHandler — Failure Semantics

All `on_*` hook methods return `()` (no `Result`). Callback handler failures
**must not** interrupt chain execution. Implementations that can fail should
catch errors internally and log via `tracing::warn!`. Panics in callback
hooks are caught by the executor and logged — they never propagate to the
caller. This matches Python's behaviour where callback exceptions are
swallowed with a warning.

### CallbackHandler — Intentional Exclusions

- **`on_text`**: General-purpose text event hook in Python. Not useful in
  a typed Rust API where each event has a specific hook.
- **`CallbackManager` / `AsyncCallbackManager` hierarchy**: Python uses a
  manager hierarchy (`CallbackManager`, `CallbackManagerForChainRun`,
  `CallbackManagerForLLMRun`, etc.) to propagate callbacks through nested
  runs and create parent/child relationships. Rust handles this through
  `RunnableConfig.callbacks` (a `Vec<Box<dyn CallbackHandler>>`) passed
  through the chain. The config carries `run_id` and `parent_run_id` for
  nesting. No separate manager types are needed.
- **`collect_runs()`**: Python context manager for collecting run metadata.
  Rust uses `tracing` spans (behind the `tracing` feature flag) for the
  same purpose — subscribe to spans to collect run data.
- **`tracing_v2_enabled()` / `tracing_enabled()`**: Python checks whether
  LangSmith tracing is active. Rust uses the `tracing` crate's standard
  `tracing::dispatcher::get_default()` or feature-flag checks.

## PromptTemplate (concrete type, not a trait)

```rust
impl PromptTemplate {
    /// Create a new template from a format string and variable names.
    pub fn new(
        template: &str,
        input_variables: Vec<String>,
        template_format: TemplateFormat,
    ) -> Result<Self, LangChainError>;

    /// Create a template from a format string, auto-extracting variable names.
    /// Equivalent to Python's PromptTemplate.from_template().
    pub fn from_template(template: &str) -> Result<Self, LangChainError>;

    /// Return a new template with some variables pre-filled.
    /// Equivalent to Python's PromptTemplate.partial().
    pub fn partial(
        &self,
        variables: HashMap<String, String>,
    ) -> Result<Self, LangChainError>;

    /// Format the template with the given variables.
    pub fn format(&self, variables: &HashMap<String, String>) -> Result<String, LangChainError>;

    /// Format into a PromptValue (can be consumed as string or messages).
    pub fn format_prompt(
        &self,
        variables: &HashMap<String, String>,
    ) -> Result<PromptValue, LangChainError>;

    /// Format into a list of messages (for chat models).
    pub fn format_messages(
        &self,
        variables: &HashMap<String, Value>,
    ) -> Result<Vec<Message>, LangChainError>;
}
```

### ChatPromptTemplate (concrete type)

```rust
impl ChatPromptTemplate {
    /// Create from a list of (role, template_string) pairs.
    /// Equivalent to Python's ChatPromptTemplate.from_messages().
    pub fn from_messages(
        messages: Vec<(MessageRole, String)>,
    ) -> Result<Self, LangChainError>;

    /// Format into messages with the given variables.
    pub fn format_messages(
        &self,
        variables: &HashMap<String, Value>,
    ) -> Result<Vec<Message>, LangChainError>;

    /// Format into a PromptValue.
    pub fn format_prompt(
        &self,
        variables: &HashMap<String, Value>,
    ) -> Result<PromptValue, LangChainError>;

    /// Return a new template with some variables pre-filled.
    pub fn partial(
        &self,
        variables: HashMap<String, Value>,
    ) -> Result<Self, LangChainError>;
}
```

**MessageRole** (enum used in `from_messages`):
- `Human`
- `AI`
- `System`
- `Placeholder` — indicates the string is a variable name to inject `Vec<Message>`

**TemplateFormat** (enum): `FString` | `Mustache` — already defined in
data-model.md. `from_template` defaults to `FString`.

**MessageTemplate Placeholder semantics**: When a `Placeholder("history")`
appears in `ChatPromptTemplate.messages`, formatting substitutes it with
the `Vec<Message>` found in the variables map under key `"history"`. This
enables injecting conversation history into chat prompts. This is the
Rust equivalent of Python's `MessagesPlaceholder`.

### Prompt — Excluded from Core (Reference Impls in `langchain` crate)

- **`FewShotPromptTemplate` / `FewShotChatMessagePromptTemplate`**: Few-shot
  prompting types that compose an `ExampleSelector` with a prompt template.
  Excluded from core prompt traits but provided in the `langchain` crate
  as concrete types. Core's `ChatPromptTemplate` with `Placeholder` covers
  the basic case of injecting example messages directly.
- **`SemanticSimilarityExampleSelector`**: Selects few-shot examples by
  `VectorStore` similarity search. Reference implementation provided in
  the `langchain` crate using the core `VectorStore` and `Embeddings` traits.

### Prompt — Permanently Excluded

- **`PipelinePromptTemplate`**: Compose multiple prompt templates into a
  pipeline. Runnable composition (`pipe`) replaces this — chain
  `PromptTemplate` runnables together instead.
- **`LengthBasedExampleSelector`**, **`MaxMarginalRelevanceExampleSelector`**:
  Niche selection strategies. `SemanticSimilarityExampleSelector` covers
  the primary use case; MMR-based selection can use the VectorStore MMR
  methods directly.
- **`DictPromptTemplate`**: Returns dict output instead of string. Python-
  specific pattern for dict-based chains. Rust uses typed structs; use
  `RunnableLambda` to transform prompt output into any shape.

### Language Model — Intentional Exclusions

- **`SimpleChatModel`**: Minimal Python base class for test models. In Rust,
  implement `BaseChatModel` directly — trait methods have clear signatures;
  no intermediate base is needed.
- **`LLM` simple base**: Python convenience class. In Rust, implement
  `BaseLLM` directly.
- **`ModelProfile`** (capability detection: `supports_tool_calling`,
  `supports_streaming`, `supports_structured_output`): Runtime capability
  detection. Rust uses trait bounds at compile time — if a model implements
  `BaseChatModel`, it supports invoke/batch/stream. Tool support is checked
  via `bind_tools` returning a result. No runtime capability enum needed.
- **`rate_limiter`**: Rate limiting is an application or infrastructure
  concern. Use `tower::RateLimit` middleware, provider-specific rate limit
  configuration, or `reqwest-middleware` in provider crates.
- **`cache`**: Model response caching is an application concern. Use
  `moka` or `cached` crates at the application layer, or implement a
  caching `Runnable` wrapper. Not a core abstraction.

### Document — Excluded from Core (Reference Impls in `langchain` crate)

- **`BaseDocumentTransformer`** (text splitting): Reference implementations
  `CharacterTextSplitter` and `RecursiveCharacterTextSplitter` provided
  in the `langchain` crate's `text_splitters` module. Essential for RAG
  document preparation.

### Document — Permanently Excluded

- **`Blob`** (binary content with media type, encoding, path): Binary
  content processing (OCR, audio transcription, PDF parsing) is not a core
  LangChain abstraction — it's a document loader concern. Provider crates
  or application code handle binary content directly.
- **`BaseDocumentCompressor`**: Document compression for context window
  management. Use `trim_messages` for messages or implement custom
  document truncation logic.

### Embeddings — Excluded from Core (Reference Impl in `langchain` crate)

- **`CacheBackedEmbeddings`**: Wraps any `Embeddings` implementation with
  an async-compatible cache store (`moka`). Reference implementation
  provided in the `langchain` crate's `cache::embeddings` module.

## Test Utilities

Test utilities shipped with `langchain-core` for unit testing chains
and agents without real API calls.

### FakeChatModel

```rust
pub struct FakeChatModel {
    responses: Vec<Message>,
    i: AtomicUsize,
}

impl FakeChatModel {
    /// Create with a list of responses to return in order (cycling).
    pub fn new(responses: Vec<Message>) -> Self;
}

// Implements BaseChatModel:
// - invoke: returns next response from the list
// - stream: yields response content one character at a time
// - batch: returns one response per input
```

### FakeEmbeddings

```rust
pub struct FakeEmbeddings {
    dimensions: usize,
}

impl FakeEmbeddings {
    /// Create with a given dimensionality. Returns deterministic
    /// vectors based on text content hash for reproducible tests.
    pub fn new(dimensions: usize) -> Self;
}

// Implements Embeddings:
// - embed_documents: returns one deterministic vector per text
// - embed_query: returns one deterministic vector
```

---

## Reference Implementations (`langchain` crate)

The following contracts define concrete types in the `langchain`
convenience crate. These are NOT core traits — they are ready-to-use
implementations of common application-level patterns.

### CacheBackedEmbeddings

```rust
pub struct CacheBackedEmbeddings<E: Embeddings, C: EmbeddingCache> {
    underlying: E,
    cache: C,
    namespace: String,
}

/// Trait for embedding cache stores.
pub trait EmbeddingCache: Send + Sync {
    fn get<'a>(&'a self, key: &'a str)
        -> BoxFuture<'a, Option<Vec<f32>>>;
    fn set<'a>(&'a self, key: &'a str, embedding: Vec<f32>)
        -> BoxFuture<'a, ()>;
}

/// In-memory cache using moka.
pub struct InMemoryEmbeddingCache { /* moka::future::Cache */ }

impl<E: Embeddings, C: EmbeddingCache> CacheBackedEmbeddings<E, C> {
    pub fn new(underlying: E, cache: C, namespace: impl Into<String>) -> Self;
}

// Implements Embeddings — checks cache first, falls back to underlying
```

### RunnableWithMessageHistory

```rust
/// Trait for chat message history stores.
pub trait ChatMessageHistory: Send + Sync {
    fn get_messages<'a>(&'a self, session_id: &'a str)
        -> BoxFuture<'a, Result<Vec<Message>, LangChainError>>;
    fn add_message<'a>(&'a self, session_id: &'a str, message: Message)
        -> BoxFuture<'a, Result<(), LangChainError>>;
    fn clear<'a>(&'a self, session_id: &'a str)
        -> BoxFuture<'a, Result<(), LangChainError>>;
}

/// In-memory history store (HashMap<String, Vec<Message>>).
pub struct InMemoryChatMessageHistory { /* ... */ }

pub struct RunnableWithMessageHistory<R, H: ChatMessageHistory> {
    runnable: R,
    history: H,
    input_messages_key: String,
    history_messages_key: String,
    session_id_fn: Box<dyn Fn(&RunnableConfig) -> String + Send + Sync>,
}

// Implements Runnable — injects history before invoke, appends after
```

### FewShotPromptTemplate

```rust
pub struct FewShotPromptTemplate {
    example_selector: Box<dyn ExampleSelector>,
    example_prompt: PromptTemplate,
    prefix: String,
    suffix: String,
    input_variables: Vec<String>,
}

pub struct FewShotChatMessagePromptTemplate {
    example_selector: Box<dyn ExampleSelector>,
    example_prompt: ChatPromptTemplate,
    input_variables: Vec<String>,
}

/// Trait for example selection strategies.
pub trait ExampleSelector: Send + Sync {
    fn select_examples<'a>(
        &'a self,
        input_variables: &'a HashMap<String, Value>,
    ) -> BoxFuture<'a, Result<Vec<HashMap<String, String>>, LangChainError>>;

    fn add_example<'a>(
        &'a self,
        example: HashMap<String, String>,
    ) -> BoxFuture<'a, Result<(), LangChainError>>;
}

/// Selects examples by VectorStore similarity search.
pub struct SemanticSimilarityExampleSelector {
    vectorstore: Box<dyn VectorStore>,
    k: usize,
    input_keys: Vec<String>,
}
```

### Text Splitters

```rust
pub struct CharacterTextSplitter {
    separator: String,
    chunk_size: usize,
    chunk_overlap: usize,
    length_function: Box<dyn Fn(&str) -> usize + Send + Sync>,
}

pub struct RecursiveCharacterTextSplitter {
    separators: Vec<String>,
    chunk_size: usize,
    chunk_overlap: usize,
    length_function: Box<dyn Fn(&str) -> usize + Send + Sync>,
}

impl CharacterTextSplitter {
    pub fn new(chunk_size: usize, chunk_overlap: usize) -> Self;
    pub fn split_text(&self, text: &str) -> Vec<String>;
    pub fn split_documents(&self, documents: &[Document]) -> Vec<Document>;
}

impl RecursiveCharacterTextSplitter {
    pub fn new(chunk_size: usize, chunk_overlap: usize) -> Self;
    pub fn split_text(&self, text: &str) -> Vec<String>;
    pub fn split_documents(&self, documents: &[Document]) -> Vec<Document>;
}
```

### Additional Output Parsers

```rust
/// Splits comma-separated model output into a list.
pub struct CommaSeparatedListOutputParser;
// Implements OutputParser<Vec<String>>

/// Constrains output to one of a fixed set of string values.
pub struct EnumOutputParser {
    allowed_values: Vec<String>,
}
// Implements OutputParser<String> — returns error if not in allowed set

/// Parses XML-formatted model output.
pub struct XMLOutputParser {
    tags: Vec<String>,  // expected tag names to extract
}
// Implements OutputParser<HashMap<String, Vec<String>>>

/// Extracts named groups from model output via regex.
pub struct RegexParser {
    regex: Regex,
    output_keys: Vec<String>,
}
// Implements OutputParser<HashMap<String, String>>

/// Wraps a parser + LLM — on parse failure, retries by sending
/// the error back to the LLM for correction.
pub struct RetryOutputParser<T> {
    parser: Box<dyn OutputParser<T>>,
    retry_chain: Box<dyn Runnable<HashMap<String, Value>, String>>,
    max_retries: usize,
}
// Implements OutputParser<T>

/// Merges results from multiple parsers.
pub struct CombiningOutputParser {
    parsers: Vec<Box<dyn OutputParser<HashMap<String, String>>>>,
}
// Implements OutputParser<HashMap<String, String>>
```

### OpenAIModerationMiddleware (in `langchain-openai` crate)

```rust
pub struct OpenAIModerationMiddleware {
    client: reqwest::Client,
    api_key: String,
    model: String,  // default: "text-moderation-latest"
}

impl OpenAIModerationMiddleware {
    pub fn new(api_key: impl Into<String>) -> Self;

    /// Check text against OpenAI's moderation endpoint.
    /// Returns Ok(()) if content passes, Err with flagged categories if not.
    pub async fn check(&self, text: &str)
        -> Result<(), ModerationError>;

    /// Wrap as a RunnableLambda that checks input before passing through.
    pub fn as_runnable(self) -> RunnableLambda<String, String>;
}
```
