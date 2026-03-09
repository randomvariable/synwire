<!--
  Sync Impact Report
  ===================================================
  Version change: 1.1.0 → 2.0.0 (MAJOR: principle removal)
  Modified principles:
    - III → II. Safety and Correctness (renumbered)
    - IV → III. Async-First with Sync Wrappers (renumbered)
    - V → IV. BDD Test-First (renumbered)
    - VI → V. Always Be Linting (renumbered)
    - VII → VI. Diataxis Documentation (renumbered)
  Added sections: none
  Removed sections:
    - II. API Conceptual Parity (removed entirely)
  Templates requiring updates:
    - .specify/templates/plan-template.md: ✅ compatible (no principle-specific refs)
    - .specify/templates/spec-template.md: ✅ compatible (no principle-specific refs)
    - .specify/templates/tasks-template.md: ✅ compatible (no principle-specific refs)
  Follow-up TODOs: none
  ===================================================
-->

# Synwire Constitution

## Core Principles

### I. Trait-Based Abstractions

All core abstractions (language models, embeddings, vector stores, retrievers,
document loaders, tools, callbacks) MUST be defined as Rust traits. Concrete
implementations MUST satisfy these traits, enabling provider-agnostic composition.

- Each trait MUST live in the `synwire-core` crate.
- Provider-specific implementations MUST live in separate crates
  (e.g. `synwire-openai`, `synwire-anthropic`).
- Traits MUST use associated types and generic bounds rather than trait objects
  where zero-cost abstraction is achievable. Use `dyn Trait` only at
  composition boundaries where dynamic dispatch is required.

**Rationale**: Rust's trait system is the idiomatic mechanism for polymorphism.
Trait-first design ensures pluggable architecture while preserving compile-time
type safety.

### II. Safety and Correctness (NON-NEGOTIABLE)

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

**Rationale**: Rust's safety guarantees are the primary reason to use Rust.
Compromising on safety negates the value proposition.

### III. Async-First with Sync Wrappers

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

### IV. BDD Test-First (NON-NEGOTIABLE)

All features MUST be developed using Behaviour-Driven Development with a
strict test-first workflow. No production code may be written until a
failing test exists that describes the desired behaviour.

- The development cycle MUST follow Red-Green-Refactor:
  1. **Red**: Write a failing test that describes expected behaviour.
  2. **Green**: Write the minimum production code to make the test pass.
  3. **Refactor**: Improve structure while keeping tests green.
- Acceptance criteria MUST be expressed as BDD scenarios using
  Given/When/Then structure (in doc comments, test names, or a dedicated
  `.feature` file if the project adopts a BDD framework).
- Unit tests MUST live in `#[cfg(test)] mod tests` within each module.
- Integration tests that require network or external services MUST live in
  `tests/` and be gated behind feature flags
  (e.g. `#[cfg(feature = "integration-tests")]`).
- Tests MUST be deterministic; use `mockall` or similar for mocking external
  dependencies in unit tests.
- `cargo test` with default features MUST pass without network access.
- Test names MUST describe behaviour, not implementation
  (e.g. `test_invoke_returns_error_when_api_key_missing`, not `test_invoke`).

**Rationale**: Test-first development catches design flaws before they become
entrenched. BDD scenarios create a shared vocabulary between specification and
code, ensuring that tests verify user-visible behaviour rather than internal
implementation details.

### V. Always Be Linting (NON-NEGOTIABLE)

All lint and static analysis warnings MUST be resolved immediately. Suppressing
warnings with `#[allow(...)]` or `#[cfg_attr(...)]` is prohibited unless the
suppression includes a justification comment and has been reviewed.

- `cargo clippy -- -D warnings` MUST pass at all times, not just in CI.
- Developers MUST run clippy before every commit; pre-commit hooks SHOULD
  enforce this automatically.
- When clippy or rustc suggests a fix, the fix MUST be applied or the
  suggestion MUST be addressed with a code change — never ignored.
- `rustfmt` MUST be applied to all code; `cargo fmt --check` MUST pass.
- If a lint rule conflicts with a project convention, the resolution is to
  configure clippy (via `clippy.toml` or `#![clippy::...]` at the crate
  root) — not to scatter per-site suppressions.

