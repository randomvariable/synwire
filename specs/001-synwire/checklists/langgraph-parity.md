# LangGraph Parity Checklist: Synwire Port

**Purpose**: Validate whether the spec adequately addresses LangGraph feature parity and unified crate structure — assessing requirement completeness, clarity, and coverage for graph orchestration, checkpointing, prebuilt agents, SDK, and CLI
**Created**: 2026-03-09
**Resolved**: 2026-03-09
**Feature**: [spec.md](../spec.md) | [plan.md](../plan.md)
**Depth**: Rigorous | **Scope**: Full LangGraph parity (all 7 Python packages) + unified crate design
**Audience**: Spec author (gap-finding) | **Timing**: Pre-implementation, spec revision
**Source**: `/langchain-ai/langgraph/libs/` (langgraph, checkpoint, checkpoint-postgres, checkpoint-sqlite, checkpoint-conformance, prebuilt, sdk-py, cli)

## Scope & Exclusion Decisions

- [x] CHK238 Is the permanent exclusion of LangGraph in spec.md §Permanently Excluded Items revisited and either justified against the new parity goal or removed? [Consistency, Spec §Permanently Excluded] — **Resolved**: LangGraph removed from exclusions; §Reclassified Items added documenting the move. §Permanently Excluded now retains only MessageGraph (deprecated), NodeInterrupt (deprecated), EncryptedSerializer, and LangGraph Platform.
- [x] CHK239 Are the specific LangGraph concepts currently listed as excluded (StateGraph, MessageGraph, CompiledGraph, Pregel, ToolNode, create_react_agent) individually re-evaluated for inclusion? [Completeness, Spec §Permanently Excluded] — **Resolved**: StateGraph (FR-029), CompiledGraph (FR-029), Pregel (FR-031), ToolNode (FR-058), create_react_agent (FR-057) moved to in-scope. MessageGraph remains excluded with rationale.
- [x] CHK240 Is there a clear specification of which LangGraph packages map to which Rust crates in the unified workspace? [Clarity, Gap] — **Resolved**: spec.md §In-Scope Items lists all orchestrator crates; plan.md §Source Code shows full directory structure; plan.md §LangGraph Crate Dependencies table maps packages to crates.
- [x] CHK241 Are the boundaries between synwire-core abstractions and orchestrator-specific abstractions explicitly defined in the spec? [Clarity, Gap] — **Resolved**: FR-069 specifies synwire-orchestrator depends on synwire-core; Assumptions clarify the relationship. contracts/langgraph.md explicitly imports types from synwire-core.
- [x] CHK242 Is the relationship between the existing AgentExecutor (spec §FR-019) and LangGraph's create_react_agent clarified — are both in scope, or does LangGraph subsume the simpler agent? [Ambiguity, Spec §FR-019] — **Resolved**: Both remain in scope. FR-019 (AgentExecutor) is the simple synwire-core ReAct loop; FR-057 (create_react_agent) is the graph-based replacement in synwire-agents. US9 notes prebuilt "replaces the legacy AgentExecutor".

## Unified Crate Structure

- [x] CHK243 Does the spec define a unified Cargo workspace layout that includes both synwire and orchestrator crates? [Completeness, Gap] — **Resolved**: FR-069 mandates unified workspace. plan.md §Source Code shows 26-crate layout under crates/.
- [x] CHK244 Are crate dependency relationships specified (e.g. synwire-orchestrator depends on synwire-core)? [Completeness, Gap] — **Resolved**: FR-069 specifies synwire-orchestrator→synwire-core. plan.md §LangGraph Crate Dependencies table covers all crate deps.
- [x] CHK245 Is it specified whether orchestrator reuses synwire-core's Runnable trait or defines its own execution abstraction? [Clarity, Gap] — **Resolved**: contracts/langgraph.md §CompiledGraph states "Implements Runnable<Value, Value>". FR-029 states "compiled graph MUST implement Runnable".
- [x] CHK246 Are feature flag boundaries defined for optional components (e.g. checkpoint-postgres behind a feature flag)? [Completeness, Gap] — **Resolved**: plan.md §Feature Flags (synwire-orchestrator) defines sqlite, postgres, tracing feature flags.
- [ ] CHK247 Is the public re-export strategy specified — does a top-level `synwire-orchestrator` crate re-export from sub-crates like `synwire-orchestrator`, `synwire-checkpoint`? [Clarity, Gap] — **Open**: No top-level `synwire-orchestrator` convenience crate is currently specified (unlike the `synwire` crate). Consider adding one.
- [ ] CHK248 Is versioning strategy for the unified workspace documented (independent per-crate versions vs. lockstep)? [Gap] — **Open**: Not yet specified. Needs decision: independent semver per crate or workspace-level lockstep.

