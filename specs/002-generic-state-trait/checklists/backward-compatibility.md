# Checklist: Backward Compatibility

**Purpose**: Verify RunnableCore unchanged, ValueState works, existing tests pass
**Feature**: [spec.md](../spec.md)
**Requirements**: FR-S14, SC-S06, SC-S10

## RunnableCore Unchanged (FR-S14)

- [ ] `RunnableCore` trait signature is identical to pre-refactor
- [ ] `RunnableCore::invoke` still takes/returns `serde_json::Value`
- [ ] `RunnableCore::batch` still takes/returns `Vec<serde_json::Value>`
- [ ] `RunnableCore::stream` still takes/returns streams of `serde_json::Value`
- [ ] All existing `RunnableCore` implementations in synwire-core compile without modification
- [ ] All existing `RunnableCore` implementations in synwire-llm-openai compile without modification
- [ ] All existing `RunnableCore` implementations in synwire-llm-ollama compile without modification

## ValueState Migration

- [ ] `ValueState` struct wraps `serde_json::Value`
- [ ] `ValueState` implements `State`
- [ ] `From<serde_json::Value> for ValueState` implemented
- [ ] `From<ValueState> for serde_json::Value` implemented
- [ ] `ValueState` is `Send + Sync + Clone`
- [ ] Existing Value-based test code compiles with `ValueState` wrapper

## Zero Regressions (SC-S06)

- [ ] `cargo test --workspace` passes with zero failures
- [ ] No test was deleted — all existing tests are migrated or preserved
- [ ] No public API was removed — only signatures changed (Value → S / ValueState)

## Notes

- Tasks: T002, T003, T014, T067, T070
