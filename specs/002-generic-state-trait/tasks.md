# Tasks: Generic State Trait

**Input**: Design documents from `/specs/002-generic-state-trait/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/traits.md

**Tests**: Constitution requires BDD Test-First (Principle IV, NON-NEGOTIABLE). Tests are included per phase.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Error type extension and backward-compatibility type needed before any generics work

- [x] T001 Add `DeserializationError { field: String, message: String }` variant to `GraphError` in crates/synwire-orchestrator/src/error.rs
- [x] T002 Implement `ValueState` wrapper struct with `State` impl, `From<Value>`, and `From<ValueState>` in crates/synwire-orchestrator/src/graph/value_state.rs
- [x] T003 Re-export `ValueState` from crates/synwire-orchestrator/src/graph/mod.rs

---

## Phase 2: Foundational (State Trait + Generic Graph)

**Purpose**: Define the `State` trait and make `StateGraph<S>` / `CompiledGraph<S>` generic. MUST complete before any user story.

**CRITICAL**: No user story work can begin until this phase is complete.

- [x] T004 [P] Write failing test: State trait with `channels()` and `from_channels()` methods compiles for a test struct in crates/synwire-orchestrator/src/graph/state.rs
- [x] T005 [P] Write failing test: `StateGraph<TestState>::new()` accepts typed `NodeFn<TestState>` in crates/synwire-orchestrator/src/graph/state.rs
- [x] T006 [P] Write failing test: `CompiledGraph<TestState>::invoke(state)` accepts and returns `TestState` in crates/synwire-orchestrator/src/graph/compiled.rs
- [x] T007 Define `State` trait with supertraits `Send + Sync + Clone + Serialize + DeserializeOwned + 'static`, required methods `channels()` and `from_channels()`, default methods `to_value()` and `from_value()` per contracts/traits.md in crates/synwire-orchestrator/src/graph/state.rs
- [x] T008 Define `NodeFn<S>` and `ConditionFn<S>` generic type aliases replacing current `NodeFn` and `ConditionFn` in crates/synwire-orchestrator/src/graph/state.rs
- [x] T009 Make `StateGraph<S: State>` generic — update struct definition, `add_node`, `add_edge`, `add_conditional_edges`, `set_entry_point`, `set_finish_point`, `compile` in crates/synwire-orchestrator/src/graph/state.rs
- [x] T010 Make `CompiledGraph<S: State>` generic — update struct definition, `new`, `invoke`, `with_recursion_limit`, `entry_point`, `node_names`, `Debug` impl in crates/synwire-orchestrator/src/graph/compiled.rs
- [x] T011 Ensure `to_mermaid()` remains state-type-independent on `CompiledGraph<S>` in crates/synwire-orchestrator/src/graph/compiled.rs
- [x] T012 Update `StateGraph` and `CompiledGraph` re-exports in crates/synwire-orchestrator/src/graph/mod.rs
- [x] T013 Update crate-level doc example in crates/synwire-orchestrator/src/lib.rs to use `ValueState` or a typed state
- [x] T014 Verify `cargo test` passes for synwire-orchestrator with all existing tests migrated to `ValueState`
- [x] T015 Verify `cargo clippy -- -D warnings` passes for synwire-orchestrator

**Checkpoint**: State trait defined, graph types generic. All existing tests pass via ValueState migration.

---

## Phase 3: User Story 1 — Custom Typed State (Priority: P1) MVP

**Goal**: Developer defines a custom state struct with `#[derive(State)]`, builds a typed graph, and invokes it.

**Independent Test**: Define a `CounterState { counter: i32 }`, build a graph with an increment node, invoke, verify `counter == 1`.

### Tests for User Story 1

- [x] T016 [P] [US1] Write failing test: `#[derive(State)]` on a struct generates `impl State` with correct `channels()` in crates/synwire-derive/src/state.rs
- [x] T017 [P] [US1] Write failing test: `#[derive(State)]` generates `from_channels()` that deserialises channel values into struct fields in crates/synwire-derive/src/state.rs
- [x] T018 [P] [US1] Write failing test: `#[derive(State)]` with `#[reducer(topic)]` maps to Topic channel in crates/synwire-derive/src/state.rs
- [x] T019 [P] [US1] Write failing integration test: build `StateGraph<CounterState>` with increment node, invoke, verify typed result in crates/synwire-orchestrator/src/graph/state.rs

