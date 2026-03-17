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

**Language Server Protocol (LSP)** -- A protocol for providing code intelligence (go-to-definition, hover, diagnostics, completions) between an editor/tool (client) and a language-specific server. Synwire wraps LSP via `async-lsp` in `synwire-lsp`.

**LanguageServerEntry** -- Configuration for a language server: command, args, file extensions, installation instructions. Stored in `LanguageServerRegistry`. Lives in `synwire-lsp`.

**LanguageServerRegistry** -- Registry of known language servers with auto-detection by file extension and installation guidance. Ships with 22 built-in entries from langserver.org. Lives in `synwire-lsp`.

**LocalProvider** -- `Vfs` implementation for real filesystem I/O, scoped to a root path. Path-traversal attacks are blocked via `normalize_path()`. Lives in `synwire-agent`.

**LspClient** -- High-level LSP client wrapping `async-lsp`'s `ServerSocket` and `MainLoop`. Provides async methods for all LSP operations and caches server capabilities and diagnostics. Lives in `synwire-lsp`.

**LspPlugin** -- `Plugin` implementation that contributes LSP tools to agents, bridges `publishDiagnostics` and other server notifications to the hook and signal systems, and auto-starts language servers. Lives in `synwire-lsp`.

**LspServerState** -- Enum for language server lifecycle: `NotStarted`, `Initializing`, `Ready`, `ShuttingDown`, `Stopped`, `Failed`. Lives in `synwire-lsp`.

**FsmStrategy** -- `ExecutionStrategy` implementation that governs agent turns with a finite state machine. Built via `FsmStrategyBuilder`. Lives in `synwire-agent`.

**FsmStrategyBuilder** -- Builder for `FsmStrategy`. Methods: `add_state`, `add_transition`, `set_initial_state`, `set_guard`, `build`. Lives in `synwire-agent`.

**ContentLengthCodec** -- `tokio_util::codec` implementation for the DAP wire format (`Content-Length: N\r\n\r\n{json}`). Shared framing protocol with LSP. Lives in `synwire-dap`.

**DapClient** -- High-level Debug Adapter Protocol client. Wraps `DapTransport` to provide methods for debug operations: breakpoints, stepping, variable inspection. Lives in `synwire-dap`.

**DapPlugin** -- `Plugin` implementation that contributes DAP tools to agents and bridges debug events (`stopped`, `output`, `terminated`) to the hook and signal systems. Lives in `synwire-dap`.

**DapSessionState** -- Enum for debug session lifecycle: `NotStarted`, `Initializing`, `Configured`, `Running`, `Stopped`, `Terminated`. Lives in `synwire-dap`.

**Debug Adapter Protocol (DAP)** -- A protocol for communicating with debuggers, analogous to LSP for language intelligence. Uses JSON messages with `Content-Length` framing over stdio.

**DebugAdapterRegistry** -- Registry of known debug adapters (codelldb, dlv-dap, debugpy, etc.) with auto-detection and installation instructions. Lives in `synwire-dap`.

**GraphError** -- Error type for graph construction, compilation, and execution errors.

**McpConnectionState** -- Enum for MCP transport lifecycle: `Disconnected`, `Connecting`, `Connected`, `Reconnecting`, `Failed`. Lives in `synwire-agent`.

**McpLifecycleManager** -- Manages connection lifecycle (reconnection, health checks) for any `McpTransport`. Lives in `synwire-agent`.

**McpTransport** -- Trait in `synwire-core` for MCP protocol transports. Three implementations in `synwire-agent`: `StdioMcpTransport`, `HttpMcpTransport`, `InProcessMcpTransport`.

**Message** -- An enum representing conversation messages: human, AI, system, or tool.

**NodeFn** -- A boxed async function that transforms graph state: `Box<dyn Fn(Value) -> BoxFuture<Result<Value, GraphError>>>`.

**Action** -- The response an agent emits after processing a `Signal`. Variants: `Continue`, `GracefulStop`, `ForceStop`, `Transition(state)`, `Custom`. Lives in `synwire-agent`.

**AgentError** -- Top-level error enum for the agent runtime. `#[non_exhaustive]`. Covers `Vfs`, `Strategy`, `Middleware`, `Permission`, `Mcp`, `Session`, `Plugin` variants. Lives in `synwire-agent`.

**AgentEvent** -- Stream item emitted by `Runner::run`. Variants include `Text`, `ToolCall`, `ToolResult`, `Thinking`, `DirectiveEmitted`, `UsageUpdate`, `Done`. Lives in `synwire-agent`.

**AgentNode** -- Core trait in `synwire-core`. Implementors receive `(Directive, &State, &Context)` and return `BoxFuture<DirectiveResult<S>>`.

**ApprovalDecision** -- Enum returned by an approval callback. Variants: `Allow`, `AllowAlways`, `AllowModified(Directive)`, `Deny`. Lives in `synwire-agent`.

**ApprovalRequest** -- Value passed to an approval callback containing the `Directive`, `RiskLevel`, and contextual metadata. Lives in `synwire-agent`.

