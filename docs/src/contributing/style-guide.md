# Documentation Style Guide

## General principles

- **Concise**: prefer short, direct sentences
- **Practical**: include runnable code examples where possible
- **Consistent**: follow the patterns established in existing docs

## Rust doc comments

### Module-level (`//!`)

Every `lib.rs` and public module must have a `//!` doc comment explaining:

1. What the module provides
2. Key types it exports

```rust
//! Embedding traits and types.
//!
//! This module provides the [`Embeddings`] trait for text embedding models,
//! plus a [`FakeEmbeddings`] implementation for deterministic testing.
```

### Item-level (`///`)

All public items (structs, enums, traits, functions, methods) must have `///` doc comments.

```rust
/// Returns the `k` most similar documents to the query.
///
/// # Errors
///
/// Returns `SynwireError` if the embedding or search operation fails.
fn similarity_search<'a>(/* ... */);
```

### Sections to include

| Section | When |
|---------|------|
| Description | Always (first paragraph) |
| `# Examples` | When the usage is not obvious |
| `# Errors` | For fallible functions |
| `# Panics` | If the function can panic (should be rare) |
| `# Safety` | For `unsafe` functions (should not exist) |

### Code examples

- Use `rust,ignore` for examples that require runtime context (async, API keys)
- Use plain `rust` for examples that should compile as doctests
- Prefer `FakeChatModel` / `FakeEmbeddings` in examples to avoid API key requirements
- Wrap async examples in `tokio_test::block_on` for doctests

```rust
/// # Examples
///
/// ```
/// use synwire_core::language_models::fake::FakeChatModel;
/// use synwire_core::language_models::traits::BaseChatModel;
/// use synwire_core::messages::Message;
///
/// # tokio_test::block_on(async {
/// let model = FakeChatModel::new(vec!["Hello!".into()]);
/// let result = model.invoke(&[Message::human("Hi")], None).await.unwrap();
/// assert_eq!(result.message.content().as_text(), "Hello!");
/// # });
/// ```
```

## mdbook documentation

### File naming

- Lowercase with hyphens: `first-chat.md`, `retry-fallback.md`
- Match the SUMMARY.md entry exactly

### Headings

- `#` for page title (one per file)
- `##` for major sections
- `###` for subsections
- Do not skip heading levels

### Code blocks

- Use `rust,ignore` for Rust code that should not be tested
- Use `toml` for Cargo.toml snippets
- Use `sh` for shell commands
- Use `mermaid` for diagrams (inside fenced code blocks)

### Tables

Use Markdown tables for structured information. Align columns for readability in source.

### Links

- Use relative paths for internal links: `[Streaming](./streaming.md)`
- Use `../` to link across sections: `[Feature Flags](../reference/feature-flags.md)`
- Link to rustdoc for API details rather than duplicating signatures

## Error documentation

Every error enum variant must have a `///` doc comment explaining:

1. When this error occurs
2. What the fields mean

```rust
/// Rate limit exceeded.
#[error("rate limit exceeded")]
RateLimit {
    /// Optional duration to wait before retrying.
    retry_after: Option<Duration>,
},
```

## Commit messages

- Start with a verb: "Add", "Fix", "Update", "Remove"
- Reference the task number if applicable
- Keep the first line under 72 characters