### Implementation for User Story 1

- [x] T020 [US1] Update `#[derive(State)]` to generate `impl State for T` instead of standalone `channels()` method — add `from_channels()` generation per contracts/traits.md in crates/synwire-derive/src/state.rs
- [x] T021 [US1] Update `#[derive(State)]` to handle `Default` for fields when channel has no value (use `unwrap_or_default()`) in crates/synwire-derive/src/state.rs
- [x] T022 [US1] Add doc comments and doc example to `State` trait showing `#[derive(State)]` usage in crates/synwire-orchestrator/src/graph/state.rs
- [x] T023 [US1] Verify all US1 tests pass and `cargo clippy -- -D warnings` is clean

**Checkpoint**: Developer can define typed state, derive State, build and invoke a generic graph.

---

## Phase 4: User Story 2 — MessagesState + ReAct Agent (Priority: P1)

**Goal**: Built-in `MessagesState` works with `create_react_agent` out of the box.

**Independent Test**: `create_react_agent` with FakeChatModel compiles and returns `CompiledGraph<MessagesState>`, invoke with a human message, verify messages accumulate.

### Tests for User Story 2

- [x] T024 [P] [US2] Write failing test: `MessagesState` implements `State` with Topic channel on `messages` in crates/synwire-orchestrator/src/messages/mod.rs
- [x] T025 [P] [US2] Write failing test: `create_react_agent` returns `CompiledGraph<MessagesState>` in crates/synwire-orchestrator/src/prebuilt/react_agent.rs
- [x] T026 [P] [US2] Write failing test: `ToolNode::into_node_fn::<MessagesState>()` compiles and executes tool calls in crates/synwire-orchestrator/src/prebuilt/tool_node.rs
- [x] T027 [P] [US2] Write failing test: ReAct agent with no tool calls terminates and returns `MessagesState` with conversation history in crates/synwire-orchestrator/src/prebuilt/react_agent.rs

### Implementation for User Story 2

- [x] T028 [US2] Create `MessagesState` struct with `#[derive(State)]` and `#[reducer(topic)]` on `messages: Vec<Message>` in crates/synwire-orchestrator/src/messages/mod.rs
- [x] T029 [US2] Add `messages` module to crates/synwire-orchestrator/src/lib.rs
- [x] T030 [US2] Update `ToolNode::into_node_fn` to be generic `into_node_fn<S: State>` — extract messages from `S` via serde, inject tool messages back via serde in crates/synwire-orchestrator/src/prebuilt/tool_node.rs
- [x] T031 [US2] Update `ToolNode::invoke` to accept `S` and return `S` (using `to_value`/`from_value` internally) in crates/synwire-orchestrator/src/prebuilt/tool_node.rs
- [x] T032 [US2] Update `tools_condition` to be generic `tools_condition<S: State>(state: &S) -> String` using serde to inspect messages in crates/synwire-orchestrator/src/prebuilt/react_agent.rs
- [x] T033 [US2] Update `create_react_agent` to return `CompiledGraph<MessagesState>` — agent node and tools node use `MessagesState` directly in crates/synwire-orchestrator/src/prebuilt/react_agent.rs
- [x] T034 [US2] Update all existing `react_agent` and `tool_node` tests to use `MessagesState` or `ValueState` in crates/synwire-orchestrator/src/prebuilt/
- [x] T035 [US2] Verify all US2 tests pass and `cargo clippy -- -D warnings` is clean

**Checkpoint**: `create_react_agent` works with typed `MessagesState`. Chat agents accumulate conversation history.

---

## Phase 5: User Story 3 — Conditional Routing on Typed State (Priority: P2)

**Goal**: Conditional edges branch on typed state fields, not JSON values.

**Independent Test**: Build graph with `|s: &MyState| if s.done { END } else { "loop" }`, verify correct routing for both values.

### Tests for User Story 3

- [x] T036 [P] [US3] Write failing test: conditional edge with typed `ConditionFn<CounterState>` routes to correct branch in crates/synwire-orchestrator/src/graph/compiled.rs
- [x] T037 [P] [US3] Write failing test: multi-branch conditional edge with 3+ branches on typed state in crates/synwire-orchestrator/src/graph/compiled.rs
- [x] T038 [P] [US3] Write failing test: looping graph with condition `state.counter < 3` terminates after 3 iterations in crates/synwire-orchestrator/src/graph/compiled.rs

