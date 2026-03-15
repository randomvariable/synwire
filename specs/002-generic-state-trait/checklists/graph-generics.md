# Checklist: Generic StateGraph and CompiledGraph

**Purpose**: Verify StateGraph/CompiledGraph are fully generic over S: State
**Feature**: [spec.md](../spec.md)
**Requirements**: FR-S02, FR-S03, FR-S05, FR-S15, FR-S16, FR-S17

## StateGraph\<S: State\>

- [ ] `StateGraph<S>` struct is parameterised by `S: State`
- [ ] `NodeFn<S>` type alias: `Box<dyn Fn(S) -> BoxFuture<Result<S, GraphError>> + Send + Sync>`
- [ ] `ConditionFn<S>` type alias: `Box<dyn Fn(&S) -> String + Send + Sync>`
- [ ] `add_node` accepts `NodeFn<S>` (not `NodeFn`)
- [ ] `add_conditional_edges` accepts `ConditionFn<S>` (not `ConditionFn`)
- [ ] `compile()` returns `Result<CompiledGraph<S>, GraphError>`
- [ ] `set_entry_point` unchanged in signature
- [ ] `set_finish_point` unchanged in signature
- [ ] `add_edge` unchanged in signature

## CompiledGraph\<S: State\>

- [ ] `CompiledGraph<S>` struct is parameterised by `S: State`
- [ ] `invoke(&self, input: S) -> Result<S, GraphError>` — typed input and output
- [ ] `to_mermaid()` is state-type-independent (no `S` in signature)
- [ ] `with_recursion_limit` unchanged in semantics
- [ ] `entry_point()` unchanged
- [ ] `node_names()` unchanged
- [ ] `Debug` impl works without requiring `S: Debug`

## Execution Semantics (FR-S17)

- [ ] Recursion limit enforced (returns `GraphError::RecursionLimit`)
- [ ] Edge validation rejects unknown node targets at compile time
- [ ] Nodes without outgoing edges rejected at compile time
- [ ] Missing entry point rejected at compile time (`GraphError::NoEntryPoint`)
- [ ] `END` sentinel terminates execution

## Send + Sync (FR-S15)

- [ ] `StateGraph<S>` is `Send` (verified by compile test or static assert)
- [ ] `CompiledGraph<S>` is `Send + Sync` (verified by compile test or static assert)

## Notes

- Tasks: T005, T006, T009, T010, T011, T014, T015
