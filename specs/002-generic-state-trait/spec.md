# Feature Specification: Generic State Trait for Graph Execution

**Feature Branch**: `002-generic-state-trait`
**Created**: 2026-03-15
**Status**: Draft
**Input**: User description: "Refactor synwire-orchestrator to introduce the State trait and make StateGraph/CompiledGraph generic over S: State, replacing the current serde_json::Value-based execution model. This is an M1 spec conformance fix required before M2 work can begin."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Define Custom Typed State for a Graph (Priority: P1)

A developer building a graph-based workflow defines their own state struct with typed fields and uses `#[derive(State)]` to generate the trait implementation. The graph builder, node functions, and executor all operate on this typed state — no manual serialisation or casting required.

**Why this priority**: This is the foundational change. Without a working `State` trait and generic `StateGraph<S>`, nothing else in this spec delivers value. Every other story depends on this.

**Independent Test**: Define a struct with `#[derive(State)]`, build a `StateGraph<MyState>`, add a node that reads/writes typed fields, compile and invoke. Verify the output state has correct typed values.

**Acceptance Scenarios**:

1. **Given** a struct annotated with `#[derive(State)]`, **When** a developer creates a `StateGraph<MyState>`, **Then** all node functions accept and return `MyState` (not `serde_json::Value`).
2. **Given** a node function that increments `state.counter`, **When** the graph is invoked with `MyState { counter: 0 }`, **Then** the result is `MyState { counter: 1 }` with no JSON intermediation.
3. **Given** a struct with a field annotated `#[reducer(topic)]`, **When** `#[derive(State)]` is applied, **Then** the generated `State` trait impl maps that field to a `Topic` channel and all other fields to `LastValue` channels.

---

### User Story 2 - Use MessagesState for Chat Agents (Priority: P1)

A developer building a chat agent uses the built-in `MessagesState` type with the `create_react_agent` factory. Message history accumulates via the Topic channel without manual merging.

**Why this priority**: `MessagesState` is the most common state type for LLM agents. The ReAct agent factory must work with it out of the box. Co-equal with US1 because the ReAct agent is the primary entry point for users.

**Independent Test**: Call `create_react_agent` with a model and tools, invoke with a user message, verify the response contains accumulated messages in the state's `messages` field.

**Acceptance Scenarios**:

1. **Given** `MessagesState` with a `messages: Vec<Message>` field using a Topic channel, **When** two nodes each append a message, **Then** both messages appear in the final state in order.
2. **Given** `create_react_agent` configured with `MessagesState`, **When** invoked with a user message, **Then** the agent executes and returns `MessagesState` with the conversation history.
3. **Given** a `ToolNode` operating on `MessagesState`, **When** the agent calls a tool, **Then** the tool result is appended to `messages` as a `ToolMessage`.

---

### User Story 3 - Conditional Routing on Typed State (Priority: P2)

A developer adds conditional edges to a graph that branch based on typed state fields rather than inspecting JSON values. Condition functions receive a reference to the typed state.

**Why this priority**: Conditional routing is core graph functionality, but it's a refinement on US1 — simple linear graphs work without it.

**Independent Test**: Build a graph with a conditional edge that checks `state.should_continue`, route to either a "loop" or "end" node, verify correct routing for both values.

**Acceptance Scenarios**:

1. **Given** a condition function `|state: &MyState| if state.done { "end" } else { "continue" }`, **When** the graph executes with `state.done = false`, **Then** the "continue" branch is taken.
2. **Given** the same graph, **When** invoked with `state.done = true`, **Then** the "end" branch is taken and the graph terminates.

---

### User Story 4 - Checkpoint and Resume with Typed State (Priority: P2)

A developer uses checkpointing to save and restore graph execution. The checkpoint layer serialises typed state to `serde_json::Value` on save and deserialises back to `S` on resume. The checkpoint crate itself does not change.

**Why this priority**: Checkpointing is essential for production use but is a serialisation boundary concern — the checkpoint crate stores `Value` and doesn't need to know about `S`.

**Independent Test**: Run a graph to an interrupt, checkpoint the state, deserialise from the checkpoint, resume execution, verify the final result matches a non-interrupted run.

**Acceptance Scenarios**:

1. **Given** a `CompiledGraph<MyState>` with checkpointing enabled, **When** the graph reaches an interrupt, **Then** the current `MyState` is serialised to `serde_json::Value` and stored via `BaseCheckpointSaver`.
2. **Given** a stored checkpoint, **When** the graph is resumed, **Then** the `Value` is deserialised back to `MyState` and execution continues from the interrupted node.
3. **Given** a state with nested structs and enums, **When** checkpointed and restored, **Then** the round-tripped state equals the original (verified by `PartialEq`).

---

### User Story 5 - Prebuilt Nodes Work with Any State Type (Priority: P3)

Prebuilt control-flow nodes (`IfElseNode`, `LoopNode`, `IterationNode`, etc.) and data-transform nodes work with any `S: State`, not just `serde_json::Value`.

**Why this priority**: These are convenience nodes. They should work with typed state but are not blocking for basic graph functionality.

**Independent Test**: Build a graph using `IfElseNode` with a custom state type, verify it routes correctly based on typed field values.

**Acceptance Scenarios**:

