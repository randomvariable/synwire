# synwire: The Umbrella Crate

`synwire` is Synwire's convenience re-export crate. It aggregates commonly used types from across the workspace into a single dependency, so application authors can write `synwire = "0.1"` in their `Cargo.toml` and get started without tracking individual crate versions.

## When to use `synwire`

Use `synwire` when you are building an **end-user application** --- a CLI agent, a web service that calls LLMs, a RAG pipeline. It re-exports `synwire-core` and provides ready-made implementations for patterns that most applications need.

If you are writing a **library crate** (a custom vector store, an LLM provider, an embedding backend), depend on `synwire-core` instead. This keeps your dependency footprint minimal and avoids pulling in implementations your users may not need.

## What it provides

Beyond re-exporting `synwire_core as core`, the crate ships several modules of its own:

| Module | Purpose |
|---|---|
| `agent::prelude` | Glob-importable set of agent types: `Agent`, `AgentNode`, `Runner`, `Directive`, `Session`, `AgentEvent`, `Usage` |
| `cache` | Moka-backed embedding cache --- wraps any `Embeddings` impl and deduplicates repeated queries |
| `chat_history` | Chat message history traits and implementations for managing conversation context windows |
| `prompts` | Few-shot prompt templates and example selectors |
| `text_splitters` | Text splitter implementations for chunking documents before embedding |
| `output_parsers` | Additional output parsers beyond those in `synwire-core` |

### Conditional modules

Several heavyweight integrations are gated behind feature flags so they impose zero cost when unused:

| Module | Feature | Re-exports |
|---|---|---|
| `sandbox` | `sandbox` | `synwire-sandbox` --- process isolation, `SandboxedAgent::with_sandbox()`, `ProcessPlugin` |
| `lsp` | `lsp` | `synwire-lsp` --- `LspPlugin`, `LanguageServerRegistry`, go-to-definition, hover, diagnostics |
| `dap` | `dap` | `synwire-dap` --- `DapPlugin`, `DebugAdapterRegistry`, breakpoints, stepping, variable inspection |

## Agent prelude

The `agent::prelude` module is designed for glob import:

```rust,no_run
use synwire::agent::prelude::*;

// Now you have Agent, AgentNode, Runner, Directive, DirectiveResult,
// AgentError, Session, SessionManager, AgentEvent, Usage, etc.
```

This avoids long import lists when writing agent application code while keeping the individual types traceable (they all originate in `synwire-core::agents`).

## Feature flags

| Flag | Enables |
|---|---|
| `openai` | `synwire-llm-openai` (not re-exported as a module, but available as a dependency) |
| `ollama` | `synwire-llm-ollama` |
| `sandbox` | `synwire-sandbox` + the `sandbox` module |
| `lsp` | `synwire-lsp` + the `lsp` module |
| `dap` | `synwire-dap` + the `dap` module |

No features are enabled by default. A minimal `synwire` dependency pulls in only `synwire-core`, `moka`, `serde`, `serde_json`, `regex`, and `tokio`.

## Dependencies

| Crate | Role |
|---|---|
| `synwire-core` | Always present --- trait definitions and shared types |
| `moka` | Concurrent cache for the embedding cache module |
| `serde` / `serde_json` | Serialization for prompt templates and output parsers |
| `regex` | Text splitter pattern matching |
| `tokio` | Async runtime |
| `tracing` | Observability |

## Ecosystem position

```text
Application code
    |
    v
synwire  (re-exports + utilities)
    |
    +-- synwire-core        (traits, shared types)
    +-- synwire-llm-openai  (optional)
    +-- synwire-llm-ollama  (optional)
    +-- synwire-sandbox     (optional)
    +-- synwire-lsp         (optional)
    +-- synwire-dap         (optional)
```

`synwire` sits at the top of the dependency graph. It is the recommended entry point for applications but is never depended on by other workspace crates.

## See also

- [synwire-core: Trait Contract Layer](./synwire-core.md) --- the trait definitions that `synwire` re-exports
- [synwire-agent: Agent Runtime](./synwire-agent.md) --- concrete agent runtime implementations
- [Feature Flags](../reference/feature-flags.md) --- full feature flag reference
