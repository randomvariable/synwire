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

## Crate Map

| Crate | Contents |
|---|---|
| `synwire-core` | All public traits (`AgentNode`, `ExecutionStrategy`, `Vfs`, `Middleware`, `Plugin`, `SessionManager`, `McpTransport`, `DirectiveExecutor`, `DirectiveFilter`, `SignalRouter`) |
| `synwire-agent` | Concrete implementations (`FsmStrategy`, `DirectStrategy`, `LocalProvider`, all middleware, `InMemorySessionManager`, MCP transports) |
| `synwire` | Re-exports: `use synwire::agent::prelude::*` for convenience |
| `synwire-test-utils` | Test helpers: `RecordingExecutor`, proptest strategies, VFS conformance suite |