### Implementation for User Story 3

- [x] T039 [US3] Verify `ConditionFn<S>` works in `add_conditional_edges` (should work from Phase 2 generics — this task confirms and adds doc examples) in crates/synwire-orchestrator/src/graph/state.rs
- [x] T040 [US3] Add doc example showing typed conditional routing in `add_conditional_edges` documentation in crates/synwire-orchestrator/src/graph/state.rs
- [x] T041 [US3] Verify all US3 tests pass

**Checkpoint**: Conditional routing works on typed state fields with zero JSON inspection.

---

## Phase 6: User Story 4 — Checkpoint Round-Trip (Priority: P2)

**Goal**: Typed state serialises to Value for checkpointing and deserialises back on resume.

**Independent Test**: Checkpoint a `MyState`, restore from checkpoint, verify equality via `PartialEq`.

### Tests for User Story 4

- [x] T042 [P] [US4] Write failing test: `State::to_value()` serialises and `State::from_value()` deserialises round-trip in crates/synwire-orchestrator/src/graph/state.rs
- [x] T043 [P] [US4] Write failing test: `to_value`/`from_value` round-trip with nested structs and enums in crates/synwire-orchestrator/src/graph/state.rs
- [x] T044 [P] [US4] Write failing test: `from_value` with incompatible JSON returns `GraphError::Checkpoint` in crates/synwire-orchestrator/src/graph/state.rs

### Implementation for User Story 4

- [x] T045 [US4] Verify `to_value` and `from_value` default methods work correctly (should work from Phase 2 — this task confirms with edge cases) in crates/synwire-orchestrator/src/graph/state.rs
- [x] T046 [US4] Add property-based test using proptest: arbitrary `MessagesState` round-trips through `to_value`/`from_value` in crates/synwire-orchestrator/src/graph/state.rs
- [x] T047 [US4] Verify all US4 tests pass

**Checkpoint**: Typed state checkpoints correctly. Existing `BaseCheckpointSaver` works without modification.

---

## Phase 7: User Story 5 — Generic Prebuilt Nodes (Priority: P3)

**Goal**: All prebuilt nodes work with any `S: State`, not just `serde_json::Value`.

**Independent Test**: `IfElseNode` with custom state type routes correctly. `LoopNode` with custom state iterates correctly.

### Tests for User Story 5

- [x] T048 [P] [US5] Write failing test: `IfElseNode<CounterState>` routes based on typed field in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [x] T049 [P] [US5] Write failing test: `LoopNode<CounterState>` iterates max_iterations times in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [x] T050 [P] [US5] Write failing test: `IterationNode<CounterState>` iterates over typed collection in crates/synwire-orchestrator/src/prebuilt/nodes.rs

### Implementation for User Story 5

- [x] T051 [US5] Make `IfElseNode<S: State>` generic — condition takes `&S`, branches operate on `S` in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [x] T052 [US5] Make `LoopNode<S: State>` generic — body and condition operate on `S` in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [x] T053 [US5] Make `IterationNode<S: State>` generic in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [x] T054 [US5] Make `TemplateTransformNode<S: State>` generic using `StateAccessor` pattern (Fn(&S)->Value / Fn(&mut S, Value)) in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [x] T055 [US5] Make `ListOperatorNode<S: State>` generic using `StateAccessor` pattern in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [x] T056 [US5] Make `VariableAggregatorNode<S: State>` generic using `StateAccessor` pattern in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [x] T057 [US5] Make `HttpRequestNode<S: State>` generic using `StateAccessor` pattern in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [x] T058 [US5] Make `QuestionClassifierNode<S: State>` generic in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [x] T059 [US5] Make `ValidationNode<S: State>` generic in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [x] T060 [US5] Update all existing prebuilt node tests to use typed state in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [x] T061 [US5] Add tests with a second concrete state type to verify generic works with multiple types in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [x] T062 [US5] Verify all US5 tests pass and `cargo clippy -- -D warnings` is clean

**Checkpoint**: All prebuilt nodes are generic. Tests verify with at least 2 concrete state types.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, examples, final validation

