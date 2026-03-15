# Custom Tool

## Using the `#[tool]` macro

The simplest way to create a tool:

```rust,ignore
use synwire_derive::tool;
use synwire_core::error::SynwireError;

/// Reverses the input string.
#[tool]
async fn reverse(text: String) -> Result<String, SynwireError> {
    Ok(text.chars().rev().collect())
}

// Use: let tool = reverse_tool()?;
```

## Using StructuredToolBuilder

For dynamic tool creation:

```rust,ignore
use synwire_core::tools::{StructuredTool, ToolOutput};

let tool = StructuredTool::builder()
    .name("word_count")
    .description("Counts words in text")
    .parameters(serde_json::json!({
        "type": "object",
        "properties": {
            "text": {"type": "string", "description": "Text to count"}
        },
        "required": ["text"]
    }))
    .func(|input| Box::pin(async move {
        let text = input["text"].as_str().unwrap_or("");
        let count = text.split_whitespace().count();
        Ok(ToolOutput {
            content: format!("{count} words"),
            artifact: None,
        })
    }))
    .build()?;
```

## Implementing the Tool trait

For full control:

```rust,ignore
use synwire_core::tools::{Tool, ToolOutput, ToolSchema};
use synwire_core::error::SynwireError;
use synwire_core::BoxFuture;

struct HttpFetcher {
    schema: ToolSchema,
}

impl HttpFetcher {
    fn new() -> Self {
        Self {
            schema: ToolSchema {
                name: "http_fetch".into(),
                description: "Fetches a URL".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {"type": "string"}
                    },
                    "required": ["url"]
                }),
            },
        }
    }
}

impl Tool for HttpFetcher {
    fn name(&self) -> &str { &self.schema.name }
    fn description(&self) -> &str { &self.schema.description }
    fn schema(&self) -> &ToolSchema { &self.schema }

    fn invoke(
        &self,
        input: serde_json::Value,
    ) -> BoxFuture<'_, Result<ToolOutput, SynwireError>> {
        Box::pin(async move {
            let url = input["url"].as_str().unwrap_or("");
            // Fetch URL...
            Ok(ToolOutput {
                content: format!("Fetched: {url}"),
                artifact: None,
            })
        })
    }
}
```

## Returning artifacts

Tools can return both text content and structured artifacts:

```rust,ignore
Ok(ToolOutput {
    content: "Found 3 results".into(),
    artifact: Some(serde_json::json!([
        {"title": "Result 1", "url": "https://example.com/1"},
        {"title": "Result 2", "url": "https://example.com/2"},
        {"title": "Result 3", "url": "https://example.com/3"},
    ])),
})
```
