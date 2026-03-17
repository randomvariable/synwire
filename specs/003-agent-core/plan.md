# Implementation Plan: Agent Core Runtime

**Branch**: `003-agent-core` | **Date**: 2026-03-15 (expanded 2026-03-16, MCP adapters 2026-03-16) | **Spec**: [spec.md](specs/003-agent-core/spec.md)
**Input**: Feature specification from `/specs/003-agent-core/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Implement a full agent runtime for synwire providing: a directive/effect system for pure testable agent logic, pluggable execution strategies (immediate + FSM), a composable plugin system with type-safe state isolation, a VFS (Virtual Filesystem) abstraction over heterogeneous data sources, middleware stack for cross-cutting concerns, three-tier signal routing, streaming events, and a convenience `Agent` builder API. Additionally: a semantic code search pipeline (AST-aware chunking, local embeddings, vector store, incremental indexing), LSP/DAP integration for structural code intelligence, process sandboxing, code dependency graphs with community detection (hit-leiden), hybrid BM25+vector search, hierarchical narrowing, configurable persistent storage layout, agent skills following the agentskills.io spec with embedded Lua/Rhai/WASM runtimes, and a standalone MCP server binary for immediate use in Claude Code, Copilot, and Cursor. Builds on the existing `synwire-core` traits (Tool, RunnableCore, BaseChatModel, State) and `synwire-orchestrator` (StateGraph, CompiledGraph).

## Technical Context

**Language/Version**: Rust stable, edition 2024, MSRV 1.85
**Primary Dependencies**: tokio 1 (async runtime), serde/serde_json 1 (serialization), thiserror 2 (errors), futures-core/futures-util 0.3 (streams), reqwest 0.12 (HTTP with rustls), uuid 1, chrono 0.4, tracing 0.1, schemars 0.8 (JSON Schema), moka 0.12 (caching)
**Storage**: SQLite via rusqlite 0.32 (checkpoint-sqlite exists), LanceDB (vector store), tantivy (BM25 index — planned), ephemeral in-memory (`MemoryProvider`), `BaseStore` trait (`StoreProvider`). `StorageLayout` coordinates all persistence with product-scoped paths and `ProjectId` (Git first-commit hash)
**Testing**: cargo-nextest, proptest 1, mockall 0.13, tokio-test 0.4, criterion 0.5 (benches)
**Target Platform**: Linux (primary), cross-platform library
**Project Type**: Rust library workspace (10 existing crates)
**Performance Goals**: Agent node invocation <1ms overhead beyond LLM call latency; directive serialization round-trip <100µs for 100 directives
**Constraints**: `#![forbid(unsafe_code)]` on core + orchestrator crates; all public types `Send + Sync`; zero panics in library code; all enums/config structs `#[non_exhaustive]`
**Scale/Scope**: Library targeting agent applications with 1-100 concurrent agents, each with 1-50 tools, 0-10 MCP servers, persistent sessions with fork/rewind. Must handle repositories up to Linux kernel scale (~70,000 files, ~30M LOC) with <2GB RSS memory. MCP server supports multiple concurrent stdio instances sharing persistent data
**Additional Dependencies (new phases)**: tree-sitter + language grammars (chunking), fastembed-rs (embeddings, default bge-small-en-v1.5), lancedb (vector store), hit-leiden (community detection), mlua (Lua runtime), rhai (scripting), extism (WASM plugins), lsp-types (LSP), dap-types (DAP), clap (CLI), tracing-subscriber + tracing-appender (logging), flock/fs2 (file locking), tantivy (BM25 — planned)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle I: Trait-Based Abstractions — PASS

- All new abstractions (`Directive`, `ExecutionStrategy`, `BackendProtocol`, `Plugin`, `SignalRouter`, `Middleware`, `AgentNode`, `DirectiveExecutor`, `DirectiveFilter`, `ApprovalCallback`) will be defined as Rust traits in `synwire-core`
- Provider-specific implementations (FilesystemBackend, GitBackend, HttpBackend, etc.) will live in separate crates or feature-gated modules
- Associated types and generic bounds used where zero-cost; `dyn Trait` only at composition boundaries (e.g., `Vec<Box<dyn Middleware>>`)

### Principle II: API Conceptual Parity — PASS (with documented deviations)

- Directives map to LangChain Python's agent actions/instructions concept but use algebraic effect pattern (deviation documented: Rust's type system enables compile-time effect verification not possible in Python)
- ExecutionStrategy maps to Python's agent executor concept
- BackendProtocol maps to Python's sandbox/tool backend concept
- Plugin system maps to Python's runner plugins
- Middleware stack maps to Python's middleware concept
- Agent builder API maps to Python's Agent class

### Principle III: Safety and Correctness — PASS

- `#![forbid(unsafe_code)]` maintained on core and orchestrator crates
- All public APIs return `Result<T, E>` — zero panics in library code
- Error types use `thiserror`, enums are `#[non_exhaustive]`
- All public types derive `Debug`; `Clone`, `Send`, `Sync` where semantically correct
- Directive serialization uses serde with `#[serde(tag = "type")]` for round-trip safety

