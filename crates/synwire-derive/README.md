# synwire-derive

Procedural macros for Synwire: `#[tool]` to generate `StructuredTool` definitions from async functions, and `#[derive(State)]` to implement the `State` trait for `StateGraph`.

## What this crate provides

- **`#[tool]`** attribute macro — applied to `async fn`, generates a `{fn_name}_tool()` factory returning `StructuredTool`
- **`#[derive(State)]`** derive macro — applied to structs, generates `State` impl for `StateGraph`
- **`#[reducer(topic)]` / `#[reducer(last_value)]`** field attributes — select the channel type per field

## Quick start

```toml
[dependencies]
synwire-derive = "0.1"
synwire-core = "0.1"
schemars = { version = "0.8", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
```

Define a tool with `#[tool]`:

```rust,no_run
use synwire_derive::tool;
use schemars::JsonSchema;
use serde::Deserialize;

/// Search the web for a query and return the top results.
#[tool]
async fn web_search(input: WebSearchInput) -> anyhow::Result<String> {
    // implementation
    Ok(format!("Results for: {}", input.query))
}

#[derive(Deserialize, JsonSchema)]
struct WebSearchInput {
    /// The search query string.
    query: String,
    /// Maximum number of results to return (default: 5).
    max_results: Option<u32>,
}

// Use the generated factory:
// let tool = web_search_tool()?;
// tool.call(serde_json::json!({ "query": "Rust async" })).await?;
```

Define typed graph state with `#[derive(State)]`:

```rust,no_run
use synwire_derive::State;
use serde::{Serialize, Deserialize};

#[derive(State, Clone, Debug, Default, Serialize, Deserialize)]
struct AgentState {
    /// Appended each turn — use for message history, event logs.
    #[reducer(topic)]
    messages: Vec<String>,

    /// Overwritten each turn — use for current node name, flags.
    #[reducer(last_value)]
    current_step: String,
}
```

## When to use macros vs manual implementation

Use the macros in 90% of cases. Implement manually when:
- Your tool parameters are dynamic or variadic
- You need custom JSON Schema validation logic
- Proc-macro diagnostics are unclear (manual `impl Tool` gives better error messages)

## Documentation

- [Derive Macros Getting Started](https://randomvariable.github.io/synwire/getting-started/derive-macros.html)
- [Custom Tool How-To](https://randomvariable.github.io/synwire/how-to/custom-tool.html)
- [Proc-Macros Explanation](https://randomvariable.github.io/synwire/explanation/synwire-derive.html)
- [Full API docs](https://docs.rs/synwire-derive)
- [Synwire documentation](https://randomvariable.github.io/synwire/)