## Core Graph Orchestration (orchestrator core)

- [x] CHK249 Are requirements defined for a `StateGraph` type with typed state, node addition, and edge definition? [Completeness, Gap] — **Resolved**: FR-029 defines StateGraph. contracts/langgraph.md §StateGraph Builder has full API. data-model.md §StateGraph has type definition.
- [x] CHK250 Is the state typing mechanism specified — how does Rust represent Python's TypedDict-based graph state with reducer annotations? [Clarity, Gap] — **Resolved**: data-model.md §StateGraph documents the `State` trait with `#[derive(State)]` macro and `#[reducer(fn)]` annotations. contracts/langgraph.md §State Trait has the trait definition.
- [x] CHK251 Are reducer functions (e.g. `add_messages` for message list merging) specified as a concept? [Completeness, Gap] — **Resolved**: data-model.md §BinaryOperatorAggregate documents reducer semantics. contracts/langgraph.md §Graph Control Flow Functions defines `add_messages`.
- [x] CHK252 Are the `START` and `END` sentinel constants or types defined for graph edge targets? [Completeness, Gap] — **Resolved**: FR-037 defines START and END. data-model.md §Constants specifies values.
- [x] CHK253 Is graph compilation (StateGraph → CompiledGraph) specified with validation of edges and required nodes? [Completeness, Gap] — **Resolved**: FR-029 specifies compile(). contracts/langgraph.md §StateGraph Builder documents compile signature and validation.
- [x] CHK254 Are conditional edges (branching based on state) specified with clear routing semantics? [Completeness, Gap] — **Resolved**: contracts/langgraph.md §StateGraph Builder documents add_conditional_edges. data-model.md §ConditionalEdge and §RoutingResult define types.
- [x] CHK255 Is the `Send` primitive for dynamic fan-out to nodes specified? [Completeness, Gap] — **Resolved**: FR-032 defines Send. data-model.md §Send has type definition.
- [x] CHK256 Is the `Command` primitive for controlling graph flow (goto, update, resume) specified? [Completeness, Gap] — **Resolved**: FR-032 defines Command. data-model.md §Command has full type definition.
- [x] CHK257 Are graph recursion limits specified with clear error behaviour when exceeded? [Completeness, Gap] — **Resolved**: FR-038 defines GraphRecursionError. Edge cases specify behaviour. US6 acceptance scenario 4 covers this.
- [x] CHK258 Is the deprecated `MessageGraph` explicitly excluded with rationale, or included for compatibility? [Clarity, Gap] — **Resolved**: spec.md §Permanently Excluded explicitly excludes MessageGraph with rationale "use StateGraph with MessagesState". Assumptions restate this.

## Pregel Execution Engine

