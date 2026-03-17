# Sandboxed Command Execution

**Time**: ~30 minutes
**Prerequisites**: Rust 1.85+, Cargo, `runc` on `$PATH`

This tutorial builds an agent whose LLM can run shell commands inside an
isolated sandbox through tool calls. Three scenarios show progressively
complex interactions:

1. **Oneshot command** — LLM runs a compiler, gets exit code + diagnostics
2. **Long-lived command with polling** — LLM starts a test suite in
   background, polls for completion, reads partial output
3. **CLI that prompts for confirmation (HITL)** — LLM runs a tool that
   asks for human input, recognises the prompt, and hands the terminal to
   the user

> **Note**: File listing, reading, and writing are handled by VFS tools
> (`vfs_list`, `vfs_read`, `vfs_write`). Use `run_command` for things VFS
> can't do: compiling, running tests, invoking CLI tools, package management.

---

## What you are building

An agent with thirteen sandbox tools (backed by [`expectrl`] for
cross-platform PTY support), automatically wired via `.with_sandbox(config)`:

| Tool | LLM calls it to... |
|------|--------------------|
| `run_command` | Execute a command (oneshot or background) |
| `open_shell` | Start an interactive PTY session |
| `shell_write` | Send keystrokes to a PTY session |
| `shell_read` | Read available output (non-blocking) |
| `shell_expect` | Wait for a regex pattern (with capture groups) |
| `shell_expect_cases` | Wait for one of N patterns (switch/case) |
| `shell_batch` | Run a send/expect sequence in one call |
| `shell_signal` | Send an OS signal (Ctrl-C, SIGTERM, etc.) |
| `list_processes` | See all running processes |
| `wait_for_process` | Block until a process exits |
| `read_process_output` | Read captured stdout/stderr |
| `kill_process` | Send a signal to a process |
| `process_stats` | Get CPU/memory/status for a process |

[`expectrl`]: https://crates.io/crates/expectrl

---

## Setup

```bash
cargo new synwire-sandbox-demo
cd synwire-sandbox-demo
```

```toml
[dependencies]
synwire = { path = "../../crates/synwire", features = ["sandbox"] }
synwire-core = { path = "../../crates/synwire-core" }
tokio = { version = "1", features = ["full"] }
```

### Build the agent

```rust,ignore
use synwire::agent::prelude::*;
use synwire::sandbox::SandboxedAgent;
use synwire_core::agents::sandbox::{
    FilesystemConfig, IsolationLevel, NetworkConfig, ProcessTracking,
    ResourceLimits, SandboxConfig, SecurityPreset, SecurityProfile,
};

let config = SandboxConfig {
    enabled: true,
    isolation: IsolationLevel::Namespace,
    filesystem: Some(FilesystemConfig {
        allow_write: vec![".".into()],
        deny_write: vec![],
        deny_read: vec![],
        inherit_readable: true,
    }),
    network: Some(NetworkConfig {
        enabled: false,
        ..Default::default()
    }),
    security: SecurityProfile {
        standard: SecurityPreset::Baseline,
        ..Default::default()
    },
    resources: Some(ResourceLimits {
        memory_bytes: Some(512 * 1024 * 1024),
        cpu_quota: Some(1.0),
        max_pids: Some(64),
        exec_timeout_secs: Some(30),
    }),
    process_tracking: ProcessTracking {
        enabled: true,
        max_tracked: Some(64),
    },
    ..Default::default()
};

// with_sandbox() does all the wiring:
// - Finds runc on $PATH
// - Creates ProcessRegistry + ProcessVisibilityScope
// - Registers ProcessPlugin with all 9 tools
// - Sets the sandbox config on the agent
let (agent, handle) = Agent::<()>::new("sandbox-agent", "gpt-4")
    .description("Agent with sandboxed command execution")
    .max_turns(20)
    .with_sandbox(config);

// handle.registry and handle.scope are available for sub-agent wiring
```

That's it for setup. The LLM now has all nine tools available. The rest of
this tutorial shows what the LLM's tool calls look like in each scenario.

---

## Scenario 1: Oneshot command — compile and check

The LLM has edited a Rust file via VFS tools and wants to check if it
compiles. It runs `cargo check` as a oneshot command:

```json
{
  "tool": "run_command",
  "input": {
    "command": "cargo",
    "args": ["check", "--message-format=json"],
    "wait": true,
    "timeout_secs": 60
  }
}
```

The tool spawns the command inside a namespace container, waits for it to
exit, and returns:

```json
{
  "pid": 42,
  "exit_code": 1,
  "timed_out": false,
  "stdout": "{\"reason\":\"compiler-message\",\"message\":{\"rendered\":\"error[E0308]: mismatched types\\n  --> src/main.rs:14:5\\n   |\\n14 |     42u32\\n   |     ^^^^^ expected `String`, found `u32`\\n\"}}",
  "stderr": "error: could not compile `myproject` (bin \"myproject\") due to 1 previous error"
}
```

