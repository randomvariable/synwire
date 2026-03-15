# Checklist: Generic Prebuilt Nodes

**Purpose**: Verify all prebuilt nodes are generic over S: State
**Feature**: [spec.md](../spec.md)
**Requirements**: FR-S11, FR-S12, FR-S13, SC-S07

## Control-Flow Nodes (FR-S11)

- [ ] `IfElseNode<S: State>` — condition takes `&S`, branches operate on `S`
- [ ] `IfElseNode` routes correctly with typed state field condition
- [ ] `LoopNode<S: State>` — body and condition operate on `S`
- [ ] `LoopNode` respects `max_iterations` with typed state
- [ ] `IterationNode<S: State>` — iterates over typed collection

## Data-Transform Nodes (FR-S12)

- [ ] `TemplateTransformNode<S: State>` — uses StateAccessor pattern
- [ ] `ListOperatorNode<S: State>` — uses StateAccessor pattern
- [ ] `VariableAggregatorNode<S: State>` — uses StateAccessor pattern

## Other Nodes (FR-S13)

- [ ] `HttpRequestNode<S: State>` — uses StateAccessor pattern
- [ ] `QuestionClassifierNode<S: State>` — generic over state
- [ ] `ValidationNode<S: State>` — generic over state

## Multi-Type Verification (SC-S07)

- [ ] At least one control-flow node tested with `CounterState` (custom type)
- [ ] At least one control-flow node tested with `MessagesState` (built-in type)
- [ ] At least one data-transform node tested with two different concrete state types
- [ ] All prebuilt nodes compile with `ValueState` (backward compat)

## Notes

- Tasks: T048-T062
- StateAccessor pattern: `Fn(&S) -> Value` for extract, `Fn(&mut S, Value)` for inject
- Nodes that only need JSON-level access use StateAccessor; nodes that need typed field access use trait bounds directly
