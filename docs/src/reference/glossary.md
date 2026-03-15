# Glossary

**Agent** -- A system that uses a language model to decide which actions to take. In Synwire, agents are implemented as state graphs with conditional edges.

**BaseChatModel** -- Core trait for chat language models. Provides `invoke`, `batch`, `stream`, `model_type`, and `bind_tools`.

**BaseChannel** -- Trait for state channels in graph execution. Manages how values are accumulated during supersteps.

**BoxFuture** -- `Pin<Box<dyn Future<Output = T> + Send + 'a>>`. Used for dyn-compatible async trait methods.

**BoxStream** -- `Pin<Box<dyn Stream<Item = T> + Send + 'a>>`. Used for streaming responses.

**CallbackHandler** -- Trait for receiving observability events during execution (LLM start/end, tool start/end, retries).

**Channel** -- A state management unit in graph execution. Each channel stores and reduces values for a single key.

**ChatChunk** -- An incremental piece of a streaming chat response. Contains `delta_content`, `delta_tool_calls`, `finish_reason`, and `usage`.

**ChatResult** -- The complete result of a chat model invocation. Contains the AI `Message`, optional generation info, and optional cost estimate.

**CompiledGraph** -- An executable graph produced by `StateGraph::compile()`. Runs the Pregel superstep loop.

**ConditionFn** -- A function that inspects graph state and returns a branch key for conditional edge routing.

**CredentialProvider** -- Trait for retrieving API keys and secrets. Implementations include `EnvCredentialProvider` and `StaticCredentialProvider`.

**Document** -- A text document with metadata, used in RAG pipelines.

**Embeddings** -- Trait for text embedding models. Provides `embed_documents` and `embed_query`.

**FakeChatModel** -- A deterministic chat model for testing. Returns pre-configured responses without API calls.

**FakeEmbeddings** -- A deterministic embedding model for testing. Returns consistent vectors without API calls.

**GraphError** -- Error type for graph construction, compilation, and execution errors.

**Message** -- An enum representing conversation messages: human, AI, system, or tool.

**NodeFn** -- A boxed async function that transforms graph state: `Box<dyn Fn(Value) -> BoxFuture<Result<Value, GraphError>>>`.

**OutputMode** -- Strategy for extracting structured output: `Native`, `Tool`, `Prompt`, or `Custom`.

**OutputParser** -- Trait for transforming raw model text into structured types.

**Pregel** -- The execution model used by `synwire-orchestrator`. Processes graphs via sequential supersteps.

**PromptTemplate** -- A string template with named variables for formatting prompts.

**ReAct** -- Reason + Act agent pattern. Loops between model invocation and tool execution until the model responds without tool calls.

**Retriever** -- Trait for document retrieval, typically backed by a vector store.

**RunnableConfig** -- Per-invocation configuration carrying callbacks, tags, and metadata.

**RunnableCore** -- Universal composition trait. Uses `serde_json::Value` for input/output.

**SecretValue** -- A wrapper that redacts secrets on `Display` and `Debug`. Prevents accidental logging of API keys.

**StateGraph** -- A builder for constructing state machines with nodes and edges, compiled into `CompiledGraph`.

**StreamMode** -- Controls what data is emitted during streaming graph execution: `Values`, `Updates`, `Debug`, `Messages`, `Custom`.

**StructuredTool** -- A concrete `Tool` implementation built via `StructuredToolBuilder`.

**Superstep** -- One iteration of the Pregel loop: execute a node, resolve the next edge.

**SynwireError** -- Top-level error enum wrapping domain-specific error types.

**SynwireErrorKind** -- Discriminant enum for matching error categories without inspecting payloads.

**Tool** -- Trait for callable tools with `name`, `description`, `schema`, and `invoke`.

**ToolSchema** -- JSON Schema description of a tool's parameters.

**VectorStore** -- Trait for storing and querying document embeddings.
