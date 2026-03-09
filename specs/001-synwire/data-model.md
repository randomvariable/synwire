# Data Model: Synwire — M1 Core + Orchestrator

**Date**: 2026-03-09
**Branch**: `001-synwire`
**Scope**: M1 types only. Agent-layer types (M2) and protocol types (M3) are deferred to [roadmap](../../docs/roadmap.md).

> Architecture review fixes applied: `Arc<dyn CallbackHandler>` in RunnableConfig (§1.3),
> generic `CompiledGraph<S>` (§1.2), layered error types (§2.1), split Runnable trait (§2.2),
> `SecretValue` backed by `secrecy` crate (§2.5), `#[non_exhaustive]` on all enums (§5.5).

---

## Core Types

### Message

A single unit of conversation. Modelled as a Rust enum (closed set).

```text
#[non_exhaustive]
Message (enum)
├── Human {
│       id: Option<String>,
│       name: Option<String>,
│       content: MessageContent,
│       additional_kwargs: HashMap<String, Value>,
│   }
├── AI {
│       id: Option<String>,
│       name: Option<String>,
│       content: MessageContent,
│       tool_calls: Vec<ToolCall>,
│       invalid_tool_calls: Vec<InvalidToolCall>,
│       usage: Option<UsageMetadata>,
│       response_metadata: Option<HashMap<String, Value>>,
│       additional_kwargs: HashMap<String, Value>,
│   }
├── System {
│       id: Option<String>,
│       name: Option<String>,
│       content: MessageContent,
│       additional_kwargs: HashMap<String, Value>,
│   }
├── Tool {
│       id: Option<String>,
│       name: Option<String>,
│       content: MessageContent,
│       tool_call_id: String,
│       status: ToolStatus,
│       artifact: Option<Value>,
│       additional_kwargs: HashMap<String, Value>,
│   }
└── Chat {
        id: Option<String>,
        name: Option<String>,
        role: String,
        content: MessageContent,
        additional_kwargs: HashMap<String, Value>,
    }
```

**Common fields** (present on all variants):
- `id: Option<String>` — unique message identifier, set by providers or callers
- `name: Option<String>` — optional sender name (e.g. for multi-agent scenarios)

**AI-specific fields**:
- `invalid_tool_calls: Vec<InvalidToolCall>` — tool calls the model attempted but failed to parse
- `response_metadata: Option<HashMap<String, Value>>` — provider metadata (logprobs, token counts, finish_reason, model version)

**Tool-specific fields**:
- `artifact: Option<Value>` — rich tool output not sent to model (e.g. images, data frames); only `content` is sent to the model

**Chat variant**: Generic message with an arbitrary `role` string. Used when models
produce or consume messages with custom roles beyond human/ai/system/tool.

**MessageContent** (enum):
- `Text(String)` — plain text
- `Blocks(Vec<ContentBlock>)` — structured content (text + images)

**ContentBlock** (enum, `#[non_exhaustive]`):
- `Text { text: String }`
- `Image { url: String, detail: Option<String> }`
- `Audio { url: String, mime_type: Option<String> }`
- `Video { url: String, mime_type: Option<String> }`
- `File { url: String, mime_type: Option<String> }`
- `Reasoning { text: String }` — chain-of-thought reasoning from models
- `Thinking { text: String }` — model thinking/scratchpad content

**Excluded ContentBlock types** (provider-specific, not core abstractions):
- `GuardContent`, `RefusalContent`, `CitationContent`, `CacheControl`
  — mapped to `response_metadata` or `additional_kwargs` by providers.

**Excluded Message Types**:
- `FunctionMessage` — deprecated, superseded by `ToolMessage`
- `AgentActionMessageLog` — niche variant, standard `AgentAction.log` covers the use case

### ToolCall

A structured request from a model to invoke a tool.

```text
ToolCall
├── id: String
├── name: String
├── arguments: HashMap<String, Value>
```

### InvalidToolCall

A tool call that failed to produce valid arguments.

```text
InvalidToolCall
├── name: Option<String>
├── arguments: Option<String>     (raw unparsed string)
├── id: Option<String>
├── error: String
```

### ToolStatus

```text
#[non_exhaustive]
ToolStatus (enum)
├── Success
├── Error
```

Serde: `"success"` / `"error"` (lowercase).

### UsageMetadata

Token usage statistics from a model invocation.

