# synwire-derive: Proc-Macros and When to Use Them

`synwire-derive` provides two proc-macros that eliminate boilerplate for the most common patterns: `#[tool]` for defining tools and `#[derive(State)]` for typed graph state.

> 📖 **Rust note:** The [`#[derive]` attribute](https://doc.rust-lang.org/book/appendix-03-derivable-traits.html) and attribute macros like `#[tool]` are *procedural macros* — Rust code that runs at compile time, reads your source code as input, and outputs new source code. They are zero-cost: the generated code is identical to what you would write by hand.

## `#[tool]`: Defining tools from async functions

Apply `#[tool]` to an `async fn` to generate a `StructuredTool` with an automatically-derived JSON Schema.

```rust,no_run
use synwire_derive::tool;
use schemars::JsonSchema;
use serde::Deserialize;

/// Calculate the area of a rectangle.
/// The tool description is taken from this doc comment.
#[tool]
async fn rectangle_area(input: RectangleInput) -> anyhow::Result<String> {
    let area = input.width * input.height;
    Ok(format!("{area} square units"))
}

#[derive(Deserialize, JsonSchema)]
struct RectangleInput {
    /// Width of the rectangle in units.
    width: f64,
    /// Height of the rectangle in units.
    height: f64,
    /// Unit label (optional, defaults to "m").
    unit: Option<String>,
}

// The macro generates rectangle_area_tool():
// let tool = rectangle_area_tool()?;
// let result = tool.call(serde_json::json!({ "width": 5.0, "height": 3.0 })).await?;
// assert_eq!(result.text(), "15 square units");
```

### How the schema is derived

The macro calls `schemars::JsonSchema` on the input type. This means:
- `String` / `&str` → `"string"`
- Integer types → `"integer"`
- Float types → `"number"`
- `bool` → `"boolean"`
- `Vec<T>` → `"array"`
- `Option<T>` → field is marked not required
- Structs → `"object"` with properties
- `#[schemars(description = "...")]` attribute → field description in schema
- `#[serde(rename = "...")]` attribute → renamed key in schema

## `#[derive(State)]`: Typed graph state

Apply `#[derive(State)]` to a struct to generate the `State` trait implementation for `StateGraph`.

```rust,no_run
use synwire_derive::State;
use serde::{Serialize, Deserialize};

#[derive(State, Clone, Debug, Default, Serialize, Deserialize)]
struct ConversationState {
    /// Message history — Topic channel appends each new message.
    #[reducer(topic)]
    messages: Vec<String>,

    /// Current processing step — LastValue channel overwrites each update.
    #[reducer(last_value)]
    current_step: String,

    /// Fields with no attribute default to LastValue.
    response_count: u32,
}
```

> 📖 **Rust note:** [Generic type parameters](https://doc.rust-lang.org/book/ch10-01-syntax.html) like `<S>` in `StateGraph<S>` let the graph work with any `State`-implementing type while retaining type safety. The `#[derive(State)]` macro generates the implementation for your specific struct.

### Field attributes and their channels

| Attribute | Channel | Behaviour |
|---|---|---|
| `#[reducer(topic)]` | `Topic` | Appends; accumulates each update (message history, event logs) |
| `#[reducer(last_value)]` | `LastValue` | Overwrites on each write (default; use for current node, flags) |
| *(none)* | `LastValue` | Defaults to `LastValue` |

## When to use macros vs manual implementation

| Use macros | Use manual `impl` |
|---|---|
| Tool parameters map cleanly to a Rust struct | Tool schema is dynamic or variadic |
| State fields have clear `LastValue` or `Topic` semantics | State needs `BinaryOperator` or `NamedBarrier` channels |
| Proc-macro error messages are clear enough | You need better diagnostics during early development |
| 90% of cases | Complex edge cases |

## Dependency requirement

Your parameter types must implement `schemars::JsonSchema`. Add to `Cargo.toml`:

```toml
[dependencies]
schemars = { version = "0.8", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
```

## See also

- [Derive Macros Getting Started](../getting-started/derive-macros.md)
- [Custom Tool How-To](../how-to/custom-tool.md)
- [Architecture](./architecture.md)
