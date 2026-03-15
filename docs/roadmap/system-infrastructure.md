# System Infrastructure

## Overview
Cross-cutting concerns: runtime isolation, security, credentials, cognitive architecture, session management, and workspace organisation. These features span M2/M3 and underpin the agent and protocol layers. The cognitive architecture draws from [Jido](https://github.com/agentjido/jido)'s separation of Thread (canonical log), Memory (mutable working state), and Identity (agent self-model). Runtime isolation uses instance-scoped architecture rather than tenant_id-only partitioning.

## Workspace Unification (FR-069)
Cargo workspace MUST be unified with both synwire and orchestrator crates under `crates/`. synwire-orchestrator depends on synwire-core for shared abstractions.

## Runtime Isolation (FR-584-587, FR-327-328)

The primary isolation mechanism is instance-scoped architecture: the entire runtime (registries, config, event bus, debug state) is scoped per `SynwireInstance`. Within an instance, optional `tenant_id` provides data-level partitioning for multi-tenant deployments.

- `SynwireInstance` builder that produces a scoped runtime. All agent operations (spawn, lookup, signal) go through the instance. `SynwireInstance::builder().with_config(config).with_store(store).with_event_bus(bus).build()` (FR-584)
- Instance-scoped registry: agent name resolution, session lookup, and checkpoint access are scoped to the instance. No cross-instance visibility unless explicitly bridged (FR-585)
- Test isolation: each test creates its own `SynwireInstance` with in-memory backends, zero shared state. `SynwireInstance::test()` convenience constructor with sensible test defaults (FR-586)
- Multi-instance coordination: `InstanceBridge` trait for explicit cross-instance signal routing. Opt-in, not default. Enables federation patterns where separate instances communicate via well-defined boundaries (FR-587)
- Within an instance, `RunnableConfig` supports optional `tenant_id`. All storage operations scope by tenant when set (FR-327)
- Checkpoint savers and store implementations enforce tenant isolation within an instance. Data partitioned by tenant_id (FR-328)

## Security

### Secret Management (FR-317-319)
- `SecretValue` wrapper preventing accidental logging/serialisation (FR-317)
- `CredentialProvider` trait with get_credential() and refresh_credential(). Env and static implementations (FR-318)
- Credential refresh support for long-running agents (retry on 401/403) (FR-319)

### Network Safety (FR-325-326)
- `SsrfProtectedClient` rejecting private/internal network addresses (FR-325)
- `HttpClientFactory` trait — all outbound HTTP uses factory-provided client (FR-326)

## Cognitive Architecture (FR-573-579, FR-371-374)

Three architecturally distinct built-in primitives for agent cognition. Thread is the canonical immutable log, Memory is mutable working state, Identity is the agent self-model. Context window management is a projection operation on Thread, not a destructive modification.

### Thread (FR-573-575)
- `Thread` type: append-only canonical conversation record. Never destructively modified. All messages, tool calls, and results are appended with monotonic sequence numbers and timestamps (FR-573)
- `Thread::project(&self, strategy: &ProjectionStrategy) -> Vec<Message>` returns a context-appropriate view. If summarisation was too aggressive, re-project from the canonical Thread. Automatic context window overflow recovery: catch error, re-project with more aggressive strategy, retry (max 3 retries) (FR-574)
- `ProjectionStrategy` trait with built-in implementations: `FullProjection` (all messages), `SlidingWindowProjection` (last N messages, preserves tool-use/tool-result pairs), `SummarizingProjection` (LLM-based summarisation), `TokenBudgetProjection` (fit within token limit) (FR-575)

### Memory (FR-576-577, FR-371-374)
- `AgentMemory` type: mutable cognitive substrate with typed, named spaces. Built-in spaces: `world` (key-value facts about the environment), `tasks` (ordered task items with status), `scratch` (temporary working memory cleared between turns). Custom spaces via `AgentMemory::create_space<T: Serialize + DeserializeOwned>(name)` (FR-576)
- Memory spaces are isolated — operations on one space cannot affect another. Each space has its own serialisation and optional persistence policy (ephemeral, session-scoped, persistent) (FR-577)
- Cross-session memory search uses composite scoring with configurable weights: recency 0.3, semantic 0.5, importance 0.2 (FR-371)
- Memory consolidation — merge semantically similar memories above threshold (default 0.85) (FR-372)
- `KnowledgeBase` type combining document sources, vector store, and unified query method (FR-373)
- Optional LLM-based importance inference for memory entries (FR-374)

### Identity (FR-578-579)
- `AgentIdentity` type: agent self-model with profile fields (name, description, capabilities, version), a monotonic revision counter incremented on each state change, and optional metadata. Used by A2A agent cards and for agent self-reference in prompts (FR-578)
- `AgentIdentity::capabilities()` returns a structured list of agent capabilities (tools available, strategies supported, output formats). Consumed by `AgentNode::description()` and A2A `AgentCard` generation (FR-579)

## Session, State & Hibernation (FR-385-387, FR-580-583)

Session management covers pause/resume, ambient context, and agent hibernation for long-running agents. Hibernation is intentional suspension to free resources, with full cognitive state preservation for later resumption.

- Run state serialisation via to_state() -> RunState and from_state(RunState) for pause/resume (FR-385)
- task_local! ambient context accessor current_run_context() for tools and hooks (FR-386)
- `SessionProvider` trait: create_session(), save_session(), load_session(), delete_session(). In-memory and file-based implementations (FR-387)
- `Hibernatable` trait with `hibernate(&self) -> HibernationRecord` and `thaw(record: HibernationRecord) -> Result<Self>`. `HibernationRecord` includes Thread, AgentMemory, AgentIdentity, strategy state, plugin states, and metadata (FR-580)
- `HibernationStore` trait with `store(agent_id, record)`, `load(agent_id)`, `list()`, `delete(agent_id)`. In-memory and file-based implementations. Pluggable storage backends (FR-581)
- Hibernation metadata: `hibernated_at` timestamp, `hibernation_reason` (explicit, idle_timeout, resource_pressure), `estimated_resume_cost` (token count to reconstruct context). Queryable via `HibernationStore::list()` (FR-582)
- Auto-hibernation policy: `IdleHibernationPolicy` with configurable idle timeout (default 30 minutes). Agent automatically hibernated after no activity. Thawed on next incoming signal/message (FR-583)

## Hooks & Lifecycle (FR-388-390)
- Bidirectional lifecycle hooks with writable fields: BeforeToolCallEvent { cancel, retry }, BeforeModelCallEvent { modified_messages } (FR-388)
- Hook execution order: "before" hooks FIFO, "after"/"cleanup" hooks LIFO (FR-389)
- pre_model_filter for modifying messages and system prompt before each model invocation (FR-390)

## Approval & HITL (FR-391-393)
- Typed approval content: ApprovalRequest and ApprovalResponse with ApprovalKind (FunctionApproval, TextApproval, StructuredDataInput) (FR-391)
- Approval ledger on RunContext with approve_tool(name, always) / reject_tool(name, always) (FR-392)
- `FeedbackProvider` trait for async/non-blocking feedback from external systems (FR-393)

## Graph State & Execution

### Typed State (FR-313-316)
- `TypedValue` enum for runtime type safety in graph state (FR-313)
- Iteration/loop nodes create nested variable scopes with shadowing (FR-314)
- Inter-node output references via structured paths (node_id.key) (FR-315)
- System variables: sys.run_id, sys.thread_id, sys.created_at, sys.step_count (FR-316)

### Node Lifecycle (FR-329-332)
- `NodeRegistry` for extensible node type registration (FR-329)
- Versioned node type registration (FR-330)
- `NodeState` enum: Pending, Running, Succeeded, Failed, Skipped, Paused (FR-331)
- step_count counter incremented at each superstep (FR-332)

### Error Handling (FR-310-312)
- `NodeErrorStrategy` enum: FailWorkflow, FailBranch, Continue. Configurable per node (FR-310)
- Retry exhausts all attempts before error strategy applies (FR-311)
- FailBranch allows sibling branches to complete; FailWorkflow cancels all parallel nodes (FR-312)

### Checkpointing (FR-323-324, FR-345-348)
- Checkpoint format versioning with format_version field (FR-323)
- `CheckpointMigration` trait for cross-version migration (FR-324)
- Partial recovery from last checkpoint on resume (FR-345)
- Node idempotency documentation and idempotent: bool flag on RetryPolicy (FR-346)
- ToolNode result truncation via max_result_size (default 100KB) (FR-347)
- Checkpoint max_checkpoint_size (default 10MB) with StateTooLarge error (FR-348)

### Streaming (FR-343-344)
- Concurrent node streams interleaved with source node_id for demux (FR-343)
- Graph streaming backpressure with configurable buffer size (default 1024) (FR-344)

### Typed Interrupts (FR-198)
- InterruptReason enum: InputRequired, AuthRequired, ConfirmationRequired, Custom (FR-198)

## Provider Fallback (FR-401)
- `MultiProvider` with ordered fallback chain for provider-level failures (FR-401)

## Visualisation (FR-394)
- `CompiledGraph::to_mermaid()` for graph visualisation (FR-394)

## Success Criteria
- **SC-051**: SecretValue never appears in Debug, Display, or serialised JSON
- **SC-054**: SSRF protection rejects private IPs and allows public IPs
- **SC-060**: Memory composite scoring ranks by weighted dimensions
- **SC-063**: Agent::run<T>() returns typed result with auto schema negotiation
- **SC-064**: RunState serialisation round-trips correctly
- **SC-065**: Bidirectional hooks can cancel tool calls
- **SC-066**: Typed approval content types round-trip through messages
- **SC-067**: CompiledGraph::to_mermaid() produces valid Mermaid syntax
- **SC-068**: MultiProvider falls back on first provider failure
- **SC-104**: Thread append-only invariant holds — no message deletion or in-place mutation after append
- **SC-105**: Thread projection with SlidingWindowProjection preserves tool-use/tool-result pairs
- **SC-106**: AgentMemory space isolation — concurrent writes to different spaces do not interfere
- **SC-107**: AgentIdentity revision counter increments monotonically on each state change
- **SC-108**: Hibernation round-trip: `thaw(hibernate(agent))` produces functionally equivalent agent
- **SC-109**: HibernationStore list returns all hibernated agents with correct metadata
- **SC-110**: SynwireInstance test isolation — two instances share no state
- **SC-111**: Instance-scoped registry prevents cross-instance agent name resolution
