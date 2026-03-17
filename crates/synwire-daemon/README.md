# synwire-daemon

Singleton background daemon for Synwire. Manages the embedding model, file watchers, indexing pipelines, and multi-repo/worktree state for a single product. MCP servers connect via a Unix domain socket as thin stdio-to-UDS proxies.

## What this crate provides

- **`RepoManager`** -- central coordinator that tracks active worktrees, registers projects by `WorktreeId`, and evicts idle entries via LRU
- **`WorktreeHandle` / `WorktreeStatus`** -- per-worktree runtime state (Idle, Indexing, Ready)
- **Lifecycle management** -- PID file, Unix domain socket listener, signal handling, 5-minute grace period after last client disconnects
- **IPC protocol** -- framed messaging over Unix domain sockets for MCP server communication
- **Indexing orchestration** -- triggers and coordinates walk/chunk/embed/store pipelines per worktree
- **Auto-launch** -- spawned as a detached process by the first MCP server; no systemd or launchctl required
- **Zero unsafe code** -- `#![forbid(unsafe_code)]`

## Quick start

```toml
[dependencies]
synwire-daemon = "0.1"
```

The daemon binary reads `SYNWIRE_PRODUCT` (defaults to `"synwire"`) and stores state under `StorageLayout::data_home()`:

```rust,no_run
use synwire_daemon::{RepoManager, WorktreeStatus};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let manager = RepoManager::new(storage_layout, 32)?;
    let handle = manager.register("/home/user/project").await?;
    assert_eq!(handle.status(), WorktreeStatus::Idle);
    Ok(())
}
```

## Documentation

- [Architecture Explanation](https://randomvariable.github.io/synwire/explanation/synwire-core.html)
- [Full API docs](https://docs.rs/synwire-daemon)
- [Synwire documentation](https://randomvariable.github.io/synwire/)
