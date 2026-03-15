# Prompt Chains

This tutorial shows how to use `PromptTemplate` and compose chains of runnables.

## Prompt templates

`PromptTemplate` formats strings with named variables:

```rust,ignore
use std::collections::HashMap;
use synwire_core::prompts::PromptTemplate;

let template = PromptTemplate::new(
    "Tell me a joke about {topic}",
    vec!["topic".into()],
);

let mut vars = HashMap::new();
vars.insert("topic".into(), "Rust programming".into());
let prompt = template.format(&vars)?;
// "Tell me a joke about Rust programming"
```

## Template + Model + Parser

Compose a prompt template with a model and output parser:

```rust,ignore
use std::collections::HashMap;
use synwire_core::language_models::{FakeChatModel, BaseChatModel};
use synwire_core::messages::Message;
use synwire_core::output_parsers::{OutputParser, StrOutputParser};
use synwire_core::prompts::PromptTemplate;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create a template
    let template = PromptTemplate::new(
        "Explain {concept} in one sentence.",
        vec!["concept".into()],
    );

    // 2. Format the prompt
    let mut vars = HashMap::new();
    vars.insert("concept".into(), "ownership in Rust".into());
    let prompt = template.format(&vars)?;

    // 3. Invoke the model
    let model = FakeChatModel::new(vec![
        "Ownership is Rust's system for managing memory without a garbage collector.".into(),
    ]);
    let result = model.invoke(&[Message::human(&prompt)], None).await?;

    // 4. Parse the output
    let parser = StrOutputParser;
    let output = parser.parse(&result.message.content().as_text())?;
    println!("{output}");

    Ok(())
}
```

## Chat prompt templates

For multi-message prompts, use `ChatPromptTemplate`:

```rust,ignore
use synwire_core::prompts::{ChatPromptTemplate, MessageTemplate};

let chat_template = ChatPromptTemplate::new(vec![
    MessageTemplate::system("You are a {role}."),
    MessageTemplate::human("{question}"),
]);
```

## RunnableCore composition

All components implement `RunnableCore` with `serde_json::Value` as the universal I/O type. This enables heterogeneous chaining:

```rust,ignore
use synwire_core::runnables::{RunnableCore, RunnableLambda, pipe};

// Create a lambda runnable
let add_prefix = RunnableLambda::new(|input: serde_json::Value| {
    let text = input.as_str().unwrap_or_default();
    Ok(serde_json::json!(format!("Processed: {text}")))
});
```

## Output parsers

| Parser | Output type | Use case |
|--------|-------------|----------|
| `StrOutputParser` | `String` | Plain text extraction |
| `JsonOutputParser` | `serde_json::Value` | JSON responses |
| `StructuredOutputParser` | Typed `T: DeserializeOwned` | Structured data |
| `ToolsOutputParser` | `Vec<ToolCall>` | Tool call extraction |

## Next steps

- [Streaming](./streaming.md) -- stream responses incrementally
- [RAG](./rag.md) -- add retrieval-augmented generation
