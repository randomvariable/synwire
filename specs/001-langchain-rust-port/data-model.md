# Data Model: LangChain Rust Port

**Date**: 2026-03-09
**Branch**: `001-langchain-rust-port`

## Core Types

### Message

A single unit of conversation. Modelled as a Rust enum (closed set).

```text
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
produce or consume messages with custom roles beyond human/ai/system/tool. Equivalent
to Python's `ChatMessage(role="custom_role", content="...")`.

**MessageContent** (enum):
- `Text(String)` — plain text
- `Blocks(Vec<ContentBlock>)` — structured content (text + images)

Python's `content_blocks` property is equivalent to the `MessageContent::Blocks` variant. No additional accessor needed.

**ContentBlock** (enum):
- `Text { text: String }`
- `Image { url: String, detail: Option<String> }`
- `Audio { url: String, mime_type: Option<String> }`
- `Video { url: String, mime_type: Option<String> }`
- `File { url: String, mime_type: Option<String> }`
- `Reasoning { text: String }` — chain-of-thought reasoning from models (e.g. Claude)
- `Thinking { text: String }` — model thinking/scratchpad content (e.g. Claude extended thinking)

**Excluded ContentBlock types** (provider-specific, not core abstractions):
- `GuardContent`: Provider-specific safety guardrail wrapper. Providers return
  this as part of their response_metadata; no core type needed.
- `RefusalContent`: Provider-specific refusal format. Mapped to content text
  with response_metadata indicating refusal.
- `CitationContent`: Provider-specific citation format. Mapped to
  response_metadata or additional_kwargs on the message.
- `CacheControl`: Provider-specific caching hints (e.g. Anthropic prompt
  caching). Belongs in provider-specific configuration, not core content blocks.

**pretty_repr / pretty_print**: Not ported. Rust uses `Debug` and `Display` trait implementations for human-readable formatting.

### Excluded Message Types

- **`FunctionMessage`**: Deprecated in Python, superseded by `ToolMessage`.
  All function-calling workflows use `ToolMessage` with `tool_call_id`. Not ported.
- **`RemoveMessage`**: LangGraph state management primitive that signals removal
  of a message by ID during graph state reduction. Not a core LangChain type —
  belongs to the LangGraph framework. Not ported.
- **`AgentActionMessageLog`**: Niche variant of `AgentAction` carrying a message
  log reference. The standard `AgentAction` with its `log` field covers
  the same use case. Not ported.

**Relationships**: Messages are ordered in a `Vec<Message>` for conversation history.

### ToolCall

A structured request from a model to invoke a tool.

```text
ToolCall
├── id: String
├── name: String
├── arguments: HashMap<String, Value>  (serde_json::Value)
```

### InvalidToolCall

A tool call that the model attempted but failed to produce valid arguments for.
Present on `AI` message variant alongside valid `tool_calls`.

```text
InvalidToolCall
├── name: Option<String>
├── arguments: Option<String>     (raw unparsed string)
├── id: Option<String>
├── error: String                 (description of what went wrong)
```

### ToolStatus

Status of a tool invocation result, carried on `Tool` message variant.

```text
ToolStatus (enum)
├── Success
├── Error
```

Serde serialisation: `"success"` / `"error"` (lowercase strings matching Python's `Literal["success", "error"]`).

### UsageMetadata

Token usage statistics from a model invocation.

```text
UsageMetadata
├── input_tokens: u64
├── output_tokens: u64
├── total_tokens: u64
```

### Document

A retrievable piece of content.

```text
Document
├── id: Option<String>
├── page_content: String
├── metadata: HashMap<String, Value>
```

**Validation**: `page_content` MUST NOT be empty for documents added to
vector stores. `metadata` is free-form but commonly contains `source`,
`page`, and `title` keys.

**`type` discriminator**: Python Document has `type: Literal["Document"]` for
JSON-based type discrimination. Not needed in Rust — struct typing is
sufficient. For JSON serialisation compatibility, use
`#[serde(tag = "type", rename = "Document")]` if round-trip interop with
Python is required.

### ChatResult

The output of a chat model invocation.

```text
ChatResult
├── message: Message (AI variant)
├── generation_info: Option<HashMap<String, Value>>
```

### LLMResult

The output of a batch LLM invocation.

```text
LLMResult
├── generations: Vec<Vec<Generation>>
├── llm_output: Option<HashMap<String, Value>>
```

### Generation

A single generation from an LLM.

```text
Generation
├── text: String
├── generation_info: Option<HashMap<String, Value>>
```

### ChatChunk

