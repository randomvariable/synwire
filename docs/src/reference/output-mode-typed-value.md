# OutputMode and TypedValue Interop

## OutputMode

`OutputMode` controls how structured output is extracted from a language model. Different providers support different mechanisms.

### Variants

| Variant | Mechanism | Provider support |
|---------|-----------|-----------------|
| `Native` | Model's native structured output (e.g., `response_format`) | OpenAI (gpt-4o+), Ollama (some models) |
| `Tool` | Tool calling to extract structured output | OpenAI, Ollama (tool-capable models) |
| `Prompt` | Format instructions embedded in the prompt | All providers (universal fallback) |
| `Custom(String)` | User-defined extraction strategy | Any |

### Fallback chain

`OutputMode::fallback_chain()` returns `[Native, Tool, Prompt]`. Use this to try the most capable mode first and fall back gracefully:

```rust,ignore
use synwire_core::output_parsers::OutputMode;

for mode in OutputMode::fallback_chain() {
    if mode.validate_compatibility(supports_native, supports_tools).is_ok() {
        // Use this mode
        break;
    }
}
```

### Compatibility validation

Check whether a provider supports a given mode before use:

```rust,ignore
let mode = OutputMode::Native;

// Returns Err if provider lacks native support
mode.validate_compatibility(
    supports_native,  // bool: provider has response_format support
    supports_tools,   // bool: provider has tool calling support
)?;
```

## TypedValue interop

`RunnableCore` uses `serde_json::Value` as its universal I/O type. To convert between typed data and `Value`:

### Serialisation

```rust,ignore
use serde::Serialize;

#[derive(Serialize)]
struct Query {
    question: String,
    context: Vec<String>,
}

let query = Query {
    question: "What is Rust?".into(),
    context: vec!["Rust is a language.".into()],
};

let value = serde_json::to_value(&query)?;
// Pass to RunnableCore::invoke
```

### Deserialisation

```rust,ignore
use serde::Deserialize;

#[derive(Deserialize)]
struct Answer {
    text: String,
    confidence: f64,
}

let result = runnable.invoke(input, None).await?;
let answer: Answer = serde_json::from_value(result)?;
```

### OutputParser with typed output

`StructuredOutputParser` combines `OutputMode` with typed deserialisation:

```rust,ignore
use synwire_core::output_parsers::StructuredOutputParser;

// Parses model output as JSON into a typed struct
let parser = StructuredOutputParser::<Answer>::new();
let answer = parser.parse(&model_output_text)?;
```

### JsonOutputParser for dynamic values

When the schema is not known at compile time:

```rust,ignore
use synwire_core::output_parsers::JsonOutputParser;

let parser = JsonOutputParser;
let value: serde_json::Value = parser.parse(&text)?;
```

## Design rationale

The `serde_json::Value` approach was chosen over generic type parameters for `RunnableCore` because:

1. **Object safety**: `Vec<Box<dyn RunnableCore>>` would not work with generic parameters
2. **Composability**: any runnable chains with any other without type conversion boilerplate
3. **Trade-off**: runtime type checking instead of compile-time, but this matches the dynamic nature of LLM outputs

`OutputMode` provides a type-safe way to select the structured output extraction strategy, while `serde_json::Value` provides the runtime flexibility needed for heterogeneous chains.