1. **Given** an `IfElseNode` configured with a typed condition on `MyState`, **When** invoked, **Then** it routes to the correct branch based on typed field values.
2. **Given** a `LoopNode` with `max_iterations: 3`, **When** the loop body modifies typed state each iteration, **Then** the final state reflects exactly 3 iterations of modification.

---

### Edge Cases

- What happens when `from_channels` receives channel values that can't deserialise into `S`? Returns `GraphError::DeserializationError` with a descriptive message including the field name and expected type.
- What happens when a `#[derive(State)]` struct has a field type that doesn't implement `Serialize + DeserializeOwned`? Compile-time error from serde's derive — the `State` trait's supertraits enforce this.
- What happens when a checkpoint contains state serialised from an older version of the struct? Returns `GraphError::DeserializationError` — checkpoint migration (FR-324) is a separate concern.
- What happens when `StateGraph::compile` is called with zero nodes? Returns `GraphError::CompileError` (existing behaviour, unchanged).
- What happens when a node function panics? Existing behaviour — panics propagate through the async runtime. The `#![forbid(unsafe_code)]` and no-panic lint policies reduce this risk.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-S01**: System MUST define a `State` trait with supertraits `Send + Sync + Clone + Serialize + DeserializeOwned + 'static` and required methods `channels()` and `from_channels()`.
- **FR-S02**: `StateGraph<S: State>` MUST accept node functions typed as `Fn(S) -> Future<Result<S, GraphError>>` (not `serde_json::Value`).
- **FR-S03**: `CompiledGraph<S: State>` MUST accept and return `S` in its `invoke` method.
- **FR-S04**: `CompiledGraph<S: State>` MUST serialise `S` to `serde_json::Value` at checkpoint boundaries and deserialise back to `S` on resume.
- **FR-S05**: Condition functions MUST receive `&S` (not `&serde_json::Value`) and return a `String` branch name.
- **FR-S06**: `#[derive(State)]` MUST generate `impl State for T` (not a standalone `channels()` method) with correct channel mappings from field annotations.
- **FR-S07**: `#[derive(State)]` MUST generate `from_channels()` that deserialises channel values into the struct's typed fields.
- **FR-S08**: System MUST provide a built-in `MessagesState` type with a `messages: Vec<Message>` field using a Topic channel.
- **FR-S09**: `create_react_agent` MUST work with `MessagesState` as its state type.
- **FR-S10**: `ToolNode` MUST be generic over `S: State` and operate on typed state.
- **FR-S11**: Prebuilt control-flow nodes (`IfElseNode`, `LoopNode`, `IterationNode`) MUST be generic over `S: State`.
- **FR-S12**: Prebuilt data-transform nodes (`TemplateTransformNode`, `ListOperatorNode`, `VariableAggregatorNode`) MUST be generic over `S: State`.
- **FR-S13**: `HttpRequestNode`, `QuestionClassifierNode`, and `ValidationNode` MUST be generic over `S: State`.
- **FR-S14**: `RunnableCore` trait MUST NOT change — it remains `serde_json::Value`-based for heterogeneous chain composability.
- **FR-S15**: All public types in `synwire-orchestrator` MUST remain `Send + Sync`.
- **FR-S16**: `CompiledGraph<S>` MUST provide `to_mermaid()` unchanged (topology-only, state-type-independent).
- **FR-S17**: Graph execution MUST respect the existing recursion limit, edge validation, and error handling semantics.
- **FR-S18**: The `State` trait MUST provide default methods `to_value(&self) -> Result<Value, GraphError>` and `from_value(v: Value) -> Result<Self, GraphError>` using serde for checkpoint serialisation boundaries.

### Key Entities

- **State**: Trait representing typed, serialisable graph state. Defines channel configuration and deserialisation from channel values.
- **StateGraph\<S\>**: Builder for constructing typed state graphs. Parameterised by state type.
- **CompiledGraph\<S\>**: Compiled, executable graph. Runs the superstep loop with typed state, serialises to Value only at checkpoint boundaries.
- **MessagesState**: Built-in state type for chat-based agents. Contains a `messages` field with Topic channel semantics.
- **NodeFn\<S\>**: Type alias for boxed async node functions operating on typed state.
- **ConditionFn\<S\>**: Type alias for boxed condition functions inspecting typed state references.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-S01**: A developer can define a custom state struct, derive `State`, build a graph, and invoke it with typed state in fewer than 15 lines of code.
- **SC-S02**: All node functions receive and return typed state — zero runtime casts or JSON parsing in user-facing graph code.
- **SC-S03**: `create_react_agent` with `MessagesState` produces a working chat agent that accumulates conversation history.
- **SC-S04**: Checkpoint round-trip (serialise typed state to Value, store, load, deserialise back to typed state) preserves all field values.
- **SC-S05**: Conditional edge routing on typed fields works correctly for all branch conditions (verified by test coverage of true/false/multi-branch paths).
- **SC-S06**: All existing orchestrator tests pass with updated signatures (zero regressions).
- **SC-S07**: All prebuilt nodes compile and function with at least two different concrete state types (verified by tests).
- **SC-S08**: `cargo clippy -- -D warnings` passes with zero warnings on the modified crates.
- **SC-S09**: Zero `unsafe` code in `synwire-orchestrator` (enforced by `#![forbid(unsafe_code)]`).
- **SC-S10**: The `RunnableCore` trait signature is unchanged — verified by checking that all existing `RunnableCore` implementations compile without modification.