```text
UsageMetadata
├── input_tokens: u64
├── output_tokens: u64
├── total_tokens: u64
├── input_token_details: Option<InputTokenDetails>
├── output_token_details: Option<OutputTokenDetails>
```

**InputTokenDetails**:
```text
InputTokenDetails
├── audio: Option<u64>
├── cache_creation: Option<u64>
├── cache_read: Option<u64>
```

**OutputTokenDetails**:
```text
OutputTokenDetails
├── audio: Option<u64>
├── reasoning: Option<u64>
```

Maps to OTel `gen_ai.usage.input_tokens`, `gen_ai.usage.output_tokens` attributes.

### Document

A retrievable piece of content.

```text
Document
├── id: Option<String>
├── page_content: String
├── metadata: HashMap<String, Value>
```

**Validation**: `page_content` MUST NOT be empty for documents added to vector stores.

### ChatResult

The output of a chat model invocation.

```text
ChatResult
├── message: Message (AI variant)
├── generation_info: Option<HashMap<String, Value>>
├── cost: Option<CostEstimate>
```

### CostEstimate

Estimated monetary cost of a model invocation.

```text
CostEstimate
├── input_cost: f64              (USD)
├── output_cost: f64             (USD)
├── total_cost: f64              (USD)
├── currency: String             (ISO 4217, default "USD")
```

### LLMResult

The output of a batch LLM invocation.

```text
LLMResult
├── generations: Vec<Vec<Generation>>
├── llm_output: Option<HashMap<String, Value>>
```

### Generation

```text
Generation
├── text: String
├── generation_info: Option<HashMap<String, Value>>
```

### ChatChunk

A single chunk of streaming output from a chat model. Rust collapses Python's
per-type chunks (`AIMessageChunk`, etc.) into a single `ChatChunk` — only AI
messages carry meaningful streaming data.

```text
ChatChunk
├── delta_content: Option<String>
├── delta_tool_calls: Vec<ToolCallChunk>
├── finish_reason: Option<String>
├── usage: Option<UsageMetadata>
```

**Chunk concatenation**: `ChatChunk` implements `merge(&mut self, other: &ChatChunk)`:
- Appends `delta_content` strings
- Merges `delta_tool_calls` by `index` — concatenates partial `arguments` strings
- Takes last non-None `finish_reason` and `usage`

### ToolCallChunk

Partial tool call received during streaming.

```text
ToolCallChunk
├── index: usize
├── id: Option<String>
├── name: Option<String>
├── arguments: Option<String>    (partial JSON, concatenated across chunks)
```

### PromptValue

```text
#[non_exhaustive]
PromptValue (enum)
├── String(String)
├── Messages(Vec<Message>)
```

Methods:
- `to_string(&self) -> String`
- `to_messages(&self) -> Vec<Message>`

### PromptTemplate

```text
PromptTemplate
├── template: String
├── input_variables: Vec<String>
├── template_format: TemplateFormat (enum: FString, Mustache)
```

**Validation**: All variables referenced in `template` MUST appear in `input_variables`.

### ChatPromptTemplate

```text
ChatPromptTemplate
├── messages: Vec<MessageTemplate>
├── input_variables: Vec<String>
```

**MessageTemplate** (enum):
- `Human(String)`, `AI(String)`, `System(String)`, `Placeholder(String)`

### RunnableConfig

Configuration passed through a runnable chain.

> **Architecture review fix §1.3**: Callbacks use `Arc<dyn CallbackHandler>` instead
> of `Box<dyn CallbackHandler>`, making `RunnableConfig` cheaply cloneable.

```text
RunnableConfig: Clone + Send + Sync
├── callbacks: Option<Vec<Arc<dyn CallbackHandler>>>
├── tags: Option<Vec<String>>
├── metadata: Option<HashMap<String, Value>>
├── max_concurrency: Option<usize>
├── run_name: Option<String>
├── run_id: Option<Uuid>
├── configurable: Option<HashMap<String, Value>>
```

### ToolSchema

```text
ToolSchema
├── name: String
├── description: String
├── parameters: Value  (JSON Schema object)
```

### ToolOutput

```text
ToolOutput
├── content: String              (text result shown to the model)
├── artifact: Option<Value>      (rich output not sent to model)
```

### ToolResult (Extended)

```text
#[non_exhaustive]
ToolResult (enum)
├── Success { content: Value }
├── Error { message: String }
└── Retry { message: String }    (sent back to model for self-correction)
```

