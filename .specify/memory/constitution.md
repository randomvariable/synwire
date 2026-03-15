<!--
  Sync Impact Report
  ===================================================
  Version change: 0.0.0 → 1.0.0 (MAJOR: initial ratification)
  Modified principles: N/A (initial)
  Added sections: Core Principles (5), Technology Stack, Development Workflow, Governance
  Removed sections: N/A (initial)
  Templates requiring updates:
    - .specify/templates/plan-template.md: ✅ compatible (no changes needed)
    - .specify/templates/spec-template.md: ✅ compatible (no changes needed)
    - .specify/templates/tasks-template.md: ✅ compatible (no changes needed)
  Follow-up TODOs: none
  ===================================================
-->

# LangChain Rust Constitution

## Core Principles

### I. Trait-Based Abstractions

All core abstractions (language models, embeddings, vector stores, retrievers,
document loaders, tools, callbacks) MUST be defined as Rust traits. Concrete
implementations MUST satisfy these traits, enabling provider-agnostic composition.

- Each trait MUST live in the `langchain-core` crate.
- Provider-specific implementations MUST live in separate crates
  (e.g. `langchain-openai`, `langchain-anthropic`).
- Traits MUST use associated types and generic bounds rather than trait objects
  where zero-cost abstraction is achievable. Use `dyn Trait` only at
  composition boundaries where dynamic dispatch is required.

**Rationale**: Rust's trait system is the idiomatic mechanism for polymorphism.
Trait-first design ensures the same pluggable architecture that makes LangChain
Python successful, while preserving compile-time type safety.

### II. API Conceptual Parity

The Rust port MUST maintain conceptual parity with the Python LangChain
abstractions. Module structure, type names, and method semantics SHOULD mirror
the Python equivalents unless Rust idiom demands deviation.

- Core modules MUST map to their Python counterparts: `messages`, `prompts`,
  `language_models`, `embeddings`, `vectorstores`, `documents`,
  `output_parsers`, `runnables`, `tools`, `callbacks`, `retrievers`.
- Deviations from the Python API MUST be documented with rationale in the
  relevant module's doc comments and in the port tracking document.
- The `Runnable` trait MUST support the invoke/batch/stream pattern from
  Python's LCEL (LangChain Expression Language).

**Rationale**: Conceptual parity lowers the barrier for developers moving
between Python and Rust LangChain. It also ensures feature completeness is
measurable against the upstream project.

### III. Safety and Correctness (NON-NEGOTIABLE)

All code MUST leverage Rust's type system and ownership model to prevent
runtime errors at compile time where possible.

- `unsafe` blocks are prohibited unless required by FFI or a performance-
  critical path, and MUST include a `// SAFETY:` comment justifying soundness.
- All public APIs MUST return `Result<T, E>` for fallible operations; panics
  in library code are forbidden.
- Error types MUST use `thiserror` for library errors and provide actionable
  context. Error enums MUST be `#[non_exhaustive]`.
- All public types MUST derive or implement `Debug`. `Clone`, `Send`, and
  `Sync` MUST be derived where semantically correct.

**Rationale**: Rust's safety guarantees are the primary reason to port to Rust.
Compromising on safety negates the value proposition of the port.

### IV. Async-First with Sync Wrappers

All I/O-bound operations (LLM calls, embedding requests, vector store queries,
document loading) MUST be implemented as `async fn` using `tokio`.

- The core crate MUST NOT depend on a specific runtime; use
  `tokio` traits via feature flags.
- Synchronous convenience wrappers MAY be provided via a `blocking` module
  or feature flag, using `tokio::runtime::Runtime::block_on`.
- Streaming responses MUST use `futures::Stream` or `tokio_stream::Stream`.

**Rationale**: LLM applications are inherently I/O-bound. Async-first design
enables concurrent request batching and streaming, which are critical for
production LLM workloads.

### V. Comprehensive Testing

Every public trait, type, and function MUST have corresponding tests.

- Unit tests MUST live in `#[cfg(test)] mod tests` within each module.
- Integration tests that require network or external services MUST live in
  `tests/` and be gated behind feature flags
  (e.g. `#[cfg(feature = "integration-tests")]`).
- Tests MUST be deterministic; use `mockall` or similar for mocking external
  dependencies in unit tests.
- `cargo test` with default features MUST pass without network access.

**Rationale**: A port without tests is an unverified translation. Tests are the
mechanism by which we confirm behavioural parity with the Python original.

## Technology Stack

- **Language**: Rust (latest stable edition, currently 2024)
- **Async runtime**: `tokio` (with runtime-agnostic core traits)
- **Serialization**: `serde` + `serde_json`
- **Error handling**: `thiserror` for library errors, `anyhow` permitted in
  examples and tests only
- **HTTP client**: `reqwest` (with `rustls` TLS backend by default)
- **Testing**: `cargo test`, `mockall` for mocking, `tokio::test` for async
- **Linting**: `clippy` (deny warnings in CI), `rustfmt` for formatting
- **Documentation**: `rustdoc` with examples; `cargo doc --no-deps` MUST
  produce zero warnings
- **Workspace layout**: Cargo workspace with member crates mirroring the
  Python monorepo structure

## Development Workflow

### Code Quality Gates

All pull requests MUST pass the following before merge:

1. `cargo fmt --check` — no formatting violations
2. `cargo clippy -- -D warnings` — no lint warnings
3. `cargo test` — all unit tests pass
4. `cargo doc --no-deps` — documentation builds without warnings
5. No `unsafe` without `// SAFETY:` justification

### Commit Standards

Commits MUST follow Conventional Commits format with a scope:

```text
feat(core): add Runnable trait with invoke/batch/stream
fix(openai): handle rate limit retry with exponential backoff
docs(readme): add quickstart example
```

### Crate Versioning

Each workspace member crate is independently versioned following SemVer:

- **MAJOR**: Breaking trait/API changes
- **MINOR**: New traits, types, or provider crates
- **PATCH**: Bug fixes, documentation, internal refactoring

The `langchain-core` crate version acts as the compatibility baseline.
Provider crates MUST declare a compatible `langchain-core` version range.

## Governance

This constitution is the authoritative reference for architectural decisions
and development standards in the LangChain Rust port. All code contributions
MUST comply with these principles.

- **Amendments** require: (1) a written proposal documenting the change and
  rationale, (2) review and approval, (3) a migration plan if the change
  affects existing code.
- **Versioning** of this constitution follows SemVer: MAJOR for principle
  removals/redefinitions, MINOR for new principles/sections, PATCH for
  clarifications and wording.
- **Compliance review**: Every pull request MUST be checked against the
  principles defined here. Violations MUST be resolved before merge.
- **Guidance**: For day-to-day development guidance beyond this constitution,
  refer to `CLAUDE.md` and `README.md` at the repository root.

**Version**: 1.0.0 | **Ratified**: 2026-03-09 | **Last Amended**: 2026-03-09
