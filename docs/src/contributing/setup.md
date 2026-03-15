# Contributor Setup

## Prerequisites

- Rust stable (1.85+, edition 2024)
- `cargo-nextest` for test execution
- `cargo-clippy` for linting (included with rustup)

## Clone and build

```sh
git clone https://github.com/randomvariable/langchain-rs.git
cd langchain-rs
cargo build --workspace
```

## Run tests

```sh
# All tests (recommended)
cargo nextest run --workspace --all-features

# Doctests (nextest does not run these)
cargo test --workspace --doc

# Single crate
cargo nextest run -p synwire-core
```

## Linting

```sh
# Clippy with workspace lints (must pass with zero warnings)
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format check
cargo fmt --all -- --check
```

## Documentation

```sh
# Build rustdoc
cargo doc --workspace --no-deps --all-features

# Check for doc warnings
cargo doc --workspace --no-deps 2>&1 | grep -c warning
```

## Workspace lints

The workspace enforces strict lints via `Cargo.toml`:

- `clippy::pedantic` and `clippy::nursery` at warn level
- `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`, `clippy::todo` are denied
- `missing_docs` is denied -- all public items must have doc comments
- `unsafe_code` is denied across all crates

## Adding a new crate

1. Create the crate directory under `crates/`
2. Add it to the workspace `members` in the root `Cargo.toml`
3. Add `[lints] workspace = true` to inherit workspace lints
4. Add `#![deny(unsafe_code)]` or `#![forbid(unsafe_code)]` to `lib.rs`
5. Add `//!` module-level documentation to `lib.rs`

## Pull request checklist

- [ ] `cargo fmt --all` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] `cargo nextest run --workspace --all-features` passes
- [ ] `cargo test --workspace --doc` passes
- [ ] New public items have doc comments
- [ ] No `unsafe` code unless justified and documented
- [ ] No new dependencies without discussion