**Retry semantics**: `ToolNode` creates a `Message::Tool` with status `Error` containing
the retry message. Retries bounded by `max_retries` (default: 1).

### SecretValue

> **Architecture review fix §2.5**: Backed by the `secrecy` crate for memory
> zeroisation on drop. Inner type is `SecretString` from `secrecy`.

```text
SecretValue
├── inner: SecretString           (zeroised on drop)
```

**Security properties**:
- `Debug`: `SecretValue(***)`
- `Display`: `***`
- `Serialize`: serialises as `null`
- `SecretValue::expose(&self) -> &str` for explicit access
- `Clone + Send + Sync + Eq + Hash`

### ToolContentType

```text
#[non_exhaustive]
ToolContentType (enum)
├── Text
├── Json(Value)
├── Image { data: Vec<u8>, mime_type: String }
├── File { data: Vec<u8>, name: String, mime_type: String }
├── Blob { data: Vec<u8>, mime_type: String }
```

### ToolCategory

```text
#[non_exhaustive]
ToolCategory (enum)
├── Builtin
├── Custom
├── Mcp
├── Remote
├── WorkflowAsTool
```

### MetadataFilter

Composable filter expressions for vector store metadata queries.

```text
#[non_exhaustive]
MetadataFilter (enum)
├── Eq { key: String, value: Value }
├── Ne { key: String, value: Value }
├── Gt { key: String, value: Value }
├── Lt { key: String, value: Value }
├── Gte { key: String, value: Value }
├── Lte { key: String, value: Value }
├── In { key: String, values: Vec<Value> }
├── And(Vec<MetadataFilter>)
├── Or(Vec<MetadataFilter>)
```

### ContentCategory

Distinguishes primary from secondary content in streaming responses.

```text
#[non_exhaustive]
ContentCategory (enum)
├── Primary       (actual response content: text, structured data)
├── Secondary     (intermediate: tool calls, reasoning, usage metrics)
```

---

## Error Types

> **Architecture review fix §2.1**: Layered error types per domain with
> `#[non_exhaustive]` on all enums. Each crate defines its own error type.
> Top-level `SynwireError` wraps via `#[from]`.

### SynwireError (synwire-core)

```text
#[non_exhaustive]
SynwireError (enum)
├── Model(ModelError)
├── Prompt(PromptError)
├── Parse(ParseError)
├── Embedding(EmbeddingError)
├── VectorStore(VectorStoreError)
├── Tool(ToolError)
├── Serialization(SerializationError)
├── Graph(GraphError)                   // #[from] SynwireGraphError
├── Io(std::io::Error)
├── Other(Box<dyn Error + Send + Sync>)
```

### ModelError

```text
#[non_exhaustive]
ModelError (enum)
├── RateLimit { retry_after: Option<Duration> }
├── AuthenticationFailed { message: String }
├── InvalidRequest { message: String }
├── ContentFiltered { message: String }
├── Timeout
├── Connection { source: Box<dyn Error + Send + Sync> }
├── Other { message: String }
```

### GraphError (synwire-orchestrator)

```text
#[non_exhaustive]
SynwireGraphError (enum)
├── RecursionLimit { message: String }
├── InvalidUpdate { message: String }
├── Interrupt { interrupts: Vec<Interrupt> }
├── EmptyInput { message: String }
├── TaskNotFound { task_id: String }
├── EmptyChannel { channel: String }
├── CompileError { message: String }
├── Checkpoint { source: Box<dyn Error + Send + Sync> }
├── Store { source: Box<dyn Error + Send + Sync> }
```

### ErrorCode

```text
#[non_exhaustive]
ErrorCode (enum)
├── GraphRecursionLimit
├── InvalidConcurrentGraphUpdate
├── InvalidGraphNodeReturnValue
├── MultipleSubgraphs
├── InvalidChatHistory
```

### SynwireErrorKind

Discriminant enum for matching errors without payload. Used by `RetryConfig` and `with_fallbacks`.

```text
#[non_exhaustive]
SynwireErrorKind (enum)
├── Model
├── Prompt
├── Parse
├── Embedding
├── VectorStore
├── Tool
├── RetryExhausted
├── Serialization
├── Graph
├── Other
```

---

## Agent Types (synwire-core — minimal)

> **Architecture review fix §3.2**: Only the minimal agent types needed for
> `create_react_agent` live in synwire-core. The full agent framework
> (Agent<D,O>, RunContext, middleware, etc.) moves to synwire-agents in M2.

