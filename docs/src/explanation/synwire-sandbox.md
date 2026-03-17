# synwire-sandbox: Process Isolation

`synwire-sandbox` provides platform-specific process isolation, resource accounting, and LLM-accessible process management tools for Synwire agents. It is the crate that makes it safe for an agent to run shell commands by bounding the blast radius of agent actions.

> For the design philosophy and detailed rationale behind the sandbox architecture (why not Docker, the two-tier OCI runtime model, PTY integration, cgroup v2 accounting), see [Sandbox Architecture Methodology](./sandbox-methodology.md). This document focuses on the crate's API and structure.

## Platform support

| Platform | Light isolation | Strong isolation |
|---|---|---|
| Linux | cgroup v2 + AppArmor | Namespace container (runc/crun) |
| macOS | `sandbox-exec` Seatbelt | Podman / Apple Container / Docker Desktop / Colima |
| Other | None (fallback) | None |

## Crate structure

### `platform`

Platform-specific backends:

- **`linux::namespace`** --- OCI runtime spec generation, container lifecycle (create, start, wait, kill), `--console-socket` PTY handoff
- **`linux::cgroup`** --- `CgroupV2Manager` for per-agent resource limits (CPU, memory, PIDs) with cleanup-on-drop via `cgroup.kill`
- **`macos::seatbelt`** --- Sandbox Profile Language (SBPL) generation from `SandboxConfig`
- **`macos::container`** --- Container runtime detection and delegation (Apple Container, Docker Desktop, Podman, Colima)

### `plugin`

Agent integration layer:

- **`ProcessPlugin`** --- contributes five management tools: `list_processes`, `kill_process`, `process_stats`, `wait_for_process`, `read_process_output`
- **`CommandPlugin`** (via `command_tools`) --- contributes four execution tools: `run_command`, `open_shell`, `shell_write`, `shell_read`
- **`SandboxContext`** --- shared state holding the process registry, sandbox configuration, and output capture settings
- **`expect_engine`** --- PTY automation via `expectrl` for interactive commands (terraform apply, ssh host key prompts, gpg passphrase entry)

### `process_registry`

In-memory registry tracking all spawned processes:

```rust,no_run
use synwire_sandbox::{ProcessRegistry, ProcessRecord, ProcessStatus};

let registry = ProcessRegistry::new();
// Processes are registered when spawned, queried by tools,
// and cleaned up when the agent session ends.
```

Each `ProcessRecord` tracks the process ID, status, spawn time, resource usage, and captured output reference.

### `output`

Output capture infrastructure:

- **`OutputMode`** --- enum controlling how process output is captured (file-backed, memory, or discarded)
- **`ProcessCapture`** --- manages file-backed output capture that survives process kills (cgroup OOM or timeout)
- **`CapturedOutput`** --- the result: stdout, stderr, and exit code

File-backed capture is the default because when an agent exceeds its resource budget and the cgroup kills its processes, in-memory pipe buffers are lost. File-backed capture ensures partial output is still recoverable.

### `visibility`

`ProcessVisibilityScope` controls which processes a tool can see:

- **`Own`** --- only processes spawned by this agent session
- **`All`** --- all processes tracked by the registry (for admin tools)

### `error`

`SandboxError` covers container runtime failures, cgroup operations, permission errors, PTY setup failures, and timeout conditions.

## Safety

The crate uses `#![deny(unsafe_code)]` with a single scoped exception: receiving a PTY controller file descriptor from the OCI runtime via `SCM_RIGHTS` requires converting a kernel-provided raw fd to an `OwnedFd`. This is the minimum unsafe surface required for PTY support.

## Dependencies

| Crate | Role |
|---|---|
| `synwire-core` | Tool traits for plugin tools |
| `expectrl` | PTY pattern matching (goexpect equivalent) |
| `oci-spec` | OCI runtime spec generation (Linux only) |
| `nix` | Unix system calls (Linux only) |
| `uuid` | Process record identifiers |
| `chrono` | Timestamps for process records |
| `which` | Runtime binary detection |
| `tempfile` | Temporary directories for OCI bundles |

## Ecosystem position

```text
synwire (umbrella, feature = "sandbox")
    |
    +-- synwire-sandbox  (this crate)
            |
            +-- synwire-core  (tool traits)
            +-- synwire-agent-skills  (optional: sandboxed skill execution)
```

`synwire-sandbox` is used directly by the `synwire` umbrella crate (behind the `sandbox` feature flag) and by `synwire-agent-skills` (behind its `sandboxed` feature flag).

## See also

- [Sandbox Architecture Methodology](./sandbox-methodology.md) --- design philosophy and rationale
- [Process Sandboxing](../how-to/process-sandbox.md) --- how-to guide
- [Sandboxed Command Execution](../tutorials/10-sandboxed-agent.md) --- tutorial