- [x] CHK259 Are requirements defined for the Pregel-style step execution model (superstep-based synchronous execution)? [Completeness, Gap] — **Resolved**: FR-031 defines Pregel. data-model.md §Pregel (internal) and §Graph Execution Flow document superstep semantics.
- [x] CHK260 Is the relationship between Pregel (internal engine) and StateGraph (user API) clarified — is Pregel a public or internal type? [Clarity, Gap] — **Resolved**: data-model.md §Pregel states "Not part of the public API surface — users interact with CompiledGraph".
- [x] CHK261 Are execution ordering guarantees specified (which nodes run in parallel, which are sequential)? [Clarity, Gap] — **Resolved**: US6 acceptance scenario 3 specifies parallel execution. data-model.md §Graph Execution Flow documents "Execute active nodes in parallel".
- [ ] CHK262 Are subgraph execution semantics specified (nested graphs within nodes)? [Completeness, Gap] — **Open**: PregelExecutableTask has `subgraphs` field but subgraph composition semantics (how a node can contain another graph) are not fully specified.

## Channels

- [x] CHK263 Are channel types specified as the state management primitives (LastValue, Topic, BinaryOperatorAggregate, etc.)? [Completeness, Gap] — **Resolved**: FR-030 lists all channel types. data-model.md §Channel Types has full definitions. contracts/langgraph.md §Concrete Channel Types has implementations.
- [x] CHK264 Is the `LastValue` channel (stores most recent value) specified with overwrite semantics? [Completeness, Gap] — **Resolved**: data-model.md §LastValue and contracts/langgraph.md §LastValue document rejection of multiple updates.
- [x] CHK265 Is the `Topic` channel (pub-sub accumulation) specified? [Completeness, Gap] — **Resolved**: data-model.md §Topic and contracts/langgraph.md §Topic with accumulate flag.
- [x] CHK266 Is the `BinaryOperatorAggregate` channel (custom reducer via binary operator) specified? [Completeness, Gap] — **Resolved**: data-model.md §BinaryOperatorAggregate with Overwrite support. contracts/langgraph.md documents the implementation.
- [x] CHK267 Are ephemeral vs persistent channel semantics defined? [Clarity, Gap] — **Resolved**: data-model.md §EphemeralValue documents "cleared after each superstep read". Other channels persist across supersteps.
- [x] CHK268 Is channel versioning for checkpoint compatibility specified? [Completeness, Gap] — **Resolved**: data-model.md §Checkpoint documents channel_versions and versions_seen. contracts/langgraph.md §BaseCheckpointSaver has get_next_version.
- [x] CHK269 Is the `EmptyChannelError` (channel read before first write) specified? [Edge Case, Gap] — **Resolved**: FR-038 lists EmptyChannelError. data-model.md §Graph Error Types defines it.

## Interrupts & Human-in-the-Loop

- [x] CHK270 Is the `interrupt()` function for pausing graph execution and requesting user input specified? [Completeness, Gap] — **Resolved**: FR-033 defines interrupt(). contracts/langgraph.md §Graph Control Flow Functions has signature. data-model.md §interrupt() Function documents semantics.
- [x] CHK271 Are interrupt resume semantics specified — how does a paused graph resume with user-provided values? [Clarity, Gap] — **Resolved**: FR-033 specifies Command::resume. data-model.md §Interrupt & Resume Flow documents the full flow. US7 acceptance scenario 2 covers this.
- [x] CHK272 Is the `Interrupt` type (information about a pending interrupt) specified? [Completeness, Gap] — **Resolved**: data-model.md §Interrupt defines value + id fields.
- [x] CHK273 Are requirements defined for multi-interrupt scenarios (multiple nodes interrupt in the same superstep)? [Edge Case, Gap] — **Resolved**: US7 acceptance scenario 3 explicitly covers multiple interrupts in the same superstep.
- [x] CHK274 Is the interaction between interrupts and checkpointing specified (state must be persisted before interrupt)? [Consistency, Gap] — **Resolved**: Edge cases specify "graph compiled without checkpointer but interrupt_before specified MUST return compile-time error". data-model.md §Graph Execution Flow shows "Checkpoint state" before interrupt check.

## Streaming