### AgentAction

```text
AgentAction
├── tool: String
├── tool_input: Value
├── log: String
```

### AgentFinish

```text
AgentFinish
├── return_values: HashMap<String, Value>
├── log: String
```

### AgentStep

```text
AgentStep
├── action: AgentAction
├── observation: String
```

### AgentDecision

```text
#[non_exhaustive]
AgentDecision (enum)
├── Action(AgentAction)
├── Actions(Vec<AgentAction>)
├── Finish(AgentFinish)
```

### AgentInput

```text
AgentInput
├── input: HashMap<String, Value>
├── intermediate_steps: Vec<AgentStep>
├── chat_history: Option<Vec<Message>>
```

### AgentExecutor

> **Architecture review fix §2.3**: `AgentExecutor` is MAY (not MUST) for M1.
> `create_react_agent` is the primary entry point. `AgentExecutor` exists for
> backward compatibility with the LangChain agent pattern.

```text
AgentExecutor
├── agent: Box<dyn RunnableCore<AgentInput, AgentDecision>>
├── tools: Vec<Box<dyn Tool>>
├── max_iterations: Option<usize>       (default: 15)
├── max_execution_time: Option<Duration>
├── return_intermediate_steps: bool     (default: false)
├── handle_parsing_errors: bool         (default: false)
```

---

## Event Streaming Types

### StreamEvent

> **Architecture review fix §2.2**: `stream_log`/`RunLogPatch` is dropped entirely.
> `stream_events` is on the `ObservableRunnable` extension trait, not `RunnableCore`.

```text
#[non_exhaustive]
StreamEvent (enum)
├── Standard {
│       event: String,
│       name: String,
│       run_id: String,
│       parent_ids: Vec<String>,
│       tags: Vec<String>,
│       metadata: HashMap<String, Value>,
│       data: EventData,
│   }
├── Custom {
│       name: String,
│       run_id: String,
│       parent_ids: Vec<String>,
│       tags: Vec<String>,
│       metadata: HashMap<String, Value>,
│       data: Value,
│   }
```

**Standard event names**:
`on_chain_start`, `on_chain_stream`, `on_chain_end`,
`on_llm_start`, `on_llm_stream`, `on_llm_end`,
`on_chat_model_start`, `on_chat_model_stream`, `on_chat_model_end`,
`on_tool_start`, `on_tool_end`,
`on_retriever_start`, `on_retriever_end`,
`on_prompt_start`, `on_prompt_end`,
`on_custom_event`.

### EventData

```text
EventData
├── input: Option<Value>
├── output: Option<Value>
├── chunk: Option<Value>
├── error: Option<String>
```

---

## Retry & Resilience Types

### RetryConfig

Configuration for `with_retry` Runnable composition.

```text
RetryConfig
├── retry_on: Vec<SynwireErrorKind>
├── max_attempts: u32                   (default: 3)
├── wait_exponential_jitter: bool       (default: true)
├── initial_interval: Duration          (default: 1s)
├── max_interval: Duration              (default: 60s)
```

### RetryState

```text
RetryState
├── attempt: u32
├── error: SynwireError
├── elapsed: Duration
```

---

## Message Utility Functions

### filter_messages

> **Architecture review fix §4.1**: Uses builder pattern instead of 7 positional parameters.

```text
MessageFilter::new()
    .include_types(&[MessageType::Human, MessageType::AI])
    .exclude_names(&["system_agent"])
    .apply(messages) -> Vec<Message>
```

### trim_messages

```text
trim_messages(
    messages: &[Message],
    max_tokens: usize,
    token_counter: &dyn Fn(&Message) -> usize,
    strategy: TrimStrategy,
    allow_partial: bool,
    start_on: Option<MessageType>,
    include_system: bool,
) -> Vec<Message>
```

`TrimStrategy` enum: `First`, `Last`.

### merge_message_runs

```text
merge_message_runs(messages: &[Message]) -> Vec<Message>
```

---

## Type Conversions

### MessageLike Trait

```text
trait MessageLike: Send + Sync {
    fn to_message(&self) -> Message;
}
```

Implementations: `Message` (identity), `&str`/`String` → `Human`, `(MessageRole, &str)` → typed.

### Key From/Into Conversions

| From | To | Notes |
|---|---|---|
| `ToolOutput` | `Message::Tool` | Maps content and artifact |
| `&str` | `Message::Human` | Wraps in HumanMessage with Text content |
| `PromptValue` | `Vec<Message>` | Extracts messages or wraps string |