### Principle IV: Async-First with Sync Wrappers — PASS

- All backend operations implemented as `async fn` using tokio
- Core traits use `BoxFuture` (existing pattern in codebase) or RPITIT where stable
- Streaming responses use `futures::Stream` (existing `BoxStream` pattern)
- Sync convenience wrappers via `blocking` feature flag

### Principle V: Comprehensive Testing — PASS

- Unit tests in `#[cfg(test)] mod tests` within each module
- Integration tests requiring network gated behind `#[cfg(feature = "integration-tests")]`
- `mockall` for mocking external dependencies
- `cargo test` with default features passes without network access
- Directive round-trip serialization tests, FSM transition validation tests, backend conformance suite

### Gate Result: **PASS** — No violations. Proceed to Phase 0.

## Project Structure

### Documentation (this feature)

```text
specs/003-agent-core/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
crates/
├── synwire-core/src/
│   ├── agents/                    # Agent runtime traits (directive, strategy, plugin, etc.)
│   ├── vfs/                       # Virtual Filesystem (was backends/)
│   │   ├── protocol.rs           # Vfs trait (was BackendProtocol)
│   │   ├── types.rs              # VfsCapabilities, response types
│   │   ├── error.rs              # VfsError (was BackendError)
│   │   ├── memory.rs             # MemoryProvider (was StateBackend)
│   │   ├── grep_options.rs       # GrepOptions, GrepOutputMode
│   │   ├── output.rs             # OutputFormat (plain text, Markdown, TOON, JSON)
│   │   ├── tools.rs              # VFS tools (vfs_tools() factory)
│   │   └── agentic_ignore.rs     # .agenticignore support
│   ├── sandbox/                   # Approval gates (was in backends/)
│   │   ├── approval.rs           # ApprovalCallback, ApprovalDecision
│   │   └── mod.rs
│   ├── mcp/                       # MCP protocol traits
│   ├── state.rs                   # State trait
│   ├── embeddings.rs              # Embeddings trait
│   ├── rerankers.rs               # Reranker trait
│   ├── vectorstores.rs            # VectorStore trait
│   └── documents.rs               # Document type
│
├── synwire-agent/src/             # Agent runtime implementations
│   ├── vfs/                       # VFS providers
│   │   ├── local.rs              # LocalProvider (was FilesystemBackend)
│   │   ├── composite.rs          # CompositeProvider (was CompositeBackend)
│   │   └── store.rs              # StoreProvider (was StoreBackend)
│   ├── middleware/
│   ├── strategies/
│   ├── mcp/
│   └── session/
│
├── synwire-chunker/src/           # Tree-sitter AST-aware code chunking
│   ├── ast_chunker.rs            # AST parsing, definition extraction
│   ├── text_chunker.rs           # Recursive text splitting fallback
│   └── language.rs               # Language detection (14 languages)
│
├── synwire-embeddings-local/src/  # Local embeddings via fastembed-rs
│   ├── embeddings.rs             # Embeddings trait impl (bge-small default)
│   └── reranker.rs               # Reranker trait impl (cross-encoder)
│
├── synwire-vectorstore-lancedb/src/ # LanceDB vector store
│   └── store.rs                  # VectorStore trait impl
│
├── synwire-index/src/             # Semantic indexing pipeline
│   ├── index.rs                  # SemanticIndex (orchestrator)
│   ├── pipeline.rs               # walk → chunk → embed → store
│   ├── walker.rs                 # Directory walking
│   ├── cache.rs                  # Cache directory management
│   ├── hashes.rs                 # Content hash registry (xxh128)
│   ├── watcher.rs                # File watcher for auto-reindexing
│   ├── config.rs                 # IndexConfig
│   └── graph/                    # Code dependency graph (planned)
│
├── synwire-lsp/src/               # LSP client integration
│   ├── client.rs                 # LspClient (lifecycle, doc sync, diagnostics)
│   ├── tools.rs                  # 12 capability-conditional tools
│   ├── plugin.rs                 # LSP plugin for agent lifecycle
│   └── registry.rs              # Multi-server registry
│
├── synwire-dap/src/               # DAP client integration
│   ├── client.rs                 # DapClient (debug sessions)
│   ├── tools.rs                  # Debug tools (breakpoints, step, eval)
│   ├── transport.rs              # Stdio DAP transport
│   └── codec.rs                  # DAP wire protocol codec
│
├── synwire-sandbox/src/           # Process sandboxing
│
├── synwire-storage/src/           # Storage layout + identity (planned)
│   ├── layout.rs                 # StorageLayout (product-scoped paths)
│   ├── identity.rs               # RepoId + WorktreeId
│   ├── concurrency.rs            # SQLite WAL helpers, atomic rename
│   ├── migration.rs              # StorageMigration trait + runner
│   └── registry.rs              # ProjectRegistry
│
├── synwire-agent-skills/src/      # Agent skills (planned)
│   ├── manifest.rs               # SKILL.md frontmatter parser
│   ├── loader.rs                 # Skill discovery + validation
│   ├── registry.rs               # Skill registry (progressive disclosure)
│   └── runtime/                  # Lua, Rhai, WASM, sequence, external
│
├── synwire-daemon/src/            # Singleton background process (planned)
│   ├── manager.rs                # Multi-repo/worktree manager
│   ├── ipc.rs                    # UDS protocol
│   └── lifecycle.rs              # Auto-start, grace period, shutdown
│
├── synwire-mcp-server/src/        # MCP server binary (planned)
│   ├── cli.rs                    # Argument parsing
│   ├── server.rs                 # MCP wiring, tool registration
│   ├── proxy.rs                  # MCP server → daemon routing
│   └── tools.rs                  # MCP tool definitions
│
├── synwire-checkpoint/            # Checkpoint persistence traits
├── synwire-checkpoint-sqlite/     # SQLite checkpoint backend (WAL mode)
├── synwire-llm-openai/           # OpenAI provider
├── synwire-llm-ollama/           # Ollama provider
├── synwire-mcp-adapters/src/      # MCP multi-server client + tool conversion
│   ├── lib.rs
│   ├── client.rs                # MultiServerMcpClient
│   ├── session.rs               # McpClientSession (guard-scoped)
│   ├── transport/
│   │   └── websocket.rs         # WebSocket transport
│   ├── convert/
│   │   ├── tool.rs              # MCP↔Synwire tool conversion
│   │   ├── content.rs           # Content type mapping
│   │   ├── resource.rs          # Resource → McpBlob
│   │   └── prompt.rs            # Prompt → Message
│   ├── interceptor.rs           # ToolCallInterceptor trait + onion chain
│   ├── callbacks.rs             # McpCallbacks (Logging, Progress)
│   ├── pagination.rs            # Cursor-based pagination (1000-page cap)
│   ├── provider.rs              # McpToolProvider (ToolProvider impl)
│   ├── validation.rs            # JSON Schema validation
│   └── error.rs                 # McpAdapterError
│
├── synwire-derive/               # Proc macros (#[tool] enhancements)
├── synwire-test-utils/           # Test helpers
└── synwire/                      # Convenience re-exports
```