- [x] T063 [P] Update doc examples in crates/synwire-orchestrator/src/lib.rs to compile (un-ignore doc tests)
- [x] T064 [P] Update quickstart.md examples to match final API in specs/002-generic-state-trait/quickstart.md
- [x] T065 [P] Add doc comments with examples to `MessagesState` in crates/synwire-orchestrator/src/messages/mod.rs
- [x] T066 [P] Add doc comments with examples to `ValueState` in crates/synwire-orchestrator/src/graph/value_state.rs
- [x] T067 Run full `cargo test --workspace` and verify zero failures
- [x] T068 Run `cargo clippy --workspace -- -D warnings` and verify zero warnings
- [x] T069 Run `cargo doc --no-deps` and verify zero warnings
- [x] T070 Verify `RunnableCore` trait is unchanged — all existing `RunnableCore` impls compile without modification

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Phase 2 — derive macro update
- **US2 (Phase 4)**: Depends on Phase 2 + Phase 3 (needs `#[derive(State)]` working for `MessagesState`)
- **US3 (Phase 5)**: Depends on Phase 2 only — can run in parallel with US1/US2
- **US4 (Phase 6)**: Depends on Phase 2 only — can run in parallel with US1/US2/US3
- **US5 (Phase 7)**: Depends on Phase 2 only — can run in parallel with US1-US4
- **Polish (Phase 8)**: Depends on all user stories complete

### User Story Dependencies

- **US1 (P1)**: Depends on Phase 2. Prerequisite for US2 (derive macro needed).
- **US2 (P1)**: Depends on US1 (needs working `#[derive(State)]`).
- **US3 (P2)**: Independent — depends only on Phase 2.
- **US4 (P2)**: Independent — depends only on Phase 2.
- **US5 (P3)**: Independent — depends only on Phase 2.

### Within Each User Story

- Tests MUST be written and FAIL before implementation (Constitution Principle IV)
- Implementation follows test-first order
- Story complete before checkpoint

### Parallel Opportunities

- Phase 1 tasks T001-T003 can run sequentially (small scope, interdependent)
- Phase 2 tests T004-T006 are parallel (different files)
- US1 tests T016-T019 are parallel
- US2 tests T024-T027 are parallel
- US3 tests T036-T038 are parallel
- US4 tests T042-T044 are parallel
- US5 tests T048-T050 are parallel
- US3, US4, US5 can run in parallel with each other (no inter-story dependencies)
- Phase 8 polish tasks T063-T066 are parallel

---

## Parallel Example: User Story 1

```bash
# Launch all US1 tests in parallel:
Task: "Write failing test: #[derive(State)] generates impl State in crates/synwire-derive/src/state.rs"
Task: "Write failing test: #[derive(State)] generates from_channels() in crates/synwire-derive/src/state.rs"
Task: "Write failing test: #[derive(State)] with #[reducer(topic)] in crates/synwire-derive/src/state.rs"
Task: "Write failing integration test: StateGraph<CounterState> in crates/synwire-orchestrator/src/graph/state.rs"

# Then implementation sequentially:
Task: "Update #[derive(State)] to generate impl State in crates/synwire-derive/src/state.rs"
```

---

## Implementation Strategy

### MVP First (US1 Only)

1. Complete Phase 1: Setup (T001-T003)
2. Complete Phase 2: Foundational (T004-T015) — generic graph types
3. Complete Phase 3: US1 (T016-T023) — derive macro generates State impl
4. **STOP and VALIDATE**: `cargo test`, `cargo clippy`, verify typed graph works
5. This alone delivers the core value: typed state graphs

### Incremental Delivery

1. Phase 1 + 2 → Generic graph infrastructure ready
2. + US1 → Developers can define and use custom typed state (MVP!)
3. + US2 → Chat agents work with MessagesState out of the box
4. + US3 → Conditional routing on typed fields
5. + US4 → Checkpoint round-trip verified
6. + US5 → All prebuilt nodes generic
7. + Polish → Documentation complete, all checks pass

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Constitution Principle IV (BDD Test-First) is NON-NEGOTIABLE — tests before implementation
- `ValueState` in Phase 1 enables incremental migration without breaking existing tests
- `RunnableCore` must NOT change (FR-S14) — verify in T070
