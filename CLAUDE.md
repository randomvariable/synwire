# Synwire Development Guidelines

## Active Technologies
- Rust stable, edition 2024, MSRV 1.85
- Async: tokio 1, futures-core/futures-util 0.3
- Serialization: serde/serde_json 1
- Errors: thiserror 2
- HTTP: reqwest 0.12 (rustls)
- IDs: uuid 1, chrono 0.4
- Observability: tracing 0.1, tracing-subscriber, tracing-appender
- Schema: schemars 0.8 (JSON Schema)
- Caching: moka 0.12
- Storage: SQLite via rusqlite 0.32 (WAL mode for concurrency), LanceDB (vectors), tantivy (BM25, planned)
- Parsing: tree-sitter + language grammars (14 languages)
- Embeddings: fastembed-rs (bge-small-en-v1.5 default, configurable)
- Community detection: hit-leiden (planned)
- Scripting: mlua (Lua), rhai, extism (WASM) — for agent skills (planned)
- Build: cargo-make (Makefile.toml at workspace root)
- Rust stable, edition 2024, MSRV 1.85 (CodeQL supports editions 2021 and 2024) + GitHub Actions runners (`ubuntu-latest`), GitHub Advanced Security (free for public repos) (004-supply-chain-security)
- N/A — this feature produces workflow YAML, TOML/YAML config files, and release SBOM artifacts (004-supply-chain-security)

## Project Structure

```text
crates/
├── synwire-core/              # Foundational traits (Vfs, Tool, agents, embeddings, vectorstores)
├── synwire-orchestrator/      # Graph execution (StateGraph<S>, CompiledGraph<S>)
├── synwire-checkpoint/        # Checkpoint persistence traits + in-memory impl
├── synwire-checkpoint-sqlite/ # SQLite checkpoint backend (WAL mode, 0600 perms)
├── synwire-llm-openai/        # OpenAI provider
├── synwire-llm-ollama/        # Ollama provider
├── synwire-derive/            # #[tool] and #[derive(State)] proc macros
├── synwire-test-utils/        # Proptest strategies, fixtures, conformance suites
├── synwire-agent/             # Agent runtime (VFS providers, middleware, strategies, MCP, sessions)
├── synwire-chunker/           # Tree-sitter AST-aware code chunking (14 languages)
├── synwire-embeddings-local/  # Local embedding + reranking via fastembed-rs
├── synwire-vectorstore-lancedb/ # LanceDB vector store impl
├── synwire-index/             # Semantic indexing pipeline (walk→chunk→embed→store)
├── synwire-lsp/               # LSP client (12 tools, capability-conditional)
├── synwire-dap/               # DAP client (debug sessions, breakpoints, eval)
├── synwire-sandbox/           # Process sandboxing (registry, isolation, output capture)
├── synwire-storage/           # StorageLayout, RepoId/WorktreeId, migrations (planned)
├── synwire-agent-skills/      # Agent skills (agentskills.io spec, Lua/Rhai/WASM) (planned)
├── synwire-daemon/            # Singleton background process per product (planned)
├── synwire-mcp-server/        # MCP server binary — stdio, thin proxy to daemon (planned)
└── synwire/                   # Convenience re-exports
```

## Commands

All commands are defined in `Makefile.toml`. Use `cargo make <task>`:

```text
cargo make ci          # Tier 1: fmt + clippy + test + doctest + doc
cargo make test        # nextest only
cargo make clippy      # lint only
cargo make fmt         # check formatting
cargo make fmt-fix     # auto-format
cargo make doc         # build docs (deny warnings)
cargo make doctest     # doc-tests only
cargo make coverage    # generate lcov coverage report
cargo make ci-full     # Tier 1 + Tier 2 (includes geiger)
cargo make nightly     # prop-tests + audit + osv-scan + MSRV check
cargo make security    # local supply-chain: audit + osv-scan + geiger
cargo make osv-scan    # OSV vulnerability scan (Cargo.lock)
cargo make sbom        # generate SPDX 2.3 JSON SBOM (requires syft)
cargo make codeql      # CodeQL SAST locally (requires codeql CLI)
cargo make scorecard   # OSSF Scorecard locally (requires scorecard CLI + GITHUB_TOKEN)
cargo make updatecli-diff   # preview dependency updates (requires GITHUB_TOKEN)
cargo make updatecli-apply  # apply dependency updates + raise PRs
```