The LLM parses the compiler JSON, identifies the type mismatch at
`src/main.rs:14`, uses VFS tools to fix the code, then runs
`cargo check` again. One tool call per compilation attempt — the
simplest path.

### How it works internally

1. `run_command` translates `SandboxConfig` to an OCI runtime spec
2. Calls `runc run --bundle <tmpdir> <id>` with stdout/stderr redirected to
   files (so output survives even if the process is killed)
3. Waits up to `timeout_secs` for the process to exit
4. Reads the captured output files
5. Returns everything in one JSON response

If `timeout_secs` is exceeded, the process is killed and `timed_out: true`
is returned.

---

## Scenario 2: Long-lived command with polling — test suite

The LLM starts the full test suite which takes a while. It uses
`wait: false` to get a PID back immediately and monitors progress:

**Turn 1** — start the tests:

```json
{
  "tool": "run_command",
  "input": {
    "command": "cargo",
    "args": ["nextest", "run", "--no-fail-fast"],
    "wait": false
  }
}
```

Response:

```json
{
  "pid": 87,
  "status": "running",
  "hint": "Use wait_for_process to block until exit, or read_process_output to read partial output."
}
```

**Turn 2** — check if it's done yet:

```json
{
  "tool": "wait_for_process",
  "input": {
    "pid": 87,
    "timeout_ms": 10000
  }
}
```

Response (still running after 10s):

```json
{
  "pid": 87,
  "status": "timeout",
  "message": "process still running after 10000ms"
}
```

**Turn 3** — read partial output to see progress:

```json
{
  "tool": "read_process_output",
  "input": {
    "pid": 87,
    "stream": "stderr"
  }
}
```

Response:

```text
    Starting 47 tests across 8 binaries
        PASS [   0.234s] synwire-core::tools::test_tool_schema_validation
        PASS [   0.567s] synwire-core::tools::test_structured_tool_invoke
        PASS [   1.123s] synwire-orchestrator::test_graph_compilation
     RUNNING synwire-sandbox::test_namespace_spawn_echo
```

The LLM sees tests are progressing and decides to wait longer.

**Turn 4** — wait for completion:

```json
{
  "tool": "wait_for_process",
  "input": {
    "pid": 87,
    "timeout_ms": 120000
  }
}
```

Response:

```json
{
  "pid": 87,
  "status": "exited",
  "exit_code": 0
}
```

**Turn 5** — read final output:

```json
{
  "tool": "read_process_output",
  "input": {
    "pid": 87,
    "stream": "stderr"
  }
}
```

```text
    Starting 47 tests across 8 binaries
        ...
     Summary [  12.456s] 47 tests run: 47 passed, 0 failed, 0 skipped
```

### Key points

- `wait: false` returns immediately — the process runs in background
- `monitor_child` updates the registry when the process exits
- `read_process_output` reads from files on disk, so output is available
  even while the process is still running (buffered by the OS)
- The LLM controls the polling loop through its FSM turns — it can do
  other work between polls
- Output files are in a `TempDir` — automatically cleaned up when the last
  `Arc<CapturedOutput>` is dropped (Go `defer` semantics)

---

## Scenario 3: CLI prompts for confirmation (HITL)

The LLM needs to run `terraform apply` which prompts the user to type
`yes` before making changes. The LLM can't (and shouldn't) type the
confirmation itself — it uses `shell_expect` to detect the prompt, then
hands off to the user.

**Turn 1** — open a shell and run terraform:

```json
{
  "tool": "open_shell",
  "input": {
    "shell": "/bin/sh"
  }
}
```

Response:

```json
{
  "session_id": "a1b2c3d4-...",
  "shell": "/bin/sh",
  "hint": "Use shell_write to send commands and shell_read to see output."
}
```

**Turn 2** — start the terraform apply:

```json
{
  "tool": "shell_write",
  "input": {
    "session_id": "a1b2c3d4-...",
    "input": "terraform apply -no-color\n"
  }
}
```

**Turn 3** — wait for the confirmation prompt using `shell_expect`:

```json
{
  "tool": "shell_expect",
  "input": {
    "session_id": "a1b2c3d4-...",
    "pattern": "Enter a value:",
    "timeout_secs": 120
  }
}
```

`shell_expect` reads from the PTY in a loop until "Enter a value:" appears
in the accumulated output, then returns everything captured up to the
match:

