//! Debug adapter registry with built-in entries for common languages.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

/// A registered debug adapter entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugAdapterEntry {
    /// Human-readable adapter name.
    pub name: String,
    /// Language identifiers this adapter supports (e.g. `"rust"`, `"go"`).
    pub language_ids: Vec<String>,
    /// Binary command to launch the adapter.
    pub command: String,
    /// Default arguments for the adapter command.
    pub args: Vec<String>,
    /// Per-platform install instructions (keys: `"macos"`, `"linux"`, `"windows"`).
    pub install_instructions: HashMap<String, String>,
    /// Project homepage URL.
    pub homepage: String,
}

/// Registry of known debug adapters.
///
/// Pre-populated with entries for common adapters (`CodeLLDB`, Delve,
/// debugpy, js-debug, java-debug). Additional entries can be registered
/// at runtime.
pub struct DebugAdapterRegistry {
    entries: Vec<DebugAdapterEntry>,
}

impl DebugAdapterRegistry {
    /// Create a registry pre-populated with built-in adapter entries.
    #[must_use]
    pub fn with_builtins() -> Self {
        Self {
            entries: builtin_entries(),
        }
    }

    /// Create an empty registry.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Register a custom adapter entry.
    pub fn register(&mut self, entry: DebugAdapterEntry) {
        self.entries.push(entry);
    }

    /// Look up adapters by language identifier.
    #[must_use]
    pub fn find_by_language(&self, language_id: &str) -> Vec<&DebugAdapterEntry> {
        self.entries
            .iter()
            .filter(|e| e.language_ids.iter().any(|l| l == language_id))
            .collect()
    }

    /// Look up an adapter by name.
    #[must_use]
    pub fn find_by_name(&self, name: &str) -> Option<&DebugAdapterEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// Return all registered entries.
    #[must_use]
    pub fn all(&self) -> &[DebugAdapterEntry] {
        &self.entries
    }

    /// Check whether a specific adapter binary is available on `PATH`.
    #[must_use]
    pub fn is_available(&self, name: &str) -> bool {
        self.find_by_name(name)
            .is_some_and(|entry| which::which(&entry.command).is_ok())
    }

    /// Detect which debug adapters are available for a project directory.
    ///
    /// Scans the project root up to 2 levels deep for file extensions, maps
    /// them to language identifiers, then checks the registry for matching
    /// adapters whose binary is on `PATH`.
    ///
    /// Supports polyglot repos -- returns multiple adapters if multiple
    /// languages are detected.
    #[must_use]
    pub fn detect_for_project(&self, project_root: &Path) -> Vec<&DebugAdapterEntry> {
        let extensions = collect_extensions(project_root, 2);
        let mut language_ids = HashSet::new();
        for ext in &extensions {
            if let Some(lang) = extension_to_language_id(ext) {
                let _inserted = language_ids.insert(lang);
            }
        }

        let mut seen = HashSet::new();
        let mut result = Vec::new();
        for lang_id in &language_ids {
            for entry in self.find_by_language(lang_id) {
                if seen.insert(&entry.name) && which::which(&entry.command).is_ok() {
                    result.push(entry);
                }
            }
        }
        result
    }
}

impl Default for DebugAdapterRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

/// Map a file extension (without leading dot) to a language identifier.
///
/// Returns `None` for unrecognised extensions.
fn extension_to_language_id(ext: &str) -> Option<&'static str> {
    match ext {
        "rs" => Some("rust"),
        "go" => Some("go"),
        "py" | "pyi" => Some("python"),
        "js" | "mjs" | "cjs" => Some("javascript"),
        "ts" | "mts" | "cts" | "tsx" => Some("typescript"),
        "java" => Some("java"),
        "c" | "h" => Some("c"),
        "cpp" | "cxx" | "cc" | "hpp" | "hxx" => Some("cpp"),
        _ => None,
    }
}

/// Directories to skip when scanning for file extensions.
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
/// Skips hidden directories and common non-source directories listed in
/// [`SKIP_DIRS`]. Returns extensions without the leading dot.
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

