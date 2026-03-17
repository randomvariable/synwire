# How to: Configure Tool Output Formats

**Goal:** Control how structured data from VFS operations and tools is serialized before being passed to the LLM.

---

## Why it matters

When an LLM calls a tool like `ls` or `find`, the result is a Rust struct (`Vec<DirEntry>`, `Vec<FindEntry>`, etc.).  Before the LLM sees it, the data must be serialized to text.  The format you choose affects:

- **Token usage** — TOON can reduce tokens by 30–60 % for tabular data
- **Model comprehension** — JSON is universally understood; TOON is optimised for LLM consumption
- **Debugging** — pretty JSON is easiest to read in logs

---

## The three formats

| Format | When to use | Token cost |
|--------|------------|------------|
| `OutputFormat::Json` | Debugging, human review | Highest |
| `OutputFormat::JsonCompact` | Bandwidth-sensitive, small payloads | Medium |
| `OutputFormat::Toon` | Production LLM agents, tabular data | Lowest |

Example — `ls` returning two files:

**JSON** (136 tokens):
```json
[
  {"name": "main.rs", "path": "/src/main.rs", "is_dir": false, "size": 1024},
  {"name": "lib.rs", "path": "/src/lib.rs", "is_dir": false, "size": 512}
]
```

**TOON** (~40 tokens):
```
[2]{name,path,is_dir,size}:
  main.rs,/src/main.rs,false,1024
  lib.rs,/src/lib.rs,false,512
```

---

## Setting the default on an agent

```rust
use synwire_core::vfs::OutputFormat;

let agent = Agent::new()
    .name("coding-agent")
    .model("claude-sonnet-4-20250514")
    .tool_output_format(OutputFormat::Toon)
    .build();
```

All tools on this agent will use TOON by default when formatting their output.

---

## Overriding per-tool

Individual tools can call `format_output` directly with a different format:

```rust
use synwire_core::vfs::{format_output, OutputFormat};

// Inside a tool implementation:
let entries = vfs.ls(path, opts).await?;

// Use JSON for this specific tool, regardless of the agent default.
let content = format_output(&entries, OutputFormat::Json)?;
```

---

## Using `format_output`

`format_output` accepts any `Serialize` value and an `OutputFormat`:

```rust
use synwire_core::vfs::{format_output, OutputFormat};

// Serialize a Vec<DirEntry> to TOON.
let text = format_output(&entries, OutputFormat::Toon)?;

// Serialize a TreeEntry to compact JSON.
let text = format_output(&tree, OutputFormat::JsonCompact)?;

// Serialize a DiffResult to pretty JSON.
let text = format_output(&diff, OutputFormat::Json)?;
```

The `toon` feature is enabled by default.  If you compile without it (`default-features = false`), `OutputFormat::Toon` falls back to pretty JSON.

---

**See also**

- [TOON specification](https://github.com/toon-format/spec)
- [How to: Use the VFS](vfs.md) — VFS providers and operations