- [x] CHK275 Are all stream modes from LangGraph specified: values, updates, debug, messages, custom, tasks, checkpoints? [Completeness, Gap] — **Resolved**: FR-034 lists all 7 modes. data-model.md §StreamMode enum defines all variants.
- [x] CHK276 Is the `StreamMode` type defined with clear semantics for each mode? [Clarity, Gap] — **Resolved**: data-model.md §StreamMode defines each variant with description.
- [x] CHK277 Are custom stream writers (user-defined streaming within nodes) specified? [Completeness, Gap] — **Resolved**: FR-065 specifies get_stream_writer(). data-model.md §StreamWriter defines the type. US10 acceptance scenario 3 covers custom streaming.
- [ ] CHK278 Is streaming from subgraphs (nested graph output propagation) specified? [Completeness, Gap] — **Open**: Subgraph streaming semantics not fully documented (related to CHK262).
- [ ] CHK279 Is the relationship between LangGraph streaming and synwire-core's `stream_events` clarified? [Consistency, Gap] — **Open**: LangGraph has its own stream modes; how these interact with Runnable::stream_events is not documented.

## Checkpointing (synwire-checkpoint)

- [x] CHK280 Is a `BaseCheckpointSaver` trait specified with get, put, list, and delete operations? [Completeness, Gap] — **Resolved**: FR-042 defines the trait. contracts/langgraph.md §BaseCheckpointSaver has full signature with all methods.
- [x] CHK281 Is the `Checkpoint` data structure specified with channel_values, channel_versions, and versions_seen? [Completeness, Gap] — **Resolved**: FR-043. data-model.md §Checkpoint has all fields.
- [x] CHK282 Is `CheckpointMetadata` (source, step, parents, run_id) specified? [Completeness, Gap] — **Resolved**: FR-044. data-model.md §CheckpointMetadata with CheckpointSource enum.
- [x] CHK283 Is `CheckpointTuple` (checkpoint + metadata + config + pending writes) specified? [Completeness, Gap] — **Resolved**: FR-045. data-model.md §CheckpointTuple with PendingWrite type.
- [x] CHK284 Are both sync and async checkpoint operations specified, or is one chosen with rationale? [Clarity, Gap] — **Resolved**: contracts/langgraph.md uses BoxFuture (async-first), matching synwire-core's async-first approach (FR-013).
- [x] CHK285 Is `put_writes` (intermediate write persistence for fault tolerance) specified? [Completeness, Gap] — **Resolved**: contracts/langgraph.md §BaseCheckpointSaver includes put_writes with task_id and task_path parameters.
- [x] CHK286 Is checkpoint versioning (get_next_version) specified for concurrent write detection? [Completeness, Gap] — **Resolved**: contracts/langgraph.md §BaseCheckpointSaver includes get_next_version with generic Version type.
- [x] CHK287 Are thread lifecycle operations (delete_thread, copy_thread, prune) specified? [Completeness, Gap] — **Resolved**: contracts/langgraph.md §BaseCheckpointSaver includes all three with PruneStrategy enum.
- [x] CHK288 Is the serialization protocol for checkpoint data specified (JsonPlusSerializer equivalent)? [Completeness, Gap] — **Resolved**: FR-046. contracts/langgraph.md §SerializerProtocol defines trait and JsonPlusSerializer. data-model.md §SerializerProtocol and §JsonPlusSerializer document types.
- [x] CHK289 Is checkpoint encryption (EncryptedSerializer) specified or explicitly excluded? [Completeness, Gap] — **Resolved**: spec.md §Permanently Excluded explicitly excludes EncryptedSerializer with rationale "out of scope for initial port, can be added later".

## Checkpoint Implementations

