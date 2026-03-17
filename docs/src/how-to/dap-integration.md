# How to: Integrate Debug Adapters

**Goal:** Give your agent debugging capabilities -- set breakpoints, step through code, inspect variables -- via the Debug Adapter Protocol (DAP).

---

## Quick start

Add `synwire-dap` to your workspace dependencies and register the `DapPlugin` on the agent builder.

```toml
[dependencies]
synwire-dap = { version = "0.1" }
```

```rust,ignore
use synwire_dap::plugin::DapPlugin;
use synwire_dap::config::DapPluginConfig;

let dap = DapPlugin::new(DapPluginConfig::default());

let agent = Agent::new("debugger", "debugging assistant")
    .plugin(Box::new(dap))
    .build()?;
```

The plugin registers these tools: `debug.launch`, `debug.attach`, `debug.set_breakpoints`, `debug.continue`, `debug.step_over`, `debug.step_in`, `debug.step_out`, `debug.variables`, `debug.evaluate`, `debug.stack_trace`, and `debug.disconnect`.

---

## Launch vs attach

DAP supports two modes for starting a debug session.

### Launch mode

The plugin spawns the debug adapter and the target program together.

```rust,ignore
// The model calls:
//   debug.launch {
//     adapter: "dlv-dap",
//     program: "./cmd/myapp",
//     args: ["--config", "dev.yaml"],
//     cwd: "/home/user/project"
//   }
```

### Attach mode

The plugin connects to an already-running process or a debug adapter listening on a port.

```rust,ignore
// The model calls:
//   debug.attach {
//     adapter: "dlv-dap",
//     pid: 12345
//   }
//
// Or attach by address:
//   debug.attach {
//     adapter: "dlv-dap",
//     host: "127.0.0.1",
//     port: 2345
//   }
```

Use launch mode for test debugging. Use attach mode for inspecting running services.

---

## Example: debug a Go test

A typical debugging session with `dlv-dap`:

```rust,ignore
use synwire_dap::plugin::DapPlugin;
use synwire_dap::config::{DapPluginConfig, DapAdapterConfig};
use synwire_agent::Agent;

let config = DapPluginConfig {
    adapters: vec![
        DapAdapterConfig {
            name: "dlv-dap".to_string(),
            command: "dlv".to_string(),
            args: vec!["dap".to_string()],
            languages: vec!["go".to_string()],
        },
    ],
    ..Default::default()
};

let dap = DapPlugin::new(config);
let agent = Agent::new("go-debugger", "Go debugging assistant")
    .plugin(Box::new(dap))
    .build()?;

// The model can now orchestrate a debugging session:
//
//   1. debug.launch { adapter: "dlv-dap", mode: "test", program: "./pkg/auth" }
//   2. debug.set_breakpoints { path: "pkg/auth/token_test.go", line: 42 }
//   3. debug.continue {}
//   4. debug.variables { scope: "local" }
//   5. debug.stack_trace {}
//   6. debug.step_over {}
//   7. debug.variables { scope: "local" }
//   8. debug.disconnect {}
```

The model receives structured data for each response: variable names, types, values, and stack frame locations. It can reason about program state and suggest fixes.

---

## Event handling

When the debuggee hits a breakpoint or throws an exception, the plugin emits a `dap_stopped` signal. Configure automatic inspection so the model receives context without an extra round-trip:

```rust,ignore
use synwire_dap::config::DapPluginConfig;

let config = DapPluginConfig {
    on_stopped: synwire_dap::config::StoppedBehaviour::AutoInspect {
        // Automatically fetch locals and the top 5 stack frames on every stop.
        include_locals: true,
        stack_depth: 5,
    },
    ..Default::default()
};
```

Available `StoppedBehaviour` variants:

| Variant | Effect |
|---------|--------|
| `Notify` | Emit the signal only; the model decides what to inspect |
| `AutoInspect { .. }` | Fetch variables and stack trace automatically, inject into context |
| `Ignore` | Suppress the signal entirely (useful when scripting bulk stepping) |

---

## Security: `debug.evaluate` requires approval

`debug.evaluate` executes arbitrary expressions in the debuggee's runtime. This is marked as `RiskLevel::Critical` and requires explicit approval through the configured approval gate.

```rust,ignore
use synwire_agent::vfs::threshold_gate::ThresholdGate;
use synwire_core::vfs::approval::{RiskLevel, AutoDenyCallback};

// Approve up to High risk automatically; debug.evaluate (Critical) still prompts.
let gate = ThresholdGate::new(RiskLevel::High, CliPrompt);
```

Other DAP tools are classified as follows:

| Risk level | Tools |
|-----------|-------|
| `None` | `debug.variables`, `debug.stack_trace` |
| `Low` | `debug.set_breakpoints`, `debug.continue`, `debug.step_over`, `debug.step_in`, `debug.step_out` |
| `Medium` | `debug.launch`, `debug.attach`, `debug.disconnect` |
| `Critical` | `debug.evaluate` |

---

## Configuration

`DapAdapterConfig` fields:

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Identifier used in tool calls |
| `command` | `String` | Adapter binary name or path |
| `args` | `Vec<String>` | CLI arguments for the adapter process |
| `languages` | `Vec<String>` | File extensions this adapter handles |
| `env` | `Vec<(String, String)>` | Extra environment variables |
| `launch_timeout` | `Duration` | Max time to wait for adapter initialisation (default: 10s) |

`DapPluginConfig` fields:

| Field | Type | Description |
|-------|------|-------------|
| `adapters` | `Vec<DapAdapterConfig>` | Registered debug adapters |
| `on_stopped` | `StoppedBehaviour` | How to handle breakpoint/exception stops |
| `max_concurrent_sessions` | `usize` | Limit simultaneous debug sessions (default: 1) |

---

**See also**

- [Explanation: synwire-dap](../explanation/synwire-dap.md) -- design rationale and DAP protocol mapping
- [How to: Integrate Language Servers](lsp-integration.md) -- LSP plugin for code intelligence
- [How to: Configure Approval Gates](approval-gates.md) -- controlling `debug.evaluate` approval
- [How to: Configure Permission Modes](permission-modes.md) -- tool-level allow/deny rules
