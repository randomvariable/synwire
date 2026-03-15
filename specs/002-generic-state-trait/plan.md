# Implementation Plan: Generic State Trait

**Branch**: `002-generic-state-trait` | **Date**: 2026-03-15 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/002-generic-state-trait/spec.md`

## Summary

Introduce a `State` trait and make `StateGraph<S>` / `CompiledGraph<S>` generic over `S: State`, replacing the current `serde_json::Value`-based graph execution model. This aligns the implementation with the M1 spec (architecture review fix §1.2) and unblocks M2 features (directives, execution strategies, cognitive primitives) that require typed state. The `RunnableCore` trait remains Value-based — only the graph layer changes.

## Technical Context

**Language/Version**: Rust stable, edition 2024
**Primary Dependencies**: serde, serde_json, tokio, futures-util, synwire-core
**Storage**: N/A (checkpoint integration uses existing Value-based `BaseCheckpointSaver`)
**Testing**: cargo test (nextest), proptest for property-based tests
**Target Platform**: Linux, macOS, Windows (library crate, cross-platform)
**Project Type**: Library (Cargo workspace member crate)
**Performance Goals**: Zero additional overhead vs current Value-based execution for the common case. Generic monomorphisation should be faster than dynamic Value dispatch.
**Constraints**: `#![forbid(unsafe_code)]`, all public types `Send + Sync`, zero panics, `cargo clippy -- -D warnings`
**Scale/Scope**: ~24 files in synwire-orchestrator, ~3 files in synwire-derive. No new crates.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
|-----------|--------|----------|
| I. Trait-Based Abstractions | PASS | `State` trait defined in orchestrator crate. Associated types used for channel configuration. |
| II. Safety and Correctness | PASS | `#![forbid(unsafe_code)]`, all `Result<T, E>`, `#[non_exhaustive]` on `GraphError`, `thiserror`-compatible errors. |
| III. Async-First | PASS | Node functions remain async. `NodeFn<S>` returns `BoxFuture<Result<S, GraphError>>`. |
| IV. BDD Test-First | PASS | Every acceptance scenario maps to a test. Tests written before implementation. |
| V. Always Be Linting | PASS | `cargo clippy -- -D warnings` enforced. `cargo fmt --check` enforced. |
| VI. Diataxis Documentation | PASS | Doc comments on all public items. Doc examples updated to use typed state. |

No violations. No complexity tracking needed.

## Project Structure

### Documentation (this feature)

```text
specs/002-generic-state-trait/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── traits.md        # State trait contract
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (repository root)

```text
crates/synwire-orchestrator/src/
├── graph/
│   ├── state.rs         # State trait, StateGraph<S>, NodeFn<S>, ConditionFn<S>
│   └── compiled.rs      # CompiledGraph<S> with typed invoke
├── channels/
│   └── traits.rs        # BaseChannel (unchanged)
├── prebuilt/
│   ├── react_agent.rs   # create_react_agent with MessagesState
│   ├── tool_node.rs     # ToolNode generic over S: State
│   └── nodes.rs         # IfElseNode<S>, LoopNode<S>, etc.
├── messages/
│   └── mod.rs           # MessagesState definition
├── error.rs             # + DeserializationError variant
└── lib.rs               # Updated doc examples

crates/synwire-derive/src/
└── state.rs             # #[derive(State)] generates impl State
```

**Structure Decision**: No new crates. Changes are contained to `synwire-orchestrator` (graph, prebuilt, error, messages modules) and `synwire-derive` (state macro). Existing module layout preserved.