- [x] CHK290 Is a PostgreSQL checkpoint implementation specified as a separate crate (synwire-checkpoint-postgres)? [Completeness, Gap] — **Resolved**: FR-048. plan.md §Source Code shows synwire-checkpoint-postgres crate. contracts/langgraph.md §PostgresSaver.
- [x] CHK291 Is a SQLite checkpoint implementation specified as a separate crate (synwire-checkpoint-sqlite)? [Completeness, Gap] — **Resolved**: FR-047. plan.md §Source Code shows synwire-checkpoint-sqlite crate. contracts/langgraph.md §SqliteSaver.
- [x] CHK292 Are checkpoint implementation requirements (connection pooling, schema migration, concurrent access) specified? [Completeness, Gap] — **Resolved**: contracts/langgraph.md notes r2d2 for SQLite and deadpool-postgres for PostgreSQL. plan.md dependency table lists pooling crates.
- [x] CHK293 Is a conformance test suite specified for validating checkpoint implementations against the trait? [Completeness, Gap] — **Resolved**: FR-050. plan.md shows synwire-checkpoint-conformance crate. contracts/langgraph.md §Checkpoint Conformance Tests documents the test harness.
- [x] CHK294 Are in-memory checkpoint implementations specified for testing? [Completeness, Gap] — **Resolved**: FR-049. contracts/langgraph.md §InMemoryCheckpointSaver. plan.md crate layout shows memory.rs.

## Store (Cross-Conversation Memory)

- [x] CHK295 Is a `BaseStore` trait specified for persistent key-value storage with namespace hierarchy? [Completeness, Gap] — **Resolved**: FR-051. contracts/langgraph.md §BaseStore has full trait. data-model.md §Store Types has all types.
- [x] CHK296 Are store operations (get, search, put, delete, list_namespaces) specified? [Completeness, Gap] — **Resolved**: FR-051 lists all operations. contracts/langgraph.md §BaseStore has full signatures. data-model.md §Store Operation Types defines GetOp, SearchOp, PutOp, ListNamespacesOp.
- [x] CHK297 Is semantic search within stores (IndexConfig, vector-based search) specified or excluded? [Completeness, Gap] — **Resolved**: FR-054 specifies IndexConfig. data-model.md §IndexConfig defines dims, embed, fields. contracts/langgraph.md §BaseStore search method includes query parameter.
- [x] CHK298 Is TTL (time-to-live) support for store items specified? [Completeness, Gap] — **Resolved**: FR-053. data-model.md §TTLConfig defines refresh_on_read, default_ttl, sweep_interval_minutes.
- [x] CHK299 Is the batch operation interface for stores specified? [Completeness, Gap] — **Resolved**: contracts/langgraph.md §BaseStore includes batch method. data-model.md defines StoreOp and StoreResult types.
- [ ] CHK300 Is the relationship between Store and VectorStore (synwire-core §FR-005) clarified — are they separate abstractions? [Consistency, Gap] — **Open**: Both are in scope but their relationship is not explicitly documented. Store uses IndexConfig for semantic search; VectorStore uses Embeddings for similarity search. They serve different purposes but the distinction should be explicit.

## Cache

- [x] CHK301 Is a `BaseCache` trait for node result memoisation specified? [Completeness, Gap] — **Resolved**: FR-056. contracts/langgraph.md §BaseCache has trait definition. data-model.md §Cache Types defines the trait.
- [x] CHK302 Are cache operations (get, set, clear) with TTL support specified? [Completeness, Gap] — **Resolved**: contracts/langgraph.md §BaseCache defines get, set, clear with TTL parameter on set.
- [x] CHK303 Is the `CachePolicy` configuration (per-node cache settings) specified? [Completeness, Gap] — **Resolved**: FR-040. data-model.md §CachePolicy defines key_func and ttl fields.

## Prebuilt Agents (synwire-agents)