**Structure Decision**: The `backends` module was refactored to `vfs` (Virtual Filesystem) with renamed traits (`BackendProtocol` to `Vfs`, `BackendError` to `VfsError`, `StateBackend` to `MemoryProvider`). Sandbox/approval concerns were separated into their own `sandbox` module. `GitBackend` and `HttpBackend` were removed. Concrete VFS providers (`LocalProvider`, `CompositeProvider`, `StoreProvider`) live in `synwire-agent`. New crates added for the semantic search pipeline (`synwire-chunker`, `synwire-embeddings-local`, `synwire-vectorstore-lancedb`, `synwire-index`), language server integration (`synwire-lsp`), debug adapter integration (`synwire-dap`), and process sandboxing (`synwire-sandbox`). Planned crates include `synwire-storage`, `synwire-agent-skills`, `synwire-daemon`, and `synwire-mcp-server`. Core traits remain in `synwire-core`; implementations in separate crates.

## Complexity Tracking

No constitution violations requiring justification. New crates (`synwire-agent`, `synwire-chunker`, `synwire-embeddings-local`, `synwire-vectorstore-lancedb`, `synwire-index`, `synwire-lsp`, `synwire-dap`, `synwire-sandbox`) follow the existing workspace pattern (core traits in `synwire-core`, implementations in separate crates). The VFS refactor simplified the core abstraction surface by removing `GitBackend`/`HttpBackend` and separating sandbox concerns.

---

## Expanded Plan: Phases 21–32 (US16–US38)

**Context**: Phases 1–20 (US1–US15) are complete. The following phases cover the VFS refactor (already implemented), semantic search pipeline (already implemented), LSP/DAP (already implemented), and new research-driven improvements + MCP server + agent skills.

### Phase 21: VFS Refactor Reconciliation [IMPLEMENTED]

**Status**: Complete — captured in spec as US16, FR-713–726.

The `backends` module was refactored to `vfs`. `BackendProtocol` → `Vfs`, `StateBackend` → `MemoryProvider`, `FilesystemBackend` → `LocalProvider`, `CompositeBackend` → `CompositeProvider`. ReadGuard and stale-read detection added. Sandbox concerns separated to `sandbox` module. Git/HTTP backends removed from VFS.

No new tasks — this phase documents already-shipped work for traceability.

### Phase 22: AST Chunking + Embeddings + Vector Store [IMPLEMENTED]

**Status**: Complete — captured in spec as US17–US19, FR-727–737.

