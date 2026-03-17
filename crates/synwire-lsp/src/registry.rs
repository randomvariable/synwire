//! Built-in language server registry with 22+ pre-configured server entries.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

/// A single language server entry in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageServerEntry {
    /// Human-readable server name (e.g. `"rust-analyzer"`).
    pub name: String,
    /// LSP language identifiers this server handles (e.g. `["rust"]`).
    pub language_ids: Vec<String>,
    /// Binary command to launch the server.
    pub command: String,
    /// Default arguments.
    pub args: Vec<String>,
    /// Platform-keyed install instructions (e.g. `{"brew": "brew install ...", "cargo": "..."}`).
    pub install_instructions: HashMap<String, String>,
    /// Homepage URL.
    pub homepage: String,
    /// File extensions this server handles (without leading dot).
    pub file_extensions: Vec<String>,
}

/// Registry of known language servers.
///
/// Use [`LanguageServerRegistry::default_registry`] to get a pre-populated
/// registry with 22+ commonly used language servers.
pub struct LanguageServerRegistry {
    entries: Vec<LanguageServerEntry>,
}

impl std::fmt::Debug for LanguageServerRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LanguageServerRegistry")
            .field("count", &self.entries.len())
            .finish()
    }
}

impl LanguageServerRegistry {
    /// Create a registry pre-populated with well-known language servers.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn default_registry() -> Self {
        let entries = vec![
            entry(
                "rust-analyzer",
                &["rust"],
                "rust-analyzer",
                &[],
                &["rs"],
                "https://rust-analyzer.github.io/",
                &[("rustup", "rustup component add rust-analyzer")],
            ),
            entry(
                "gopls",
                &["go"],
                "gopls",
                &["serve"],
                &["go"],
                "https://pkg.go.dev/golang.org/x/tools/gopls",
                &[("go", "go install golang.org/x/tools/gopls@latest")],
            ),
            entry(
                "pylsp",
                &["python"],
                "pylsp",
                &[],
                &["py", "pyi"],
                "https://github.com/python-lsp/python-lsp-server",
                &[("pip", "pip install python-lsp-server")],
            ),
            entry(
                "pyright",
                &["python"],
                "pyright-langserver",
                &["--stdio"],
                &["py", "pyi"],
                "https://github.com/microsoft/pyright",
                &[("npm", "npm install -g pyright")],
            ),
            entry(
                "typescript-language-server",
                &[
                    "typescript",
                    "typescriptreact",
                    "javascript",
                    "javascriptreact",
                ],
                "typescript-language-server",
                &["--stdio"],
                &["ts", "tsx", "js", "jsx", "mjs", "cjs"],
                "https://github.com/typescript-language-server/typescript-language-server",
                &[(
                    "npm",
                    "npm install -g typescript-language-server typescript",
                )],
            ),
            entry(
                "clangd",
                &["c", "cpp", "objective-c", "objective-cpp"],
                "clangd",
                &[],
                &["c", "h", "cpp", "hpp", "cc", "cxx", "hxx", "m", "mm"],
                "https://clangd.llvm.org/",
                &[("apt", "apt install clangd"), ("brew", "brew install llvm")],
            ),
            entry(
                "jdtls",
                &["java"],
                "jdtls",
                &[],
                &["java"],
                "https://github.com/eclipse-jdtls/eclipse.jdt.ls",
                &[("brew", "brew install jdtls")],
            ),
            entry(
                "omnisharp",
                &["csharp"],
                "OmniSharp",
                &["--languageserver"],
                &["cs", "csx"],
                "https://github.com/OmniSharp/omnisharp-roslyn",
                &[("dotnet", "dotnet tool install -g omnisharp")],
            ),
            entry(
                "solargraph",
                &["ruby"],
                "solargraph",
                &["stdio"],
                &["rb", "gemspec", "rake"],
                "https://solargraph.org/",
                &[("gem", "gem install solargraph")],
            ),
            entry(
                "ruby-lsp",
                &["ruby"],
                "ruby-lsp",
                &[],
                &["rb", "gemspec", "rake"],
                "https://github.com/Shopify/ruby-lsp",
                &[("gem", "gem install ruby-lsp")],
            ),
            entry(
                "lua-language-server",
                &["lua"],
                "lua-language-server",
                &[],
                &["lua"],
                "https://github.com/LuaLS/lua-language-server",
                &[("brew", "brew install lua-language-server")],
            ),
            entry(
                "bash-language-server",
                &["shellscript"],
                "bash-language-server",
                &["start"],
                &["sh", "bash", "zsh"],
                "https://github.com/bash-lsp/bash-language-server",
                &[("npm", "npm install -g bash-language-server")],
            ),
            entry(
                "yaml-language-server",
                &["yaml"],
                "yaml-language-server",
                &["--stdio"],
                &["yml", "yaml"],
                "https://github.com/redhat-developer/yaml-language-server",
                &[("npm", "npm install -g yaml-language-server")],
            ),
            entry(
                "kotlin-language-server",
                &["kotlin"],
                "kotlin-language-server",
                &[],
                &["kt", "kts"],
                "https://github.com/fwcd/kotlin-language-server",
                &[("brew", "brew install kotlin-language-server")],
            ),
            entry(
                "metals",
                &["scala"],
                "metals",
                &[],
                &["scala", "sc", "sbt"],
                "https://scalameta.org/metals/",
                &[("coursier", "cs install metals")],
            ),
            entry(
                "haskell-language-server",
                &["haskell"],
                "haskell-language-server-wrapper",
                &["--lsp"],
                &["hs", "lhs"],
                "https://haskell-language-server.readthedocs.io/",
                &[("ghcup", "ghcup install hls")],
            ),
            entry(
                "elixir-ls",
                &["elixir"],
                "elixir-ls",
                &[],
                &["ex", "exs"],
                "https://github.com/elixir-lsp/elixir-ls",
                &[(
                    "mix",
                    "See https://github.com/elixir-lsp/elixir-ls#installation",
                )],
            ),
            entry(
                "zls",
                &["zig"],
                "zls",
                &[],
                &["zig"],
                "https://github.com/zigtools/zls",
                &[("zig", "See https://github.com/zigtools/zls#installation")],
            ),
            entry(
                "ocamllsp",
                &["ocaml"],
                "ocamllsp",
                &[],
                &["ml", "mli"],
                "https://github.com/ocaml/ocaml-lsp",
                &[("opam", "opam install ocaml-lsp-server")],
            ),
            entry(
                "sourcekit-lsp",
                &["swift"],
                "sourcekit-lsp",
                &[],
                &["swift"],
                "https://github.com/apple/sourcekit-lsp",
                &[("xcode", "Included with Xcode")],
            ),
            entry(
                "intelephense",
                &["php"],
                "intelephense",
                &["--stdio"],
                &["php"],
                "https://intelephense.com/",
                &[("npm", "npm install -g intelephense")],
            ),
            entry(
                "terraform-ls",
                &["terraform"],
                "terraform-ls",
                &["serve"],
                &["tf", "tfvars"],
                "https://github.com/hashicorp/terraform-ls",
                &[("brew", "brew install hashicorp/tap/terraform-ls")],
            ),
            entry(
                "dockerfile-language-server",
                &["dockerfile"],
                "docker-langserver",
                &["--stdio"],
                &["Dockerfile"],
                "https://github.com/rcjsuen/dockerfile-language-server-nodejs",
                &[("npm", "npm install -g dockerfile-language-server-nodejs")],
            ),
        ];

