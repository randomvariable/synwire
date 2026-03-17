//! WebAssembly runtime.
//!
//! Executes WASM plugins using the [`extism`] SDK. Plugins receive their input
//! as JSON and can call back into the host via registered host functions.
//!
//! # Host functions available to plugins
//!
//! | Import name | Signature | Description |
//! |-------------|-----------|-------------|
//! | `vfs_read_file` | `(path: string) → string` | Read file within project root |
//! | `vfs_list_dir` | `(path: string) → string` | JSON array of directory entries |
//! | `vfs_stat` | `(path: string) → string` | JSON metadata object |
//! | `vfs_head` | `(path: string, n: i64) → string` | First *n* lines |
//! | `vfs_tail` | `(path: string, n: i64) → string` | Last *n* lines |
//! | `vfs_grep` | `(pattern: string, path: string) → string` | JSON grep results |
//! | `vfs_glob` | `(pattern: string) → string` | JSON path array |
//! | `vfs_find` | `(path: string, name_pat: string) → string` | JSON path array |
//! | `vfs_write` | `(path: string, content: string) → string` | Write file (`"ok"` or `"ERROR: …"`) |
//! | `vfs_append` | `(path: string, content: string) → string` | Append to file |
//! | `host_log` | `(level: string, msg: string)` | Emit tracing log |
//!
//! In addition, `__context` is injected into the input JSON with:
//! - `project_root` — path string
//! - `available_tools` — string array
//!
//! # Example input
//!
//! ```json
//! {
//!     "wasm_path": "/path/to/plugin.wasm",
//!     "function": "run",
//!     "input": { "x": 42 }
//! }
//! ```

use std::path::PathBuf;

use extism::{Function, Manifest, Plugin, UserData, ValType, Wasm};
use serde::Deserialize;
use tracing::debug;

use crate::error::SkillError;
use crate::runtime::path_safety::safe_resolve;
use crate::runtime::{SkillContext, SkillExecutor, SkillInput, SkillOutput};

/// Default entry-point function name called on the WASM plugin.
const DEFAULT_FUNCTION: &str = "run";