A single chunk of streaming output from a chat model. This is the universal
streaming type — Rust does NOT use separate chunk types per message variant
(AIMessageChunk, HumanMessageChunk, etc.) as Python does. Python's per-type
chunks exist because of class inheritance; Rust's enum + ChatChunk is
equivalent.

```text
ChatChunk
├── delta_content: Option<String>
├── delta_tool_calls: Vec<ToolCallChunk>
├── finish_reason: Option<String>
├── usage: Option<UsageMetadata>
```

**Per-type chunk mapping**: Python has separate `AIMessageChunk`, `HumanMessageChunk`,
`SystemMessageChunk`, `ToolMessageChunk`, and `FunctionMessageChunk`, each with
`__add__` for concatenation. Rust collapses ALL of these into `ChatChunk`:
- `AIMessageChunk` → `ChatChunk` (carries delta_content, delta_tool_calls, usage)
- `HumanMessageChunk` → Not applicable — humans don't stream
- `SystemMessageChunk` → Not applicable — system messages don't stream
- `ToolMessageChunk` → Not applicable — tool results are complete, not streamed
- `FunctionMessageChunk` → Excluded alongside `FunctionMessage` (deprecated)

Only `AIMessageChunk` carries meaningful streaming data; the others exist in Python
for type consistency but are rarely used in practice.

**Chunk concatenation**: `ChatChunk` implements a `merge(&mut self, other: &ChatChunk)`
method (and optionally `std::ops::AddAssign`) with these semantics:
- Appends `delta_content` strings (None + Some(s) = Some(s))
- Merges `delta_tool_calls` by `index` — concatenates partial `arguments` strings
- Takes last non-None `finish_reason`
- Takes last non-None `usage`

### ToolCallChunk

A partial tool call received during streaming. Accumulated by index to
reconstruct a complete `ToolCall`.

```text
ToolCallChunk
├── index: usize
├── id: Option<String>
├── name: Option<String>
├── arguments: Option<String>    (partial JSON string, concatenated across chunks)
```

### PromptValue

The output of formatting a prompt template. Can be consumed as either a
string (for LLMs) or a list of messages (for chat models).

```text
PromptValue (enum)
├── String(String)
├── Messages(Vec<Message>)
```

Methods:
- `to_string(&self) -> String` — returns the string or formats messages
- `to_messages(&self) -> Vec<Message>` — returns messages or wraps string in a HumanMessage

### PromptTemplate

A parameterised template for formatting prompts.

```text
PromptTemplate
├── template: String
├── input_variables: Vec<String>
├── template_format: TemplateFormat (enum: FString, Mustache)
```

**Validation**: All variables referenced in `template` MUST appear in
`input_variables`. Formatting with missing variables returns an error.

### ChatPromptTemplate

A sequence of message templates for chat model prompts.

```text
ChatPromptTemplate
├── messages: Vec<MessageTemplate>
├── input_variables: Vec<String>
```

**MessageTemplate** (enum):
- `Human(String)` — template string for human message
- `AI(String)` — template string for AI message
- `System(String)` — template string for system message
- `Placeholder(String)` — variable name to insert a Vec<Message>

### RunnableConfig

Configuration passed through a runnable chain.

```text
RunnableConfig
├── callbacks: Option<Vec<Box<dyn CallbackHandler>>>
├── tags: Option<Vec<String>>
├── metadata: Option<HashMap<String, Value>>
├── max_concurrency: Option<usize>
├── run_name: Option<String>
├── run_id: Option<Uuid>
├── configurable: Option<HashMap<String, Value>>
```

### ToolSchema

The JSON Schema description of a tool's input.

```text
ToolSchema
├── name: String
├── description: String
├── parameters: Value  (JSON Schema object)
```

### ToolOutput

The return value from a tool invocation. Separates the model-visible content
from the rich artifact that may be used downstream but not sent to the model.

```text
ToolOutput
├── content: String              (text result shown to the model)
├── artifact: Option<Value>      (rich output not sent to model — images, data, etc.)
```

When constructing a `Tool` message from a `ToolOutput`, `content` maps to the
message content and `artifact` maps to the message's `artifact` field.

## Agent Types

### AgentAction

A request from an agent to execute a tool.

```text
AgentAction
├── tool: String            (name of the tool to execute)
├── tool_input: Value       (input to pass to the tool — string or dict)
├── log: String             (reasoning / chain-of-thought text)
```

### AgentFinish

The final return value from an agent.

```text
AgentFinish
├── return_values: HashMap<String, Value>
├── log: String             (final reasoning text)
```

### AgentStep

The result of executing one agent action (action + tool observation).

