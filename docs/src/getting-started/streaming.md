# Streaming

This tutorial shows how to stream responses from chat models token by token.

## Basic streaming

Every `BaseChatModel` provides a `stream` method that returns a `BoxStream` of `ChatChunk` values:

```rust,ignore
use futures_util::StreamExt;
use synwire_core::language_models::{FakeChatModel, BaseChatModel};
use synwire_core::messages::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = FakeChatModel::new(vec!["Hello, world!".into()])
        .with_chunk_size(3); // Split into 3-character chunks

    let messages = vec![Message::human("Greet me")];
    let mut stream = model.stream(&messages, None).await?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(content) = &chunk.delta_content {
            print!("{content}");
        }
    }
    println!();
    // Output: Hel lo, wo rld !

    Ok(())
}
```

## Streaming with OpenAI

```rust,ignore
use futures_util::StreamExt;
use synwire_core::language_models::BaseChatModel;
use synwire_core::messages::Message;
use synwire_llm_openai::ChatOpenAI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = ChatOpenAI::builder()
        .model("gpt-4o-mini")
        .api_key_env("OPENAI_API_KEY")
        .build()?;

    let messages = vec![Message::human("Write a haiku about Rust")];
    let mut stream = model.stream(&messages, None).await?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(content) = &chunk.delta_content {
            print!("{content}");
        }
    }
    println!();

    Ok(())
}
```

## ChatChunk fields

| Field | Type | Description |
|-------|------|-------------|
| `delta_content` | `Option<String>` | Incremental text content |
| `delta_tool_calls` | `Vec<ToolCallChunk>` | Incremental tool call data |
| `finish_reason` | `Option<String>` | `"stop"` on the final chunk |
| `usage` | `Option<UsageMetadata>` | Token usage (final chunk only) |

## Collecting streamed output

To accumulate the full response:

```rust,ignore
let mut full_text = String::new();
while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    if let Some(content) = &chunk.delta_content {
        full_text.push_str(content);
    }
}
```

## Error handling during streaming

Errors can occur mid-stream. Handle them per-chunk:

```rust,ignore
while let Some(result) = stream.next().await {
    match result {
        Ok(chunk) => { /* process chunk */ }
        Err(e) => {
            eprintln!("Stream error: {e}");
            break;
        }
    }
}
```

## Testing streams

`FakeChatModel` supports configurable chunking and error injection for stream testing:

```rust,ignore
// Split into 5-char chunks, inject error after 2 chunks
let model = FakeChatModel::new(vec!["abcdefghij".into()])
    .with_chunk_size(5)
    .with_stream_error_after(2);
```

## Next steps

- [RAG](./rag.md) -- retrieval-augmented generation
- [Tools and Agents](./tools-agent.md) -- tool-using agents
