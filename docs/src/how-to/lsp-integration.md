# How to: Integrate Language Servers

**Goal:** Connect your agent to Language Server Protocol servers for code intelligence -- go-to-definition, hover, diagnostics, completions.

---

## Quick start

Add `synwire-lsp` to your workspace dependencies and register the `LspPlugin` on the agent builder.

```toml
[dependencies]
synwire-lsp = { version = "0.1" }
```

```rust,ignore
use synwire_lsp::plugin::LspPlugin;
use synwire_lsp::registry::LanguageServerRegistry;
use synwire_lsp::config::LspPluginConfig;

let registry = LanguageServerRegistry::default_registry();
let lsp = LspPlugin::new(registry, LspPluginConfig::default());

let agent = Agent::new("coder", "coding assistant")
    .plugin(Box::new(lsp))
    .build()?;
```

The plugin registers five tools: `lsp_hover`, `lsp_goto_definition`, `lsp_references`, `lsp_diagnostics`, and `lsp_completion`. It also injects a system prompt telling the model which language servers are available.

---

## Auto-start

`LspPlugin` detects language servers on `PATH` based on file extension. The first time a tool is called for a file, the plugin:

1. Looks up the file extension in the `LanguageServerRegistry`.
2. Checks whether the server binary is available via `which::which()`.
3. Spawns the server in `--stdio` mode if found.
4. Performs the LSP `initialize` / `initialized` handshake.
5. Sends `textDocument/didOpen` for the target file.

Subsequent calls to the same server reuse the running process. The plugin shuts down all servers when the agent session ends.

If no server is found for a language, the tool returns a structured error describing which server is expected and how to install it.

---

## Example: hover and go-to-definition

A typical exchange with `rust-analyzer`:

```rust,ignore
use synwire_lsp::plugin::LspPlugin;
use synwire_lsp::registry::LanguageServerRegistry;
use synwire_lsp::config::LspPluginConfig;
use synwire_agent::Agent;

let registry = LanguageServerRegistry::default_registry();
let lsp = LspPlugin::new(registry, LspPluginConfig::default());

let agent = Agent::new("coder", "Rust coding assistant")
    .plugin(Box::new(lsp))
    .build()?;

// The model can now call:
//   lsp_hover { path: "src/main.rs", line: 42, column: 10 }
//   lsp_goto_definition { path: "src/main.rs", line: 42, column: 10 }
//   lsp_references { path: "src/lib.rs", line: 15, column: 4 }
//   lsp_diagnostics { path: "src/lib.rs" }
//   lsp_completion { path: "src/main.rs", line: 42, column: 10 }
```

The model sees structured results containing type signatures, documentation strings, file locations, and severity-tagged diagnostics.

---

## Document sync with VFS

When the agent writes or edits files through VFS tools, the LSP server must know about the changes to provide accurate results. `LspPlugin` subscribes to VFS write hooks automatically:

- `write` / `append` triggers `textDocument/didOpen` or `textDocument/didChange`.
- `edit` triggers `textDocument/didChange` with incremental edits.
- `rm` triggers `textDocument/didClose`.

This means the model can write code via VFS, then immediately call `lsp_diagnostics` to check for errors -- without manual synchronisation.

```rust,ignore
use synwire_lsp::config::LspPluginConfig;

// Disable automatic sync if you manage notifications yourself.
let config = LspPluginConfig {
    auto_sync_vfs: false,
    ..Default::default()
};
```

---

## Multi-server mode

Agents working across multiple languages spawn one server per language automatically. The plugin routes each tool call to the correct server based on file extension.

```rust,ignore
// The model opens a Go file and a Rust file in the same session.
//   lsp_hover { path: "cmd/main.go", line: 10, column: 5 }   -> gopls
//   lsp_hover { path: "src/lib.rs", line: 20, column: 8 }    -> rust-analyzer
```

Servers are started lazily. A project touching five languages only spawns servers for the languages the model actually queries.

To cap resource usage:

```rust,ignore
let config = LspPluginConfig {
    max_concurrent_servers: 3,
    server_idle_timeout: std::time::Duration::from_secs(300),
    ..Default::default()
};
```

Servers idle beyond `server_idle_timeout` are shut down and restarted on the next request.

---

## Configuration

Use `LspServerConfig` for fine-grained control over individual servers.

```rust,ignore
use synwire_lsp::config::{LspPluginConfig, LspServerConfig};
use synwire_lsp::registry::LanguageServerRegistry;
use std::collections::HashMap;

let mut overrides = HashMap::new();
overrides.insert("rust".to_string(), LspServerConfig {
    command: "rust-analyzer".to_string(),
    args: vec![],
    initialization_options: serde_json::json!({
        "checkOnSave": { "command": "clippy" },
        "cargo": { "allFeatures": true }
    }),
    env: vec![("RUST_LOG".to_string(), "info".to_string())],
    root_uri_override: None,
});

let config = LspPluginConfig {
    server_overrides: overrides,
    ..Default::default()
};

let registry = LanguageServerRegistry::default_registry();
let lsp = LspPlugin::new(registry, config);
```

`LspServerConfig` fields:

| Field | Type | Description |
|-------|------|-------------|
| `command` | `String` | Server binary name or path |
| `args` | `Vec<String>` | CLI arguments appended after the command |
| `initialization_options` | `serde_json::Value` | Sent in the LSP `initialize` request |
| `env` | `Vec<(String, String)>` | Extra environment variables for the server process |
| `root_uri_override` | `Option<String>` | Override the workspace root URI |

---

**See also**

- [Explanation: synwire-lsp](../explanation/synwire-lsp.md) -- design rationale and protocol details
- [How to: Configure Language Servers](langserver-registry.md) -- built-in server list, custom entries, TOML config
- [How to: Integrate Debug Adapters](dap-integration.md) -- DAP plugin for debugging support
- [How to: Use the Virtual Filesystem (VFS)](vfs.md) -- VFS providers and document sync