**Rationale**: Lint warnings are early signals of bugs, non-idiomatic patterns,
or maintenance hazards. Ignoring them creates a broken-windows effect where
developers stop trusting the toolchain's feedback. Fixing warnings immediately
keeps the codebase clean and the signal-to-noise ratio high.

### VI. Diataxis Documentation

All documentation MUST follow the Diataxis framework, organising content into
four distinct modes: tutorials, how-to guides, reference, and explanation.
Documentation MUST be proven correct by tests wherever possible.

- **Tutorials** (learning-oriented): Step-by-step guides for newcomers. Each
  tutorial MUST have a corresponding integration test or example binary that
  exercises the same steps, ensuring the tutorial stays current.
- **How-to guides** (task-oriented): Recipes for specific tasks. Code samples
  in how-to guides MUST be extracted from or validated against compilable
  examples in the `examples/` directory.
- **Reference** (information-oriented): API docs generated by `rustdoc`.
  Every public item MUST have a doc comment. `cargo doc --no-deps` MUST
  produce zero warnings. Doc-tests (`///` examples) MUST compile and pass.
- **Explanation** (understanding-oriented): Architecture decisions, design
  rationale, and conceptual overviews. These live in `docs/` and SHOULD
  reference the relevant code modules.
- Documentation MUST be updated in the same PR as the code it describes.
  A PR that adds or changes public API without updating docs MUST NOT merge.
- Doc-test coverage: Every public function and method SHOULD include at least
  one `///` example that doubles as a compile-time test.

**Rationale**: The Diataxis framework prevents documentation from becoming an
undifferentiated wall of text. Proving documentation in tests eliminates stale
examples — the most common failure mode of technical documentation. When docs
are tested, they serve as both user guidance and regression tests.

## Technology Stack

- **Language**: Rust (latest stable edition, currently 2024)
- **Async runtime**: `tokio` (with runtime-agnostic core traits)
- **Serialization**: `serde` + `serde_json`
- **Error handling**: `thiserror` for library errors, `anyhow` permitted in
  examples and tests only
- **HTTP client**: `reqwest` (with `rustls` TLS backend by default)
- **Testing**: `cargo test`, `mockall` for mocking, `tokio::test` for async,
  BDD scenarios in test names and doc comments
- **Linting**: `clippy` (deny warnings always), `rustfmt` for formatting
- **Documentation**: `rustdoc` with Diataxis structure; `cargo doc --no-deps`
  MUST produce zero warnings; doc-tests MUST compile and pass
- **Workspace layout**: Cargo workspace with member crates

## Development Workflow

### Code Quality Gates

All pull requests MUST pass the following before merge:

1. `cargo fmt --check` — no formatting violations
2. `cargo clippy -- -D warnings` — no lint warnings
3. `cargo test` — all unit and doc-tests pass
4. `cargo doc --no-deps` — documentation builds without warnings
5. No `unsafe` without `// SAFETY:` justification
6. No `#[allow(...)]` without a justification comment
7. BDD test exists for every new behaviour (test-first evidence in commits)
8. Documentation updated for all public API changes (Diataxis compliance)

### Commit Standards

Commits MUST follow Conventional Commits format with a scope:

```text
feat(core): add Runnable trait with invoke/batch/stream
fix(openai): handle rate limit retry with exponential backoff
docs(readme): add quickstart example
test(core): add BDD scenarios for Runnable error handling
```

Test-first commits SHOULD appear as a pair: the test commit (red) followed
by the implementation commit (green), or as a single commit containing both
with the test file changes listed first.

### Crate Versioning

Each workspace member crate is independently versioned following SemVer:

- **MAJOR**: Breaking trait/API changes
- **MINOR**: New traits, types, or provider crates
- **PATCH**: Bug fixes, documentation, internal refactoring

The `synwire-core` crate version acts as the compatibility baseline.
Provider crates MUST declare a compatible `synwire-core` version range.

## Governance

This constitution is the authoritative reference for architectural decisions
and development standards in the Synwire project. All code contributions
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

**Version**: 2.0.0 | **Ratified**: 2026-03-09 | **Last Amended**: 2026-03-09