---

## Prelude Module

```text
pub use crate::{
    // Core traits (split per arch review §2.2)
    RunnableCore, ObservableRunnable,
    BaseChatModel, BaseLLM, Embeddings, VectorStore, Retriever,
    Tool, OutputParser, CallbackHandler, MessageLike,

    // Core types
    Message, MessageContent, ContentBlock, ChatResult, ChatChunk,
    Document, PromptValue, PromptTemplate, ChatPromptTemplate,
    ToolCall, ToolSchema, ToolOutput, RunnableConfig,

    // Agent types (minimal — full agent API is M2)
    AgentAction, AgentFinish, AgentStep, AgentDecision,

    // Error types (layered per arch review §2.1)
    SynwireError, ModelError, SynwireErrorKind,

    // Type aliases
    BoxFuture, BoxStream,

    // Message utilities
    MessageFilter, trim_messages, merge_message_runs,
};
```

## Type Aliases

```text
BoxFuture<'a, T>  = Pin<Box<dyn Future<Output = T> + Send + 'a>>
BoxStream<'a, T>  = Pin<Box<dyn Stream<Item = T> + Send + 'a>>
Embedding         = Vec<f32>
```

---

## State Transitions

### Message Flow Through a Chain

```text
Input Variables → PromptTemplate.format() → Vec<Message>
  → BaseChatModel.invoke() → ChatResult (AI Message)
  → OutputParser.parse() → Structured Output (T)
```

### Streaming Flow

```text
Input Variables → PromptTemplate.format() → Vec<Message>
  → BaseChatModel.stream() → BoxStream<ChatChunk>
  → Accumulate via ChatChunk.merge() to reconstruct full response
```

### RAG Flow

```text
Query → Embeddings.embed_query() → Vec<f32>
  → VectorStore.similarity_search() → Vec<Document>
  → Documents injected into PromptTemplate context
  → BaseChatModel.invoke() → Answer
```

### Graph Execution Flow (StateGraph)

> **Architecture review fix §1.2**: `StateGraph<S>` and `CompiledGraph<S>` remain
> generic over `S: State`. Type erasure to `Value` occurs only at serialisation
> boundaries (checkpoint save/load).

```text
StateGraph::<S>::new()
  → .add_node("name", action)
  → .add_edge(START, "first")
  → .add_conditional_edges("node", routing_fn)
  → .add_edge("last", END)
  → .compile(checkpointer, store, interrupt_before, interrupt_after)
  → CompiledGraph<S> (implements RunnableCore<S, S>)

CompiledGraph::<S>::invoke(input, config)
  → Pregel Execution:
    → Superstep 0: Write input to channels
    → Loop (superstep N):
      → Determine active nodes
      → Execute active nodes in parallel
      → Collect node outputs as channel writes
      → Apply channel reducers
      → Checkpoint state (if checkpointer configured)
      → Check for interrupts → persist and return StateSnapshot
      → Check for END / no more active nodes → break
      → Check recursion limit → RecursionLimit error
    → Return final state values
```

### Interrupt & Resume Flow

```text
Graph.invoke(input, config)
  → Execution reaches node with interrupt_before
  → State checkpointed
  → Returns StateSnapshot with interrupts: [Interrupt { value, id }]

Graph.invoke(Command::resume(value), config_with_thread_id)
  → Load checkpoint for thread_id
  → Inject resume value
  → Continue execution from interrupt point
  → Return final state
```

### Store Operation Flow

```text
Node execution context:
  → get_store() → &BaseStore
  → store.put(("user", user_id), "prefs", {"theme": "dark"})
  → store.get(("user", user_id), "prefs") → Some(Item)
  → store.search(("user",), query="theme preferences") → Vec<SearchItem>
```

---

## LangGraph Types (synwire-orchestrator)

### Channel Types

#### BaseChannel (trait)

```text
BaseChannel<Value, Update, Checkpoint> (trait)
├── fn from_checkpoint(cp: Checkpoint) -> Self
├── fn update(values: &[Update]) -> Result<bool, InvalidUpdateError>
├── fn get() -> Result<Value, EmptyChannelError>
├── fn is_available() -> bool
├── fn checkpoint() -> Checkpoint
├── fn consume() -> bool              (default: false)
├── fn finish() -> bool               (default: false)
```