/// Parameters extracted from the input JSON for WASM plugin execution.
#[derive(Debug, Deserialize)]
struct WasmArgs {
    /// Path to the `.wasm` plugin file.
    wasm_path: PathBuf,
    /// The function to call within the plugin (defaults to `"run"`).
    #[serde(default)]
    function: Option<String>,
    /// The value to JSON-serialize and pass as input to the plugin.
    #[serde(default)]
    input: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Shared host-function user data
// ---------------------------------------------------------------------------

/// Data shared with all host functions that need filesystem access.
#[derive(Clone)]
struct VfsData {
    root: PathBuf,
    /// Tool names the skill manifest declares as allowed; checked for writes.
    allowed_tools: Vec<String>,
}

impl VfsData {
    fn allows_write(&self) -> bool {
        self.allowed_tools
            .iter()
            .any(|t| t == "write" || t == "vfs.write" || t.contains("write"))
    }
}

// ---------------------------------------------------------------------------
// Host functions (registered with each Plugin)
// ---------------------------------------------------------------------------

extism::host_fn!(host_vfs_read_file(user_data: VfsData; path: String) -> String {
    let data = user_data.get()?;
    let guard = data.lock().map_err(|e| extism::Error::msg(e.to_string()))?;
    let root = guard.root.clone();
    drop(guard);
    safe_resolve(&root, &path)
        .and_then(|r| std::fs::read_to_string(&r).map_err(|e| SkillError::Runtime {
            runtime: "wasm".to_owned(),
            message: e.to_string(),
        }))
        .map_err(|e| extism::Error::msg(e.to_string()))
});

extism::host_fn!(host_vfs_list_dir(user_data: VfsData; path: String) -> String {
    let data = user_data.get()?;
    let guard = data.lock().map_err(|e| extism::Error::msg(e.to_string()))?;
    let root = guard.root.clone();
    drop(guard);
    let resolved = safe_resolve(&root, &path).map_err(|e| extism::Error::msg(e.to_string()))?;
    let names: Vec<String> = std::fs::read_dir(&resolved)
        .map_err(|e| extism::Error::msg(e.to_string()))?
        .filter_map(std::result::Result::ok)
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    serde_json::to_string(&names).map_err(|e| extism::Error::msg(e.to_string()))
});

extism::host_fn!(host_vfs_stat(user_data: VfsData; path: String) -> String {
    let data = user_data.get()?;
    let guard = data.lock().map_err(|e| extism::Error::msg(e.to_string()))?;
    let root = guard.root.clone();
    drop(guard);
    let resolved = safe_resolve(&root, &path).map_err(|e| extism::Error::msg(e.to_string()))?;
    let meta = std::fs::metadata(&resolved).map_err(|e| extism::Error::msg(e.to_string()))?;
    let v = serde_json::json!({
        "size": meta.len(),
        "is_dir": meta.is_dir(),
        "is_file": meta.is_file(),
        "readonly": meta.permissions().readonly(),
    });
    serde_json::to_string(&v).map_err(|e| extism::Error::msg(e.to_string()))
});

extism::host_fn!(host_vfs_head(user_data: VfsData; path: String, n: i64) -> String {
    let data = user_data.get()?;
    let guard = data.lock().map_err(|e| extism::Error::msg(e.to_string()))?;
    let root = guard.root.clone();
    drop(guard);
    let resolved = safe_resolve(&root, &path).map_err(|e| extism::Error::msg(e.to_string()))?;
    let content = std::fs::read_to_string(&resolved).map_err(|e| extism::Error::msg(e.to_string()))?;
    let count = usize::try_from(n.max(0)).unwrap_or(0);
    Ok(content.lines().take(count).collect::<Vec<_>>().join("\n"))
});

extism::host_fn!(host_vfs_tail(user_data: VfsData; path: String, n: i64) -> String {
    let data = user_data.get()?;
    let guard = data.lock().map_err(|e| extism::Error::msg(e.to_string()))?;
    let root = guard.root.clone();
    drop(guard);
    let resolved = safe_resolve(&root, &path).map_err(|e| extism::Error::msg(e.to_string()))?;
    let content = std::fs::read_to_string(&resolved).map_err(|e| extism::Error::msg(e.to_string()))?;
    let all: Vec<&str> = content.lines().collect();
    let count = usize::try_from(n.max(0)).unwrap_or(0);
    let start = all.len().saturating_sub(count);
    Ok(all[start..].join("\n"))
});

extism::host_fn!(host_vfs_grep(user_data: VfsData; pattern: String, path: String) -> String {
    let data = user_data.get()?;
    let guard = data.lock().map_err(|e| extism::Error::msg(e.to_string()))?;
    let root = guard.root.clone();
    drop(guard);
    let resolved = safe_resolve(&root, &path).map_err(|e| extism::Error::msg(e.to_string()))?;
    let mut hits: Vec<serde_json::Value> = Vec::new();
    let collect = |fp: &std::path::Path, pat: &str, acc: &mut Vec<serde_json::Value>| {
        let Ok(c) = std::fs::read_to_string(fp) else { return };
        for (i, line) in c.lines().enumerate() {
            if line.contains(pat) {
                acc.push(serde_json::json!({
                    "file": fp.to_string_lossy(),
                    "line": i + 1,
                    "content": line,
                }));
            }
        }
    };
    if resolved.is_file() {
        collect(&resolved, &pattern, &mut hits);
    } else {
        for e in walkdir::WalkDir::new(&resolved).follow_links(false) {
            let Ok(e) = e else { continue };
            if e.file_type().is_file() {
                collect(e.path(), &pattern, &mut hits);
            }
        }
    }
    serde_json::to_string(&hits).map_err(|e| extism::Error::msg(e.to_string()))
});

extism::host_fn!(host_vfs_glob(user_data: VfsData; pattern: String) -> String {
    let data = user_data.get()?;
    let guard = data.lock().map_err(|e| extism::Error::msg(e.to_string()))?;
    let root = guard.root.clone();
    drop(guard);
    let matcher = globset::GlobBuilder::new(&pattern)
        .literal_separator(true)
        .build()
        .map(|g| g.compile_matcher())
        .map_err(|e| extism::Error::msg(e.to_string()))?;
    let paths: Vec<String> = walkdir::WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter_map(|e| {
            let rel = e.path().strip_prefix(&root).unwrap_or_else(|_| e.path()).to_owned();
            if matcher.is_match(&rel) {
                Some(rel.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();
    serde_json::to_string(&paths).map_err(|e| extism::Error::msg(e.to_string()))
});

extism::host_fn!(host_vfs_find(user_data: VfsData; path: String, name_pat: String) -> String {
    let data = user_data.get()?;
    let guard = data.lock().map_err(|e| extism::Error::msg(e.to_string()))?;
    let root = guard.root.clone();
    let resolved = safe_resolve(&guard.root, &path).map_err(|e| extism::Error::msg(e.to_string()))?;
    drop(guard);
    let paths: Vec<String> = walkdir::WalkDir::new(&resolved)
        .follow_links(false)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter_map(|e| {
            if e.file_name().to_string_lossy().contains(name_pat.as_str()) {
                let rel = e.path().strip_prefix(&root).unwrap_or_else(|_| e.path()).to_owned();
                Some(rel.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();
    serde_json::to_string(&paths).map_err(|e| extism::Error::msg(e.to_string()))
});

extism::host_fn!(host_vfs_write(user_data: VfsData; path: String, content: String) -> String {
    let data = user_data.get()?;
    let guard = data.lock().map_err(|e| extism::Error::msg(e.to_string()))?;
    if !guard.allows_write() {
        return Ok("ERROR: 'write' not in allowed_tools".to_owned());
    }
    let root = guard.root.clone();
    drop(guard);
    match safe_resolve(&root, &path) {
        Err(e) => Ok(format!("ERROR: {e}")),
        Ok(r) => {
            if let Some(p) = r.parent() {
                if let Err(e) = std::fs::create_dir_all(p) {
                    return Ok(format!("ERROR: {e}"));
                }
            }
            Ok(std::fs::write(&r, content.as_bytes())
                .map_or_else(|e| format!("ERROR: {e}"), |()| "ok".to_owned()))
        }
    }
});

extism::host_fn!(host_vfs_append(user_data: VfsData; path: String, content: String) -> String {
    let data = user_data.get()?;
    let guard = data.lock().map_err(|e| extism::Error::msg(e.to_string()))?;
    if !guard.allows_write() {
        return Ok("ERROR: 'write' not in allowed_tools".to_owned());
    }
    let root = guard.root.clone();
    drop(guard);
    match safe_resolve(&root, &path) {
        Err(e) => Ok(format!("ERROR: {e}")),
        Ok(r) => {
            use std::io::Write as _;
            Ok(std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&r)
                .and_then(|mut f| f.write_all(content.as_bytes()))
                .map_or_else(|e| format!("ERROR: {e}"), |()| "ok".to_owned()))
        }
    }
});

extism::host_fn!(host_log(_user_data: (); level: String, msg: String) {
    match level.to_lowercase().as_str() {
        "error" => tracing::error!("{}", msg),
        "warn"  => tracing::warn!("{}", msg),
        "debug" => tracing::debug!("{}", msg),
        "trace" => tracing::trace!("{}", msg),
        _       => tracing::info!("{}", msg),
    }
    Ok(())
});

// ---------------------------------------------------------------------------
// Build host function list
// ---------------------------------------------------------------------------

/// Construct the vector of host [`Function`]s to register with a plugin.
fn build_host_fns(vfs_data: &VfsData) -> Vec<Function> {
    let ud = || UserData::new(vfs_data.clone());
    let log_ud = || UserData::new(());
    vec![
        Function::new(
            "vfs_read_file",
            [ValType::I64],
            [ValType::I64],
            ud(),
            host_vfs_read_file,
        ),
        Function::new(
            "vfs_list_dir",
            [ValType::I64],
            [ValType::I64],
            ud(),
            host_vfs_list_dir,
        ),
        Function::new(
            "vfs_stat",
            [ValType::I64],
            [ValType::I64],
            ud(),
            host_vfs_stat,
        ),
        Function::new(
            "vfs_head",
            [ValType::I64, ValType::I64],
            [ValType::I64],
            ud(),
            host_vfs_head,
        ),
        Function::new(
            "vfs_tail",
            [ValType::I64, ValType::I64],
            [ValType::I64],
            ud(),
            host_vfs_tail,
        ),
        Function::new(
            "vfs_grep",
            [ValType::I64, ValType::I64],
            [ValType::I64],
            ud(),
            host_vfs_grep,
        ),
        Function::new(
            "vfs_glob",
            [ValType::I64],
            [ValType::I64],
            ud(),
            host_vfs_glob,
        ),
        Function::new(
            "vfs_find",
            [ValType::I64, ValType::I64],
            [ValType::I64],
            ud(),
            host_vfs_find,
        ),
        Function::new(
            "vfs_write",
            [ValType::I64, ValType::I64],
            [ValType::I64],
            ud(),
            host_vfs_write,
        ),
        Function::new(
            "vfs_append",
            [ValType::I64, ValType::I64],
            [ValType::I64],
            ud(),
            host_vfs_append,
        ),
        Function::new(
            "host_log",
            [ValType::I64, ValType::I64],
            [],
            log_ud(),
            host_log,
        ),
    ]
}

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

/// Executor that runs WASM plugins via Extism.
///
/// A new [`Plugin`] is instantiated for each execution call from the specified
/// `.wasm` file to ensure isolation between invocations. WASM plugins can call
/// back into the host via the registered host functions listed in the module
/// docs.
#[derive(Debug, Default)]
pub struct WasmRuntime {
    /// Whether the WASM plugin is allowed to use WASI.
    allow_wasi: bool,
}

impl WasmRuntime {
    /// Create a new [`WasmRuntime`] with WASI disabled.
    pub const fn new() -> Self {
        Self { allow_wasi: false }
    }

    /// Create a new [`WasmRuntime`] with WASI access enabled.
    pub const fn with_wasi() -> Self {
        Self { allow_wasi: true }
    }

    fn execute_inner(
        &self,
        wasm_args: WasmArgs,
        context: Option<&SkillContext>,
    ) -> Result<SkillOutput, SkillError> {
        let function = wasm_args.function.as_deref().unwrap_or(DEFAULT_FUNCTION);

        debug!(
            wasm_path = %wasm_args.wasm_path.display(),
            function = function,
            has_context = context.is_some(),
            "loading and executing wasm plugin"
        );

        if !wasm_args.wasm_path.exists() {
            return Err(SkillError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("wasm file not found: {}", wasm_args.wasm_path.display()),
            )));
        }

        let wasm = Wasm::file(&wasm_args.wasm_path);
        let manifest = Manifest::new([wasm]);

        // Build host functions with VFS data from context (or defaults).
        let vfs_data = context.map_or_else(
            || VfsData {
                root: PathBuf::new(),
                allowed_tools: Vec::new(),
            },
            |ctx| VfsData {
                root: ctx.project_root.clone(),
                allowed_tools: ctx.available_tools.clone(),
            },
        );
        let host_fns = build_host_fns(&vfs_data);

        let mut plugin =
            Plugin::new(&manifest, host_fns, self.allow_wasi).map_err(|e| SkillError::Runtime {
                runtime: "wasm".to_owned(),
                message: format!("failed to instantiate plugin: {e}"),
            })?;

        // Build input JSON, injecting __context metadata.
        let plugin_input = {
            let mut obj = match wasm_args.input {
                serde_json::Value::Object(map) => map,
                other => {
                    let mut m = serde_json::Map::new();
                    let _ = m.insert("value".to_owned(), other);
                    m
                }
            };
            if let Some(ctx) = context {
                let _ = obj.insert(
                    "__context".to_owned(),
                    serde_json::json!({
                        "project_root": ctx.project_root,
                        "available_tools": ctx.available_tools,
                    }),
                );
            }
            serde_json::Value::Object(obj)
        };

        let input_json = serde_json::to_string(&plugin_input).map_err(|e| SkillError::Runtime {
            runtime: "wasm".to_owned(),
            message: format!("failed to serialize input: {e}"),
        })?;

        let output_str: &str = plugin
            .call::<&str, &str>(function, &input_json)
            .map_err(|e| SkillError::Runtime {
                runtime: "wasm".to_owned(),
                message: format!("plugin call failed: {e}"),
            })?;

        let json_result: serde_json::Value =
            serde_json::from_str(output_str).map_err(|e| SkillError::Runtime {
                runtime: "wasm".to_owned(),
                message: format!("plugin output is not valid JSON: {e}"),
            })?;

        Ok(SkillOutput {
            result: json_result,
        })
    }
}

impl SkillExecutor for WasmRuntime {
    fn execute(&self, input: SkillInput) -> Result<SkillOutput, SkillError> {
        self.execute_with_context(input, None)
    }

    fn execute_with_context(
        &self,
        input: SkillInput,
        context: Option<&SkillContext>,
    ) -> Result<SkillOutput, SkillError> {
        let wasm_args: WasmArgs = serde_json::from_value(input.args)
            .map_err(|e| SkillError::InvalidManifest(format!("invalid wasm runtime args: {e}")))?;
        self.execute_inner(wasm_args, context)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::runtime::SkillInput;

    #[test]
    fn missing_wasm_file_returns_io_error() {
        let err = WasmRuntime::new()
            .execute(SkillInput {
                args: serde_json::json!({"wasm_path": "/nonexistent/plugin.wasm", "input": {}}),
            })
            .expect_err("missing wasm");
        assert!(matches!(err, SkillError::Io(_)));
    }

    #[test]
    fn missing_wasm_path_returns_manifest_error() {
        let err = WasmRuntime::new()
            .execute(SkillInput {
                args: serde_json::json!({"input": {}}),
            })
            .expect_err("missing wasm_path");
        assert!(matches!(err, SkillError::InvalidManifest(_)));
    }

    #[test]
    fn default_function_is_run() {
        let args: WasmArgs = serde_json::from_value(serde_json::json!({
            "wasm_path": "/some/path.wasm",
            "input": {"x": 1}
        }))
        .expect("deserialize");
        assert!(args.function.is_none());
    }

    #[test]
    fn context_injected_into_input() {
        // Verify the __context injection logic (no real WASM needed).
        let mut obj = serde_json::Map::new();
        let _ = obj.insert("key".to_owned(), serde_json::json!("value"));
        let ctx_json = serde_json::json!({
            "project_root": "/home/user/project",
            "available_tools": ["grep", "read"],
        });
        let _ = obj.insert("__context".to_owned(), ctx_json);
        let built = serde_json::Value::Object(obj);

        assert_eq!(built["key"], serde_json::json!("value"));
        assert_eq!(
            built["__context"]["project_root"],
            serde_json::json!("/home/user/project")
        );
        assert_eq!(
            built["__context"]["available_tools"],
            serde_json::json!(["grep", "read"])
        );
    }

    #[test]
    fn non_object_input_wrapped() {
        // Verify primitive input wrapping under "value".
        let input_val = serde_json::json!(42);
        let mut map = serde_json::Map::new();
        let _ = map.insert("value".to_owned(), input_val);
        let built = serde_json::Value::Object(map);
        assert_eq!(built["value"], serde_json::json!(42));
    }

    #[test]
    fn host_fns_build_without_panic() {
        let vfs_data = VfsData {
            root: PathBuf::from("/tmp"),
            allowed_tools: vec![],
        };
        let fns = build_host_fns(&vfs_data);
        // 11 host functions registered.
        assert_eq!(fns.len(), 11);
    }

    #[test]
    fn vfs_data_write_guard() {
        let vd_no = VfsData {
            root: PathBuf::from("/tmp"),
            allowed_tools: vec!["grep".to_owned()],
        };
        assert!(!vd_no.allows_write());

        let vd_yes = VfsData {
            root: PathBuf::from("/tmp"),
            allowed_tools: vec!["write".to_owned()],
        };
        assert!(vd_yes.allows_write());
    }
}
