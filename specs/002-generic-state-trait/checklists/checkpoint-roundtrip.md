# Checklist: Checkpoint Round-Trip

**Purpose**: Verify typed state survives checkpoint serialisation boundary
**Feature**: [spec.md](../spec.md)
**Requirements**: FR-S04, SC-S04

## Serialisation Boundary (FR-S04)

- [ ] `State::to_value()` produces valid `serde_json::Value` for flat structs
- [ ] `State::to_value()` produces valid `serde_json::Value` for nested structs
- [ ] `State::to_value()` produces valid `serde_json::Value` for structs with enums
- [ ] `State::from_value()` reconstructs flat structs correctly
- [ ] `State::from_value()` reconstructs nested structs correctly
- [ ] `State::from_value()` reconstructs structs with enums correctly

## Round-Trip Correctness (SC-S04)

- [ ] `from_value(to_value(state))` equals original state (verified by `PartialEq`)
- [ ] Round-trip preserves `Vec<Message>` field ordering
- [ ] Round-trip preserves `HashMap` field contents
- [ ] Round-trip preserves `Option<T>` field (both Some and None)
- [ ] Round-trip preserves nested struct field values

## Error Cases

- [ ] `from_value` with incompatible JSON returns `GraphError::Checkpoint`
- [ ] `from_value` with missing required field returns `GraphError::Checkpoint`
- [ ] `from_value` with wrong type for field returns `GraphError::Checkpoint`

## Checkpoint Crate Unchanged

- [ ] `BaseCheckpointSaver` trait signature is unchanged
- [ ] `InMemoryCheckpointSaver` compiles without modification
- [ ] `SqliteSaver` compiles without modification
- [ ] Checkpoint conformance tests pass without modification

## Property-Based Verification

- [ ] Proptest: arbitrary `MessagesState` round-trips correctly through `to_value`/`from_value`

## Notes

- Tasks: T042-T047
- The checkpoint crate stores `serde_json::Value` — it never sees `S` directly
- `CompiledGraph<S>` handles the boundary: `S -> to_value() -> checkpoint store -> from_value() -> S`