#### LastValue

Stores the most recent value. Rejects multiple updates in one superstep.

```text
LastValue<V>
├── value: Option<V>
├── update([V]) → InvalidUpdateError if len != 1
```

#### Topic

Pub-sub accumulation channel.

```text
Topic<V>
├── values: Vec<V>
├── accumulate: bool
├── update(values) → flattens lists, appends
```

#### BinaryOperatorAggregate

Accumulates using a reducer function.

```text
BinaryOperatorAggregate<V>
├── value: Option<V>
├── operator: fn(V, V) -> V
├── update(values) → applies operator cumulatively
│   → Overwrite(v) bypasses reducer
```

#### AnyValue

Accepts any single value. Last wins without error.

```text
AnyValue<V>
├── value: Option<V>
```

#### EphemeralValue

Temporary value cleared after each superstep read.

```text
EphemeralValue<V>
├── value: Option<V>
├── guard: bool
```

#### NamedBarrierValue

Barrier that fires when all named triggers received.

```text
NamedBarrierValue<V>
├── names: HashSet<V>
├── seen: HashSet<V>
├── is_available() → seen == names
```

### Graph Builder Types

#### StateGraph

> **Architecture review fix §1.2**: Generic over `S: State` throughout.

```text
StateGraph<S: State>
├── nodes: HashMap<String, NodeDef>
├── edges: Vec<Edge>
├── conditional_edges: Vec<ConditionalEdge>
│
├── fn new() -> Self
├── fn add_node(name, action) -> &mut Self
├── fn add_edge(source, target) -> &mut Self
├── fn add_conditional_edges(source, path, path_map) -> &mut Self
├── fn compile(
│       checkpointer, store, cache,
│       interrupt_before, interrupt_after,
│       debug, retry_policy, cache_policy,
│   ) -> Result<CompiledGraph<S>, GraphCompileError>
```

**State typing**: Defined as a struct implementing `State` trait:

```rust
#[derive(State)]
struct AgentState {
    #[reducer(add_messages)]
    messages: Vec<Message>,        // BinaryOperatorAggregate
    next_action: String,           // LastValue (default)
}
```

#### Edge / ConditionalEdge

```text
Edge { source: String, target: String }

ConditionalEdge {
    source: String,
    path: Box<dyn Fn(&S) -> RoutingResult>,
    path_map: Option<HashMap<String, String>>,
}
```

#### RoutingResult

```text
#[non_exhaustive]
RoutingResult (enum)
├── Node(String)
├── Nodes(Vec<String>)
├── Send(Send)
├── Sends(Vec<Send>)
```

### Control Flow Types

#### Send

```text
Send
├── node: String
├── arg: Value
```

#### Command

```text
Command<N = String>
├── graph: Option<String>
├── update: Option<Value>
├── resume: Option<Value>
├── goto: Vec<Send | N>
```

#### Overwrite / Interrupt

```text
Overwrite<V> { value: V }     // bypass channel reducer

Interrupt { value: Value, id: String }

fn interrupt(value: Value) -> Result<Value, GraphInterrupt>
```

### Execution Types

#### CompiledGraph

> **Architecture review fix §1.2**: Generic over `S: State`. Erases to Value
> only at serialisation boundaries.

```text
CompiledGraph<S: State>: RunnableCore<S, S>
├── pregel: Pregel<S>
│
├── fn invoke(input: S, config: RunnableConfig) -> Result<S, SynwireGraphError>
├── fn stream(input: S, config, stream_mode) -> BoxStream<(String, Value)>
├── fn get_state(config) -> Result<StateSnapshot<S>, SynwireGraphError>
├── fn get_state_history(config, limit, before) -> BoxStream<StateSnapshot<S>>
├── fn update_state(config, values, as_node) -> Result<RunnableConfig, SynwireGraphError>
├── fn to_mermaid() -> String
```

#### PregelTask

```text
PregelTask
├── id: String
├── name: String
├── path: Vec<PathSegment>
├── error: Option<SynwireGraphError>
├── interrupts: Vec<Interrupt>
├── state: Option<StateSnapshot<S>>
├── result: Option<Value>
```

### State & Snapshot Types

#### StateSnapshot

```text
StateSnapshot<S>
├── values: S
├── next: Vec<String>
├── config: RunnableConfig
├── metadata: Option<CheckpointMetadata>
├── created_at: Option<String>
├── parent_config: Option<RunnableConfig>
├── tasks: Vec<PregelTask>
├── interrupts: Vec<Interrupt>
```