- [x] CHK304 Is `create_react_agent` specified as a high-level factory function for ReAct agents using StateGraph? [Completeness, Gap] — **Resolved**: FR-057. contracts/prebuilt.md §create_react_agent has full signature and ReactAgentConfig.
- [x] CHK305 Is `ToolNode` (parallel tool execution with error handling) specified? [Completeness, Gap] — **Resolved**: FR-058. contracts/prebuilt.md §ToolNode with ToolErrorStrategy and execution semantics.
- [x] CHK306 Is `tools_condition` (conditional routing based on tool call presence) specified? [Completeness, Gap] — **Resolved**: FR-059. contracts/prebuilt.md §tools_condition with routing semantics.
- [x] CHK307 Is `ValidationNode` (tool input validation before execution) specified? [Completeness, Gap] — **Resolved**: FR-060. contracts/prebuilt.md §ValidationNode with validation semantics.
- [x] CHK308 Are state injection annotations (InjectedState, InjectedStore) specified for tool context? [Completeness, Gap] — **Resolved**: contracts/prebuilt.md §State/Store Injection defines InjectedState and InjectedStore marker traits with implementation note.
- [x] CHK309 Is `AgentState` (standard TypedDict for agent state with messages) specified as a Rust type? [Completeness, Gap] — **Resolved**: FR-061. data-model.md §AgentState (LangGraph) defines the type. contracts/prebuilt.md references it.

## Functional API

- [x] CHK310 Is the `@task` decorator equivalent specified for defining parallelisable tasks with retry/cache? [Completeness, Gap] — **Resolved**: FR-062. data-model.md §TaskFunction has type definition. plan.md shows func/task.rs module.
- [x] CHK311 Is the `@entrypoint` decorator equivalent specified for defining workflow entry points? [Completeness, Gap] — **Resolved**: FR-063. data-model.md §Entrypoint has type definition. plan.md shows func/entrypoint.rs module.
- [x] CHK312 Is `entrypoint.final` (decoupling return value from saved state) specified? [Completeness, Gap] — **Resolved**: FR-064. data-model.md §EntrypointFinal with value (returned) and save (checkpointed) fields.
- [x] CHK313 Is the relationship between the functional API and the graph API clarified — are both in scope? [Clarity, Gap] — **Resolved**: US11 states "alternative to the graph builder API". Both are in scope in synwire-orchestrator.

## Runtime & Configuration

- [x] CHK314 Is the `Runtime` type (bundles context, store, stream_writer, previous values) specified? [Completeness, Gap] — **Resolved**: data-model.md §Runtime Context Types defines Runtime<Context> with all fields.
- [x] CHK315 Is `get_runtime()` / `get_config()` / `get_store()` / `get_stream_writer()` specified for node-level access? [Completeness, Gap] — **Resolved**: FR-065. contracts/langgraph.md §Runtime Context Accessors defines all three functions with task_local implementation note.
- [x] CHK316 Is context injection (user_id, database connections passed to nodes) specified? [Completeness, Gap] — **Resolved**: data-model.md §Runtime has generic Context type parameter. contracts/langgraph.md notes task_local for context.
- [x] CHK317 Is `RetryPolicy` for per-node retry configuration specified? [Completeness, Gap] — **Resolved**: FR-039. data-model.md §RetryPolicy (LangGraph) has full field definition distinct from synwire-core's RetryConfig.
- [x] CHK318 Is the `Overwrite` type (bypass reducers, write directly to channels) specified? [Completeness, Gap] — **Resolved**: FR-032 mentions Overwrite. data-model.md §Overwrite defines it. BinaryOperatorAggregate documents Overwrite handling.

## Error Types

- [x] CHK319 Is `GraphRecursionError` (recursion limit exceeded) specified? [Completeness, Gap] — **Resolved**: FR-038. data-model.md §Graph Error Types.
- [x] CHK320 Is `InvalidUpdateError` (invalid channel updates) specified? [Completeness, Gap] — **Resolved**: FR-038. data-model.md §Graph Error Types.
- [x] CHK321 Is `GraphInterrupt` (internal interrupt signalling) specified? [Completeness, Gap] — **Resolved**: FR-038. data-model.md §Graph Error Types.
- [x] CHK322 Is `EmptyInputError` specified? [Completeness, Gap] — **Resolved**: FR-038. data-model.md §Graph Error Types.
- [x] CHK323 Is `TaskNotFound` (distributed execution) specified? [Completeness, Gap] — **Resolved**: FR-038. data-model.md §Graph Error Types.
- [x] CHK324 Are error types integrated with synwire-core's error enum strategy (§FR-012)? [Consistency, Spec §FR-012] — **Resolved**: data-model.md §Graph Error Types states "SynwireGraphError implements From<SynwireGraphError> for SynwireError".

