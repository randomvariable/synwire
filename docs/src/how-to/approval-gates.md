# How to: Configure Approval Gates

**Goal:** Control which operations an agent is allowed to perform without user intervention by wiring approval callbacks.

---

## Core types

An `ApprovalRequest` is passed to the gate whenever a risky operation is about to run:

```rust
pub struct ApprovalRequest {
    pub operation: String,        // e.g. "write_file", "kill_process"
    pub description: String,      // human-readable summary of what will happen
    pub risk: RiskLevel,
    pub timeout_secs: Option<u64>,
    pub context: serde_json::Value, // operation arguments or extra metadata
}
```

`RiskLevel` is an ordered enum (lower to higher):

| Variant | Typical operations |
|---------|-------------------|
| `None` | Read-only (file read, ls, status) |
| `Low` | Reversible writes (write file, edit) |
| `Medium` | Deletions, overwrites |
| `High` | System changes, process spawning |
| `Critical` | Irreversible or destructive |

The callback returns an `ApprovalDecision`:

| Variant | Effect |
|---------|--------|
| `Allow` | Proceed once |
| `Deny` | Block this invocation |
| `AllowAlways` | Proceed and cache approval for this operation name |
| `Abort` | Stop the entire agent run |
| `AllowModified { modified_context }` | Proceed with a rewritten context |

---

## AutoApproveCallback

Approves everything. Use in tests or sandboxed environments where unrestricted execution is acceptable.

```rust
use synwire_core::vfs::approval::AutoApproveCallback;

let gate = AutoApproveCallback;
```

---

## AutoDenyCallback

Denies everything. Use as a safe default when building `ThresholdGate` with an inner callback that should never actually fire.

```rust
use synwire_core::vfs::approval::AutoDenyCallback;

let gate = AutoDenyCallback;
```

---

## ThresholdGate

Auto-approves any operation whose risk is at or below `threshold`. Operations above the threshold are delegated to an inner `ApprovalCallback`. Decisions of `AllowAlways` are cached per operation name so the inner callback is not called again.

```rust
use synwire_agent::vfs::threshold_gate::ThresholdGate;
use synwire_core::vfs::approval::{AutoDenyCallback, RiskLevel};

// Auto-approve up to Medium risk; deny anything Higher automatically.
let gate = ThresholdGate::new(RiskLevel::Medium, AutoDenyCallback);
```

Use with an interactive callback for production:

```rust
use synwire_core::vfs::approval::{ApprovalCallback, ApprovalDecision, ApprovalRequest};
use synwire_core::BoxFuture;

struct CliPrompt;

impl ApprovalCallback for CliPrompt {
    fn request(&self, req: ApprovalRequest) -> BoxFuture<'_, ApprovalDecision> {
        Box::pin(async move {
            eprintln!("[approval] {} — {:?}", req.description, req.risk);
            eprint!("Allow? [y/N/always] ");
            // Read from stdin in a real implementation.
            ApprovalDecision::Allow
        })
    }
}

let gate = ThresholdGate::new(RiskLevel::Low, CliPrompt);
```

---

## Implementing a custom ApprovalCallback

```rust
use synwire_core::vfs::approval::{ApprovalCallback, ApprovalDecision, ApprovalRequest};
use synwire_core::BoxFuture;

struct PolicyCallback {
    allowed_operations: Vec<String>,
}

impl ApprovalCallback for PolicyCallback {
    fn request(&self, req: ApprovalRequest) -> BoxFuture<'_, ApprovalDecision> {
        Box::pin(async move {
            if self.allowed_operations.iter().any(|op| req.operation.starts_with(op)) {
                ApprovalDecision::Allow
            } else {
                ApprovalDecision::Deny
            }
        })
    }
}
```

The `ApprovalCallback` trait requires `Send + Sync`. Use `Arc<Mutex<_>>` for any mutable internal state.

---

## Interplay with PermissionMode

`ThresholdGate` enforces risk-based decisions independently of `PermissionMode`. For rule-based tool-name filtering, see [How to: Configure Permission Modes](permission-modes.md). A typical setup layers both:

1. `PermissionRule` patterns allow or deny by tool name before the operation is submitted.
2. `ThresholdGate` intercepts anything that reaches execution and applies risk thresholds.

---

**See also**

- [How to: Configure Permission Modes](permission-modes.md)
- [How to: Configure the Middleware Stack](middleware.md)
- [Explanation: Architecture](../explanation/architecture.md)

> **Background**: [Context Engineering for AI Agents](https://www.promptingguide.ai/agents/context-engineering) — how to design the context and controls around an AI agent, including approval mechanisms.