#### StreamMode

> **Architecture review fix §2.10**: Lossless vs lossy semantics defined per mode.

```text
#[non_exhaustive]
StreamMode (enum)
├── Values       (lossless — full state after each superstep)
├── Updates      (lossless — per-node state deltas)
├── Debug        (lossy — detailed execution events, may drop for slow consumers)
├── Messages     (lossy — LLM message chunks/tokens, may drop for slow consumers)
├── Custom       (lossy — user-defined via stream_writer)
├── Tasks        (lossless — task lifecycle events)
├── Checkpoints  (lossless — checkpoint writes)
```

**Backpressure**: Lossless modes suspend producers when consumer is slow.
Lossy modes drop events for lagging subscribers and emit a `DroppedEvents`
marker with count.

### Configuration Types

#### RetryPolicy (LangGraph)

Per-node retry. Separate from synwire-core's `RetryConfig` (Runnable composition).

```text
RetryPolicy
├── initial_interval: Duration         (default: 500ms)
├── backoff_factor: f64                (default: 2.0)
├── max_interval: Duration             (default: 128s)
├── max_attempts: u32                  (default: 3)
├── jitter: bool                       (default: true)
├── retry_on: Box<dyn Fn(&SynwireGraphError) -> bool>
```

#### CachePolicy

```text
CachePolicy
├── key_func: fn(&[Value]) -> Vec<u8>
├── ttl: Option<u64>
```

#### MessagesState

Convenience state for message-centric workflows.

```text
#[derive(State)]
MessagesState {
    #[reducer(add_messages)]
    messages: Vec<Message>,
}
```

### Managed Values

```text
IsLastStep: bool
RemainingSteps: usize
```

### Constants

```text
START: &str = "__start__"
END: &str = "__end__"
```

### TypedValue

Runtime type-safe value. Opt-in alternative to `serde_json::Value`.

```text
#[non_exhaustive]
TypedValue (enum)
├── String(String)
├── Integer(i64)
├── Float(f64)
├── Boolean(bool)
├── Secret(SecretValue)
├── List(Vec<TypedValue>)
├── Map(HashMap<String, TypedValue>)
├── Json(Value)
├── None
```

### NodeErrorStrategy

```text
#[non_exhaustive]
NodeErrorStrategy (enum)
├── FailWorkflow          (default)
├── FailBranch
├── Continue
```

### NodeState

```text
#[non_exhaustive]
NodeState (enum)
├── Pending
├── Running
├── Succeeded
├── Failed { error: SynwireGraphError }
├── Skipped { reason: String }
├── Paused { interrupt: Interrupt }
```

### GraphExecutionMetrics

```text
GraphExecutionMetrics
├── total_input_tokens: u64
├── total_output_tokens: u64
├── total_tokens: u64
├── model_invocations: u64
├── execution_duration: Duration
├── step_count: u64
├── node_metrics: HashMap<String, NodeMetrics>
```

### NodeMetrics

```text
NodeMetrics
├── duration: Duration
├── input_tokens: u64
├── output_tokens: u64
├── retries: u32
├── state: NodeState
```

### HttpClientConfig

```text
HttpClientConfig
├── timeout: Duration
├── ssrf_protection: bool           (default: true)
├── allow_list: Vec<IpNet>
├── proxy: Option<String>
├── user_agent: Option<String>
```

---

## Checkpoint Types (synwire-checkpoint)

### Checkpoint

```text
Checkpoint
├── v: u32                             (format version, currently 1)
├── id: String
├── ts: String                         (ISO 8601)
├── channel_values: HashMap<String, Value>
├── channel_versions: HashMap<String, ChannelVersion>
├── versions_seen: HashMap<String, HashMap<String, ChannelVersion>>
├── updated_channels: Option<Vec<String>>
```

> **Architecture review fix §2.4**: `SecretValue` in checkpoints serialises as a
> sentinel reference `{"__secret__": "key_name"}`. Secrets are re-fetched from
> `CredentialProvider` on restore. Checkpoint files use mode `0600`.

### ChannelVersion

```text
#[non_exhaustive]
ChannelVersion (enum)
├── Int(i64)
├── Float(f64)
├── String(String)
```

### CheckpointMetadata

```text
CheckpointMetadata
├── source: CheckpointSource
├── step: i64
├── parents: HashMap<String, String>
├── run_id: Option<String>
```