```text
AgentStep
├── action: AgentAction
├── observation: String     (tool output / observation text)
```

### AgentDecision

What the agent LLM decided to do (parsed from model output).

```text
AgentDecision (enum)
├── Action(AgentAction)           (single tool call)
├── Actions(Vec<AgentAction>)     (parallel tool calls)
├── Finish(AgentFinish)           (agent is done)
```

### AgentInput

Input to the agent's inner Runnable (LLM + output parser).

```text
AgentInput
├── input: HashMap<String, Value>
├── intermediate_steps: Vec<AgentStep>
├── chat_history: Option<Vec<Message>>
```

### AgentExecutor

Runs a ReAct-style loop: LLM → parse → tool call → observation → LLM →
... → AgentFinish. Implements `Runnable<HashMap<String, Value>, HashMap<String, Value>>`.

```text
AgentExecutor
├── agent: Box<dyn Runnable<AgentInput, AgentDecision>>
├── tools: Vec<Box<dyn Tool>>
├── max_iterations: Option<usize>       (default: 15)
├── max_execution_time: Option<Duration>
├── return_intermediate_steps: bool     (default: false)
├── handle_parsing_errors: bool         (default: false)
```

## Event Streaming Types

### StreamEvent

A structured event emitted by `stream_events` on a Runnable.

```text
StreamEvent (enum)
├── Standard {
│       event: String,          // "on_chain_start", "on_llm_stream", etc.
│       name: String,           // runnable name
│       run_id: String,
│       parent_ids: Vec<String>,
│       tags: Vec<String>,
│       metadata: HashMap<String, Value>,
│       data: EventData,
│   }
├── Custom {
│       name: String,           // user-defined event name
│       run_id: String,
│       parent_ids: Vec<String>,
│       tags: Vec<String>,
│       metadata: HashMap<String, Value>,
│       data: Value,            // arbitrary user data
│   }
```

**Standard event names** (following Python convention):
`on_chain_start`, `on_chain_stream`, `on_chain_end`,
`on_llm_start`, `on_llm_stream`, `on_llm_end`,
`on_chat_model_start`, `on_chat_model_stream`, `on_chat_model_end`,
`on_tool_start`, `on_tool_end`,
`on_retriever_start`, `on_retriever_end`,
`on_prompt_start`, `on_prompt_end`,
`on_custom_event`.

### EventData

Data payload within a standard StreamEvent.

```text
EventData
├── input: Option<Value>
├── output: Option<Value>
├── chunk: Option<Value>
├── error: Option<String>
```

### RunLogPatch

A JSON-Patch (RFC 6902) style log diff emitted by `stream_log`.

```text
RunLogPatch
├── ops: Vec<JsonPatchOp>
```

### JsonPatchOp

A single JSON Patch operation.

```text
JsonPatchOp
├── op: String                  ("add", "replace", "remove")
├── path: String                (JSON Pointer path)
├── value: Option<Value>
```

## Retry & Resilience Types

### RetryConfig

Configuration for `with_retry` Runnable composition.

```text
RetryConfig
├── retry_on: Vec<LangChainErrorKind>
├── max_attempts: u32                   (default: 3)
├── wait_exponential_jitter: bool       (default: true)
├── initial_interval: Duration          (default: 1s)
├── max_interval: Duration              (default: 60s)
```

### RetryState

State passed to the `on_retry` callback, describing the current retry attempt.

```text
RetryState
├── attempt: u32               (1-based attempt number)
├── error: LangChainError      (the error that triggered the retry)
├── elapsed: Duration          (time since first attempt)
```

### LangChainErrorKind

Discriminant enum for matching `LangChainError` variants without payload.
Used by `RetryConfig` and `with_fallbacks` to specify which errors to handle.

```text
LangChainErrorKind (enum)
├── Model
├── Prompt
├── Parse
├── Embedding
├── VectorStore
├── Tool
├── Agent
├── RetryExhausted
├── Serialization
├── Other
```

## Message Utility Functions

Standalone functions for common message list manipulations:

### filter_messages

Filter a message list by type, name, or ID.

```text
filter_messages(
    messages: &[Message],
    include_types: Option<&[MessageType]>,
    exclude_types: Option<&[MessageType]>,
    include_names: Option<&[String]>,
    exclude_names: Option<&[String]>,
    include_ids: Option<&[String]>,
    exclude_ids: Option<&[String]>,
) -> Vec<Message>
```

`MessageType` enum: `Human`, `AI`, `System`, `Tool`, `Chat`.

### trim_messages

Trim a message list to fit within a token budget.

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

`TrimStrategy` enum: `First` (keep first N that fit), `Last` (keep last N that fit).

