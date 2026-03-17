# Migration Guide

## Pre-StorageLayout to StorageLayout paths

Before `StorageLayout` was introduced, Synwire stored data under ad-hoc paths. This guide covers moving existing data to the new layout.

### What changed

| Data | Old path | New path |
|------|----------|---------|
| Vector indices | `$CACHE/synwire/indices/` | `StorageLayout::index_cache(worktree)` |
| Code graphs | `$CACHE/synwire/graphs/` | `StorageLayout::graph_dir(worktree)` |
| Agent skills | `$DATA/synwire/skills/` | `StorageLayout::skills_dir()` |

The new paths include the product name and (for per-worktree data) a `WorktreeId` key.

### Shell migration

The index cache and graph cache are regenerable — they can be deleted and will be rebuilt on next use. Skills are durable and should be migrated.

```sh
# Product name to migrate to (must match --product-name you will use)
PRODUCT="synwire"

# Determine your worktree key
WORKTREE_KEY=$(synwire-mcp-server --project . --product-name "$PRODUCT" --print-worktree-key 2>/dev/null || echo "unknown")

# New base paths (Linux/XDG)
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/$PRODUCT"
CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/$PRODUCT"

# Migrate skills (durable — copy, verify, then remove old)
if [ -d "$HOME/.local/share/synwire/skills" ]; then
    mkdir -p "$DATA_DIR/skills"
    cp -r "$HOME/.local/share/synwire/skills/." "$DATA_DIR/skills/"
    echo "Skills migrated to $DATA_DIR/skills/"
fi

# Index and graph caches are regenerable — safe to move or delete
# Option A: delete (will be rebuilt on next index)
rm -rf "$HOME/.cache/synwire/indices"
rm -rf "$HOME/.cache/synwire/graphs"

# Option B: move (avoids rebuild, but only works if worktree_key is known)
# mkdir -p "$CACHE_DIR/indices"
# mv "$HOME/.cache/synwire/indices" "$CACHE_DIR/indices/$WORKTREE_KEY"
```

> The `--print-worktree-key` flag is not implemented in v0.1. Use option A (delete) unless you need to preserve index data, in which case build `synwire-storage` separately to compute the key programmatically.

### Programmatic key derivation

```rust,no_run
use synwire_storage::{StorageLayout, WorktreeId};
use std::path::Path;

let layout = StorageLayout::new("synwire")?;
let wid = WorktreeId::for_path(Path::new("."))?;

println!("index cache: {}", layout.index_cache(&wid).display());
println!("graph dir:   {}", layout.graph_dir(&wid).display());
```

Run this small program from your project root to get the exact destination paths for your machine.

### Config file migration

If you previously used a custom `SYNWIRE_CACHE_DIR` or `SYNWIRE_DATA_DIR` environment variable, these still work in v0.1 and take the highest precedence. No changes are needed if you rely on them.

If you used a project-local config at `$PROJECT/.synwire/config.json` (pre-StorageLayout), rename it to `.$PRODUCT/config.json` where `$PRODUCT` is your product name:

```sh
# Rename .synwire to .myapp (if using product name 'myapp')
mv .synwire .myapp
```

## See also

- [synwire-storage explanation](../explanation/synwire-storage.md) — layout architecture
- [synwire-storage](../explanation/synwire-storage.md) — configuration hierarchy
