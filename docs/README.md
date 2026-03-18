# Synwire Documentation

Synwire is an async Rust agent runtime. This documentation follows the [Diataxis](https://diataxis.fr/) framework — four distinct types of content for four distinct needs.

## Navigation

| Need | Start here |
|---|---|
| **Learning** — new to Synwire, want to understand it by doing | [Tutorials](src/tutorials/) |
| **Doing** — know the basics, need to accomplish a specific task | [How-To Guides](src/how-to/) |
| **Understanding** — want to know *why* things work the way they do | [Explanation](src/explanation/) |
| **Looking up** — need the exact API signature or field names | [Reference](src/reference/) |

> **mdBook**: All docs are in `docs/src/`. Build with `mdbook build docs/` or browse with `mdbook serve docs/`.

---

## Tutorials — Learning-Oriented

Step-by-step guides that teach core concepts by building working examples.

| Tutorial | What you'll learn |
|---|---|
| [Your First Agent](tutorials/01-first-agent.md) | Building and running a minimal agent |
| [Testing Without Side Effects](tutorials/02-pure-directive-testing.md) | Directive/effect pattern, pure functions, zero-side-effect tests |
| [Execution Strategies](tutorials/03-execution-strategies.md) | `DirectStrategy` and `FsmStrategy`, state machines, guard conditions |
| [Plugin State Isolation](tutorials/04-plugin-state-isolation.md) | Type-safe plugin state, `PluginStateKey`, `PluginHandle` |
| [File and Shell Operations](tutorials/05-vfs-operations.md) | `Vfs`, `MemoryProvider`, `LocalProvider`, grep |

## How-To Guides — Task-Oriented

Goal-focused instructions for specific tasks. Assume you already know the basics.

| Guide | Goal |
|---|---|
| [VFS Providers](how-to/vfs.md) | Use Filesystem, Process, Archive, Pipeline, Composite, and Store VFS providers |
| [Middleware Stack](how-to/middleware.md) | Configure and compose middleware |
| [Approval Gates](how-to/approval-gates.md) | Require human approval for risky operations |
| [Session Management](how-to/session-management.md) | List, resume, fork, rewind, tag sessions |
| [MCP Integration](how-to/mcp-integration.md) | Connect stdio, HTTP, and in-process MCP servers |
| [Signal Routing](how-to/signal-routing.md) | Three-tier signal routing configuration |
| [Permission Modes](how-to/permission-modes.md) | Configure `PermissionMode` and rules |
| [Advanced Search](how-to/grep-search.md) | Grep with context, filtering, and output modes |

## Explanation — Understanding-Oriented

Conceptual discussions of architecture and design decisions.

| Topic | Explains |
|---|---|
| [Directive/Effect Architecture](explanation/directive-effect-architecture.md) | Why agent nodes return pure data instead of executing side effects |
| [Plugin State Isolation](explanation/plugin-state-isolation.md) | How the type system prevents cross-plugin state contamination |
| [Three-Tier Signal Routing](explanation/three-tier-signal-routing.md) | Why signals are routed through three priority levels |
| [Middleware Execution Model](explanation/middleware-execution-model.md) | Ordering, composition, and early termination |
| [FSM Strategy Design](explanation/fsm-strategy-design.md) | How the finite state machine is implemented and why |
| [Crate Architecture](explanation/crate-structure.md) | Layer boundaries: `synwire-core` vs `synwire-agent` vs `synwire` |
| [Error Taxonomy](explanation/error-taxonomy.md) | `AgentError`, `VfsError`, `StrategyError` design |

## Reference — Information-Oriented

Complete, accurate API listings. The primary reference is generated rustdoc:

```sh
cargo doc --open -p synwire-core -p synwire-agent
```

Supplementary reference:
- [Public Traits](reference/traits.md) — every `pub trait` with method signatures
- [Public Types](reference/types.md) — every `pub struct` and `pub enum` with fields/variants
- [Reference Index](reference/README.md) — complete API surface overview

---

## Installing the MCP Server

Pre-built binaries for Linux (amd64/arm64) and macOS (amd64/arm64) are attached to each [GitHub Release](https://github.com/randomvariable/synwire/releases).

**Homebrew (macOS/Linux — recommended)**:

```bash
brew install randomvariable/tap/synwire-mcp-server
```

**Direct download (macOS)**:

```bash
# After downloading and extracting the archive:
xattr -d com.apple.quarantine ./synwire-mcp-server
```

> macOS Sequoia 15.1+ sets a quarantine flag on files downloaded via a browser. Run the `xattr` command above to remove it, or install via Homebrew (which re-signs the binary automatically).

---

## Crate Map

| Crate | Contents |
|---|---|
| `synwire-core` | Foundational traits: `Vfs`, `Tool`, agent types, embeddings, vector stores, MCP transport, sampling |
| `synwire-orchestrator` | Graph execution engine: `StateGraph<S>`, `CompiledGraph<S>` |
| `synwire-derive` | Proc macros: `#[tool]`, `#[derive(State)]` |
| `synwire-checkpoint` | Checkpoint persistence traits + in-memory implementation |
| `synwire-checkpoint-sqlite` | SQLite checkpoint backend (WAL mode, 0600 permissions) |
| `synwire-llm-openai` | OpenAI LLM provider |
| `synwire-llm-ollama` | Ollama LLM provider |
| `synwire-agent` | Agent runtime: VFS providers, middleware, strategies, MCP, sessions |
| `synwire-mcp-adapters` | MCP client adapters: multi-server aggregation, stdio/HTTP/WebSocket transports, tool conversion |
| `synwire-chunker` | Tree-sitter AST-aware code chunking (14 languages) |
| `synwire-embeddings-local` | Local embedding and reranking via fastembed-rs |
| `synwire-vectorstore-lancedb` | LanceDB vector store implementation |
| `synwire-index` | Semantic indexing pipeline: walk → chunk → embed → store; BM25 hybrid search, code graphs |
| `synwire-lsp` | LSP client (12 tools, capability-conditional) |
| `synwire-dap` | DAP debug client (sessions, breakpoints, evaluate) |
| `synwire-sandbox` | Process sandboxing: registry, isolation, output capture, approval gates |
| `synwire-storage` | `StorageLayout`, `RepoId`/`WorktreeId`, migrations |
| `synwire-agent-skills` | Agent skills (agentskills.io spec, Lua/Rhai/WASM runtimes) |
| `synwire-daemon` | Singleton background process per product |
| `synwire-mcp-server` | MCP server binary — stdio proxy to daemon |
| `synwire` | Convenience re-exports |