```json
{
  "matched": true,
  "pattern": "Enter a value:",
  "output": "$ terraform apply -no-color\n\nTerraform will perform the following actions:\n\n  # aws_instance.web will be created\n  + resource \"aws_instance\" \"web\" {\n      + ami           = \"ami-0c55b159cbfafe1f0\"\n      + instance_type = \"t2.micro\"\n      + tags          = {\n          + \"Name\" = \"production-web\"\n        }\n    }\n\nPlan: 1 to add, 0 to change, 0 to destroy.\n\nDo you want to perform these actions?\n  Terraform will perform the actions described above.\n  Only 'yes' will be accepted to approve.\n\n  Enter a value: "
}
```

The LLM gets `matched: true` and the full plan output in a single tool
call — no polling loop needed.

**Turn 4** — the LLM recognises this is a confirmation prompt and
**hands off to the user**:

The LLM responds to the user (not a tool call):

> Terraform wants to create an `aws_instance.web` (t2.micro in
> production). The plan is:
>
> - **+1 resource**: `aws_instance.web`
> - No changes or deletions
>
> The terminal is waiting for you to type `yes` to approve or anything
> else to cancel. Please type your response in the shell session.

The user types `yes` directly in the PTY (the controller fd is connected
to their terminal via the `SandboxHandle`).

**Turn 5** — wait for terraform to finish applying:

```json
{
  "tool": "shell_expect",
  "input": {
    "session_id": "a1b2c3d4-...",
    "pattern": "Apply complete!",
    "timeout_secs": 300
  }
}
```

Response:

```json
{
  "matched": true,
  "pattern": "Apply complete!",
  "output": "  Enter a value: yes\n\naws_instance.web: Creating...\naws_instance.web: Still creating... [10s elapsed]\naws_instance.web: Creation complete after 23s [id=i-0a1b2c3d4e5f67890]\n\nApply complete! Resources: 1 added, 0 changed, 0 destroyed.\n"
}
```

The LLM confirms the apply succeeded. Two `shell_expect` calls replaced
what would have been a manual polling loop of `shell_read` calls.

### Why this matters

The LLM never types `yes` itself — it **reads** the prompt, **explains**
it to the user, and **waits** for the user to confirm. This pattern
applies to any CLI that requires human confirmation:

- `terraform apply` / `terraform destroy`
- `kubectl delete` with `--confirm`
- `apt upgrade` / `dnf update`
- SSH host key verification
- GPG key trust decisions
- Database migration tools (`diesel migration run --confirm`)
- Any interactive installer

### How PTY handoff works

```text
LLM agent                     synwire                     runc
    │                            │                           │
    │── open_shell() ──────────▶│                           │
    │                            │── bind(console.sock) ───▶│
    │                            │── spawn("runc run        │
    │                            │    --console-socket ...")─▶│
    │                            │                           │── create PTY
    │                            │                           │── setsid + TIOCSCTTY
    │                            │◀── SCM_RIGHTS(pty_fd) ───│
    │◀── {session_id} ──────────│                           │
    │                            │                           │
    │── shell_write(cmd) ──────▶│── write(pty_fd) ────────▶│── container stdin
    │── shell_expect(           │                           │
    │     "Enter a value:")────▶│── read loop ◀────────────│── container stdout
    │                            │   (polls every 100ms)    │
    │◀── {matched: true,        │                           │
    │     output: "Plan: 1...   │                           │
    │     Enter a value:"}──────│                           │
    │                            │                           │
    │ "Please type yes to       │                           │
    │  confirm in the terminal" │                           │
    │                            │                           │
    │                       USER │── write(pty_fd) ────────▶│── "yes\n"
    │                            │                           │── applies changes
    │── shell_expect(           │                           │
    │     "Apply complete!") ──▶│── read loop ◀────────────│── "Apply complete!"
    │◀── {matched: true} ──────│                           │
```

### When to use each mode

| Scenario | Tool | Why |
|----------|------|-----|
| Compile, lint, format | `run_command(wait: true)` | One call, one answer |
| Test suite, long build | `run_command(wait: false)` + polling | LLM controls timing |
| Detect CLI prompt | `shell_expect(pattern)` | Blocks until pattern appears — no polling loop |
| CLI asks for confirmation | `shell_expect` → hand to user | User types the approval |
| Multiple possible outcomes | `shell_expect_cases` | Match first of N patterns with flow control |
| Multi-step scripted flow | `shell_batch` | Send/expect sequence in one call |
| Cancel a running command | `shell_signal("SIGINT")` | Ctrl-C equivalent |
| SSH/GPG key prompts | `shell_expect("password:")` → hand to user | Secrets stay in the PTY |
| Raw PTY I/O | `shell_write` + `shell_read` | When `expect` patterns are unknown |
| File listing, read, write | VFS tools | No sandbox needed |

---

## Scenario 4: Switch/case with `shell_expect_cases`

