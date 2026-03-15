# Common Errors

## SynwireError variants

### Model errors

| Error | Cause | Resolution |
|-------|-------|------------|
| `ModelError::RateLimit` | API rate limit exceeded | Wait for `retry_after` duration, or use `RunnableRetry` |
| `ModelError::AuthenticationFailed` | Invalid or missing API key | Check `OPENAI_API_KEY` or equivalent |
| `ModelError::InvalidRequest` | Malformed request | Check message format and model parameters |
| `ModelError::ContentFiltered` | Content safety filter triggered | Modify input content |
| `ModelError::Timeout` | Request timed out | Increase timeout or retry |
| `ModelError::Connection` | Network connectivity issue | Check network, retry |

### Tool errors

| Error | Cause | Resolution |
|-------|-------|------------|
| `ToolError::NotFound` | Tool name not registered | Check tool name spelling |
| `ToolError::InvalidName` | Name does not match `[a-zA-Z0-9_-]{1,64}` | Fix tool name |
| `ToolError::ValidationFailed` | Input does not match schema | Check tool call arguments |
| `ToolError::InvocationFailed` | Tool execution failed | Check tool implementation |
| `ToolError::PathTraversal` | Path traversal attempt detected | Security check -- do not bypass |
| `ToolError::Timeout` | Tool execution timed out | Increase timeout |

### Parse errors

| Error | Cause | Resolution |
|-------|-------|------------|
| `ParseError::ParseFailed` | Could not parse model output | Check output format, add format instructions |
| `ParseError::FormatMismatch` | Output does not match expected format | Improve prompt or use structured output |

### Embedding errors

| Error | Cause | Resolution |
|-------|-------|------------|
| `EmbeddingError::Failed` | Embedding API call failed | Check API key and model name |
| `EmbeddingError::DimensionMismatch` | Vector dimensions do not match | Ensure consistent embedding model |

### Vector store errors

| Error | Cause | Resolution |
|-------|-------|------------|
| `VectorStoreError::NotFound` | Document ID not found | Check document was added |
| `VectorStoreError::DimensionMismatch` | Embedding dimensions mismatch | Use same embedding model for add and query |

### Other

| Error | Cause |
|-------|-------|
| `SynwireError::Prompt` | Prompt template variable missing or invalid |
| `SynwireError::Credential` | Credential provider failed |
| `SynwireError::Serialization` | JSON serialisation/deserialisation failed |
| `SynwireError::Io` | File system or I/O error |

## GraphError variants

| Error | Cause | Resolution |
|-------|-------|------------|
| `RecursionLimit` | Exceeded step limit | Increase limit or fix loop |
| `NoEntryPoint` | `set_entry_point` not called | Call `graph.set_entry_point("node")` |
| `DuplicateNode` | Two nodes with same name | Use unique names |
| `TaskNotFound` | Edge references unknown node | Check node names |
| `CompileError` | Node has no outgoing edges | Add edges for all nodes |
| `EmptyInput` | Empty state provided | Provide initial state |
| `Interrupt` | Graph paused for human input | Handle interrupt, resume later |
| `MultipleValues` | LastValue channel got >1 value | Use Topic channel or fix graph |

## Error kind matching

Use `SynwireErrorKind` for retry and fallback decisions:

```rust,ignore
use synwire_core::error::SynwireErrorKind;

match err.kind() {
    SynwireErrorKind::Model => { /* retry */ }
    SynwireErrorKind::Parse => { /* re-prompt */ }
    SynwireErrorKind::Credential => { /* fail fast */ }
    _ => { /* handle other */ }
}
```