        Self { entries }
    }

    /// Create an empty registry.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Find a server entry by file extension (without the leading dot).
    #[must_use]
    pub fn find_by_extension(&self, ext: &str) -> Option<&LanguageServerEntry> {
        self.entries
            .iter()
            .find(|e| e.file_extensions.iter().any(|fe| fe == ext))
    }

    /// Find a server entry by LSP language identifier.
    #[must_use]
    pub fn find_by_language_id(&self, id: &str) -> Option<&LanguageServerEntry> {
        self.entries
            .iter()
            .find(|e| e.language_ids.iter().any(|lid| lid == id))
    }

    /// Return all entries.
    #[must_use]
    pub fn all_entries(&self) -> &[LanguageServerEntry] {
        &self.entries
    }

    /// Add a custom server entry.
    pub fn add_entry(&mut self, entry: LanguageServerEntry) {
        self.entries.push(entry);
    }

    /// Detect which language servers are available for a project directory.
    ///
    /// Scans the project root up to `max_depth` levels deep for file extensions,
    /// matches them against the registry, checks if the binary is on `PATH` via
    /// `which::which`, and returns all matching entries.
    ///
    /// Supports polyglot repos -- returns multiple entries if multiple languages
    /// are detected (e.g., Rust + TypeScript in a full-stack project).
    #[must_use]
    pub fn detect_for_project(&self, project_root: &Path) -> Vec<&LanguageServerEntry> {
        let extensions = collect_extensions(project_root, 2);
        let mut seen = HashSet::new();
        let mut result = Vec::new();

        for ext in &extensions {
            if let Some(entry) = self.find_by_extension(ext) {
                if seen.insert(&entry.name) && which::which(&entry.command).is_ok() {
                    result.push(entry);
                }
            }
        }
        result
    }

    /// Check whether a specific server binary is available on `PATH`.
    #[must_use]
    pub fn is_available(&self, name: &str) -> bool {
        self.entries
            .iter()
            .any(|e| e.name == name && which::which(&e.command).is_ok())
    }
}

impl Default for LanguageServerRegistry {
    fn default() -> Self {
        Self::default_registry()
    }
}

/// Directories to skip when scanning for file extensions.
///
/// These are non-source directories commonly found in project roots.
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    ".hg",
    ".svn",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    ".tox",
    ".venv",
    "venv",
    ".env",
    "vendor",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    "coverage",
    ".cargo",
    ".rustup",
];

