# Synwire Development Guidelines

## Active Technologies

- Rust (stable, edition 2024)
- Dependencies: serde, serde_json, tokio, futures-util, reqwest (rustls), thiserror
- Build: cargo-make (Makefile.toml at workspace root)

## Project Structure

```text
crates/
├── synwire-core/           # Foundational traits
├── synwire-orchestrator/   # Graph execution (StateGraph<S>, CompiledGraph<S>)
├── synwire-checkpoint/     # Checkpoint persistence
├── synwire-checkpoint-sqlite/
├── synwire-llm-openai/     # OpenAI provider
├── synwire-llm-ollama/     # Ollama provider
├── synwire-derive/         # #[tool] and #[derive(State)] proc macros
├── synwire-test-utils/     # Proptest strategies, fixtures
└── synwire/                # Convenience re-exports
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
cargo make nightly     # prop-tests + audit + MSRV check
```

## Code Style

- Rust (stable, edition 2024): Follow standard conventions
- `#![forbid(unsafe_code)]` on core and orchestrator crates
- All public types must be `Send + Sync`
- All fallible operations return `Result<T, E>` — zero panics in library code
- `#[non_exhaustive]` on all enums and config structs

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