New crates: `synwire-chunker` (tree-sitter, 14 languages), `synwire-embeddings-local` (fastembed-rs, bge-small-en-v1.5 default, configurable), `synwire-vectorstore-lancedb`.

No new tasks — this phase documents already-shipped work.

### Phase 23: Semantic Indexing Pipeline [IMPLEMENTED]

**Status**: Complete — captured in spec as US20, FR-738–745.

New crate: `synwire-index`. Walk → chunk → embed → store pipeline with incremental re-indexing (xxh128 hashes), file watcher, background async indexing, optional cross-encoder reranking.

No new tasks — this phase documents already-shipped work.

### Phase 24: LSP + DAP + Sandbox [IMPLEMENTED]

**Status**: Complete — captured in spec as US21–US23, FR-746–761.

New crates: `synwire-lsp` (12 tools, capability-conditional), `synwire-dap` (debug sessions), `synwire-sandbox` (process isolation).

No new tasks — this phase documents already-shipped work.

### Phase 25: Storage Layout and Project Identity (P1, US36)

**Goal**: Configurable persistent storage with product-scoped paths, stable project identity, cache/durable split, locking, and migration.

**New crate**: `synwire-storage` (or module in `synwire-core`)

**Key types**:
- `StorageLayout` — product-scoped path computation
- `RepoId` — Git first-commit hash (repository family, shared across worktrees)
- `WorktreeId` — `RepoId` + worktree root hash (specific working copy)
- `SynwireDaemon` — singleton per product, manages all repos/worktrees/clones, owns embedding model + watchers + global tier. MCP servers connect via UDS
- `StorageMigration` — per-subsystem version tracking + migration

**Depends on**: Nothing (foundational infrastructure). **Blocks**: Phase 26–32 (all persistence consumers).

**Key design decisions**:
- Durable data (`$XDG_DATA_HOME/<product>/`) vs cache (`$XDG_CACHE_HOME/<product>/`)
- Two-level identity: `RepoId` (first-commit hash, shared across worktrees) + `WorktreeId` (RepoId + worktree root hash)
- Singleton `synwire-daemon` per product — manages all repos/worktrees/clones, owns embedding model, file watchers, global tier. MCP servers connect via UDS as thin proxies
- Per-`WorktreeId` indices: each worktree/branch gets its own vector store, BM25, graph, communities
- Daemon handles cross-project operations: dependency index, xref graph, clone_repo, global experience pool
- Global tier (`global/`) for cross-project data (registry, experience pool, dependency index, xrefs)
- Config hierarchy: `SYNWIRE_DATA_DIR` env > programmatic override > project-local `.<product>/config.json` > platform default
- Existing checkpoint crates unchanged — `StorageLayout` provides paths

**Files**:
```
crates/synwire-storage/src/
├── lib.rs
├── layout.rs         # StorageLayout
├── identity.rs       # RepoId + WorktreeId (Git worktree-aware)
├── concurrency.rs    # ensure_wal_mode(), atomic rename helpers
├── migration.rs      # StorageMigration trait + runner
├── registry.rs       # ProjectRegistry (global/registry.json)
└── error.rs          # StorageError

crates/synwire-daemon/src/
├── main.rs           # Daemon entry point, PID file, socket listener
├── cli.rs            # --product-name, --project (pre-warm), --log-level
├── manager.rs        # Multi-repo/worktree manager, LRU eviction
├── ipc.rs            # UDS protocol: request/response for index/search/graph ops
├── lifecycle.rs      # Auto-start, grace period, shutdown
└── proxy.rs          # MCP server → daemon request routing
```

### Phase 26: Per-Method Chunking + File Skeletons (P1, US24–US25)

**Goal**: Fine-grained semantic search at method level; token-efficient file overviews.

**Modifies**: `synwire-chunker`, `synwire-core/src/vfs/`

**Key changes**:
- `ast_chunker.rs`: Recurse one level into `impl_item`, `class_body`, etc. → per-method chunks with `Type::method` symbol metadata
- New `skeleton` VFS operation: tree-sitter strips bodies, emits signatures + line numbers
- Text splitter fallback unchanged

**Depends on**: Phase 25 (StorageLayout for cache paths). Can start chunker work immediately.

### Phase 27: Hierarchical Narrowing (P1, US26)

**Goal**: Agentless-style 3-phase localization middleware/tool.

**Modifies**: `synwire-agent/src/middleware/` or `synwire-core/src/vfs/tools.rs`