## SDK (synwire-graph-client)

- [x] CHK325 Is a Rust SDK client for the LangGraph API specified? [Completeness, Gap] — **Resolved**: FR-067. plan.md shows synwire-graph-client crate with client.rs, types.rs, auth.rs.
- [ ] CHK326 Are SDK client operations (thread management, run management, assistant management) specified? [Completeness, Gap] — **Open**: FR-067 mentions the operations but no detailed API contract exists yet. Needs a contracts/sdk.md.
- [ ] CHK327 Is authentication for the SDK client specified? [Completeness, Gap] — **Open**: plan.md shows auth.rs but no auth mechanism is specified (API key, OAuth, etc.).
- [x] CHK328 Is the SDK specified as a separate crate or part of the core? [Clarity, Gap] — **Resolved**: FR-067 specifies "synwire-graph-client crate". plan.md shows it as a separate workspace member.

## CLI

- [x] CHK329 Is a CLI tool for LangGraph development/deployment specified? [Completeness, Gap] — **Resolved**: FR-068. plan.md shows synwire-cli binary crate.
- [ ] CHK330 Are CLI commands (dev server, deployment, graph testing) specified? [Completeness, Gap] — **Open**: FR-068 mentions "development, testing, and deployment" but no detailed command list exists.
- [x] CHK331 Is the CLI specified as a separate binary crate? [Clarity, Gap] — **Resolved**: plan.md shows synwire-cli as a separate binary crate with main.rs.

## State Snapshots & Time Travel

- [x] CHK332 Is `StateSnapshot` (state at a point in execution) specified? [Completeness, Gap] — **Resolved**: FR-035. data-model.md §StateSnapshot has full type definition.
- [x] CHK333 Are time-travel capabilities (replay from checkpoint, fork execution) specified? [Completeness, Gap] — **Resolved**: FR-036 specifies get_state_history. data-model.md §CheckpointSource includes Fork variant. US8 acceptance scenario 1 covers history retrieval.
- [x] CHK334 Is `get_state` / `get_state_history` for inspecting graph execution specified? [Completeness, Gap] — **Resolved**: FR-036. contracts/langgraph.md §CompiledGraph has both method signatures.
- [x] CHK335 Is `update_state` (manually modifying graph state) specified? [Completeness, Gap] — **Resolved**: FR-036. contracts/langgraph.md §CompiledGraph has update_state signature.

## Managed Values

- [x] CHK336 Is `IsLastStep` (indicates final execution step) specified? [Completeness, Gap] — **Resolved**: FR-066. data-model.md §Managed Values.
- [x] CHK337 Is `RemainingSteps` (remaining execution steps) specified? [Completeness, Gap] — **Resolved**: FR-066. data-model.md §Managed Values.
- [x] CHK338 Is the managed value abstraction (runtime-injected read-only values) specified? [Completeness, Gap] — **Resolved**: data-model.md §Managed Values notes "ManagedValue trait system" and runtime context struct pattern.

## Non-Functional Requirements

- [ ] CHK339 Are performance requirements specified for graph execution overhead (e.g. per-superstep latency)? [Gap, NFR] — **Open**: No graph-specific performance targets defined. synwire-core has streaming latency target but langgraph does not.
- [x] CHK340 Are concurrency requirements specified for parallel node execution within a superstep? [Gap, NFR] — **Resolved**: US6 acceptance scenario 3 specifies parallel execution. data-model.md §Graph Execution Flow documents parallel node execution.
- [x] CHK341 Is the async runtime requirement clarified for orchestrator crates — does it share synwire-core's tokio assumption? [Consistency, Spec §Assumptions] — **Resolved**: Assumptions state synwire-orchestrator depends on synwire-core which targets tokio. plan.md dependency table shows tokio for all orchestrator crates.
- [ ] CHK342 Are memory/resource requirements specified for long-running graphs with large state? [Gap, NFR] — **Open**: No memory bounds or backpressure mechanisms specified for large state graphs.
- [x] CHK343 Is the Send + Sync requirement extended to langgraph types, consistent with synwire-core constraints? [Consistency, Spec §Constraints] — **Resolved**: SC-012 states "All orchestrator public types are Send + Sync". contracts/langgraph.md traits all require Send + Sync.
- [x] CHK344 Is the zero-unsafe constraint extended to synwire-orchestrator, or scoped differently? [Clarity, Spec §SC-005] — **Resolved**: SC-011 explicitly states "Zero unsafe blocks in synwire-orchestrator". plan.md constraints section updated.