### merge_message_runs

Merge consecutive messages of the same type into single messages.

```text
merge_message_runs(messages: &[Message]) -> Vec<Message>
```

Concatenates content of adjacent messages with the same variant. Useful for
cleaning up conversation history before sending to a model.

### Excluded Message Utilities

- **`convert_to_messages`**: Python converts dicts/tuples to Message objects.
  Rust uses `From`/`Into` trait implementations instead (see §Type Conversions).
- **`messages_to_dict` / `messages_from_dict`**: Python serialisation helpers.
  Rust uses `serde_json::to_value` / `serde_json::from_value` directly on
  `Vec<Message>` — no separate utility needed.

## Type Conversions

### MessageLike Trait

Trait for types that can be converted to a `Message`. Enables ergonomic APIs
that accept various input types.

```text
trait MessageLike: Send + Sync {
    fn to_message(&self) -> Message;
}
```

Implementations:
- `Message` → identity
- `&str` / `String` → `Message::Human { content: Text(s) }`
- `(MessageRole, &str)` → Message of the given role with text content

### Into<Vec<Message>> Implementations

Convenience conversions for common input types to message lists:

- `PromptValue::Messages(msgs)` → the inner `Vec<Message>`
- `PromptValue::String(s)` → `vec![Message::Human { content: Text(s) }]`
- `&str` → `vec![Message::Human { content: Text(s.to_string()) }]`
- `String` → `vec![Message::Human { content: Text(s) }]`

These enable `BaseChatModel::invoke` callers to pass `&str` or `PromptValue`
where `&[Message]` is expected (after conversion).

### Key From/Into Conversions

| From | To | Notes |
|---|---|---|
| `ToolOutput` | `Message::Tool` | Maps content and artifact |
| `&str` | `Message::Human` | Wraps in HumanMessage with Text content |
| `PromptValue` | `Vec<Message>` | Extracts messages or wraps string |
| `Vec<(String, String)>` | `HashMap<String, Value>` | For prompt variable maps |

## Prelude Module

The `langchain_core::prelude` module re-exports the most commonly used types
and traits for convenience:

```text
pub use crate::{
    // Core traits
    BaseChatModel, BaseLLM, Embeddings, VectorStore, Retriever,
    Runnable, Tool, OutputParser, CallbackHandler, MessageLike,

    // Core types
    Message, MessageContent, ContentBlock, ChatResult, ChatChunk,
    Document, PromptValue, PromptTemplate, ChatPromptTemplate,
    ToolCall, ToolSchema, ToolOutput, RunnableConfig,

    // Agent types
    AgentAction, AgentFinish, AgentStep, AgentDecision, AgentExecutor,

    // Error types
    LangChainError, LangChainErrorKind,

    // Type aliases
    BoxFuture, BoxStream,

    // Composition functions
    pipe, with_config, with_retry, with_fallbacks, as_tool,

    // Message utilities
    filter_messages, trim_messages, merge_message_runs,
};
```

The top-level `langchain` crate re-exports `langchain_core::prelude::*` plus
`langchain_openai::{ChatOpenAI, OpenAIEmbeddings}` for single-import convenience.

## Type Aliases

```text
BoxFuture<'a, T>  = Pin<Box<dyn Future<Output = T> + Send + 'a>>
BoxStream<'a, T>  = Pin<Box<dyn Stream<Item = T> + Send + 'a>>
Embedding         = Vec<f32>
```

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
  → Each ChatChunk contains: delta_content, delta_tool_calls, finish_reason, usage
  → Accumulate via ChatChunk.merge() to reconstruct full response
```

### RAG Flow

```text
Query (String)
  → Embeddings.embed_query() → Vec<f32>
  → VectorStore.similarity_search() → Vec<Document>
  → Documents injected into PromptTemplate context
  → BaseChatModel.invoke() → Answer
```

### Agent Execution Flow (ReAct Loop)

```text
User Input (HashMap<String, Value>)
  → AgentExecutor.invoke()
    → Loop:
      → Agent Runnable.invoke(AgentInput) → AgentDecision
        → If AgentDecision::Action(action):
          → on_agent_action callback
          → Tool.invoke(action.tool_input) → observation
          → Append AgentStep to intermediate_steps
          → Continue loop
        → If AgentDecision::Actions(actions):
          → Execute tools in parallel
          → Append all AgentSteps
          → Continue loop
        → If AgentDecision::Finish(finish):
          → on_agent_finish callback
          → Return finish.return_values
      → Check max_iterations / max_execution_time
  → HashMap<String, Value> (return values)
```
