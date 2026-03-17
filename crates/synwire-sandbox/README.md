# synwire-sandbox

Platform-specific process sandboxing for Synwire agents. Provides process isolation, resource accounting, output capture, and LLM-accessible process management tools.

## What this crate provides

- **`ProcessRegistry`** -- in-memory registry of spawned processes with lifecycle tracking (`Running`, `Exited`, `Signaled`)
- **`ProcessRecord`** -- per-process metadata: PID, command, cgroup path, CPU/memory stats, captured output
- **`CapturedOutput` / `OutputMode`** -- stdout/stderr capture with configurable modes
- **`ProcessVisibilityScope`** -- controls which processes an agent can see and manage
- **Platform-adaptive isolation** -- namespace containers on Linux (via OCI runtime), Seatbelt on macOS, graceful fallback elsewhere
- **Resource accounting** -- cgroup v2 CPU and memory stats on Linux

## Platform support

| Platform | Light isolation | Strong isolation |
|----------|----------------|-----------------|
| Linux    | cgroup v2 + AppArmor | Namespace container (runc/crun) |
| macOS    | `sandbox-exec` Seatbelt | Podman / Lima |
| Other    | None (fallback) | None |

## Quick start

```toml
[dependencies]
synwire-sandbox = "0.1"
```

Track processes via the registry:

```rust,no_run
use synwire_sandbox::{ProcessRegistry, ProcessRecord, ProcessStatus};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let registry = ProcessRegistry::new();

    let record = ProcessRecord::new(1234, "cargo", vec!["build".into()]);
    registry.insert(record).await;

    for (pid, rec) in registry.list().await {
        println!("PID {pid}: {} ({:?})", rec.command, rec.status);
    }
    Ok(())
}
```

## Documentation

- [Process Sandbox Guide](https://randomvariable.github.io/synwire/how-to/process-sandbox.html)
- [Sandbox Methodology](https://randomvariable.github.io/synwire/explanation/sandbox-methodology.html)
- [Full API docs](https://docs.rs/synwire-sandbox)
- [Synwire documentation](https://randomvariable.github.io/synwire/)