### CheckpointSource

```text
#[non_exhaustive]
CheckpointSource (enum)
├── Input
├── Loop
├── Update
├── Fork
```

### CheckpointTuple

```text
CheckpointTuple
├── config: RunnableConfig
├── checkpoint: Checkpoint
├── metadata: CheckpointMetadata
├── parent_config: Option<RunnableConfig>
├── pending_writes: Option<Vec<PendingWrite>>
```

### PendingWrite

```text
PendingWrite
├── task_id: String
├── channel: String
├── value: Value
```

### SerializerProtocol (trait)

```text
SerializerProtocol (trait)
├── fn dumps_typed(obj: &Value) -> Result<(String, Vec<u8>), SerializeError>
├── fn loads_typed(data: (&str, &[u8])) -> Result<Value, DeserializeError>
```

---

## Store Types (synwire-checkpoint)

### Item

```text
Item
├── value: HashMap<String, Value>
├── key: String
├── namespace: Vec<String>
├── created_at: DateTime<Utc>
├── updated_at: DateTime<Utc>
```

### SearchItem

```text
SearchItem
├── (inherits Item fields)
├── score: Option<f64>
```

### Store Operations

```text
GetOp { namespace, key, refresh_ttl }
SearchOp { namespace_prefix, filter, limit, offset, query, refresh_ttl }
PutOp { namespace, key, value, index, ttl }
ListNamespacesOp { match_conditions, max_depth, limit, offset }
```

### TTLConfig / IndexConfig

```text
TTLConfig { refresh_on_read, default_ttl, sweep_interval_minutes }
IndexConfig { dims, embed, fields }
```

---

## Cache Types

### BaseCache (trait)

```text
BaseCache<V> (trait)
├── async fn get(keys) -> HashMap<Key, V>
├── async fn set(pairs: HashMap<Key, (V, Option<u64>)>)
├── async fn clear(namespaces: Option<&[Vec<String>]>)
```

---

## Functional API Types

### TaskFunction

```text
TaskFunction<Args, Ret>
├── func: Box<dyn Fn(Args) -> BoxFuture<Result<Ret>>>
├── retry_policy: Vec<RetryPolicy>
├── cache_policy: Option<CachePolicy>
├── name: Option<String>
```

### Entrypoint / EntrypointFinal

```text
Entrypoint<Context = ()> {
    checkpointer, store, cache, cache_policy, retry_policy
}

EntrypointFinal<R, S> {
    value: R,    // returned to caller
    save: S,     // persisted to checkpoint
}
```

---

## Runtime Context Types

### Runtime

```text
Runtime<Context = ()>
├── context: Context
├── store: Option<&dyn BaseStore>
├── stream_writer: Option<StreamWriter>
├── config: RunnableConfig
```

### StreamWriter

```text
StreamWriter = Box<dyn Fn(Value) + Send + Sync>
```

---

## Deferred Types (M2/M3)

The following type categories are deferred to later milestones. See [roadmap](../../docs/roadmap.md).

- **Agent framework types** (M2): `RunContext<D>`, `OutputMode<T>`, `ModelProfile`, `AgentResult<O>`, `AgentStreamEvent<O>`, `ModelSpec`, `Agent<D,O>` builder
- **Backend protocol types** (M2): `BackendProtocol`, `SandboxBackendProtocol`, `CompositeBackend`, `FileInfo`, `GrepMatch`, `WriteResult`, `EditResult`, `ExecuteResponse`
- **Middleware types** (M2): `SkillDefinition`, `SubAgent`, `AgentState` (extended)
- **Guardrail types** (M2): `GuardrailResult`, `ToolTimeoutBehavior`, `InputGuardrail`, `OutputGuardrail`
- **Approval types** (M2): `ApprovalKind`, `ApprovalRequest`, `ApprovalResponse`
- **Session types** (M2): `RunState`, `SessionProvider`
- **Protocol types** (M3): `AgUiEvent`, `AgUiRuntime`, `FrontendTool`, A2A types, Agent Spec types
- **DSPy types** (M3): `Signature`, `Module`, `Prediction`, `Adapter`
- **Evaluation types** (M3): `Score`, `Scorer`, `EvalCase`, `EvalResult`, `EvalSummary`, `Experiment`
- **Extraction types** (M2): `PartialStream<T>`, `IterableStream<T>`, `Maybe<T>`, `BatchProcessor<T>`