**Key changes**:
- Compound tool or middleware composing `tree` → `skeleton`/`document_symbols` → `read_range`
- LLM-guided ranking at each phase (uses the agent's own model)

**Depends on**: Phase 26 (skeleton operation).

### Phase 28: Code Dependency Graph (P2, US27)

**Goal**: Cross-file call/import/inherit graph with multi-hop traversal.

**New feature**: `code-graph` on `synwire-index`

**Key design decisions**:
- Graph storage: disk-backed adjacency (memory-mapped or SQLite for edges) to handle Linux kernel scale (1M+ edges)
- Nodes: `(file, symbol)` tuples. Edges: typed (calls, imports, contains, inherits)
- Built from tree-sitter ASTs during indexing pipeline
- Incremental: file change recomputes only that file's edges
- New VFS operations: `graph_query(symbol, depth, direction)`, `graph_search(query, hops)`

**Depends on**: Phase 25 (StorageLayout), Phase 26 (per-method chunking for fine-grained nodes).

**Files**:
```
crates/synwire-index/src/
├── graph/
│   ├── mod.rs         # CodeGraph struct
│   ├── builder.rs     # AST → edges extraction
│   ├── storage.rs     # Disk-backed adjacency (SQLite or mmap)
│   ├── query.rs       # graph_query, graph_search
│   └── types.rs       # Node, Edge, GraphQuery, GraphResult
```

### Phase 29: Hybrid BM25 + Vector Search (P2, US28)

**Goal**: Combined lexical + semantic search.

**Modifies**: `synwire-index`

**Key design decisions**:
- BM25 via `tantivy` (disk-backed, handles 70K files)
- Built during same indexing pipeline alongside vector embeddings
- New VFS operation: `hybrid_search(query, alpha, top_k)`
- Incremental: tantivy supports document updates

**Depends on**: Phase 25 (StorageLayout for index paths).

### Phase 30: GraphRAG Community Detection (P2, US35)

**Goal**: Hierarchical community structure over code graph via hit-leiden.

**Modifies**: `synwire-index`

**Key design decisions**:
- `hit-leiden` dependency for Leiden community detection
- Runs after code graph construction (Phase 28)
- `CommunityState` persistence via `into_parts()`/`from_parts()` in StorageLayout
- Incremental: `CommunityState::update(delta_edges)` on file changes (63-136x faster than full recluster)
- LLM-generated community summaries stored in `communities/summaries/`
- New VFS operations: `communities`, `community_members`, `community_summary`, `community_search`

**Depends on**: Phase 28 (code graph).

### Phase 31: Agent Skills (P3, US33)

**Goal**: Agent Skills spec implementation with embedded runtimes.

**New crate**: `synwire-agent-skills`

**Key design decisions**:
- Follows [agentskills.io spec](https://agentskills.io/specification): `SKILL.md` frontmatter + Markdown instructions
- Progressive disclosure: name+description at startup, full body on activation, files on demand
- Discovery from `$DATA/<product>/skills/` (global) + `.<product>/skills/` (project-local, configurable)
- Synwire extension: optional `runtime` field in frontmatter (`lua`, `rhai`, `wasm`, `tool-sequence`, `external`)
- Lua via `mlua`, Rhai native, WASM via `extism` (source preserved alongside `.wasm`)
- `external` (subprocess) permitted but discouraged with warning
- VFS operations exposed to scripting runtimes as host functions

**Depends on**: Phase 25 (StorageLayout for skills directory).

**Files**:
```
crates/synwire-agent-skills/src/
├── lib.rs
├── manifest.rs       # SKILL.md frontmatter parsing + validation
├── loader.rs         # Discovery, validation, registration
├── registry.rs       # Skill registry with progressive disclosure
├── runtime/
│   ├── mod.rs        # SkillRuntime trait
│   ├── lua.rs        # mlua binding with VFS host functions
│   ├── rhai.rs       # Rhai binding with VFS host functions
│   ├── wasm.rs       # Extism binding with PDK host functions
│   ├── sequence.rs   # Tool-sequence executor
│   └── external.rs   # Subprocess executor (discouraged)
└── error.rs
```

### Phase 32: Standalone MCP Server Binary (P0, US38)

**Goal**: Ship a binary that exposes all synwire tools via MCP stdio transport.

**New crate**: `synwire-mcp-server` (binary)

**Priority**: P0 — this is the "use it today" deliverable. Can be built incrementally as each phase ships (e.g., v0.1 with just VFS+grep+index, adding graph/communities/skills as they land).

**Key design decisions**:
- Stdio only (no HTTP). Each editor instance spawns its own process
- Multiple instances share persistent data via synwire-daemon singleton + native backend concurrency (SQLite WAL, LanceDB, tantivy)
- Process exits cleanly when editor closes stdio pipe
- CLI: `--project <path>`, `--product-name <name>`, `--lsp <cmd>`, `--dap <cmd>`, `--embedding-model <model>`, `--log-level <level>`, `--config <path>`
- Logging: `tracing` to stderr + rotated log files in `StorageLayout.logs_dir()`
- Auto-discovers and loads agent skills from skills directories
- Single static binary via `cargo install synwire-mcp-server`

**Depends on**: Phase 25 (StorageLayout). Can incrementally integrate Phases 26–31 as they complete.

**Files**:
```
crates/synwire-mcp-server/src/
├── main.rs           # clap CLI, StorageLayout init, MCP server setup
├── cli.rs            # Argument parsing and config file loading
├── server.rs         # MCP server wiring (tools → VFS + index + LSP + DAP + skills)
├── tools.rs          # MCP tool definitions with JSON Schema + LLM descriptions
└── config.rs         # TOML/JSON config file format
```

### Phase 32a: Tool Search (US39)

**Goal**: Framework-level progressive tool discovery with `ToolSearchIndex`.

Implements FR-897–915: multi-vector embedding, hybrid scoring, namespace grouping, progressive disclosure, token budget allocation, iterative residual retrieval, tool co-occurrence graph, query intent extraction, adaptive scoring, feedback loop.

**Depends on**: Phase 25 (StorageLayout for persistence), Phase 32 (MCP server integration).

### Phase 32b: MCP Sampling Integration

**Goal**: `SamplingProvider` trait for tool-internal LLM access.

Community summary generation, hierarchical narrowing ranking, experience pool summaries — all lazy/on-demand, zero calls during indexing. Graceful degradation when sampling unavailable.

**Depends on**: Phase 32 (MCP server), Phase 30 (community detection for summaries).

### Phase 32c: Auto Repository Clone and Mount (US37)

**Goal**: `clone_repo` tool + `RepoFetchDetector` middleware.

Clone GitHub repos on demand, mount into VFS, auto-detect repeated file-by-file fetches. Includes `repo_gc` for cache cleanup.

**Depends on**: Phase 25 (StorageLayout for repos_cache), Phase 32 (MCP server tool exposure).

### Phase 32d: Tool Search Enhancements from Research (US39)

**Goal**: Paper-derived improvements for `ToolSearchIndex` (FR-909–915).

Iterative residual retrieval (ProTIP), tool co-occurrence graph (AutoTool), query intent extraction (Re-Invoke), seen/unseen adaptive scoring (ToolRerank), feedback loop from logs, parameter-type verification.

**Depends on**: Phase 32a (base ToolSearchIndex).

### Phase 33: Remaining Research Features (P2–P3, US29–US32, US34)

**Goal**: Test-guided fault localization, repository memory, dynamic call graph, MCTS search, dataflow retrieval.

These are research-tier features that build on the infrastructure from Phases 25–32. Each can be implemented independently:

- **US29 (SBFL)**: Integrates DAP coverage with semantic search. Depends on Phase 28 (graph) + existing `synwire-dap`.
- **US30 (Repository memory)**: Experience pool using `BaseStore` + StorageLayout. Depends on Phase 25. Global tier for cross-project patterns.
- **US31 (Dynamic call graph)**: Composites LSP goto-definition with semantic search. Depends on Phase 24 (LSP) + Phase 28 (graph).
- **US32 (MCTS)**: New execution strategy. Depends on Phase 28 (graph) for search trajectories.
- **US34 (Dataflow retrieval)**: Depends on Phase 24 (LSP) + Phase 28 (graph).

### Phase 34: Cross-Project Features (P2, US36 global tier)

**Goal**: Global dependency index, cross-project code references, project registry.

**Depends on**: Phase 25 (StorageLayout global tier), Phase 28 (code graph).

- Dependency index from manifest files (`Cargo.toml`, `go.mod`, `package.json`, `pyproject.toml`)
- Cross-project xref graph: when a dependency is also a locally-indexed project, link call sites to definitions
- Project registry tracking all known projects with metadata

### Phase 36: MCP Multi-Server Client + WebSocket Transport (P1, US40)

**Goal**: `MultiServerMcpClient` managing N named MCP servers with simultaneous connect, tool aggregation, health monitoring. WebSocket transport variant.

**New crate**: `synwire-mcp-adapters`

**Key design decisions**:
- Built on `rmcp` SDK — re-exports rmcp types for ergonomics
- Async-only (no sync variants) — aligns with MCP's inherently async nature
- `MultiServerMcpClient` accepts `HashMap<String, Connection>` + callbacks + interceptors + `tool_name_prefix` flag
- WebSocket transport via `tokio-tungstenite`
- `McpClientSession` with guard-based cleanup (RAII drop teardown)
- Cursor-based pagination with 1000-page safeguard on all listing operations
- Server health monitoring with reconnection via `McpLifecycleManager` (from synwire-agent)
- Per-transport and per-tool timeout support

**Depends on**: Phase 19 (existing MCP traits in synwire-core), Phase 24 (existing transports in synwire-agent). Independent of Phases 25–35.

**Files**:
```
crates/synwire-mcp-adapters/src/
├── lib.rs
├── client.rs             # MultiServerMcpClient
├── session.rs            # McpClientSession (guard-scoped)
├── transport/
│   └── websocket.rs      # WebSocket transport
├── pagination.rs         # Cursor-based pagination (1000-page cap)
├── callbacks.rs          # McpCallbacks (Logging, Progress)
└── error.rs              # McpAdapterError
```

### Phase 37: MCP Tool Conversion + Content Mapping (P1, US41–US42)

**Goal**: Bidirectional MCP↔Synwire tool conversion. Resource and prompt retrieval.

**Modifies**: `synwire-mcp-adapters`

**Key design decisions**:
- `convert_mcp_tool_to_synwire_tool()`: MCP tool → Synwire `Tool` with annotations as metadata
- `to_mcp_tool()`: Synwire tool → MCP tool, validates args_schema, rejects injected args
- Content type mapping: Text/Image/ResourceLink/EmbeddedResource direct; AudioContent → UnsupportedContent
- `isError` flag → `ToolException`
- Resource loading: static only, dynamic excluded
- Prompt conversion: role-based mapping, multi-content support

**Depends on**: Phase 36 (MultiServerMcpClient for server access).

**Files**:
```
crates/synwire-mcp-adapters/src/convert/
├── tool.rs               # MCP↔Synwire tool conversion
├── content.rs            # Content type mapping
├── resource.rs           # get_resources(), convert_mcp_resource_to_blob()
└── prompt.rs             # get_prompt(), convert_mcp_prompt_message()
```

### Phase 38: Tool Call Interceptors + JSON Schema Validation (P2, US43)

**Goal**: Composable onion/middleware interceptor chain around MCP tool calls. Client-side schema validation.

**Modifies**: `synwire-mcp-adapters`

**Key design decisions**:
- `ToolCallInterceptor` trait with `McpToolCallRequest` → `McpToolCallResult`
- Onion ordering: outer interceptors wrap inner; short-circuit supported
- Panic-safe via `catch_unwind` at each interceptor boundary
- JSON Schema validation before invocation using `jsonschema` crate
- Validation rejects malformed arguments without network round-trip

**Depends on**: Phase 36 (client infrastructure).

**Files**:
```
crates/synwire-mcp-adapters/src/
├── interceptor.rs        # ToolCallInterceptor trait + chain executor
└── validation.rs         # JSON Schema validation
```

### Phase 39: Tool Provider Abstraction + Classification (P1, US44, US46)

**Goal**: `ToolProvider` trait with Static/MCP/Composite implementations. `ToolCategory`, `ToolKind`, `ToolContentType`.

**Modifies**: `synwire-core/src/tools/`, `synwire-mcp-adapters`

**Key design decisions**:
- `ToolProvider` trait in `synwire-core` (not adapter-specific)
- `StaticToolProvider`: fixed set from `Vec<Box<dyn Tool>>`
- `McpToolProvider`: delegates to `MultiServerMcpClient` (in synwire-mcp-adapters)
- `CompositeToolProvider`: aggregates, configurable name collision resolution
- `ToolCategory` enum: Builtin, Custom, Mcp, Remote, WorkflowAsTool
- `ToolKind` enum: read, edit, search, execute, other (for permission UIs)
- `ToolContentType` enum: Text, Image, File, Json (on `ToolOutput`)

**Depends on**: Phase 36 (MultiServerMcpClient for McpToolProvider).

**Files**:
```
crates/synwire-core/src/tools/
├── provider.rs           # ToolProvider trait + StaticToolProvider + CompositeToolProvider
├── category.rs           # ToolCategory, ToolKind
└── content_type.rs       # ToolContentType (on ToolOutput)

crates/synwire-mcp-adapters/src/
└── provider.rs           # McpToolProvider
```

### Phase 40: Tool Operational Controls (P2, US45)

**Goal**: Per-tool timeout, usage limits, enablement predicates, name validation, result truncation, argument validation.

**Modifies**: `synwire-core/src/tools/`, `synwire-orchestrator/src/prebuilt/tool_node.rs`

**Key design decisions**:
- `ToolConfig` struct: timeout, timeout_behavior, is_enabled, max_usage_count, max_result_size
- Name validation regex `^[a-zA-Z0-9_-]{1,64}$` enforced at `Tool` construction
- `ToolNode` truncation at max_result_size (default 100 KB)
- Argument validation via JSON Schema before invocation
- Usage counting per-session (resets on new session)
- `timeout_behavior`: ReturnError (default) or RaiseException

**Depends on**: Phase 39 (tool types).

### Phase 41: `#[tool]` Proc-Macro Enhancements (P1, US47)

**Goal**: Generate full `Tool` impl from async fn, including name, description, JSON Schema, invocation wrapper.

**Modifies**: `synwire-derive/src/tool.rs`

**Key design decisions**:
- Derive tool name from function name (snake_case)
- `#[tool(description = "...")]` attribute for description
- JSON Schema from parameter types via `schemars`
- Return type must be `Result<ToolOutput, ToolError>`
- Generates `name()`, `description()`, `args_schema()`, `run()` methods
- Sets `ToolCategory::Custom` and default `ToolKind::other` (overridable via `#[tool(kind = "edit")]`)

**Depends on**: Phase 39 (ToolCategory, ToolKind).

### Phase 42: Compiled Graph as Tool (P2, US48)

**Goal**: `CompiledGraph::as_tool()` for graph-in-graph composition. Graph-as-node.

**Modifies**: `synwire-orchestrator/src/`

**Key design decisions**:
- `as_tool()` returns `Box<dyn Tool>` wrapping the graph
- Input: graph's input state (serialised as JSON)
- Output: graph's output state (serialised as `ToolOutput`)
- Graph-as-node: `CompiledGraph` implements a trait allowing it to be used as a `StateGraph` node
- Inner graph errors propagate as tool errors with full context
- Inner graph maintains its own checkpoint state independently

**Depends on**: Phase 39 (ToolCategory::WorkflowAsTool).

---

## Phase Dependencies (Expanded)

```
Phase 25 (StorageLayout + Daemon) ─── BLOCKS ALL BELOW ───
    │
    ├── Phase 26 (Per-method chunking + skeletons)
    │       └── Phase 27 (Hierarchical narrowing)
    │
    ├── Phase 28 (Code graph)
    │       ├── Phase 29 (Hybrid BM25+vector)
    │       ├── Phase 30 (Community detection)
    │       │       └── Phase 32b (MCP sampling — needs communities)
    │       ├── Phase 33 (Research features)
    │       └── Phase 34 (Cross-project)
    │
    ├── Phase 31 (Agent skills)
    │
    ├── Phase 32 (MCP server binary) ── incremental, ships v0.1 early
    │       ├── Phase 32a (Tool search)
    │       │       └── Phase 32d (Tool search enhancements)
    │       └── Phase 32c (Clone repo)
    │
    └── Phase 35 (Documentation) ── after all feature phases

Phase 19 (existing MCP traits) ─── INDEPENDENT PATH ───
    │
    └── Phase 36 (MultiServerMcpClient + WebSocket)
            ├── Phase 37 (Tool conversion + content mapping)
            │       └── Phase 38 (Interceptors + schema validation)
            └── Phase 39 (ToolProvider + classification) ── modifies synwire-core
                    ├── Phase 40 (Operational controls)
                    ├── Phase 41 (#[tool] proc-macro enhancements)
                    └── Phase 42 (CompiledGraph::as_tool)
```

**Note**: Phases 36–42 (MCP Adapters) are independent of Phases 25–35 (Storage, Search, Skills). They can run in parallel.

## Implementation Strategy (Expanded)

### Priority Order

**Track A** (Storage + Search + Features — sequential, blocks on Phase 25):
1. **Phase 25** (StorageLayout + Daemon) — foundational, blocks everything in track A
2. **Phase 32 v0.1** (MCP server with existing VFS+grep+index) — immediate value
3. **Phase 32a** (Tool search) — prevents context explosion with 40+ tools
4. **Phase 26** (per-method chunking + skeletons) — biggest search quality win
5. **Phase 27** (hierarchical narrowing) — cheap, high-impact localization
6. **Phase 28** (code graph) — enables graph features
7. **Phase 29** (hybrid search) — complements semantic search
8. **Phase 31** (agent skills) — extensibility
9. **Phase 32c** (clone repo) — workflow friction removal
10. **Phase 30** (community detection) — GraphRAG
11. **Phase 32b** (MCP sampling) — enables community summaries + narrowing ranking
12. **Phase 32d** (tool search enhancements) — paper-derived improvements
13. **Phase 33** (research features) — incremental value
14. **Phase 34** (cross-project) — cross-cutting
15. **Phase 35** (documentation) — after all features ship

**Track B** (MCP Adapters — independent, can start immediately):
1. **Phase 36** (MultiServerMcpClient + WebSocket) — core MCP client
2. **Phase 37** (Tool conversion + content mapping) — bidirectional interop
3. **Phase 38** (Interceptors + schema validation) — middleware layer
4. **Phase 39** (ToolProvider + classification) — tool discovery abstraction
5. **Phase 40** (Operational controls) — production guardrails
6. **Phase 41** (#[tool] proc-macro) — developer ergonomics
7. **Phase 42** (CompiledGraph::as_tool) — graph composition

### Parallel Opportunities

After Phase 25:
- **Agent A**: Phase 26 + 27 (chunking pipeline)
- **Agent B**: Phase 28 + 29 (graph + hybrid search)
- **Agent C**: Phase 31 (agent skills — independent crate)
- **Agent D**: Phase 32 (MCP server — incremental integration)

Independent of Phase 25 (can start now):
- **Agent E**: Phase 36 → 37 → 38 (MCP adapters client + conversion + interceptors)
- **Agent F**: Phase 39 → 40 → 41 → 42 (ToolProvider + controls + macro + graph-as-tool)

### Incremental MCP Server Releases

| Version | Includes |
|---------|----------|
| v0.1 | VFS tools + grep + semantic search (existing) |
| v0.2 | + per-method chunking + skeletons + hierarchical narrowing |
| v0.3 | + code graph + hybrid search |
| v0.4 | + agent skills |
| v0.5 | + community detection + cross-project |
| v1.0 | + research features (SBFL, MCTS, dataflow) |