/// Collect unique file extensions from a directory tree up to `max_depth` levels.
///
/// Skips hidden directories (starting with `.`) and common non-source
/// directories listed in [`SKIP_DIRS`]. Returns extensions without the
/// leading dot.
fn collect_extensions(root: &Path, max_depth: usize) -> Vec<String> {
    let mut extensions = HashSet::new();
    collect_extensions_recursive(root, max_depth, 0, &mut extensions);
    extensions.into_iter().collect()
}

/// Recursive helper for [`collect_extensions`].
fn collect_extensions_recursive(
    dir: &Path,
    max_depth: usize,
    current_depth: usize,
    extensions: &mut HashSet<String>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries {
        let Ok(entry) = entry else { continue };

        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if file_type.is_dir() {
            // Skip hidden directories and known non-source directories.
            if name_str.starts_with('.') || SKIP_DIRS.contains(&name_str.as_ref()) {
                continue;
            }
            if current_depth < max_depth {
                collect_extensions_recursive(
                    &entry.path(),
                    max_depth,
                    current_depth + 1,
                    extensions,
                );
            }
        } else if file_type.is_file() {
            if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                let _inserted = extensions.insert(ext.to_owned());
            }
        }
    }
}

/// Helper to build a `LanguageServerEntry` concisely.
fn entry(
    name: &str,
    language_ids: &[&str],
    command: &str,
    args: &[&str],
    file_extensions: &[&str],
    homepage: &str,
    install: &[(&str, &str)],
) -> LanguageServerEntry {
    LanguageServerEntry {
        name: name.into(),
        language_ids: language_ids.iter().map(|s| (*s).into()).collect(),
        command: command.into(),
        args: args.iter().map(|s| (*s).into()).collect(),
        install_instructions: install
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect(),
        homepage: homepage.into(),
        file_extensions: file_extensions.iter().map(|s| (*s).into()).collect(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_has_entries() {
        let reg = LanguageServerRegistry::default_registry();
        assert!(reg.all_entries().len() >= 22);
    }

    #[test]
    fn find_by_extension_rust() {
        let reg = LanguageServerRegistry::default_registry();
        let entry = reg.find_by_extension("rs").unwrap();
        assert_eq!(entry.name, "rust-analyzer");
    }

    #[test]
    fn find_by_extension_go() {
        let reg = LanguageServerRegistry::default_registry();
        let entry = reg.find_by_extension("go").unwrap();
        assert_eq!(entry.name, "gopls");
    }

    #[test]
    fn find_by_language_id() {
        let reg = LanguageServerRegistry::default_registry();
        let entry = reg.find_by_language_id("python").unwrap();
        // Could be pylsp or pyright -- just check one exists.
        assert!(entry.name == "pylsp" || entry.name == "pyright");
    }

    #[test]
    fn find_unknown_returns_none() {
        let reg = LanguageServerRegistry::default_registry();
        assert!(reg.find_by_extension("foobar").is_none());
        assert!(reg.find_by_language_id("foobar").is_none());
    }

    #[test]
    fn add_custom_entry() {
        let mut reg = LanguageServerRegistry::empty();
        reg.add_entry(LanguageServerEntry {
            name: "my-lsp".into(),
            language_ids: vec!["mylang".into()],
            command: "my-lsp-binary".into(),
            args: vec![],
            install_instructions: HashMap::new(),
            homepage: String::new(),
            file_extensions: vec!["myl".into()],
        });
        assert!(reg.find_by_extension("myl").is_some());
    }

    #[test]
    fn collect_extensions_from_temp_dir() {
        let dir = std::env::temp_dir().join("synwire_lsp_test_collect_ext");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.join("src/lib.rs"), "").unwrap();
        std::fs::write(dir.join("package.json"), "{}").unwrap();
        // Hidden dir should be skipped
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::write(dir.join(".git/config"), "").unwrap();

        let exts = super::collect_extensions(&dir, 2);
        assert!(exts.contains(&"rs".to_owned()));
        assert!(exts.contains(&"json".to_owned()));
        // .git/config should not contribute
        assert!(!exts.iter().any(|e| e == "config" || e.is_empty()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_for_project_returns_empty_for_nonexistent_dir() {
        let reg = LanguageServerRegistry::default_registry();
        let result = reg.detect_for_project(std::path::Path::new("/nonexistent/path/12345"));
        assert!(result.is_empty());
    }

    #[test]
    fn is_available_returns_false_for_unknown() {
        let reg = LanguageServerRegistry::default_registry();
        // No server named "nonexistent-server-xyz" exists
        assert!(!reg.is_available("nonexistent-server-xyz"));
    }

    #[test]
    fn all_entries_have_required_fields() {
        let reg = LanguageServerRegistry::default_registry();
        for entry in reg.all_entries() {
            assert!(!entry.name.is_empty(), "entry has no name");
            assert!(
                !entry.command.is_empty(),
                "entry {} has no command",
                entry.name
            );
            assert!(
                !entry.language_ids.is_empty(),
                "entry {} has no language_ids",
                entry.name
            );
            assert!(
                !entry.file_extensions.is_empty(),
                "entry {} has no file_extensions",
                entry.name
            );
        }
    }
}
