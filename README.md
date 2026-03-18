# Synwire

A Rust framework for building LLM-powered applications and agents with full async support and compile-time type safety.

Synwire provides idiomatic Rust implementations of core LLM abstractions — language models, embeddings, vector stores, graph-based orchestration, tools, and more — drawing from LangChain/LangGraph patterns adapted for Rust's type system and ownership model.

## Status

**M1 complete** — core traits, orchestrator with generic typed state, OpenAI and Ollama providers, checkpointing. See the [roadmap](docs/roadmap.md) for upcoming work units.

## Architecture

```text
crates/
├── synwire-core/                  # Foundational traits: Vfs, Tool, agents, embeddings, vector stores, MCP
├── synwire-orchestrator/          # Graph execution: StateGraph<S>, CompiledGraph<S>
├── synwire-checkpoint/            # Checkpoint persistence traits + in-memory impl
├── synwire-checkpoint-sqlite/     # SQLite checkpoint backend (WAL mode)
├── synwire-llm-openai/            # OpenAI provider
├── synwire-llm-ollama/            # Ollama provider
├── synwire-derive/                # Proc macros: #[tool], #[derive(State)]
├── synwire-agent/                 # Agent runtime: VFS providers, middleware, strategies, MCP, sessions
├── synwire-mcp-adapters/          # MCP client: multi-server, stdio/HTTP/WebSocket transports
├── synwire-chunker/               # Tree-sitter AST-aware code chunking (14 languages)
├── synwire-embeddings-local/      # Local embedding + reranking via fastembed-rs
├── synwire-vectorstore-lancedb/   # LanceDB vector store
├── synwire-index/                 # Semantic indexing pipeline: walk → chunk → embed → store
├── synwire-lsp/                   # LSP client (12 tools)
├── synwire-dap/                   # DAP debug client (sessions, breakpoints, evaluate)
├── synwire-sandbox/               # Process sandboxing: isolation, approval gates
├── synwire-storage/               # StorageLayout, RepoId/WorktreeId
├── synwire-agent-skills/          # Agent skills (agentskills.io spec, Lua/Rhai/WASM)
├── synwire-daemon/                # Singleton background process per product
├── synwire-mcp-server/            # MCP server binary — stdio proxy to daemon
├── synwire-test-utils/            # Proptest strategies and test fixtures (not published)
└── synwire/                       # Convenience re-exports
```

### Design Principles

- **Trait-based**: Core abstractions are Rust traits in `synwire-core`, with provider crates supplying concrete implementations
- **Generic typed state**: `StateGraph<S>` and `CompiledGraph<S>` are generic over `S: State` — compile-time type safety, not runtime JSON casting
- **Async-first**: All I/O operations are `async` (tokio), with optional sync wrappers
- **Type-safe**: `Result<T, E>` for all fallible operations, `#![forbid(unsafe_code)]` on core crates, zero panics in library code
- **BDD test-first**: All features developed with test-first workflow using proptest for property-based testing

See [`.specify/memory/constitution.md`](.specify/memory/constitution.md) for the full project constitution.

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

## Prerequisites

- Rust (latest stable, edition 2024)
- [cargo-make](https://github.com/sagiegurari/cargo-make) (`cargo install cargo-make`)
- [cargo-nextest](https://nexte.st/) (`cargo install cargo-nextest`)

## Getting Started

```bash
# Build all crates
cargo build

# Run all Tier 1 CI checks (fmt + clippy + test + doctest + doc)
cargo make ci

# Run tests only
cargo make test

# Auto-format
cargo make fmt-fix
```

## Quick Example

Chat with an LLM:

```rust
use synwire_llm_openai::ChatOpenAI;
use synwire_core::language_models::BaseChatModel;
use synwire_core::messages::Message;

let model = ChatOpenAI::builder()
    .model("gpt-4o")
    .api_key(std::env::var("OPENAI_API_KEY")?)
    .build()?;

let messages = vec![Message::human("What is Rust?")];
let result = model.invoke(&messages, None).await?;
println!("{}", result.message.content());
```

ReAct agent with tools:

```rust
use synwire_orchestrator::prebuilt::create_react_agent_messages;
use synwire_orchestrator::messages::MessagesState;
use synwire_core::messages::Message;
use synwire_core::tools::StructuredTool;

// Define a tool the agent can call
let weather_tool = StructuredTool::builder()
    .name("get_weather")
    .description("Get the current weather for a city")
    .parameters(serde_json::json!({
        "type": "object",
        "properties": { "city": { "type": "string" } },
        "required": ["city"]
    }))
    .func(|input| Box::pin(async move {
        let city = input["city"].as_str().unwrap_or("unknown");
        Ok(synwire_core::tools::ToolOutput {
            content: format!("{city}: 18°C, partly cloudy"),
            artifact: None,
        })
    }))
    .build()?;

let agent = create_react_agent_messages(model, vec![Box::new(weather_tool)])?;
let result = agent.invoke(MessagesState {
    messages: vec![Message::human("What's the weather in London?")],
}).await?;

for msg in &result.messages {
    println!("{msg:?}");
}
```

Graph with typed state — useful for multi-step workflows where each node
reads and writes structured fields instead of raw JSON:

```rust
use synwire_derive::State;
use synwire_orchestrator::graph::StateGraph;
use synwire_orchestrator::constants::END;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, State)]
struct ResearchState {
    query: String,
    #[reducer(topic)]
    sources: Vec<String>,
    summary: String,
}

let mut graph = StateGraph::<ResearchState>::new();
graph.add_node("search", Box::new(|mut state: ResearchState| {
    Box::pin(async move {
        state.sources.push(format!("Result for: {}", state.query));
        Ok(state)
    })
}))?;
graph.add_node("summarise", Box::new(|mut state: ResearchState| {
    Box::pin(async move {
        state.summary = format!("Found {} sources", state.sources.len());
        Ok(state)
    })
}))?;
graph.set_entry_point("search");
graph.add_edge("search", "summarise");
graph.set_finish_point("summarise");

let compiled = graph.compile()?;
let result = compiled.invoke(ResearchState {
    query: "Rust async runtimes".into(),
    sources: vec![],
    summary: String::new(),
}).await?;
assert_eq!(result.summary, "Found 1 sources");
```

## Available `cargo make` Tasks

| Task | Description |
|------|-------------|
| `cargo make ci` | Tier 1: fmt + clippy + test + doctest + doc |
| `cargo make test` | Run tests with nextest |
| `cargo make clippy` | Lint with deny warnings |
| `cargo make fmt` | Check formatting |
| `cargo make fmt-fix` | Auto-format |
| `cargo make doc` | Build docs (deny warnings) |
| `cargo make coverage` | Generate lcov coverage report |
| `cargo make ci-full` | Tier 1 + Tier 2 (includes geiger) |
| `cargo make nightly` | Property tests + audit + MSRV check |

## Documentation

- [Roadmap](docs/roadmap.md) — work units and critical path to AG-UI
- [Architecture](docs/src/explanation/architecture.md) — crate organisation and design decisions
- [Getting Started](docs/src/getting-started/first-chat.md) — tutorials
- [How-To Guides](docs/src/how-to/custom-tool.md) — task-oriented recipes
- [Project Constitution](.specify/memory/constitution.md) — development principles and quality gates

## Contributing

Contributions welcome. Please read the [project constitution](.specify/memory/constitution.md) for coding standards and architectural principles before submitting a PR.

## Licence

Dual-licensed under Apache 2.0 and MIT — see [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT).
