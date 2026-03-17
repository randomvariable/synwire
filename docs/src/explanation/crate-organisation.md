# Crate Organisation

Synwire is organised as a Cargo workspace with focused, single-responsibility crates.

## Workspace structure

```text
crates/
  synwire-core/              Core traits and types (zero Synwire deps)
  synwire-orchestrator/      Graph execution engine (depends on core)
  synwire-checkpoint/        Checkpoint traits + in-memory impl
  synwire-checkpoint-sqlite/ SQLite checkpoint backend
  synwire-llm-openai/        OpenAI provider
  synwire-llm-ollama/        Ollama provider
  synwire-derive/            Proc macros (#[tool], #[derive(State)])
  synwire-test-utils/        Fake models, proptest strategies, fixtures
  synwire/                   Re-exports, caches, text splitters, prompts
  synwire-agent/             Agent runtime (VFS, middleware, strategies, MCP, sessions)
  synwire-chunker/           Tree-sitter AST-aware code chunking (14 languages)
  synwire-embeddings-local/  Local embedding + reranking via fastembed-rs
  synwire-vectorstore-lancedb/ LanceDB vector store
  synwire-index/             Semantic indexing pipeline (walk→chunk→embed→store)
  synwire-storage/           StorageLayout, RepoId/WorktreeId
  synwire-agent-skills/      Agent skills (agentskills.io spec, Lua/Rhai/WASM)
  synwire-lsp/               LSP client (language server integration)
  synwire-dap/               DAP client (debug adapter integration)
  synwire-sandbox/           Process sandboxing
  synwire-mcp-server/        Standalone MCP server binary (stdio transport)
```

## Design rationale

### Why separate crates?

1. **Compile time**: users only compile what they use. An Ollama-only project does not compile OpenAI code.
2. **Dependency isolation**: `synwire-core` has minimal dependencies. Provider crates add `reqwest`, `eventsource-stream`, etc.
3. **Feature flag surface**: each crate has independent feature flags rather than one mega-crate with dozens of flags.
4. **Clear API boundaries**: traits in `synwire-core` cannot depend on implementations in provider crates.

### Dependency graph

```mermaid
graph TD
    core[synwire-core]
    orch[synwire-orchestrator]
    ckpt[synwire-checkpoint]
    sqlite[synwire-checkpoint-sqlite]
    openai[synwire-llm-openai]
    ollama[synwire-llm-ollama]
    derive[synwire-derive]
    test[synwire-test-utils]
    umbrella[synwire]
    agent[synwire-agent]
    chunker[synwire-chunker]
    emb[synwire-embeddings-local]
    lance[synwire-vectorstore-lancedb]
    idx[synwire-index]
    storage[synwire-storage]
    skills[synwire-agent-skills]
    lsp[synwire-lsp]
    dap[synwire-dap]
    sandbox[synwire-sandbox]
    mcp[synwire-mcp-server]

    core --> orch
    core --> ckpt
    core --> openai
    core --> ollama
    core --> derive
    core --> umbrella
    ckpt --> sqlite
    core --> test
    ckpt --> test
    orch --> test
    core --> agent
    core --> chunker
    core --> emb
    core --> lance
    chunker --> idx
    emb --> idx
    lance --> idx
    storage --> idx
    storage --> agent
    storage --> mcp
    agent --> mcp
    idx --> mcp
    skills --> mcp
    lsp --> mcp
    dap --> mcp
    sandbox --> agent
```

### synwire-core

The foundation crate. Defines all core traits (`BaseChatModel`, `Embeddings`, `VectorStore`, `Tool`, `RunnableCore`, `OutputParser`, `CallbackHandler`), error types, message types, and credentials. Has zero dependencies on other Synwire crates.

### synwire-orchestrator

Graph-based orchestration. Depends on `synwire-core` for trait definitions. Contains `StateGraph`, `CompiledGraph`, channels, prebuilt agents (ReAct), and the Pregel execution engine.

### synwire-checkpoint

Checkpoint abstraction layer. Defines `BaseCheckpointSaver` and `BaseStore` traits, plus an `InMemoryCheckpointSaver` for testing.

### synwire-checkpoint-sqlite

Concrete checkpoint backend using SQLite via `rusqlite` + `r2d2` connection pooling.

### Provider crates

`synwire-llm-openai` and `synwire-llm-ollama` implement `BaseChatModel` and `Embeddings` for their respective APIs. They depend on `synwire-core` and HTTP-related crates.

### synwire-derive

Procedural macro crate. Must be a separate crate due to Rust's proc-macro rules. Depends on `syn`, `quote`, `proc-macro2`.

### synwire-test-utils

Shared test infrastructure: `FakeChatModel` (also in core for convenience), `FakeEmbeddings`, proptest strategies for all core types, and fixture builders.

### synwire (umbrella)

Convenience crate that re-exports core and optionally includes provider crates via feature flags (`openai`, `ollama`). Also provides higher-level utilities: embedding cache, chat history, few-shot prompts, text splitters.

## Choosing which crates to depend on

For most applications, depend on the umbrella `synwire` crate with the required feature flags:

```toml
[dependencies]
synwire = { version = "0.1", features = ["openai"] }
tokio = { version = "1", features = ["full"] }
```

This gives you a single import path covering the most commonly needed types:

```rust,no_run
use synwire::agent::prelude::*;
use synwire_llm_openai::ChatOpenAI;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let model = ChatOpenAI::builder()
        .model("gpt-4o")
        .api_key_env("OPENAI_API_KEY")
        .build()?;

    // Runner, AgentNode, Directive, AgentError etc. come from synwire::agent::prelude
    Ok(())
}
```

For **publishable extension crates** (custom backends, providers, or strategies), depend on `synwire-core` only. This avoids pulling in concrete implementations your users may not need:

```toml
[dependencies]
# Publishable extension crate: traits only, no implementations
synwire-core = "0.1"
```

```rust,no_run
use synwire_core::language_models::chat::BaseChatModel;

// A custom provider that implements BaseChatModel from synwire-core.
// Downstream applications can mix it with any backend or strategy
// from synwire-agent without a version coupling.
pub struct MyCustomChatModel {
    // model configuration
}
```

The rule of thumb: **applications use `synwire`; libraries use `synwire-core`**.
