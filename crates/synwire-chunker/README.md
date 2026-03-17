# synwire-chunker

Tree-sitter AST-aware code chunking for Synwire semantic search. Splits source files into semantic chunks — functions, structs, classes — using tree-sitter for recognised languages, with a recursive character text splitter fallback for everything else.

## Quick start

```rust
use synwire_chunker::{Chunker, ChunkOptions};

let chunker = Chunker::new();
let docs = chunker.chunk_file(
    "src/main.rs",
    "pub fn greet(name: &str) -> String { format!(\"Hello, {name}!\") }",
);
assert!(!docs.is_empty());

// Each Document has metadata: file, language, symbol, line_start, line_end
if let Some(symbol) = docs[0].metadata.get("symbol") {
    println!("Chunked function: {symbol}");
}
```

## Supported languages

| Language | Extensions | AST chunking |
|----------|-----------|:------------:|
| Rust | `.rs` | Yes |
| Python | `.py` | Yes |
| JavaScript | `.js`, `.mjs`, `.cjs` | Yes |
| TypeScript | `.ts`, `.tsx` | Yes |
| Go | `.go` | Yes |
| Java | `.java` | Yes |
| C | `.c`, `.h` | Yes |
| C++ | `.cpp`, `.cc`, `.cxx`, `.hpp`, `.hxx` | Yes |
| C# | `.cs` | Yes |
| Ruby | `.rb` | Yes |
| Bash | `.sh`, `.bash` | Yes |
| JSON | `.json` | Text fallback |
| TOML | `.toml` | Text fallback |
| YAML | `.yaml`, `.yml` | Text fallback |
| HTML | `.html`, `.htm` | Text fallback |
| CSS | `.css` | Text fallback |
| Markdown | `.md`, `.markdown` | Text fallback |

Languages marked "Text fallback" are recognised for metadata purposes but use the recursive character splitter because no compatible tree-sitter grammar is available.

Unrecognised extensions also use the text splitter.

## Configuration

```rust
use synwire_chunker::{Chunker, ChunkOptions};

let opts = ChunkOptions {
    chunk_size: 2000,   // target bytes per chunk (default: 1500)
    overlap: 300,       // overlap bytes between consecutive chunks (default: 200)
};
let chunker = Chunker::with_options(opts);
```

`chunk_size` and `overlap` apply only to the text splitter fallback. AST chunks are always one definition per chunk, regardless of size.

## Chunk metadata

Every `Document` produced carries a metadata map:

| Key | AST | Text | Type | Description |
|-----|:---:|:----:|------|-------------|
| `file` | Yes | Yes | `String` | Source file path |
| `language` | Yes | No | `String` | Lowercase language name |
| `symbol` | When found | No | `String` | Definition name (e.g. `authenticate`) |
| `line_start` | Yes | Yes | `Number` | 1-indexed first line |
| `line_end` | Yes | Yes | `Number` | 1-indexed last line |
| `chunk_index` | No | Yes | `Number` | 0-based sequential position |

## How AST chunking works

1. Detect language from file extension via `detect_language(path)`.
2. Parse the file with the corresponding tree-sitter grammar.
3. Walk immediate children of the root node looking for definition-level nodes (functions, classes, structs, traits, etc.).
4. Each definition becomes one `Document`, regardless of size.
5. Falls back to text splitter if: language unrecognised, no grammar available, parse fails, or no definitions found.

Nested definitions (a helper function inside a class) are captured within their parent — they are not split out separately. This keeps each chunk self-contained.

## How text splitting works

The recursive character splitter tries split points in decreasing granularity order:

1. Paragraph boundary (`\n\n`)
2. Newline (`\n`)
3. Space (` `)
4. Any character

At each level it finds the last occurrence of the separator that keeps the chunk within `chunk_size` bytes. Consecutive chunks share `overlap` bytes of context so content straddling a split boundary appears in both chunks.

## See also

- [synwire-index](../synwire-index/README.md) — uses `synwire-chunker` as part of the indexing pipeline
- [synwire-embeddings-local](../synwire-embeddings-local/README.md) — embeds the chunks produced here