The LLM runs `ssh user@host` and doesn't know whether it will get a
password prompt, a host key verification prompt, or a shell prompt
(already authenticated). `shell_expect_cases` handles all three:

```json
{
  "tool": "shell_expect_cases",
  "input": {
    "session_id": "a1b2c3d4-...",
    "cases": [
      {
        "pattern": "password:",
        "tag": "needs_user",
        "label": "Password prompt — hand off to user"
      },
      {
        "pattern": "Are you sure you want to continue connecting",
        "tag": "needs_user",
        "label": "Host key verification — hand off to user"
      },
      {
        "pattern": "\\$\\s*$",
        "tag": "ok",
        "label": "Shell prompt — already authenticated"
      }
    ],
    "timeout_secs": 30
  }
}
```

Response (password prompt was first):

```json
{
  "matched": true,
  "matched_case": 0,
  "tag": "needs_user",
  "label": "Password prompt — hand off to user",
  "output": "user@host's password: ",
  "captures": ["password:"]
}
```

The LLM sees `tag: "needs_user"` and tells the user to type their
password. If the `tag` had been `"ok"`, the LLM would continue
autonomously.

### Auto-response with capture groups

Cases can include a `respond` field for auto-responses. Use `$1`, `$2` to
substitute captured regex groups:

```json
{
  "cases": [
    {
      "pattern": "version (\\d+\\.\\d+)",
      "tag": "ok",
      "respond": "Detected version $1\n",
      "label": "Version detected"
    }
  ]
}
```

---

## Scenario 5: Scripted interaction with `shell_batch`

The LLM needs to run a multi-step CLI interaction in one tool call — no
round-trips. `shell_batch` runs send/expect steps sequentially:

```json
{
  "tool": "shell_batch",
  "input": {
    "session_id": "a1b2c3d4-...",
    "steps": [
      { "type": "send", "input": "git status --porcelain\n" },
      { "type": "expect", "pattern": "\\$\\s*$", "timeout_secs": 5 },
      { "type": "send", "input": "cargo test --no-fail-fast 2>&1 | tail -5\n" },
      { "type": "expect", "pattern": "test result:", "timeout_secs": 120 }
    ],
    "timeout_secs": 30
  }
}
```

Response:

```json
{
  "steps": [
    { "index": 0, "step_type": "send", "success": true },
    {
      "index": 1, "step_type": "expect", "success": true,
      "output": "$ git status --porcelain\n M src/lib.rs\n?? src/new_file.rs\n$ ",
      "captures": ["$ "]
    },
    { "index": 2, "step_type": "send", "success": true },
    {
      "index": 3, "step_type": "expect", "success": true,
      "output": "$ cargo test ...\ntest result: ok. 47 passed; 0 failed; 0 ignored",
      "captures": ["test result:"]
    }
  ],
  "completed": 4,
  "total": 4
}
```

The LLM gets both the working tree status and test results in a single
tool call. If any step fails (timeout or expect error), execution stops
and the partial results are returned.

### Batch with switch/case

Batches support `expect_cases` steps for branching:

```json
{
  "steps": [
    { "type": "send", "input": "npm publish\n" },
    {
      "type": "expect_cases",
      "cases": [
        { "pattern": "Enter OTP:", "tag": "needs_user", "label": "2FA required" },
        { "pattern": "npm ERR!", "tag": "fail", "label": "Publish failed" },
        { "pattern": "\\+ my-package@", "tag": "ok", "label": "Published successfully" }
      ],
      "timeout_secs": 60
    }
  ]
}
```

---

## Parent-child visibility

When a parent agent spawns sub-agents, each gets its own
`ProcessRegistry`. The parent can see all sub-agent processes; sub-agents
can only see their own:

```rust,ignore
let (parent_agent, parent_handle) = Agent::<()>::new("parent", "gpt-4")
    .with_sandbox(config.clone());

let child_registry = Arc::new(RwLock::new(ProcessRegistry::new(Some(16))));
parent_handle.scope
    .add_child_registry("research-agent", Arc::clone(&child_registry))
    .await;
```

| Operation | Parent sees | Child sees |
|-----------|:-----------:|:----------:|
| `list_processes` | own + child | own only |
| `read_process_output` | own + child | own only |
| `kill_process` | own only | own only |

---

## Next steps

- **Sandbox setup**: [Process Sandboxing](../how-to/process-sandbox.md) —
  cgroup delegation, gVisor, WSL2, macOS Seatbelt
- **Approval gates**: [Approval Gates](../how-to/approval-gates.md) —
  require human approval before executing commands
- **Permission modes**: [Permission Modes](../how-to/permission-modes.md) —
  control which tools need approval
- **VFS operations**: [File and Shell Operations](./05-backend-operations.md) —
  the virtual filesystem layer above the sandbox
