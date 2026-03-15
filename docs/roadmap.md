# Synwire Roadmap

Post-M1 features organised as concrete work units, each sized for one `/speckit.specify` cycle. M1 (Core + Orchestrator) is defined in [spec.md](../specs/001-synwire/spec.md). The generic State trait refactor (002) is complete.

## Work Units

Each unit produces one speckit feature branch (`NNN-short-name`), one `synwire-*` crate (or modification to an existing crate), and is independently shippable.

### Critical Path (AG-UI)

```text
001-synwire (done) → 002-generic-state-trait (done)
    → 003-agent-core ──→ 004-mcp-adapters ──→ 006-ag-ui
           │                                      ↑
           ├──→ 005-cognitive-session ────────────┘
           └──→ 011-observability ───────────────┘
```

### Full Sequence

| # | Unit | Crate(s) | Depends On | FRs | Status |
|---|------|----------|------------|-----|--------|
| 001 | [M1 Core + Orchestrator](../specs/001-synwire/spec.md) | synwire-core, synwire-orchestrator, synwire-checkpoint, synwire-checkpoint-sqlite, synwire-llm-openai, synwire-llm-ollama, synwire-derive, synwire-test-utils, synwire | — | FR-001–FR-068 | Done |
| 002 | [Generic State Trait](../specs/002-generic-state-trait/spec.md) | synwire-orchestrator, synwire-derive | 001 | FR-S01–FR-S18 | Done |
| 003 | [Agent Core](#003-agent-core) | synwire-agents | 002 | FR-070–094, FR-133–163, FR-363–366, FR-557–572 | — |
| 004 | [MCP Adapters](#004-mcp-adapters) | synwire-mcp-adapters | 003 | FR-112–132, FR-333–335, FR-357–362 | — |
| 005 | [Cognitive Architecture & Sessions](#005-cognitive-session) | synwire-agents | 003 | FR-573–583, FR-385–393, FR-371–377 | — |
| 006 | [AG-UI Protocol](#006-ag-ui) | synwire-ag-ui | 003, 004 | FR-213–260 | — |
| 007 | [Neo4j Provider](#007-neo4j) | synwire-neo4j | 002 | FR-336–339 (Neo4j subset) | — |
| 008 | [Temporal Provider](#008-temporal) | synwire-temporal | 002 | FR-336–339 (Temporal subset) | — |
| 009 | [Search Providers](#009-search-providers) | synwire-serper, synwire-searxng | 002 | FR-336–339 (search subset) | — |
| 010 | [PostgreSQL Provider](#010-postgresql) | synwire-postgres | 002 | FR-336–339 (PostgreSQL subset) | — |
| 011 | [Observability](#011-observability) | synwire-core, synwire-agents | 003 | FR-320–322, FR-340–342, FR-378–381, FR-511–556, FR-588–591 | — |

### Deferred (not on critical path)

| Unit | Scope | Depends On |
|------|-------|------------|
| Evaluation Framework | Harbor sandbox, scorers, LLM-as-judge | 003 |
| A2A Protocol | JSON-RPC + REST + gRPC, task lifecycle | 003 |
| Structured Output & DSPy | Signatures, predict modules, teleprompt | 003 |
| Additional Providers | Qdrant, pgvector | 002 |
| Sandboxes & CLI | Local sandbox, K8s sandbox, CLI binary | 003 |
| Instance Scoping | Instance-scoped runtime isolation | 005 |

---

## Work Unit Summaries

### 003 — Agent Core

**Crate**: `synwire-agents`

The minimum viable agent runtime. Agents are pure functions returning directives; a separate executor handles side effects.

**Scope**:
- Directive system: `Directive` enum, `DirectiveResult<S>`, `DirectiveExecutor` trait, `DirectiveFilter`, serialisation (FR-557–562)
- Execution strategies: `ExecutionStrategy` trait, `DirectStrategy`, `FsmStrategy` with transitions and guards (FR-563–567)
- Plugin system with state isolation: `PluginStateKey`, typed state accessors, merge-on-compose (FR-143–144, FR-568–570)
- Signal routing: three-tier priority (strategy / agent / plugin), `SignalRouter` trait (FR-571–572)
- `AgentNode` trait, `Agent<D, O>` builder, `RunContext<D>`, `OutputMode<T>`, `ModelSelector` (FR-133–138)
- Agent callbacks: `BeforeAgentCallback`, `AfterAgentCallback` (FR-139)
- Runner: session lookup, routing, invocation, event collection (FR-160–163)
- Backend protocol: `BackendProtocol`, `BackendFactory`, `FileOperationError` (FR-070–074)
- Backend implementations: StateBackend, StoreBackend, FilesystemBackend, CompositeBackend (FR-075–079)
- Middleware: Filesystem, PatchToolCalls, Summarisation, PromptCaching (FR-081–082, FR-087–089)
- Execution control: `max_turns`, `run_error_handlers`, `tool_error_formatter` (FR-363–366)
- Streaming events: partial vs final, `turn_complete`, `is_final_response()` (FR-157–159)

**Not included** (deferred): SubAgentMiddleware, SkillsMiddleware, TodoListMiddleware, MemoryMiddleware, agent transfer/handoff, workflow agents (Sequential/Parallel/Loop), sandbox backends, CLI.

### 004 — MCP Adapters

**Crate**: `synwire-mcp-adapters`

Multi-server MCP client with bidirectional tool conversion.

**Scope**:
- `MultiServerMcpClient`: connection lifecycle, health checks (FR-112–116)
- Four transports: Stdio, SSE, StreamableHttp, WebSocket (FR-117–120)
- Bidirectional tool conversion: MCP tool ↔ Synwire `Tool` trait (FR-121–124)
- Cursor-based tool pagination with 1000-page safeguard (FR-125–126)
- Tool interceptor pattern (onion/middleware style) (FR-127–128)
- MCP callbacks: LoggingMessage, Progress, Elicitation (FR-129–132)
- Tool system enrichment: categories, `ToolProvider` trait, `#[tool]` macro enhancements (FR-333–335, FR-357–362)

### 005 — Cognitive Architecture & Sessions

**Crate**: `synwire-agents` (extends 003)

Cognitive primitives, session management, approval/HITL. Builds on agent core.

**Scope**:
- Thread: append-only canonical log, `ProjectionStrategy` (Full, SlidingWindow, Summarizing, TokenBudget) (FR-573–575)
- Memory: `AgentMemory` with typed named spaces (world, tasks, scratch), persistence policies (FR-576–577)
- Identity: `AgentIdentity` with profile, revision counter, capabilities (FR-578–579)
- Memory scoring: composite scoring (recency/semantic/importance), consolidation, `KnowledgeBase`, LLM importance inference (FR-371–374)
- Session management: `RunState` serialisation, `SessionProvider`, in-memory + file backends (FR-385–387)
- Hibernation: `Hibernatable` trait, `HibernationStore`, metadata, auto-hibernation policy (FR-580–583)
- Hooks & lifecycle: bidirectional hooks (cancel/retry), FIFO/LIFO ordering, `pre_model_filter` (FR-388–390)
- Approval & HITL: `ApprovalRequest`/`ApprovalResponse`, approval ledger, `FeedbackProvider` (FR-391–393)

### 006 — AG-UI Protocol

**Crate**: `synwire-ag-ui`

SSE streaming protocol for frontend integration.

**Scope**:
- SSE transport with W3C Trace Context propagation (FR-213–218)
- 14 event types: RunStarted, RunFinished, RunError, StepStarted, StepFinished, TextMessageStart, TextMessageContent, TextMessageEnd, ToolCallStart, ToolCallArgs, ToolCallEnd, StateSnapshot, StateDelta, Custom (FR-219–240)
- Frontend tools: agent can request UI actions from the client (FR-241–245)
- State synchronisation: full snapshot + delta streaming (FR-246–250)
- Generative UI: structured content blocks for rich rendering (FR-251–255)
- Client SDK types: `AgUiClient`, `RunConfig`, event stream consumer (FR-256–260)

### 007 — Neo4j Provider

**Crate**: `synwire-neo4j`

Neo4j as vector store and graph store.

**Scope**:
- `Neo4jVectorStore` implementing `VectorStore` trait with index management
- Cypher-based metadata filtering
- Graph-aware retrieval (traverse relationships during search)
- Connection pooling via `neo4rs`

### 008 — Temporal Provider

**Crate**: `synwire-temporal`

Temporal for durable workflow orchestration.

**Scope**:
- `TemporalWorkflowRunner` for executing Synwire graphs as Temporal workflows
- Activity wrapping for graph nodes
- Signal/query support mapped to graph interrupts
- Retry and timeout policies bridged from Temporal to Synwire `RetryPolicy`

### 009 — Search Providers

**Crates**: `synwire-serper`, `synwire-searxng`

Search engine integrations as `Tool` implementations.

**Scope**:
- `SerperSearchTool` implementing `Tool` trait with Google Search API (web, images, news, places)
- `SearxngSearchTool` implementing `Tool` trait with self-hosted SearXNG instance
- Result parsing into structured `Document` types for RAG pipelines
- Rate limiting and credential management via `CredentialProvider`

### 010 — PostgreSQL Provider

**Crate**: `synwire-postgres`

PostgreSQL as document store and structured data source.

**Scope**:
- `PostgresDocumentLoader` implementing `DocumentLoader` trait with configurable SQL queries
- `PostgresVectorStore` implementing `VectorStore` trait with pgvector extension support
- Connection pooling via `sqlx`
- Transaction-safe batch operations for document ingestion

### 011 — Observability

**Crate**: `synwire-core` (callback/tracing extensions), `synwire-agents` (agent instrumentation)

OpenTelemetry-based observability with GenAI semantic conventions and per-agent debug recording. Behind a single `tracing` feature flag.

**Scope**:
- OTel GenAI semantic convention attributes for LLM, tool, agent, embedding, retrieval spans (FR-511–521)
- Callback system extensions: embedding hooks, token usage metadata, cost estimation, TTFT tracking (FR-522–527)
- Event bus: typed subscribe/publish, feature-gated events (AgentStart, AgentEnd, ModelCall, ToolCall, MemoryWrite, Handoff) (FR-381, FR-528–529)
- Tracing configuration: sensitive data exclusion, `TracingAgentWrapper<A>`, content filtering, `SecretValue` auto-redaction (FR-378–380, FR-530–531)
- Distributed tracing: W3C Trace Context for A2A/MCP/AG-UI boundaries (FR-532–535)
- Metrics: OTel GenAI metrics (token usage, operation duration, TTFC), aggregate/per-node metrics, `QuotaEnforcer` (FR-320–322, FR-536)
- OTLP export: generic exporter via opentelemetry-otlp, feature-gated (FR-540–541)
- Span lifecycle: streaming, batch, retry, fallback, handoff, nested graph, concurrent superstep spans (FR-542–551)
- Per-agent debug recording: `DebugRecorder`, bounded ring buffer, `debug_events()` accessor, EventBus integration (FR-588–591)
- Non-functional: <50us per span, concurrent callback safety, bounded attribute memory (FR-552–556)

---

## Parallel Opportunities

```text
After 002 (done):
  ├── 003 (agent core)        — critical path
  ├── 007 (neo4j)             — independent, can start now
  ├── 008 (temporal)          — independent, can start now
  ├── 009 (search providers)  — independent, can start now
  └── 010 (postgresql)        — independent, can start now

After 003:
  ├── 004 (mcp adapters)      — critical path
  ├── 005 (cognitive/session) — can parallel with 004
  └── 011 (observability)     — can parallel with 004, 005

After 004 + 005:
  └── 006 (ag-ui)             — critical path, final deliverable
```

007–010 have no dependency on the agent framework — they implement provider traits from M1. They can all be built in parallel with 003.

## FR Index

Feature requirements are distributed across the topic documents. Detailed FR definitions remain in the topic documents under `docs/roadmap/`.

| FR Range | Topic | Document |
|----------|-------|----------|
| FR-070–094 | Agent backend, middleware, factory | [agents.md](roadmap/agents.md) |
| FR-112–132 | MCP adapters | [mcp-and-tools.md](roadmap/mcp-and-tools.md) |
| FR-133–163 | Agent convenience API, system enhancements | [agents.md](roadmap/agents.md) |
| FR-213–260 | AG-UI protocol | [ag-ui.md](roadmap/ag-ui.md) |
| FR-333–335 | Tool system enrichment | [mcp-and-tools.md](roadmap/mcp-and-tools.md) |
| FR-336–339 | Provider integrations | [providers.md](roadmap/providers.md) |
| FR-357–362 | Tool enhancements | [mcp-and-tools.md](roadmap/mcp-and-tools.md) |
| FR-363–370 | Execution control, handoff | [agents.md](roadmap/agents.md) |
| FR-371–393 | Memory, sessions, hooks, HITL | [system-infrastructure.md](roadmap/system-infrastructure.md) |
| FR-557–572 | Directives, strategies, plugins, routing | [agents.md](roadmap/agents.md) |
| FR-573–583 | Cognitive architecture, hibernation | [system-infrastructure.md](roadmap/system-infrastructure.md) |
| FR-511–556 | OTel spans, metrics, tracing config, OTLP export | [observability.md](roadmap/observability.md) |
| FR-588–591 | Debug recording | [observability.md](roadmap/observability.md) |
