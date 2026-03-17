# How to: Configure Language Servers

**Goal:** Discover, install, and configure language servers for your agent's LSP integration.

---

## Built-in servers

`LanguageServerRegistry::default_registry()` ships with 23 entries covering the most common languages. The plugin auto-starts whichever server is found on `PATH` when the model first queries a file of that language.

| Language | Server | Command | Install |
|----------|--------|---------|---------|
| Rust | rust-analyzer | `rust-analyzer` | `rustup component add rust-analyzer` |
| Go | gopls | `gopls serve` | `go install golang.org/x/tools/gopls@latest` |
| Python | pylsp | `pylsp` | `pip install python-lsp-server` |
| Python | pyright | `pyright-langserver --stdio` | `npm install -g pyright` |
| TypeScript/JS | typescript-language-server | `typescript-language-server --stdio` | `npm install -g typescript-language-server typescript` |
| C/C++ | clangd | `clangd` | `apt install clangd` / `brew install llvm` |
| Java | jdtls | `jdtls` | Eclipse JDT.LS manual setup |
| C# | csharp-ls | `csharp-ls` | `dotnet tool install csharp-ls` |
| Ruby | solargraph | `solargraph stdio` | `gem install solargraph` |
| Ruby | ruby-lsp | `ruby-lsp` | `gem install ruby-lsp` |
| Lua | lua-language-server | `lua-language-server` | GitHub releases |
| Bash | bash-language-server | `bash-language-server start` | `npm install -g bash-language-server` |
| YAML | yaml-language-server | `yaml-language-server --stdio` | `npm install -g yaml-language-server` |
| Kotlin | kotlin-language-server | `kotlin-language-server` | GitHub releases |
| Scala | metals | `metals` | `coursier install metals` |
| Haskell | haskell-language-server | `haskell-language-server-wrapper --lsp` | `ghcup install hls` |
| Elixir | elixir-ls | `language_server.sh` | GitHub releases |
| Zig | zls | `zls` | GitHub releases |
| OCaml | ocaml-lsp | `ocamllsp` | `opam install ocaml-lsp-server` |
| Swift | sourcekit-lsp | `sourcekit-lsp` | Bundled with Xcode/Swift toolchain |
| PHP | phpactor | `phpactor language-server` | `composer global require phpactor/phpactor` |
| Terraform | terraform-ls | `terraform-ls serve` | `brew install hashicorp/tap/terraform-ls` |
| Dockerfile | dockerfile-language-server | `docker-langserver --stdio` | `npm install -g dockerfile-language-server-nodejs` |

Languages with two entries (Python, Ruby) use a priority order. The registry tries the first match and falls back to the second if the binary is not found.

---

## Checking availability

Before starting an agent, verify that the servers you need are installed:

```rust,ignore
use synwire_lsp::registry::LanguageServerRegistry;

let registry = LanguageServerRegistry::default_registry();

// Check a single language.
if let Some(entry) = registry.lookup("rust") {
    match which::which(&entry.command) {
        Ok(path) => println!("rust-analyzer found at {}", path.display()),
        Err(_) => eprintln!("rust-analyzer not found; install with: {}", entry.install_hint),
    }
}

// Check all registered languages and report missing servers.
for entry in registry.all_entries() {
    let available = which::which(&entry.command).is_ok();
    println!(
        "{:<20} {:<30} {}",
        entry.language,
        entry.server_name,
        if available { "OK" } else { "MISSING" }
    );
}
```

The `LspPlugin` performs this check lazily at first use. Missing servers produce a structured tool error rather than a panic.

---

## Custom server config via TOML

Define additional servers or override built-in entries in a TOML file:

```toml
# lsp-servers.toml

[[servers]]
language = "nix"
server_name = "nil"
command = "nil"
args = []
install_hint = "nix profile install nixpkgs#nil"
extensions = ["nix"]

[[servers]]
language = "rust"
server_name = "rust-analyzer"
command = "rust-analyzer"
args = []
install_hint = "rustup component add rust-analyzer"
extensions = ["rs"]

[servers.initialization_options]
checkOnSave = { command = "clippy" }
cargo = { allFeatures = true }
```

Load the file into the registry:

```rust,ignore
use synwire_lsp::registry::LanguageServerRegistry;
use std::path::Path;

let mut registry = LanguageServerRegistry::default_registry();
registry.load_toml(Path::new("lsp-servers.toml"))?;
```

Entries with the same `(language, server_name)` pair replace the built-in entry. New language/server pairs are appended.

---

## Custom server config via API

Add entries programmatically when TOML is not convenient:

```rust,ignore
use synwire_lsp::registry::{LanguageServerRegistry, ServerEntry};

let mut registry = LanguageServerRegistry::default_registry();

registry.register(ServerEntry {
    language: "nix".to_string(),
    server_name: "nil".to_string(),
    command: "nil".to_string(),
    args: vec![],
    extensions: vec!["nix".to_string()],
    install_hint: "nix profile install nixpkgs#nil".to_string(),
    initialization_options: serde_json::Value::Null,
    priority: 0,
});
```

Lower `priority` values are tried first. The default entries use priority 0 for the primary server and 10 for alternatives.

---

## Per-language overrides

When multiple servers are registered for the same language, the registry picks the highest-priority (lowest number) server whose binary exists on `PATH`. Override the selection explicitly:

```rust,ignore
use synwire_lsp::registry::LanguageServerRegistry;

let mut registry = LanguageServerRegistry::default_registry();

// Prefer pyright over pylsp for Python files.
registry.set_priority("python", "pyright", 0);
registry.set_priority("python", "pylsp", 10);

// Or disable a server entirely.
registry.disable("python", "pylsp");
```

The `disable` method removes the entry from consideration without deleting it. Re-enable with `registry.enable("python", "pylsp")`.

To check which server would be selected for a language:

```rust,ignore
if let Some(entry) = registry.resolve("python") {
    println!("Python will use: {} ({})", entry.server_name, entry.command);
}
```

`resolve` checks both priority and binary availability, returning the best candidate.

---

## `ServerEntry` fields

| Field | Type | Description |
|-------|------|-------------|
| `language` | `String` | Language identifier (used in registry lookups) |
| `server_name` | `String` | Human-readable server name |
| `command` | `String` | Binary name or absolute path |
| `args` | `Vec<String>` | CLI arguments appended after the command |
| `extensions` | `Vec<String>` | File extensions that map to this server |
| `install_hint` | `String` | Shown to the model when the binary is missing |
| `initialization_options` | `serde_json::Value` | Sent in the LSP `initialize` request |
| `priority` | `u8` | Lower wins when multiple servers match (default: 0) |

---

**See also**

- [How to: Integrate Language Servers](lsp-integration.md) -- using `LspPlugin` with the agent builder
- [Explanation: synwire-lsp](../explanation/synwire-lsp.md) -- architecture and protocol handling