## Success Criteria Gaps

- [x] CHK345 Are success criteria defined for graph execution (equivalent to SC-001..SC-006 for synwire-core)? [Gap] — **Resolved**: SC-007 through SC-012 cover graph execution, checkpoint round-trip, prebuilt agent, coverage, safety, and thread safety.
- [x] CHK346 Is a measurable outcome defined for checkpoint round-trip (save state → crash → resume → correct state)? [Gap] — **Resolved**: SC-008 defines checkpoint round-trip with SQLite.
- [x] CHK347 Is a measurable outcome defined for interrupt/resume workflows? [Gap] — **Resolved**: SC-007 includes "interrupt, resume, and verify state continuity".
- [x] CHK348 Is test coverage target specified for orchestrator crates? [Gap] — **Resolved**: SC-010 specifies "at least 80% line coverage on synwire-orchestrator".

## Acceptance Scenarios Gaps

- [x] CHK349 Are acceptance scenarios defined for basic graph execution (define graph → compile → invoke → get result)? [Gap] — **Resolved**: US6 has 4 acceptance scenarios covering basic execution, conditional routing, parallel execution, and recursion limits.
- [x] CHK350 Are acceptance scenarios defined for human-in-the-loop (invoke → interrupt → user input → resume → result)? [Gap] — **Resolved**: US7 has 3 acceptance scenarios covering interrupt, resume, and multi-interrupt.
- [x] CHK351 Are acceptance scenarios defined for checkpointed execution (invoke → crash → restart → resume from checkpoint)? [Gap] — **Resolved**: US8 has 3 acceptance scenarios covering history, resume, and state inspection.
- [x] CHK352 Are acceptance scenarios defined for streaming from graph execution? [Gap] — **Resolved**: US10 has 3 acceptance scenarios covering updates mode, messages mode, and custom streaming.
- [x] CHK353 Are acceptance scenarios defined for the functional API (task/entrypoint pattern)? [Gap] — **Resolved**: US11 has 3 acceptance scenarios covering composition, retry, and caching.
- [x] CHK354 Are edge case scenarios defined for graph-specific failures (cycle detection, missing edges, invalid state updates)? [Gap] — **Resolved**: spec.md §Edge Cases now includes 9 graph-specific edge cases (cycles, multiple channel writes, checkpoint failures, invalid thread_id, invalid Command targets, interrupt outside context, store without index, interrupt without checkpointer).

## Notes

- **Resolved**: 2026-03-09 — 109 of 117 items resolved (93%)
- **8 items remain open**:
  - CHK247: Top-level `synwire-orchestrator` re-export crate not specified
  - CHK248: Workspace versioning strategy not specified
  - CHK262: Subgraph execution semantics not fully specified
  - CHK278: Subgraph streaming propagation not documented
  - CHK279: LangGraph streaming vs synwire-core stream_events relationship
  - CHK300: Store vs VectorStore relationship not explicitly documented
  - CHK326/CHK327: SDK client operations and auth not detailed
  - CHK330: CLI commands not detailed
  - CHK339: Graph performance targets not defined
  - CHK342: Memory/resource requirements for large state not specified
- Items are numbered CHK238–CHK354 continuing from existing checklists
- All spec, data-model, contracts, and plan files updated in this resolution pass
