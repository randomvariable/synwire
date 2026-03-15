# Checklist: State Trait Contract

**Purpose**: Verify the State trait implementation matches contracts/traits.md
**Feature**: [spec.md](../spec.md)
**Requirements**: FR-S01, FR-S06, FR-S07, FR-S18

## State Trait Definition

- [ ] `State` trait has supertrait `Send`
- [ ] `State` trait has supertrait `Sync`
- [ ] `State` trait has supertrait `Clone`
- [ ] `State` trait has supertrait `Serialize`
- [ ] `State` trait has supertrait `DeserializeOwned`
- [ ] `State` trait has supertrait `'static`
- [ ] `channels()` method returns `Vec<(String, Box<dyn BaseChannel>)>`
- [ ] `from_channels()` method accepts `&HashMap<String, Box<dyn BaseChannel>>`
- [ ] `from_channels()` returns `Result<Self, GraphError>`
- [ ] `to_value()` default method serialises via `serde_json::to_value`
- [ ] `from_value()` default method deserialises via `serde_json::from_value`
- [ ] `to_value()` error maps to `GraphError::Checkpoint`
- [ ] `from_value()` error maps to `GraphError::Checkpoint`

## #[derive(State)] Macro

- [ ] Generates `impl State for T` (not standalone `channels()`)
- [ ] Unannotated fields map to `LastValue` channel
- [ ] `#[reducer(topic)]` fields map to `Topic` channel
- [ ] `from_channels()` deserialises each field from its channel value via `serde_json::from_value`
- [ ] `from_channels()` uses `unwrap_or_default()` when channel has no value
- [ ] `from_channels()` returns `GraphError::DeserializationError` with field name on deserialisation failure
- [ ] Derive macro rejects non-struct types with compile error
- [ ] Derive macro rejects tuple structs with compile error

## Notes

- Verify against contracts/traits.md for exact signatures
- Tasks: T004, T007, T008, T016-T022