**ArchiveManager** -- `Vfs` implementation for reading and writing tar/zip/gzip archives, scoped to a root path. Lives in `synwire-agent`.

**VfsError** -- Error enum for `Vfs` operations. `#[non_exhaustive]`. Covers `Io`, `Permission`, `NotFound`, `Timeout`, `Custom` variants. Lives in `synwire-agent`.

**Vfs** -- Trait in `synwire-core` defining all file, shell, HTTP, and process operations an agent may perform as algebraic effects.

**CompositeProvider** -- Routes `Vfs` calls to different VFS providers by path prefix (mount table). Lives in `synwire-agent`.

**Directive** -- Enum in `synwire-core` representing an intended effect returned by `AgentNode::process`. Variants: `Emit { event }`, `SpawnAgent { … }`, `StopChild { id }`, `Stop { reason }`, `SpawnTask { … }`, `StopTask { id }`, `RunInstruction { … }`, `Schedule { … }`, `Cron { … }`, `Custom(Box<dyn …>)`.

**DirectiveExecutor** -- Trait in `synwire-core` that carries out `Directive` values. Returns `BoxFuture<DirectiveResult<S>>`.

**DirectiveFilter** -- Trait in `synwire-core` that intercepts `Directive` values before execution; can allow, deny, or transform them.

**DirectiveResult** -- Type alias for `Result<AgentEvent, AgentError>`. The output of `DirectiveExecutor::execute`. The `S` type parameter is the agent's state type.

**DirectStrategy** -- `ExecutionStrategy` implementation that passes control entirely to the model with no state machine constraints. Lives in `synwire-agent`.

**ExecutionStrategy** -- Trait in `synwire-core` controlling how the `Runner` sequences turns. Two built-in implementations: `DirectStrategy` and `FsmStrategy` (both in `synwire-agent`).

**OutputMode** -- Strategy for extracting structured output: `Native`, `Tool`, `Prompt`, or `Custom`.

**OutputParser** -- Trait for transforming raw model text into structured types.

**GrepMatch** -- Struct returned by grep operations. Fields: `path`, `line_number`, `line`, `context_before`, `context_after`. Lives in `synwire-agent`.

**GrepOptions** -- Config struct for grep/search operations. Fields: `pattern`, `paths`, `file_type`, `glob`, `context_lines`, `output_mode`, `invert`, `count_only`, `case_insensitive`. Lives in `synwire-agent`.

**HookRegistry** -- Registry of lifecycle hooks (`before_run`, `after_run`, `on_event`) attached to a `Runner`. Lives in `synwire-agent`.

**InMemorySessionManager** -- In-process `SessionManager` implementation. Session data is lost on process exit. Lives in `synwire-agent`.

**Shell** -- `Vfs` implementation for sandboxed shell command execution. Working directory is scoped to `root`. Lives in `synwire-agent`.

**Middleware** -- Trait in `synwire-core` applied before each agent turn. Receives and can mutate the context; returns `MiddlewareResult`.

**MiddlewareStack** -- Ordered list of `Middleware` instances applied left-to-right before each turn. Short-circuits on `MiddlewareResult::Stop`. Lives in `synwire-agent`.

**PermissionBehavior** -- Enum for what happens when a `PermissionRule` matches. Variants: `Allow`, `Deny`, `Ask`. Lives in `synwire-agent`.

**PermissionMode** -- Enum preset for agent permission posture. Variants: `Unrestricted`, `Restricted`, `Sandboxed`, `ApprovalRequired`, `Custom(Vec<PermissionRule>)`. Lives in `synwire-agent`.

**PermissionRule** -- A single declarative rule mapping a tool pattern to a `PermissionBehavior`. Lives in `synwire-agent`.

**PipelineExecutor** -- Runs a sequence of pipeline stages sequentially with a shared timeout. Lives in `synwire-agent`.

**Plugin** -- Trait in `synwire-core` for stateful components with lifecycle hooks: `before_run`, `after_run`, `on_event`, `signal_routes`.

**PluginHandle** -- Type-safe accessor for plugin state isolated by `PluginStateKey`. Prevents cross-plugin state access. Lives in `synwire-agent`.

**PluginStateKey** -- Marker trait used to namespace plugin state within the shared agent state. One type per plugin. Lives in `synwire-core`.

**ProcessManager** -- `Vfs` implementation for spawning, monitoring, and killing background processes. Lives in `synwire-agent`.

**Pregel** -- The execution model used by `synwire-orchestrator`. Processes graphs via sequential supersteps.

**PromptTemplate** -- A string template with named variables for formatting prompts.

**ReAct** -- Reason + Act agent pattern. Loops between model invocation and tool execution until the model responds without tool calls.

**RiskLevel** -- Enum classifying how dangerous a `Directive` is. Variants: `None`, `Low`, `Medium`, `High`, `Critical`. Lives in `synwire-agent`.

**Runner** -- Entry point for the agent runtime in `synwire-agent`. Drives the turn loop wrapping `AgentNode` + `ExecutionStrategy` + middleware + VFS + session.

**Retriever** -- Trait for document retrieval, typically backed by a vector store.

