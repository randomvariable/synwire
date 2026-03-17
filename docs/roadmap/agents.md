# Agent Framework

## Overview

The agent framework is a Milestone 2 feature spanning three crates: `synwire-agents`, `synwire-sandbox-local`, and `synwire-cli`. It provides the full agent runtime including a backend protocol for file and shell operations, a composable middleware stack with plugin state isolation, the agent graph and factory with directive-based execution, pluggable execution strategies, a CLI binary, a convenience API for ergonomic agent construction, prebuilt agents and workflow nodes, and sandbox integrations. The execution model draws from [Jido](https://github.com/agentjido/jido)'s pure functional agent architecture: agents return typed effect descriptions (directives) alongside state changes, enabling fully testable logic with zero side effects.

## Backend Protocol

The `Vfs` trait (FR-070) defines a pluggable interface for file operations (`ls_info`, `read`, `write`, `edit`, `grep_raw`, `glob_info`, `upload_files`, `download_files`) with both sync and async variants. `SandboxVfs` (FR-071) extends this with shell execution (`execute`/`aexecute`) and a unique `id` property.

Backend response types (FR-072) include `WriteResult`, `EditResult`, `ExecuteResponse`, `FileInfo`, `GrepMatch`, `FileDownloadResponse`, and `FileUploadResponse`. A standardised `FileOperationError` (FR-073) provides consistent error codes: `file_not_found`, `permission_denied`, `is_directory`, and `invalid_path`. `BackendFactory` (FR-074) is a type alias enabling late-binding construction at graph compile time. Path traversal protection (FR-350) is applied consistently across all backend types.

## Backend Implementations

- **MemoryProvider** (FR-075): Ephemeral, per-conversation file storage in agent state with `files_update` dicts for checkpointing.
- **StoreProvider** (FR-076): Persistent cross-conversation storage via `BaseStore` with configurable namespace isolation.
- **LocalProvider** (FR-077): Virtual mode with path traversal protection; real mode in `synwire-sandbox-local`. Includes symlink traversal protection.
- **Shell** (FR-078): Extends `LocalProvider` with unrestricted shell execution, environment variable control, output truncation, and timeout.
- **CompositeProvider** (FR-079): Routes file operations to sub-backends by path prefix using longest-match-first semantics, with aggregated listings and cross-backend search.
- **BaseSandbox** (FR-080): Abstract type implementing all `Vfs` operations by delegating to `execute()`. Subclasses implement only `execute()`, `upload_files()`, `download_files()`, and `id`.

## Middleware Stack

The middleware stack (FR-081) provides stackable, ordered middleware that adds tools, modifies system prompts, and transforms state. The default stack order is: TodoList, Filesystem, Summarisation, PromptCaching, PatchToolCalls.

- **FilesystemMiddleware** (FR-082): Exposes backend file operations as agent tools.
- **TodoListMiddleware** (FR-083): Provides a `write_todos` tool for task management.
- **MemoryMiddleware** (FR-084): Loads AGENTS.md-style memory files into the system prompt as `<agent_memory>` tags.
- **SkillsMiddleware** (FR-085): Loads agent skills from the backend with YAML frontmatter (name, description, license, compatibility, metadata, allowed_tools). Supports progressive disclosure.
- **SubAgentMiddleware** (FR-086): Adds a `task` tool for spawning isolated subagents with configurable name, description, system_prompt, tools, model, and middleware.
- **SummarisationMiddleware** (FR-087): Triggers context summarisation based on token or message count thresholds.
- **PatchToolCallsMiddleware** (FR-088): Detects dangling tool calls and adds synthetic `ToolMessage` responses.
- **PromptCachingMiddleware** (FR-089, FR-403): Provider-specific prompt caching, initially targeting Anthropic.

## Agent Execution Model

The execution model separates agent decision logic from side-effect execution. Agent nodes are pure functions: they receive state, return updated state plus typed effect descriptions (directives). A separate executor interprets directives into actual side effects. This makes agent logic fully unit-testable without mocks or process spawning.

### Directive System (FR-557-562)

- `Directive` enum with typed variants: Emit(Signal), SpawnAgent(AgentConfig), StopChild(AgentId), Schedule(Duration, Signal), RunInstruction(Action, Route), Cron(CronExpr, Signal), Stop. Custom variants via `Directive::Custom(Box<dyn DirectivePayload>)` (FR-557)
- `DirectiveResult<S>` return type combining `(S, Vec<Directive>)` — agent nodes return updated state plus zero or more directives. State changes are applied immediately; directives are deferred to the executor (FR-558)
- `DirectiveExecutor` trait with `execute_directive(&self, directive: Directive) -> Result<()>`. Default implementation provided by graph runtime. Custom executors supported for testing (record directives without executing) and dry-run analysis (FR-559)
- `RunInstruction` directive enables pure agents to request runtime execution: the executor runs the action, then routes the result back into the agent as a new input. Keeps agent logic pure while supporting multi-step workflows that depend on runtime results (FR-560)
- `DirectiveFilter` trait allows middleware to inspect, transform, or suppress directives before execution. Enables policy enforcement (e.g. block SpawnAgent in sandboxed mode) and audit logging (FR-561)
- Directive serialisation for replay and what-if analysis: directives implement `Serialize`/`Deserialize`, enabling recording of agent decisions and replaying them against different executor implementations (FR-562)

### Execution Strategies (FR-563-567)

Pluggable execution strategies decouple *what* an agent does from *how* it orchestrates. The same agent logic can run under different strategies depending on the use case.

- `ExecutionStrategy` trait with `execute(&self, agent: &Agent, action: Action, state: &S) -> StrategyResult<S>`, `tick(&self, state: &S) -> StrategyResult<S>` for multi-step continuation, and `snapshot(&self) -> StrategySnapshot` for stable execution views (FR-563)
- `DirectStrategy` — execute actions immediately and sequentially. Default for simple request/response workflows. Stateless, no strategy overhead (FR-564)
- `FsmStrategy` — finite state machine with explicit state transitions. Enforces "action X is only valid in state Y" constraints. Uses `RunInstruction` directives to keep agent logic pure while routing runtime results back through the FSM (FR-565)
- `FsmTransition` type defining from_state, to_state, action, and optional guard condition. `FsmStrategy::builder().add_transition()` builder. Invalid transitions return `StrategyError::InvalidTransition` with current state and attempted action (FR-566)
- Strategy-level signal routing: strategies can intercept incoming signals/messages before they reach agent logic. `ExecutionStrategy::signal_routes()` returns priority-ordered route mappings (FR-567)

### Execution Control (FR-363-366)

- `max_turns: Option<u32>` (FR-363) defaults to 10 and limits model invocation cycles.
- `run_error_handlers` (FR-364) use `RunErrorAction` with Continue, Retry, and Abort variants.
- `tool_error_formatter` (FR-365) allows custom formatting of tool errors before returning them to the LLM.
- Middleware early termination (FR-366) is supported via `MiddlewareResult::Terminate(AgentResult)`.

## Agent Graph and Factory

The `create_agent()` factory (FR-090) returns a `CompiledStateGraph` and accepts model, system_prompt, tools, middleware stack, backends, subagents, response_format, context_schema, checkpoint, store, and execution_strategy parameters. The returned graph uses `DirectiveResult<S>` as its node return type — nodes return state changes and directives, the graph executor handles directive interpretation.

Default built-in tools (FR-091) include `write_todos`, `ls`, `read_file`, `write_file`, `edit_file`, `glob`, and `grep`, plus `execute` when a sandbox is present and `task` when subagents are configured. Interrupt point configuration (FR-092) supports HITL approval via `interrupt_before` and `interrupt_after`. Dynamic system prompts (FR-093) are generated based on execution mode, available tools, and loaded skills. Agent state (FR-094) carries messages, files, todos, structured_response, memory_contents, and skills_metadata.

## Convenience API

The `Agent<D, O>` struct (FR-133) provides a builder API with typed dependencies, structured output, and automatic output mode negotiation. `RunContext<D>` (FR-134) carries typed deps, model reference, retry count, usage, and metadata.

`OutputMode<T>` (FR-135) is an enum with variants Tool, Native, Prompt, and Custom, where Tool is the default and universal mode. `ToolResult::Retry(String)` (FR-136) enables tool-initiated model self-correction. `ModelSelector` (FR-137) supports `by_name`, `by_provider`, and `by_capability` constructors.

## Agent System Enhancements

The **AgentNode** trait (FR-138) defines `name()`, `description()`, `run()` returning `Stream<AgentEvent>`, and `sub_agents()`. Agent callbacks (FR-139) provide `BeforeAgentCallback` and `AfterAgentCallback` for per-agent observability.

Agent transfer (FR-140) enables LLM-initiated delegation to peer or parent agents via a special tool call. Transfer control policies (FR-141) include `DisallowTransferToParent` and `DisallowTransferToPeers`. The agent tree hierarchy (FR-142) uses isolated conversation branches with namespaced identifiers (e.g. `root.planner.coder`).

### Plugin System (FR-143-144, FR-568-570)

The plugin system is runner-scoped with `on_user_message`, `on_event`, `before_run`, and `after_run` hooks (FR-143, FR-144). Each plugin declares a `PluginStateKey` with an associated `State` type and `fn key() -> &'static str`. Plugin state is nested under the key in agent state, preventing interference between plugins (FR-568). `AgentState::plugin_state<P: PluginStateKey>(&self) -> &P::State` and `plugin_state_mut<P>(&mut self) -> &mut P::State` provide type-safe access to plugin-owned state slices without downcasting (FR-569). When multiple plugins are composed, their state schemas are merged automatically; conflicting keys produce a compile-time error via the type system (FR-570).

### Signal Routing (FR-571-572)

Signal/message routing uses three-tier priority: (1) execution strategy routes (FR-567), (2) agent-level routes (declared at agent definition via `signal_routes`), (3) plugin-contributed routes. First match wins. Enables strategies to gate signals based on execution state (FR-571). `SignalRouter` trait with `route(&self, signal: &Signal) -> Option<Action>`. Composed from strategy, agent, and plugin routers. Routing decisions are logged at debug level for observability (FR-572).

### Services and Artifacts

`MemoryService` (FR-148) is a trait with `add_session_to_memory()` and `search_memory()` methods; `InMemoryMemoryService` is provided out of the box. `ArtifactService` (FR-149) is a trait for save, load, list, and delete operations on artifacts. Scoped key prefixes (FR-150) include `app:`, `user:`, `session:`, and `temp:`.

### Workflow Agents

Workflow agents include `SequentialAgent` (FR-145), `ParallelAgent` (FR-146), and `LoopAgent` with `max_iterations` (FR-147).

### HITL and Tools

HITL confirmation (FR-151, FR-152) is available via a `with_confirmation()` wrapper with `ConfirmationPredicate`. Long-running tools (FR-153) support an `is_long_running` flag, progress events, and async completion.

### Dynamic Instructions and Streaming

Dynamic instructions (FR-154 through FR-156) include an `instruction_provider` function, artifact injection via `{artifact.NAME}` placeholders, and `global_instruction` for hierarchy-wide prompts. Streaming events distinguish partial from final results (FR-157) with a `turn_complete` signal (FR-158) and `is_final_response()` logic (FR-159).

### Runner

The Runner (FR-160, FR-161) manages session lookup, routing, invocation, and event collection. Error recovery (FR-162) uses `OnModelErrorCallback` with an optional substitute response, and a `skip_summarization` flag (FR-163) prevents summarisation for technical errors.

## Prebuilt Agents and Nodes

`create_react_agent()` returns a compiled `StateGraph` with `ReActAgentConfig`. `ToolNode` executes tool calls in parallel with an error strategy (Continue or Raise) and retry config. `tools_condition()` provides a routing function for conditional edges. `ValidationNode` validates tool call arguments.

Prebuilt control-flow nodes (FR-304) include `IfElseNode`, `LoopNode`, and `IterationNode`. Prebuilt data-transform nodes (FR-305) include `TemplateTransformNode`, `ListOperatorNode`, and `VariableAggregatorNode`. `HttpRequestNode` (FR-306) handles outbound HTTP with an SSRF-protected client. `QuestionClassifierNode` (FR-307) performs LLM-based input classification and routing.

## CLI

The `synwire-cli` binary (FR-095) uses `create_cli_agent()` which wraps `create_agent()` with CLI-specific middleware and HITL approval gates. It supports agent listing, reset/clone, and dynamic system prompt generation (FR-096).

HITL approval gates (FR-097) require confirmation for destructive operations (shell execution, writes, web requests, delegation) while auto-approving read-only operations. The CLI uses `CompositeProvider` (FR-098) to route large results and conversation history to temporary directories.

## Partner Sandboxes

`KagentSandbox` (FR-109) implements `SandboxVfs` via `BaseSandbox` with a 30-minute default timeout, unique ID, async interface, and kagent API file transfer (FR-110). Partner sandbox crates (FR-111) depend only on `synwire-sandbox` and the provider SDK, not the full agents SDK.

## Handoff and Multi-Agent

`HandoffHistoryFilter` (FR-367) is a trait for transforming conversation history during agent transfer. The built-in `nest_handoff_history()` (FR-368) collapses the outgoing agent's conversation into a summary message. Handoff `is_enabled` predicates (FR-369) control whether handoffs are offered as LLM tools. `delegation_count` (FR-370) tracks delegations per task/turn in `RunContext`.

## Success Criteria

- **SC-013**: All backends pass `Vfs` conformance tests.
- **SC-014**: Middleware stack assembles and invokes without conflicts.
- **SC-015**: CLI agent provides interactive HITL approval for shell execution and auto-approves reads.
- **SC-018**: `synwire-agents` tests pass with at least 80% line coverage.
- **SC-019**: Zero `unsafe` blocks in `synwire-agents`.
- **SC-020**: All agents public types are `Send + Sync`.
- **SC-026**: Working tool-calling agent with structured output in fewer than 5 lines via `Agent::builder()`.
- **SC-027**: Agent API auto-selects optimal `OutputMode` per model.
- **SC-031**: Workflow agents produce correct results.
- **SC-032**: Agent transfer switches active agent correctly.
- **SC-033**: Runner catches panics without crashing.
- **SC-050**: Prebuilt workflow nodes produce correct results.
- **SC-055**: Input guardrails halt execution when tripwire triggered.
- **SC-058**: `max_turns` limits agent conversation loops.
- **SC-059**: Handoff history filter transforms conversation correctly.
- **SC-097**: Agent returning directives without executor produces no side effects (pure testability).
- **SC-098**: `DirectiveFilter` can suppress SpawnAgent directives in sandboxed mode.
- **SC-099**: Directive round-trip serialisation preserves all variants.
- **SC-100**: `FsmStrategy` rejects actions not valid in current state with `InvalidTransition` error.
- **SC-101**: Same agent logic produces identical results under `DirectStrategy` and `FsmStrategy` for valid transitions.
- **SC-102**: Plugin state isolation prevents cross-plugin state interference with two plugins writing concurrently.
- **SC-103**: Three-tier signal routing resolves strategy routes before agent and plugin routes.

## Research Findings

- **deepagents-parity**: All 151 items resolved. Full SDK, CLI, Harbor, and partner sandbox coverage.
- **agents-middleware-parity**: The Python agents/middleware system is architecturally distinct and out of scope for the initial port. Focus is on core ReAct agents with LangGraph.
- **agents-usability-parity**: All 57 items resolved (FR-353 through FR-403). Covers guardrails, memory, HITL, and sessions drawn from CrewAI, OpenAI Agents SDK, Strands, and Microsoft Agent Framework.
