# Custom Channel

Channels control how state is accumulated in graph execution. Synwire provides built-in channels, but you can implement your own.

## Built-in channels

| Channel | Behaviour |
|---------|-----------|
| `LastValue` | Stores the most recent value (overwrites) |
| `Topic` | Appends all values (accumulator) |
| `AnyValue` | Accepts any single value |
| `BinaryOperator` | Combines values with a custom function |
| `NamedBarrier` | Synchronisation barrier |
| `Ephemeral` | Value cleared after each read |

## Implementing BaseChannel

```rust,ignore
use synwire_orchestrator::channels::traits::BaseChannel;
use synwire_orchestrator::error::GraphError;

struct MaxChannel {
    key: String,
    value: Option<serde_json::Value>,
}

impl MaxChannel {
    fn new(key: impl Into<String>) -> Self {
        Self { key: key.into(), value: None }
    }
}

impl BaseChannel for MaxChannel {
    fn key(&self) -> &str { &self.key }

    fn update(&mut self, values: Vec<serde_json::Value>) -> Result<(), GraphError> {
        for v in values {
            match (&self.value, &v) {
                (Some(current), _) if v.as_f64() > current.as_f64() => {
                    self.value = Some(v);
                }
                (None, _) => {
                    self.value = Some(v);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn get(&self) -> Option<&serde_json::Value> { self.value.as_ref() }

    fn checkpoint(&self) -> serde_json::Value {
        self.value.clone().unwrap_or(serde_json::Value::Null)
    }

    fn restore_checkpoint(&mut self, value: serde_json::Value) {
        self.value = Some(value);
    }

    fn consume(&mut self) -> Option<serde_json::Value> { self.value.take() }

    fn is_available(&self) -> bool { self.value.is_some() }
}
```

## Channel requirements

When implementing `BaseChannel`:

- `update` receives a batch of values from a single superstep
- `get` must return the current accumulated value
- `checkpoint`/`restore_checkpoint` enable state persistence
- `consume` takes the value and resets the channel
- Implement `Send + Sync` (required by the trait bound)
