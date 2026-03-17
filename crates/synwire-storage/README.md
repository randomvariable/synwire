# synwire-storage

Product-scoped storage path management for the Synwire workspace. Provides `StorageLayout` for computing stable, platform-correct paths for all Synwire subsystems, and `RepoId`/`WorktreeId` for two-level project identity.

## Quick start

```rust,no_run
use synwire_storage::{StorageLayout, WorktreeId};
use std::path::Path;

let layout = StorageLayout::new("myapp")?;
let worktree = WorktreeId::for_path(Path::new("/path/to/repo"))?;

// Durable data
println!("{}", layout.session_db("sess-001").display());
println!("{}", layout.experience_db(&worktree).display());
println!("{}", layout.skills_dir().display());

// Cache (may be deleted and regenerated)
println!("{}", layout.index_cache(&worktree).display());
println!("{}", layout.graph_dir(&worktree).display());
```

## StorageLayout

`StorageLayout` computes all Synwire storage paths for a given product name using a consistent hierarchy rooted at the platform data and cache directories.

### Path layout

```text
$XDG_DATA_HOME/<product>/          (Linux)
~/Library/Application Support/<product>/   (macOS)
%APPDATA%/<product>/               (Windows)

├── sessions/<session_id>.db        — checkpoint databases
├── experience/<worktree_key>.db    — per-worktree experience pool
├── skills/                         — global agent skills
├── logs/                           — rotating log files
├── daemon.pid                      — daemon PID file
├── daemon.sock                     — daemon UDS socket
└── global/
    ├── registry.json               — project registry
    ├── experience.db               — cross-project experience
    ├── dependencies.db             — cross-project dependency index
    └── config.json                 — global product config

$XDG_CACHE_HOME/<product>/

├── indices/<worktree_key>/         — vector + BM25 indices
├── graphs/<worktree_key>/          — code dependency graphs
├── communities/<worktree_key>/     — community detection state
├── lsp/<worktree_key>/             — LSP server caches
├── models/                         — embedding model download cache
└── repos/<owner>/<repo>/           — cloned repositories
```

### Durable vs cache

| Location | Durability | Examples |
|----------|-----------|---------|
| `$DATA/<product>/` | Durable — never delete | Sessions, experience, skills, logs |
| `$CACHE/<product>/` | Regenerable — safe to delete | Indices, graphs, communities, cloned repos |

### Path methods

| Method | Returns | Purpose |
|--------|---------|---------|
| `session_db(id)` | `PathBuf` | SQLite checkpoint DB for a session |
| `experience_db(worktree)` | `PathBuf` | Per-worktree experience pool |
| `skills_dir()` | `PathBuf` | Global agent skills directory |
| `logs_dir()` | `PathBuf` | Rotating log files |
| `daemon_pid_file()` | `PathBuf` | Daemon PID file |
| `daemon_socket()` | `PathBuf` | Daemon Unix domain socket |
| `global_experience_db()` | `PathBuf` | Cross-project experience |
| `global_dependency_db()` | `PathBuf` | Cross-project dependency index |
| `index_cache(worktree)` | `PathBuf` | Vector + BM25 index cache |
| `graph_dir(worktree)` | `PathBuf` | Code dependency graph |
| `communities_dir(worktree)` | `PathBuf` | Community detection state |
| `repos_cache()` | `PathBuf` | Root of cloned repos |
| `repo_cache(owner, repo)` | `PathBuf` | A specific cloned repo |

### Construction

```rust,no_run
use synwire_storage::{StorageLayout, StorageConfig};

// Platform-default paths
let layout = StorageLayout::new("myapp")?;

// Override root (useful in tests)
let layout = StorageLayout::with_root("/tmp/test", "myapp");

// Apply programmatic config on top
let config = StorageConfig {
    data_home: Some("/data/myapp".into()),
    ..Default::default()
};
let layout = StorageLayout::new("myapp")?.with_config(&config);
```

### Project-local config

Each project can supply `.<product>/config.json` to override paths for that project:

```json
{
  "data_home": "/custom/data",
  "cache_home": "/custom/cache"
}
```

Load it:

```rust,no_run
let cfg = layout.load_project_config(std::path::Path::new("/path/to/project"))?;
if let Some(cfg) = cfg {
    let layout = layout.with_config(&cfg);
}
```

## Two-level identity

Synwire identifies projects at two levels to support multi-worktree repositories cleanly.

### RepoId

`RepoId` is stable across all clones and worktrees of the same repository. It is derived from:

1. **Git available**: SHA-1 of the first (root) commit (`git rev-list --max-parents=0 HEAD`)
2. **Git unavailable**: SHA-256 of the canonical directory path

```rust,no_run
use synwire_storage::identity::RepoId;
use std::path::Path;

let id = RepoId::for_path(Path::new("/path/to/repo"))?;
println!("{id}");  // e.g. "a3f2c1..."
```

### WorktreeId

`WorktreeId` identifies a specific working copy within a repository family. It combines `RepoId` with a SHA-256 of the canonicalised worktree root path.

```rust,no_run
use synwire_storage::WorktreeId;
use std::path::Path;

let wid = WorktreeId::for_path(Path::new("/path/to/repo"))?;
println!("{}", wid.key());        // "a3f2c1...-def456789012"
println!("{}", wid.display_name); // "myrepo@main"
```

The `key()` method produces a compact string safe for use in directory names: `<repo_id>-<worktree_hash[:12]>`.

### Why two levels?

| Scenario | RepoId | WorktreeId |
|----------|--------|-----------|
| Two clones of same repo | Same | Different |
| Same repo, two branches (worktrees) | Same | Different |
| Two unrelated repos | Different | Different |

This lets the experience pool and dependency index be shared across branches of the same repo (via `RepoId`), while vector indices are per-worktree (via `WorktreeId`).

## Configuration

| Source | Priority | Applies to |
|--------|----------|-----------|
| `SYNWIRE_DATA_DIR` env var | Highest | Data home |
| `SYNWIRE_CACHE_DIR` env var | Highest | Cache home |
| `StorageLayout::with_root(root, name)` | Explicit | Both |
| `.<product>/config.json` | Per-project | Either |
| Platform default (`directories::BaseDirs`) | Lowest | Both |

CLI override example:

```sh
SYNWIRE_DATA_DIR=/mnt/data/synwire synwire-mcp-server --project .
```

## Migration

Future versions will provide `StorageLayout::migrate(from_version)` to move or rewrite paths between schema versions. In v0.1, no migration is needed — all paths are new.

Path mapping from pre-StorageLayout layouts:

| Old path | New path |
|----------|---------|
| `$CACHE/synwire/indices/` | `StorageLayout::index_cache(worktree)` |
| `$CACHE/synwire/graphs/` | `StorageLayout::graph_dir(worktree)` |
| `$DATA/synwire/skills/` | `StorageLayout::skills_dir()` |

See the [migration guide](../../docs/src/how-to/migration.md) for shell commands.

## Feature flags

No optional features. Always depends on `directories` and `sha2`.
