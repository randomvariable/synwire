# How to: Use the Virtual Filesystem (VFS)

**Goal:** Give an LLM agent filesystem-like access to heterogeneous data sources through a single interface that mirrors coreutils.

---

## Quick start

Instantiate a VFS provider, call `vfs_tools` to get pre-built tools, and pass them to `create_react_agent`.  The LLM can then `ls`, `read`, `grep`, `tree`, `find`, `write`, `edit`, etc.

```rust
use std::sync::Arc;
use synwire_agent::vfs::local::LocalProvider;
use synwire_core::vfs::{Vfs, OutputFormat, vfs_tools};
use synwire_orchestrator::prebuilt::create_react_agent;

// 1. Create a VFS scoped to the project directory.
let vfs: Arc<dyn Vfs> = Arc::new(
    LocalProvider::new("/home/user/project")?
);

// 2. Get all tools the provider supports — automatically.
let tools = vfs_tools(Arc::clone(&vfs), OutputFormat::Toon);

// 3. Hand them to the react agent.
let graph = create_react_agent(model, tools)?;
let result = graph.invoke(initial_state).await?;
```

That's it.  `vfs_tools` inspects `capabilities()` and only includes tools the provider actually supports.  The `OutputFormat` controls how structured results (directory listings, grep matches, etc.) are serialized before the LLM sees them.

The LLM sees these as callable tools:

```text
Agent: "Let me explore the project."
  → ls { path: ".", recursive: false }
  → tree { path: "src", max_depth: 2 }
  → head { path: "src/main.rs", lines: 20 }
  → grep { pattern: "TODO", path: "src", file_type: "rust" }
  → edit { path: "src/main.rs", old: "// TODO", new: "// DONE" }
```

---

## VFS providers

Choose the provider based on what data the LLM should access.

### `LocalProvider` — real filesystem with path-traversal protection

All operations are scoped to `root`.  Any path escaping `root` is rejected with `VfsError::PathTraversal`.

```rust
use synwire_agent::vfs::local::LocalProvider;

let vfs: Arc<dyn Vfs> = Arc::new(LocalProvider::new("/home/user/project")?);
let tools = vfs_tools(vfs, OutputFormat::Toon);
```

### `MemoryProvider` — ephemeral in-memory storage

No persistence.  Ideal for agent scratchpads and test fixtures.

```rust
use synwire_core::vfs::MemoryProvider;

let vfs: Arc<dyn Vfs> = Arc::new(MemoryProvider::new());
let tools = vfs_tools(vfs, OutputFormat::Toon);
// The LLM can write and read files in memory during its run.
// Everything is lost when the provider is dropped.
```

### `CompositeProvider` — mount multiple providers by path prefix

The LLM sees a unified filesystem.  Routes operations to the provider whose prefix is the longest match.

```rust
use synwire_agent::vfs::composite::{CompositeProvider, Mount};
use synwire_agent::vfs::local::LocalProvider;
use synwire_core::vfs::MemoryProvider;

let vfs: Arc<dyn Vfs> = Arc::new(CompositeProvider::new(vec![
    Mount {
        prefix: "/workspace".to_string(),
        backend: Box::new(LocalProvider::new("/home/user/project")?),
    },
    Mount {
        prefix: "/scratch".to_string(),
        backend: Box::new(MemoryProvider::new()),
    },
]));
let tools = vfs_tools(vfs, OutputFormat::Toon);
// /workspace/... → real files     /scratch/... → ephemeral in-memory
```

The LLM can call `mount` to discover what's available:

```text
Agent: → mount {}
Result:
  /workspace  LocalProvider   [ls, read, write, edit, grep, glob, ...]
  /scratch    MemoryProvider  [ls, read, write, edit, grep, glob, ...]
```

**Cross-boundary operations:** `cp` and `mv` across mount boundaries work automatically — the composite reads from the source mount and writes to the destination mount.  For `mv`, the source is deleted after the write succeeds.

**Root-level operations:** `ls /` shows mount prefixes as virtual directories.  `grep` from root searches all mounts.  `pwd` returns `/`.

### `StoreProvider` — key-value persistence as a filesystem

Wraps any [`BaseStore`] implementation.  Keys map to paths as `/<namespace>/<key>`.

```rust
use synwire_agent::vfs::store::{InMemoryStore, StoreProvider};

let vfs: Arc<dyn Vfs> = Arc::new(StoreProvider::new("agent1", InMemoryStore::new()));
let tools = vfs_tools(vfs, OutputFormat::Json);
```

---

## Available tools

`vfs_tools` generates these tools (only those the provider supports, plus `mount` which is always available):

| Tool | Coreutil | Description |
|------|----------|-------------|
| `mount` | `mount` | Show mounted providers, their paths, and capabilities |
| `pwd` | `pwd` | Print working directory |
| `cd` | `cd` | Change working directory |
| `ls` | `ls` | List directory contents (`-a`, `-R`, long format) |
| `tree` | `tree` | Recursive directory tree (`-L`, `-d`) |
| `read` | `cat` | Read entire file |
| `head` | `head` | First N lines (`-n`) |
| `tail` | `tail` | Last N lines (`-n`) |
| `stat` | `stat` | File metadata |
| `wc` | `wc` | Line/word/byte counts |
| `write` | `>` | Write file (create/overwrite) |
| `append` | `>>` | Append to file |
| `mkdir` | `mkdir` | Create directory (`-p`) |
| `touch` | `touch` | Create empty file / update timestamp |
| `edit` | `sed` | Find and replace in file |
| `diff` | `diff` | Compare two files (`-U`) |
| `rm` | `rm` | Remove file/directory (`-r`, `-f`) |
| `cp` | `cp` | Copy (`-r`, `-n`) |
| `mv` | `mv` | Move / rename |
| `grep` | `grep` | Search file contents (regex, `-i`, file type filter) |
| `glob` | — | Find files by glob pattern |
| `find` | `find` | Search by name, type, depth, size |

---

## Capability checking

Providers declare what they support via `VfsCapabilities` bitflags.  `vfs_tools` handles this automatically, but you can check manually:

```rust
use synwire_core::vfs::types::VfsCapabilities;

if vfs.capabilities().contains(VfsCapabilities::FIND) {
    // provider supports find
}
```

---

## Sandbox tools (non-VFS)

Command execution, process management, and archive handling live in the `sandbox` module — useful for coding agents but a different concern from filesystem abstraction.

```rust
use synwire_agent::sandbox::shell::Shell;
use synwire_agent::sandbox::process::ProcessManager;
use synwire_agent::sandbox::archive::ArchiveManager;
```

---

**See also**

- [How to: Tool Output Formats](tool-output-formats.md) — JSON vs TOON, setting defaults per agent
- [How to: Perform Advanced Search](grep-search.md) — `GrepOptions` reference
