# How to: Configure Permission Modes

**Goal:** Apply a `PermissionMode` preset and define `PermissionRule` patterns to control which tool operations the agent may perform.

---

## PermissionMode presets

`PermissionMode` is `Copy` and `Default` (defaults to `Default`). It expresses a broad policy for the agent session:

```rust
use synwire_core::agents::permission::PermissionMode;

let mode = PermissionMode::Default;         // prompt for dangerous ops
let mode = PermissionMode::AcceptEdits;     // auto-approve file modifications
let mode = PermissionMode::PlanOnly;        // read-only, no mutations
let mode = PermissionMode::BypassAll;       // auto-approve everything (caution)
let mode = PermissionMode::DenyUnauthorized; // deny unless a pre-approved rule matches
```

| Mode | Behaviour |
|------|-----------|
| `Default` | Allow safe operations; prompt for dangerous ones |
| `AcceptEdits` | Auto-approve write/edit/rm; prompt for higher-risk ops |
| `PlanOnly` | Block all mutations; safe for dry-run or planning phases |
| `BypassAll` | Approve all operations without prompting |
| `DenyUnauthorized` | Deny any operation that has no matching `Allow` rule |

---

## PermissionRule

Rules match tool names using glob patterns and assign a `PermissionBehavior`:

```rust
use synwire_core::agents::permission::{PermissionBehavior, PermissionRule};

let rules = vec![
    // Allow all file reads without prompting.
    PermissionRule {
        tool_pattern: "read_file".to_string(),
        behavior: PermissionBehavior::Allow,
    },
    // Always ask before writing.
    PermissionRule {
        tool_pattern: "write_file".to_string(),
        behavior: PermissionBehavior::Ask,
    },
    // Block process spawning entirely.
    PermissionRule {
        tool_pattern: "spawn_background".to_string(),
        behavior: PermissionBehavior::Deny,
    },
    // Wildcard: allow all git operations.
    PermissionRule {
        tool_pattern: "git_*".to_string(),
        behavior: PermissionBehavior::Allow,
    },
];
```

`PermissionBehavior` values:

| Variant | Meaning |
|---------|---------|
| `Allow` | Permit without prompting |
| `Deny` | Block immediately |
| `Ask` | Delegate to the approval callback |

---

## How rules interact with approval callbacks

Rules are evaluated before the operation reaches an approval gate:

1. The runner matches the tool name against all `PermissionRule` patterns in order.
2. On `Deny` — the operation is blocked immediately; the approval callback is not called.
3. On `Allow` — the operation proceeds; the approval callback is not called.
4. On `Ask` (or no matching rule) — the operation is forwarded to the `ApprovalCallback` (e.g. `ThresholdGate`).

Under `DenyUnauthorized` mode, any tool with no matching rule and no `Ask` result is blocked as if `Deny` were returned.

Under `BypassAll` mode, `Ask` results are treated as `Allow` — the approval callback is still invoked but the answer is ignored.

---

## Serialisation

Both types derive `Serialize` and `Deserialize`, so rules can be loaded from a config file:

```rust
use synwire_core::agents::permission::{PermissionMode, PermissionRule};

let rules: Vec<PermissionRule> = serde_json::from_str(r#"[
    {"tool_pattern": "read_file", "behavior": "Allow"},
    {"tool_pattern": "write_file", "behavior": "Ask"},
    {"tool_pattern": "rm", "behavior": "Deny"}
]"#)?;

let mode: PermissionMode = serde_json::from_str(r#""AcceptEdits""#)?;
```

---

**See also**

- [How to: Configure Approval Gates](approval-gates.md)
- [How to: Configure Signal Routing](signal-routing.md)
- [Explanation: Architecture](../explanation/architecture.md)
