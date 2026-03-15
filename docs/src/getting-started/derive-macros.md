# Derive Macros

Synwire provides two procedural macros for ergonomic definitions: `#[tool]` and `#[derive(State)]`.

## `#[tool]` attribute macro

Transforms an annotated async function into a `StructuredTool` factory. The original function is preserved, and a companion `{name}_tool()` function is generated.

### Usage

```rust,ignore
use synwire_derive::tool;
use synwire_core::error::SynwireError;

/// Searches the web for information.
#[tool]
async fn search(query: String) -> Result<String, SynwireError> {
    Ok(format!("Results for: {query}"))
}

// Generated: search_tool() -> Result<StructuredTool, SynwireError>
```

### Parameter type mapping

Function parameters are automatically mapped to JSON Schema types:

| Rust type | JSON Schema type |
|-----------|-----------------|
| `String`, `&str` | `"string"` |
| `i32`, `u64`, etc. | `"integer"` |
| `f32`, `f64` | `"number"` |
| `bool` | `"boolean"` |
| `Vec<T>` | `"array"` |

### Documentation

Doc comments on the function become the tool's description:

```rust,ignore
/// Calculates the sum of two numbers.
#[tool]
async fn add(a: i64, b: i64) -> Result<String, SynwireError> {
    Ok(format!("{}", a + b))
}
// Tool description: "Calculates the sum of two numbers."
```

## `#[derive(State)]` derive macro

Generates channel configuration from struct field annotations for use with `StateGraph`.

### Usage

```rust,ignore
use synwire_derive::State;

#[derive(State)]
struct AgentState {
    /// Messages accumulate via a Topic channel
    #[reducer(topic)]
    messages: Vec<String>,

    /// Current step uses LastValue (default)
    current_step: String,
}

// Generated: AgentState::channels() -> Vec<(String, Box<dyn BaseChannel>)>
```

### Channel types

| Annotation | Channel type | Behaviour |
|-----------|-------------|-----------|
| (none) | `LastValue` | Overwrites with the latest value |
| `#[reducer(topic)]` | `Topic` | Appends values (accumulator) |

### Using with StateGraph

```rust,ignore
use synwire_orchestrator::graph::StateGraph;

let channels = AgentState::channels();
// Use channels when configuring the graph
```

## Combining both macros

A typical agent combines `#[tool]` for tool definitions and `#[derive(State)]` for graph state:

```rust,ignore
use synwire_derive::{tool, State};
use synwire_core::error::SynwireError;

#[derive(State)]
struct MyAgentState {
    #[reducer(topic)]
    messages: Vec<String>,
    result: String,
}

/// Looks up information in a database.
#[tool]
async fn lookup(key: String) -> Result<String, SynwireError> {
    Ok(format!("Value for {key}"))
}
```

## Next steps

- [Custom Tool](../how-to/custom-tool.md) -- more tool patterns
- [Graph Agents](./graph-agent.md) -- use State with graphs