/// Built-in adapter entries for the five supported adapters.
#[allow(clippy::too_many_lines)] // Data definition -- splitting would reduce clarity.
fn builtin_entries() -> Vec<DebugAdapterEntry> {
    vec![
        DebugAdapterEntry {
            name: "codelldb".into(),
            language_ids: vec!["rust".into(), "c".into(), "cpp".into()],
            command: "codelldb".into(),
            args: vec!["--port".into(), "0".into()],
            install_instructions: {
                let mut m = HashMap::new();
                let _ = m.insert(
                    "linux".into(),
                    "Download from https://github.com/vadimcn/codelldb/releases".into(),
                );
                let _ = m.insert(
                    "macos".into(),
                    "Download from https://github.com/vadimcn/codelldb/releases".into(),
                );
                let _ = m.insert(
                    "windows".into(),
                    "Download from https://github.com/vadimcn/codelldb/releases".into(),
                );
                m
            },
            homepage: "https://github.com/vadimcn/codelldb".into(),
        },
        DebugAdapterEntry {
            name: "dlv-dap".into(),
            language_ids: vec!["go".into()],
            command: "dlv".into(),
            args: vec!["dap".into()],
            install_instructions: {
                let mut m = HashMap::new();
                let _ = m.insert(
                    "linux".into(),
                    "go install github.com/go-delve/delve/cmd/dlv@latest".into(),
                );
                let _ = m.insert(
                    "macos".into(),
                    "go install github.com/go-delve/delve/cmd/dlv@latest".into(),
                );
                let _ = m.insert(
                    "windows".into(),
                    "go install github.com/go-delve/delve/cmd/dlv@latest".into(),
                );
                m
            },
            homepage: "https://github.com/go-delve/delve".into(),
        },
        DebugAdapterEntry {
            name: "debugpy".into(),
            language_ids: vec!["python".into()],
            command: "python".into(),
            args: vec!["-m".into(), "debugpy.adapter".into()],
            install_instructions: {
                let mut m = HashMap::new();
                let _ = m.insert("linux".into(), "pip install debugpy".into());
                let _ = m.insert("macos".into(), "pip install debugpy".into());
                let _ = m.insert("windows".into(), "pip install debugpy".into());
                m
            },
            homepage: "https://github.com/microsoft/debugpy".into(),
        },
        DebugAdapterEntry {
            name: "js-debug".into(),
            language_ids: vec!["javascript".into(), "typescript".into(), "node".into()],
            command: "js-debug-adapter".into(),
            args: Vec::new(),
            install_instructions: {
                let mut m = HashMap::new();
                let _ = m.insert("linux".into(), "npm install -g @vscode/js-debug".into());
                let _ = m.insert("macos".into(), "npm install -g @vscode/js-debug".into());
                let _ = m.insert("windows".into(), "npm install -g @vscode/js-debug".into());
                m
            },
            homepage: "https://github.com/microsoft/vscode-js-debug".into(),
        },
        DebugAdapterEntry {
            name: "java-debug".into(),
            language_ids: vec!["java".into()],
            command: "java".into(),
            args: vec!["-agentlib:jdwp=transport=dt_socket,server=y,suspend=n".into()],
            install_instructions: {
                let mut m = HashMap::new();
                let _ = m.insert(
                    "linux".into(),
                    "Install via JDT.LS: https://github.com/microsoft/java-debug".into(),
                );
                let _ = m.insert(
                    "macos".into(),
                    "Install via JDT.LS: https://github.com/microsoft/java-debug".into(),
                );
                let _ = m.insert(
                    "windows".into(),
                    "Install via JDT.LS: https://github.com/microsoft/java-debug".into(),
                );
                m
            },
            homepage: "https://github.com/microsoft/java-debug".into(),
        },
    ]
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn builtin_registry_has_five_entries() {
        let registry = DebugAdapterRegistry::with_builtins();
        assert_eq!(registry.all().len(), 5);
    }

    #[test]
    fn find_by_language_rust() {
        let registry = DebugAdapterRegistry::with_builtins();
        let adapters = registry.find_by_language("rust");
        assert_eq!(adapters.len(), 1);
        assert_eq!(adapters[0].name, "codelldb");
    }

    #[test]
    fn find_by_language_go() {
        let registry = DebugAdapterRegistry::with_builtins();
        let adapters = registry.find_by_language("go");
        assert_eq!(adapters.len(), 1);
        assert_eq!(adapters[0].name, "dlv-dap");
    }

    #[test]
    fn find_by_name() {
        let registry = DebugAdapterRegistry::with_builtins();
        assert!(registry.find_by_name("debugpy").is_some());
        assert!(registry.find_by_name("nonexistent").is_none());
    }

    #[test]
    fn extension_to_language_id_known() {
        assert_eq!(super::extension_to_language_id("rs"), Some("rust"));
        assert_eq!(super::extension_to_language_id("go"), Some("go"));
        assert_eq!(super::extension_to_language_id("py"), Some("python"));
        assert_eq!(super::extension_to_language_id("ts"), Some("typescript"));
        assert_eq!(super::extension_to_language_id("cpp"), Some("cpp"));
    }

    #[test]
    fn extension_to_language_id_unknown() {
        assert_eq!(super::extension_to_language_id("xyz"), None);
        assert_eq!(super::extension_to_language_id(""), None);
    }

    #[test]
    fn detect_for_project_returns_empty_for_nonexistent_dir() {
        let registry = DebugAdapterRegistry::with_builtins();
        let result = registry.detect_for_project(std::path::Path::new("/nonexistent/path/12345"));
        assert!(result.is_empty());
    }

    #[test]
    fn collect_extensions_from_temp_dir() {
        let dir = std::env::temp_dir().join("synwire_dap_test_collect_ext");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("main.go"), "package main").unwrap();
        std::fs::write(dir.join("src/helper.py"), "").unwrap();

        let exts = super::collect_extensions(&dir, 2);
        assert!(exts.contains(&"go".to_owned()));
        assert!(exts.contains(&"py".to_owned()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn register_custom_entry() {
        let mut registry = DebugAdapterRegistry::empty();
        assert!(registry.all().is_empty());

        registry.register(DebugAdapterEntry {
            name: "my-adapter".into(),
            language_ids: vec!["lua".into()],
            command: "lua-debug".into(),
            args: Vec::new(),
            install_instructions: HashMap::new(),
            homepage: "https://example.com".into(),
        });

        assert_eq!(registry.all().len(), 1);
        let found = registry.find_by_language("lua");
        assert_eq!(found.len(), 1);
    }
}
