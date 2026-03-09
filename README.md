# Synwire

A Rust framework for building LLM-powered applications and agents with full async support and compile-time type safety.

Synwire provides trait-based abstractions for language models, embeddings, vector stores, document loaders, prompts, runnables, tools, and graph-based agent orchestration — all designed for idiomatic Rust.

## Status

**Early development**

## Architecture

Cargo workspace with modular crates:

```text
synwire/
├── crates/
│   ├── synwire-core/            # Core traits and base abstractions
│   │   ├── messages/            # Chat message types (Human, AI, System, Tool)
│   │   ├── prompts/             # Prompt templates and composition
│   │   ├── language_models/     # BaseLLM, BaseChatModel traits
│   │   ├── embeddings/          # Embedding model trait
│   │   ├── vectorstores/        # Vector store trait
│   │   ├── documents/           # Document types
│   │   ├── runnables/           # Runnable trait (invoke/batch/stream)
│   │   ├── output_parsers/      # Output parsing traits
│   │   ├── tools/               # Tool definition and invocation
│   │   ├── callbacks/           # Callback handler traits
│   │   └── retrievers/          # Retriever trait
│   ├── synwire-orchestrator/    # Graph-based agent orchestration
│   ├── synwire-openai/          # OpenAI provider integration
│   ├── synwire-anthropic/       # Anthropic provider integration
│   └── synwire/                 # High-level convenience re-exports
├── examples/                    # Usage examples
└── Cargo.toml                   # Workspace root
```

### Design Principles

- **Trait-based**: Core abstractions are Rust traits in `synwire-core`, with provider crates supplying concrete implementations
- **Async-first**: All I/O operations are `async` (tokio), with optional sync wrappers
- **Type-safe**: Leverages Rust's type system — `Result<T, E>` for all fallible operations, no panics in library code

See [`.specify/memory/constitution.md`](.specify/memory/constitution.md) for the full project constitution.

## Prerequisites

- Rust (latest stable, edition 2024)
- Cargo

## Getting Started

```bash
# Clone the repository
git clone https://github.com/randomvariable/synwire.git
cd synwire

# Build all crates
cargo build

# Run tests
cargo test

# Build documentation
cargo doc --open --no-deps
```

## Usage

```rust
use synwire_core::prelude::*;

// Example: invoke a chat model (provider crate required)
let model = synwire_openai::ChatOpenAI::new("gpt-4o");
let response = model.invoke("Hello, world!").await?;
println!("{}", response.content());
```

*(API is subject to change during early development.)*

## Contributing

Contributions are welcome. Please read the [project constitution](.specify/memory/constitution.md) for coding standards and architectural principles before submitting a PR.

## Licence

MIT — see [LICENSE](LICENSE) for details.