## Code Style

- Rust (stable, edition 2024): Follow standard conventions
- `#![forbid(unsafe_code)]` on core and orchestrator crates
- All public types must be `Send + Sync`
- All fallible operations return `Result<T, E>` — zero panics in library code
- `#[non_exhaustive]` on all enums and config structs
- Tool output to LLMs: plain text, Markdown, TOON, or JSON only. Structured data: TOON or JSON

## Markdown Pitfalls

- Bare `<S>`, `<T>`, `<D>` etc. in Markdown headings or body text render as HTML tags (strikethrough for `<S>`). Always wrap generic type parameters in backticks: `` `DirectiveResult<S>` `` not `DirectiveResult<S>`.

## Architecture (003-agent-core)

### Core Abstractions (synwire-core)
- `Vfs` trait: filesystem-like interface over heterogeneous data sources (was `BackendProtocol`)
- `VfsCapabilities` bitflags: 30 capability flags (was `BackendCapabilities`)
- `MemoryProvider` / `LocalProvider` / `CompositeProvider` / `StoreProvider` (was `StateBackend` / `FilesystemBackend` / `CompositeBackend` / `StoreBackend`)
- ReadGuard: enforces "must read before edit", stale-read detection via watch/check_stale
- Sandbox module: Shell, ProcessManager, ArchiveManager, approval gates (separated from VFS)
- `ToolSearchIndex`: framework-level progressive tool discovery with embedding-based retrieval and namespace grouping
- `SamplingProvider`: trait for tool-internal LLM access (MCP sampling or direct model invocation)

### Daemon Architecture
- `synwire-daemon`: singleton per product, manages all repos/worktrees/clones
- Owns: embedding model, file watchers, indexing pipelines, global tier (registry, deps, xrefs, experience)
- MCP servers connect via Unix domain socket as thin stdio↔UDS proxies
- Auto-launched by first MCP server, 5-min grace period after last client
- No systemd/launchctl — spawned as detached process

### Identity & Storage
- `RepoId`: git first-commit hash (shared across worktrees of same repo)
- `WorktreeId`: RepoId + worktree root path hash (per-branch index)
- `StorageLayout`: product-scoped paths, durable ($DATA) vs cache ($CACHE) split
- Concurrency: SQLite WAL + LanceDB + tantivy native — no external file locks

### Agent Skills
- Follow agentskills.io spec: `SKILL.md` + `scripts/` + `references/` + `assets/`
- Synwire extension: optional `runtime` field (lua, rhai, wasm, tool-sequence, external)
- Discovery: `$DATA/<product>/skills/` (global) + `.<product>/skills/` (project-local)
- Progressive disclosure: name+description at startup, full body on activation

## Spec Location

- Spec: `specs/003-agent-core/spec.md` (39 user stories, 497 FRs, 135 SCs)
- Plan: `specs/003-agent-core/plan.md` (Phases 1-34)
- Tasks: `specs/003-agent-core/tasks.md` (243 tasks, 144 complete)
- Checklists: `specs/003-agent-core/checklists/` (5 checklists, 257 items, all resolved)
- Research: `docs/tempresearch/` (3 research docs on SWE-bench, code localization, tool search)

## Recent Changes
- 004-supply-chain-security: Added Rust stable, edition 2024, MSRV 1.85 (CodeQL supports editions 2021 and 2024) + GitHub Actions runners (`ubuntu-latest`), GitHub Advanced Security (free for public repos)
- 003-agent-core: Expanded spec with 23 new user stories (US16-US39) covering VFS, semantic search, LSP/DAP, code graphs, community detection, agent skills, MCP server, daemon, tool search
- 003-agent-core: Refactored all `Backend*` terminology to `*Provider`/`Vfs` throughout spec
