# Synwire Roadmap

Post-M1 features organised as concrete work units, each sized for one `/speckit.specify` cycle. M1 (Core + Orchestrator) is defined in [spec.md](../specs/001-synwire/spec.md). The generic State trait refactor (002) is complete. The agent core (003) is complete. MCP adapters (004) are complete.

## Work Units

Each unit produces one speckit feature branch (`NNN-short-name`), one `synwire-*` crate (or modification to an existing crate), and is independently shippable.

### Critical Path (AG-UI)

```text
001-synwire (done) → 002-generic-state-trait (done) → 003-agent-core (done)
    → 004-mcp-adapters ──→ 006-ag-ui
           │                    ↑
           ├──→ 005-cognitive-session ─┘
           └──→ 011-observability ────┘
```

### Full Sequence

| # | Unit | Crate(s) | Depends On | FRs | Status |
|---|------|----------|------------|-----|--------|
| 001 | [M1 Core + Orchestrator](../specs/001-synwire/spec.md) | synwire-core, synwire-orchestrator, synwire-checkpoint, synwire-checkpoint-sqlite, synwire-llm-openai, synwire-llm-ollama, synwire-derive, synwire-test-utils, synwire | — | FR-001–FR-068 | Done |
| 002 | [Generic State Trait](../specs/002-generic-state-trait/spec.md) | synwire-orchestrator, synwire-derive | 001 | FR-S01–FR-S18 | Done |
| 003 | [Agent Core](#003-agent-core) | synwire-agent, synwire-chunker, synwire-index, synwire-embeddings-local, synwire-vectorstore-lancedb, synwire-lsp, synwire-dap, synwire-sandbox, synwire-storage, synwire-agent-skills, synwire-mcp-server | 002 | FR-070–094, FR-133–163, FR-363–366, FR-557–572, FR-618–632, FR-840–915 | Done |
| 004 | [MCP Adapters](#004-mcp-adapters) | synwire-mcp-adapters | 003 | FR-112–132, FR-333–335, FR-357–362 | Done |
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

### 003 — Agent Core ✓ Done

**Crates**: `synwire-agent`, `synwire-chunker`, `synwire-index`, `synwire-embeddings-local`, `synwire-vectorstore-lancedb`, `synwire-lsp`, `synwire-dap`, `synwire-sandbox`, `synwire-storage`, `synwire-agent-skills`, `synwire-mcp-server`

243 tasks across 35 phases. Spec: [spec.md](../specs/003-agent-core/spec.md).

**Implemented**:
- Directive system, FSM/MCTS execution strategies, plugin state isolation, three-tier signal routing
- `AgentNode`, `RunContext`, `OutputMode`, `ModelSelector`, streaming events
- `Vfs` trait (30 capability flags), `LocalProvider`, `MemoryProvider`, `CompositeProvider`, `StoreProvider`, `ReadGuard`
- Middleware: `HierarchicalNarrowing`, `RepoFetchDetector`, `PatchToolCalls`, `PromptCaching`
- `synwire-chunker`: tree-sitter AST-aware chunking, 14 languages, per-method qualified symbols, `skeleton` VFS op
- `synwire-index`: semantic pipeline (walk→chunk→embed→store), BM25 hybrid search (tantivy, `hybrid-search` feature), code dependency graph (`code-graph` feature), community detection via label propagation (`community-detection` feature), xref graph
- `synwire-embeddings-local`: fastembed-rs, bge-small-en-v1.5 default, reranking
- `synwire-vectorstore-lancedb`: LanceDB vector store
- `synwire-lsp`: 12-tool LSP client (hover, goto-definition, references, document-symbols, rename, …)
- `synwire-dap`: DAP debug client (sessions, breakpoints, evaluate)
- `synwire-sandbox`: process isolation, approval gates, `ShellSandbox`, `ProcessManager`, `ArchiveManager`
- `synwire-storage`: `StorageLayout` (product-scoped paths, durable/cache split), `RepoId`, `WorktreeId`, `DependencyIndex` (Cargo/go.mod/npm/pyproject)
- `synwire-agent-skills`: agentskills.io spec, Lua/Rhai/WASM/tool-sequence/external runtimes, progressive disclosure, `SkillRegistry`
- `synwire-mcp-server`: standalone stdio MCP binary, 20+ tools, `ToolSearchIndex` (hybrid keyword+vector, progressive disclosure, ~85% token reduction), `SamplingProvider` trait, agent skills auto-discovery, daily-rotating log files
- Research features: SBFL Ochiai fault localisation, experience pool (SQLite, two-tier local+global), dynamic call graph, dataflow tracer

### 004 — MCP Adapters ✓ Done

**Crate**: `synwire-mcp-adapters`

Multi-server MCP client with bidirectional tool conversion.

**Implemented**:
- `McpTransport` trait, `McpConnectionState`, `McpServerStatus`, `McpToolDescriptor` — `synwire-core/src/mcp/traits.rs`
- `McpServerConfig` — `synwire-core/src/mcp/config.rs`
- `ElicitationRequest`/`ElicitationResult`/`ElicitationCallback` — `synwire-core/src/mcp/elicitation.rs`
- `McpLifecycleManager` (connect, reconnect, health monitoring) — `synwire-agent/src/mcp/lifecycle.rs`
- `StdioMcpTransport`, `HttpMcpTransport` (SSE + StreamableHttp), `InProcessMcpTransport` — `synwire-agent/src/mcp/`
- `SamplingProvider` trait — `synwire-core/src/agents/sampling.rs`
- `MultiServerMcpClient`: aggregate tools from multiple servers, health checks — `synwire-mcp-adapters/src/client.rs`
- WebSocket transport — `synwire-mcp-adapters/src/transport/websocket.rs`
- Bidirectional tool conversion: MCP tool ↔ Synwire `Tool` trait — `synwire-mcp-adapters/src/convert/tool.rs`
- Cursor-based tool pagination with 1000-page safeguard — `synwire-mcp-adapters/src/pagination.rs`
- Tool interceptor pattern (onion/middleware style) — `synwire-mcp-adapters/src/interceptor.rs`
- MCP callbacks: LoggingMessage, Progress — `synwire-mcp-adapters/src/callbacks.rs`
- Tool system enrichment: `ToolProvider` trait, `#[tool]` macro enhancements — `synwire-mcp-adapters/src/provider.rs`

### 005 — Cognitive Architecture & Sessions

**Crate**: `synwire-agent` (extends 003)

Cognitive primitives, session management, approval/HITL. Builds on agent core.

**Already implemented (in 003)**:
- `Session`, `SessionManager`, `SessionMetadata` traits — `synwire-core/src/agents/session.rs`
- `InMemorySessionManager` + `MountedRepo` session state — `synwire-agent/src/session/`
- `PreToolUseHook`, `PostToolUseHook`, `PreModelHook` with timeout enforcement and glob matching — `synwire-core/src/agents/hooks.rs`
- `PermissionMode`, `PermissionBehavior`, `PermissionRule` — `synwire-core/src/agents/permission.rs`
- `SandboxConfig`, `IsolationLevel`, filesystem rules — `synwire-core/src/agents/sandbox.rs`
- `synwire-sandbox`: process isolation, approval gates, `ShellSandbox`, `ProcessManager`
- Experience pool (two-tier SQLite-backed, local-first fallback) — `synwire-agent/src/experience/`

**Remaining scope**:
- Thread: append-only canonical log, `ProjectionStrategy` (Full, SlidingWindow, Summarizing, TokenBudget) (FR-573–575)
- Memory: `AgentMemory` with typed named spaces (world, tasks, scratch), persistence policies (FR-576–577)
- Identity: `AgentIdentity` with profile, revision counter, capabilities (FR-578–579)
- Memory scoring: composite scoring (recency/semantic/importance), consolidation, `KnowledgeBase`, LLM importance inference (FR-371–374)
- Hibernation: `Hibernatable` trait, `HibernationStore`, metadata, auto-hibernation policy (FR-580–583)
- Approval & HITL: `ApprovalRequest`/`ApprovalResponse`, approval ledger, `FeedbackProvider` (FR-391–393)
- Hooks: bidirectional (cancel/retry), FIFO/LIFO ordering, `pre_model_filter` (FR-388–390; pre/post-tool and pre-model hooks are done)

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
After 003 (done):
  ├── 004 (mcp adapters)      — critical path, can start now
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
