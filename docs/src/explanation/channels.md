# Channel System

Channels are the state management mechanism in `synwire-orchestrator`. Each channel manages a single key in the graph state.

## Purpose

In the Pregel execution model, multiple nodes may write to the same state key during a superstep. Channels define how those writes are combined:

- **LastValue**: the most recent write wins (overwrite semantics)
- **Topic**: all writes are appended (accumulator semantics)
- **AnyValue**: accepts any single value
- **BinaryOperator**: combines values with a custom function
- **NamedBarrier**: synchronisation primitive for fan-in patterns
- **Ephemeral**: value is cleared after each read

## BaseChannel trait

```rust,ignore
pub trait BaseChannel: Send + Sync {
    fn key(&self) -> &str;
    fn update(&mut self, values: Vec<serde_json::Value>) -> Result<(), GraphError>;
    fn get(&self) -> Option<&serde_json::Value>;
    fn checkpoint(&self) -> serde_json::Value;
    fn restore_checkpoint(&mut self, value: serde_json::Value);
    fn consume(&mut self) -> Option<serde_json::Value>;
    fn is_available(&self) -> bool;
}
```

## Channel selection guide

| Use case | Channel | Reason |
|----------|---------|--------|
| Single current value | `LastValue` | Overwrites; always has the latest |
| Message history | `Topic` | Appends; preserves full history |
| Intermediate computation | `Ephemeral` | Cleared after read; no state accumulation |
| Custom reduction | `BinaryOperator` | User-defined combine function |
| Fan-in synchronisation | `NamedBarrier` | Waits for all writers |

## Checkpointing interaction

Channels participate in checkpointing via `checkpoint()` and `restore_checkpoint()`. When a graph is paused and resumed, the channel state is serialised to JSON and restored.

## Derive macro integration

The `#[derive(State)]` macro generates channel configuration from struct annotations:

```rust,ignore
#[derive(State)]
struct AgentState {
    #[reducer(topic)]  // -> Topic channel
    messages: Vec<String>,
    current: String,    // -> LastValue channel (default)
}
```

## Error handling

`LastValue` channels return `GraphError::MultipleValues` if they receive more than one value in a single `update` call. `Topic` channels accept any number of values.