**RunnableConfig** -- Per-invocation configuration carrying callbacks, tags, and metadata.

**RunnableCore** -- Universal composition trait. Uses `serde_json::Value` for input/output.

**SecretValue** -- A wrapper that redacts secrets on `Display` and `Debug`. Prevents accidental logging of API keys.

**Session** -- Persisted agent session containing message history, metadata, and plugin state. Lives in `synwire-agent`.

**SessionManager** -- Trait in `synwire-core` for session CRUD. Methods: `create`, `get`, `update`, `delete`, `list`, `fork`, `rewind`, `tag`.

**SessionMetadata** -- Fields on a `Session`: `id`, `created_at`, `updated_at`, `tags`, `thread_id`, `agent_id`. Lives in `synwire-agent`.

**Signal** -- A value delivered to the agent's signal router. Carries a `SignalKind` and optional payload. Lives in `synwire-agent`.

**SignalKind** -- Enum of signal categories. Variants: `Stop`, `UserMessage`, `ToolResult`, `Timer`, `Custom(String)`. Lives in `synwire-agent`.

**SignalRoute** -- A mapping from a `SignalKind` pattern to an `Action`, used in `ComposedRouter`. Lives in `synwire-agent`.

**MemoryProvider** -- Ephemeral in-memory `Vfs` implementation. All data is lost when the `Runner` drops. Safe for sandboxed agents. Lives in `synwire-agent`.

**StateGraph** -- A builder for constructing state machines with nodes and edges, compiled into `CompiledGraph`.

**StreamMode** -- Controls what data is emitted during streaming graph execution: `Values`, `Updates`, `Debug`, `Messages`, `Custom`.

**StoreProvider** -- `Vfs` implementation backed by a `BaseStore` for K-V persistence. Lives in `synwire-agent`.

**StrategyError** -- Error enum for `ExecutionStrategy` failures. `#[non_exhaustive]`. Covers `InvalidTransition`, `GuardFailed`, `StateNotFound`, `Custom`. Lives in `synwire-agent`.

**StructuredTool** -- A concrete `Tool` implementation built via `StructuredToolBuilder`.

**Superstep** -- One iteration of the Pregel loop: execute a node, resolve the next edge.

**SynwireError** -- Top-level error enum wrapping domain-specific error types.

**SynwireErrorKind** -- Discriminant enum for matching error categories without inspecting payloads.

**ThresholdGate** -- Approval gate that triggers human approval when a `Directive`'s `RiskLevel` meets or exceeds a configured threshold. Lives in `synwire-agent`.

**Tool** -- Trait for callable tools with `name`, `description`, `schema`, and `invoke`.

**ToolSchema** -- JSON Schema description of a tool's parameters.

**Usage** -- Token/cost accounting for a single agent turn. Fields: `input_tokens`, `output_tokens`, `cache_read_tokens`, `cache_write_tokens`. Lives in `synwire-agent`.

**VectorStore** -- Trait for storing and querying document embeddings.

**RepoId** -- Stable identifier for a repository family (all clones and worktrees of the same repo). Derived from the SHA-1 of the first (root) Git commit, or SHA-256 of the canonical path when Git is unavailable. Lives in `synwire-storage`.

**WorktreeId** -- Identifies a specific working copy within a repository family. Combines `RepoId` with a SHA-256 of the canonicalised worktree root path. `key()` returns a compact filesystem-safe string. Lives in `synwire-storage`.

**StorageLayout** -- Computes all Synwire storage paths for a given product name using platform-appropriate base directories (`$XDG_DATA_HOME`, `$XDG_CACHE_HOME`). Separates durable data from regenerable cache. Configuration hierarchy: env vars > programmatic override > project config > platform default. Lives in `synwire-storage`.

**CommunityState** -- Persisted result of HIT-Leiden community detection over the code graph. Stores community membership, community summaries (generated via `SamplingProvider`), and inter-community edge weights. Persisted at `StorageLayout::communities_dir()`. Lives in `synwire-index` (behind `community-detection` feature flag).

**SamplingProvider** -- Trait for tool-internal LLM access. Two implementations: `McpSampling` (delegates to MCP host via `sampling/createMessage`) and `DirectModelSampling` (uses a configured `BaseChatModel` directly). Used by community summary generation and hierarchical narrowing to avoid zero calls during indexing. Lives in `synwire-core`.

**ToolSearchIndex** -- Framework-level tool registry that supports progressive tool discovery via embedding-based retrieval and namespace grouping. Reduces context token usage by ~85% vs listing all tools upfront. Supports hybrid scoring (vector + keyword boosting), seen/unseen adaptive penalties, and transition graph boosting. Lives in `synwire-core`.

**agentskills.io** -- Open specification for discoverable, composable agent skills. A skill is a directory containing `SKILL.md` (YAML frontmatter + instructions), `scripts/`, `references/`, and `assets/`. Synwire extends the spec with an optional `runtime` field (`lua`, `rhai`, `wasm`, `tool-sequence`, `external`). Implemented by `synwire-agent-skills`.
