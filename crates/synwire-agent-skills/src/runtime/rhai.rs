//! Rhai scripting runtime.
//!
//! Executes Rhai scripts within a sandboxed [`rhai::Engine`]. Scripts receive
//! an `input` variable and, when a [`SkillContext`] is provided, a `ctx`
//! variable of type `Ctx` exposing the full capability set via dot-call syntax.
//!
//! # Context API (`ctx`)
//!
//! | Method | Signature | Description |
//! |--------|-----------|-------------|
//! | `read_file` | `(path) → string` | Read file contents |
//! | `list_dir` | `(path) → array` | List directory entries |
//! | `head` | `(path, n) → string` | First *n* lines |
//! | `tail` | `(path, n) → string` | Last *n* lines |
//! | `stat` | `(path) → map` | File metadata (`size`, `is_dir`, `is_file`, `readonly`) |
//! | `grep` | `(pattern, path) → array` | Substring search → `[#{file, line, content}]` |
//! | `glob` | `(pattern) → array` | Glob file paths |
//! | `find` | `(path, name_pat) → array` | Name-pattern walk |
//! | `tree` | `(path, depth) → array` | Directory tree → `[#{path, is_dir}]` |
//! | `write` | `(path, content) → string` | Write file (`"ok"` or `"ERROR: …"`) |
//! | `append` | `(path, content) → string` | Append to file |
//! | `mkdir` | `(path) → string` | Create directory |
//! | `log` | `(level, msg)` | Emit tracing log |
//! | `progress` | `(msg)` | Emit progress message |
//! | `tool` | `(name, args) → string` | Invoke a tool (requires tool\_provider) |
//! | `sample` | `(system, prompt, max_tokens) → string` | LLM sampling |
//!
//! Error returns from write/tool/sample use the `"ERROR: …"` prefix convention
//! so scripts can check `result.starts_with("ERROR:")` without throwing.
//!
//! This module is gated behind the `rhai-runtime` feature flag.
//!
//! # Example input
//!
//! ```json
//! {
//!     "script": "ctx.read_file(\"README.md\")",
//!     "input": { "x": 21 }
//! }
//! ```

use std::sync::Arc;

use ::rhai::{Dynamic, Engine, Scope};
use serde::Deserialize;
use tracing::debug;

use crate::error::SkillError;
use crate::runtime::path_safety::safe_resolve;
use crate::runtime::{SkillContext, SkillExecutor, SkillInput, SkillOutput, block_on_result};

/// Parameters extracted from the input JSON for Rhai script execution.
#[derive(Debug, Deserialize)]
struct RhaiArgs {
    /// The Rhai source code to evaluate.
    script: String,
    /// The value to bind as the `input` variable in the script scope.
    #[serde(default)]
    input: serde_json::Value,
}

// ---------------------------------------------------------------------------
// RhaiCtx — the `ctx` object exposed to scripts
// ---------------------------------------------------------------------------

/// Custom type exposed to Rhai scripts as the `ctx` variable.
///
/// Rhai calls methods via dot-call: `ctx.read_file(path)`. Each method is
/// registered with the engine using `engine.register_fn`.
#[derive(Clone)]
pub(crate) struct RhaiCtx {
    root: Arc<std::path::PathBuf>,
    allowed_tools: Vec<String>,
    tool_provider: Option<Arc<dyn synwire_core::tools::ToolProvider>>,
    sampling: Option<Arc<dyn synwire_core::agents::sampling::SamplingProvider>>,
    progress_tx: Option<tokio::sync::mpsc::Sender<String>>,
}

impl RhaiCtx {
    fn allows_write(&self) -> bool {
        self.allowed_tools
            .iter()
            .any(|t| t == "write" || t == "vfs.write" || t.contains("write"))
    }

    // ---- read_file --------------------------------------------------------
    pub(crate) fn read_file(&mut self, path: &str) -> String {
        match safe_resolve(&self.root, path) {
            Err(e) => format!("ERROR: {e}"),
            Ok(r) => std::fs::read_to_string(&r)
                .unwrap_or_else(|e| format!("ERROR: failed to read '{}': {e}", r.display())),
        }
    }

    // ---- list_dir ---------------------------------------------------------
    pub(crate) fn list_dir(&mut self, path: &str) -> ::rhai::Array {
        let Ok(r) = safe_resolve(&self.root, path) else {
            return ::rhai::Array::new();
        };
        let Ok(entries) = std::fs::read_dir(&r) else {
            return ::rhai::Array::new();
        };
        entries
            .filter_map(|e| {
                e.ok()
                    .map(|e| Dynamic::from(e.file_name().to_string_lossy().to_string()))
            })
            .collect()
    }

    // ---- head -------------------------------------------------------------
    pub(crate) fn head(&mut self, path: &str, n: i64) -> String {
        match safe_resolve(&self.root, path) {
            Err(e) => format!("ERROR: {e}"),
            Ok(r) => match std::fs::read_to_string(&r) {
                Err(e) => format!("ERROR: {e}"),
                Ok(c) => {
                    let count = usize::try_from(n.max(0)).unwrap_or(0);
                    c.lines().take(count).collect::<Vec<_>>().join("\n")
                }
            },
        }
    }

    // ---- tail -------------------------------------------------------------
    pub(crate) fn tail(&mut self, path: &str, n: i64) -> String {
        match safe_resolve(&self.root, path) {
            Err(e) => format!("ERROR: {e}"),
            Ok(r) => match std::fs::read_to_string(&r) {
                Err(e) => format!("ERROR: {e}"),
                Ok(c) => {
                    let all: Vec<&str> = c.lines().collect();
                    let count = usize::try_from(n.max(0)).unwrap_or(0);
                    let start = all.len().saturating_sub(count);
                    all[start..].join("\n")
                }
            },
        }
    }

    // ---- stat -------------------------------------------------------------
    pub(crate) fn stat(&mut self, path: &str) -> ::rhai::Map {
        let mut m = ::rhai::Map::new();
        match safe_resolve(&self.root, path).and_then(|r| {
            std::fs::metadata(&r).map_err(|e| SkillError::Runtime {
                runtime: "rhai".to_owned(),
                message: e.to_string(),
            })
        }) {
            Err(e) => {
                let _ = m.insert("error".into(), Dynamic::from(e.to_string()));
            }
            Ok(meta) => {
                let size = i64::try_from(meta.len()).unwrap_or(i64::MAX);
                let _ = m.insert("size".into(), Dynamic::from(size));
                let _ = m.insert("is_dir".into(), Dynamic::from(meta.is_dir()));
                let _ = m.insert("is_file".into(), Dynamic::from(meta.is_file()));
                let _ = m.insert(
                    "readonly".into(),
                    Dynamic::from(meta.permissions().readonly()),
                );
            }
        }
        m
    }

    // ---- grep -------------------------------------------------------------
    pub(crate) fn grep(&mut self, pattern: &str, path: &str) -> ::rhai::Array {
        let Ok(resolved) = safe_resolve(&self.root, path) else {
            return ::rhai::Array::new();
        };
        let mut hits: Vec<(String, usize, String)> = Vec::new();
        let collect = |fp: &std::path::Path, pat: &str, acc: &mut Vec<_>| {
            let Ok(c) = std::fs::read_to_string(fp) else {
                return;
            };
            for (i, line) in c.lines().enumerate() {
                if line.contains(pat) {
                    acc.push((fp.to_string_lossy().into_owned(), i + 1, line.to_owned()));
                }
            }
        };
        if resolved.is_file() {
            collect(&resolved, pattern, &mut hits);
        } else {
            for e in walkdir::WalkDir::new(&resolved).follow_links(false) {
                let Ok(e) = e else { continue };
                if e.file_type().is_file() {
                    collect(e.path(), pattern, &mut hits);
                }
            }
        }
        hits.into_iter()
            .map(|(file, line, content)| {
                let mut m = ::rhai::Map::new();
                let _ = m.insert("file".into(), Dynamic::from(file));
                let line_num = i64::try_from(line).unwrap_or(i64::MAX);
                let _ = m.insert("line".into(), Dynamic::from(line_num));
                let _ = m.insert("content".into(), Dynamic::from(content));
                Dynamic::from_map(m)
            })
            .collect()
    }

    // ---- glob -------------------------------------------------------------
    pub(crate) fn glob(&mut self, pattern: &str) -> ::rhai::Array {
        let Ok(matcher) = globset::GlobBuilder::new(pattern)
            .literal_separator(true)
            .build()
            .map(|g| g.compile_matcher())
        else {
            return ::rhai::Array::new();
        };
        let root = Arc::clone(&self.root);
        walkdir::WalkDir::new(root.as_ref())
            .follow_links(false)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter_map(|e| {
                let rel = e
                    .path()
                    .strip_prefix(root.as_ref())
                    .unwrap_or_else(|_| e.path())
                    .to_owned();
                if matcher.is_match(&rel) {
                    Some(Dynamic::from(rel.to_string_lossy().to_string()))
                } else {
                    None
                }
            })
            .collect()
    }

    // ---- find -------------------------------------------------------------
    pub(crate) fn find(&mut self, path: &str, name_pat: &str) -> ::rhai::Array {
        let Ok(resolved) = safe_resolve(&self.root, path) else {
            return ::rhai::Array::new();
        };
        let root = Arc::clone(&self.root);
        walkdir::WalkDir::new(&resolved)
            .follow_links(false)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter_map(|e| {
                if e.file_name().to_string_lossy().contains(name_pat) {
                    let rel = e
                        .path()
                        .strip_prefix(root.as_ref())
                        .unwrap_or_else(|_| e.path())
                        .to_owned();
                    Some(Dynamic::from(rel.to_string_lossy().to_string()))
                } else {
                    None
                }
            })
            .collect()
    }

    // ---- tree -------------------------------------------------------------
    pub(crate) fn tree(&mut self, path: &str, depth: i64) -> ::rhai::Array {
        let Ok(resolved) = safe_resolve(&self.root, path) else {
            return ::rhai::Array::new();
        };
        let root = Arc::clone(&self.root);
        walkdir::WalkDir::new(&resolved)
            .max_depth(usize::try_from(depth.max(0)).unwrap_or(0))
            .follow_links(false)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .map(|e| {
                let rel = e
                    .path()
                    .strip_prefix(root.as_ref())
                    .unwrap_or_else(|_| e.path())
                    .to_owned();
                let mut m = ::rhai::Map::new();
                let _ = m.insert(
                    "path".into(),
                    Dynamic::from(rel.to_string_lossy().to_string()),
                );
                let _ = m.insert("is_dir".into(), Dynamic::from(e.file_type().is_dir()));
                Dynamic::from_map(m)
            })
            .collect()
    }

    // ---- write ------------------------------------------------------------
    pub(crate) fn write(&mut self, path: &str, content: &str) -> String {
        if !self.allows_write() {
            return "ERROR: 'write' not in allowed_tools".to_owned();
        }
        match safe_resolve(&self.root, path) {
            Err(e) => format!("ERROR: {e}"),
            Ok(r) => {
                if let Some(p) = r.parent() {
                    if let Err(e) = std::fs::create_dir_all(p) {
                        return format!("ERROR: {e}");
                    }
                }
                match std::fs::write(&r, content) {
                    Ok(()) => "ok".to_owned(),
                    Err(e) => format!("ERROR: {e}"),
                }
            }
        }
    }

    // ---- append -----------------------------------------------------------
    pub(crate) fn append(&mut self, path: &str, content: &str) -> String {
        if !self.allows_write() {
            return "ERROR: 'write' not in allowed_tools".to_owned();
        }
        match safe_resolve(&self.root, path) {
            Err(e) => format!("ERROR: {e}"),
            Ok(r) => {
                use std::io::Write as _;
                match std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&r)
                    .and_then(|mut f| f.write_all(content.as_bytes()))
                {
                    Ok(()) => "ok".to_owned(),
                    Err(e) => format!("ERROR: {e}"),
                }
            }
        }
    }

    // ---- mkdir ------------------------------------------------------------
    pub(crate) fn mkdir(&mut self, path: &str) -> String {
        if !self.allows_write() {
            return "ERROR: 'write' not in allowed_tools".to_owned();
        }
        match safe_resolve(&self.root, path) {
            Err(e) => format!("ERROR: {e}"),
            Ok(r) => match std::fs::create_dir_all(&r) {
                Ok(()) => "ok".to_owned(),
                Err(e) => format!("ERROR: {e}"),
            },
        }
    }

    // ---- log --------------------------------------------------------------
    // Rhai method registration requires `&mut self` even though `log` does not
    // access instance fields.
    #[allow(clippy::unused_self)]
    pub(crate) fn log(&mut self, level: &str, msg: &str) {
        match level.to_lowercase().as_str() {
            "error" => tracing::error!("{}", msg),
            "warn" => tracing::warn!("{}", msg),
            "debug" => tracing::debug!("{}", msg),
            "trace" => tracing::trace!("{}", msg),
            _ => tracing::info!("{}", msg),
        }
    }

    // ---- progress ---------------------------------------------------------
    pub(crate) fn progress(&mut self, msg: &str) {
        if let Some(tx) = &self.progress_tx {
            let tx = tx.clone();
            let msg = msg.to_owned();
            let _ = block_on_result(async move { tx.send(msg).await.map_err(|e| e.to_string()) });
        }
    }

    // ---- tool -------------------------------------------------------------
    // Rhai method registration requires `Dynamic` by value.
    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn tool(&mut self, name: &str, args: Dynamic) -> String {
        let Some(ref provider) = self.tool_provider else {
            return "ERROR: tool provider not configured".to_owned();
        };
        let provider = Arc::clone(provider);
        let args_json = match ::rhai::serde::from_dynamic::<serde_json::Value>(&args) {
            Ok(v) => v,
            Err(e) => return format!("ERROR: failed to convert args: {e}"),
        };
        let name_owned = name.to_owned();
        match block_on_result(async move {
            let tool = provider
                .get_tool(&name_owned)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("tool '{name_owned}' not found"))?;
            tool.invoke(args_json)
                .await
                .map(|o| o.content)
                .map_err(|e| e.to_string())
        }) {
            Ok(content) => content,
            Err(e) => format!("ERROR: {e}"),
        }
    }

    // ---- sample -----------------------------------------------------------
    pub(crate) fn sample(&mut self, system: &str, prompt: &str, max_tokens: i64) -> String {
        use synwire_core::agents::sampling::SamplingRequest;
        let Some(ref provider) = self.sampling else {
            return "ERROR: sampling provider not configured".to_owned();
        };
        let provider = Arc::clone(provider);
        let mut req = SamplingRequest::new(prompt);
        if !system.is_empty() {
            req = req.with_system(system);
        }
        if max_tokens > 0 {
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            let mt = max_tokens as u32;
            req = req.with_max_tokens(mt);
        }
        match block_on_result(async move {
            provider
                .sample(req)
                .await
                .map(|r| r.text)
                .map_err(|e| e.to_string())
        }) {
            Ok(text) => text,
            Err(e) => format!("ERROR: {e}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Engine configuration
// ---------------------------------------------------------------------------

/// Register the `RhaiCtx` type and all its methods with an engine.
fn register_ctx_type(engine: &mut Engine) {
    let _ = engine.register_type_with_name::<RhaiCtx>("Ctx");
    let _ = engine.register_fn("read_file", RhaiCtx::read_file);
    let _ = engine.register_fn("list_dir", RhaiCtx::list_dir);
    let _ = engine.register_fn("head", RhaiCtx::head);
    let _ = engine.register_fn("tail", RhaiCtx::tail);
    let _ = engine.register_fn("stat", RhaiCtx::stat);
    let _ = engine.register_fn("grep", RhaiCtx::grep);
    let _ = engine.register_fn("glob", RhaiCtx::glob);
    let _ = engine.register_fn("find", RhaiCtx::find);
    let _ = engine.register_fn("tree", RhaiCtx::tree);
    let _ = engine.register_fn("write", RhaiCtx::write);
    let _ = engine.register_fn("append", RhaiCtx::append);
    let _ = engine.register_fn("mkdir", RhaiCtx::mkdir);
    let _ = engine.register_fn("log", RhaiCtx::log);
    let _ = engine.register_fn("progress", RhaiCtx::progress);
    let _ = engine.register_fn("tool", RhaiCtx::tool);
    let _ = engine.register_fn("sample", RhaiCtx::sample);
}

/// Register backward-compat free functions `read_file` and `list_dir` that
/// operate on the given root without needing a `ctx` object.
fn register_compat_fns(engine: &mut Engine, ctx: &SkillContext) {
    let root = Arc::new(ctx.project_root.clone());

    let read_root = Arc::clone(&root);
    let _ = engine.register_fn("read_file", move |path: &str| -> String {
        match safe_resolve(&read_root, path) {
            Err(e) => format!("ERROR: {e}"),
            Ok(r) => std::fs::read_to_string(&r).unwrap_or_else(|e| format!("ERROR: {e}")),
        }
    });

    let list_root = root;
    let _ = engine.register_fn("list_dir", move |path: &str| -> ::rhai::Array {
        let Ok(r) = safe_resolve(&list_root, path) else {
            return ::rhai::Array::new();
        };
        let Ok(entries) = std::fs::read_dir(&r) else {
            return ::rhai::Array::new();
        };
        entries
            .filter_map(|e| {
                e.ok()
                    .map(|e| Dynamic::from(e.file_name().to_string_lossy().to_string()))
            })
            .collect()
    });
}

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

/// Executor that evaluates Rhai scripts.
///
/// A new [`rhai::Engine`] is created for each execution call to ensure
/// isolation between skill invocations.
#[derive(Debug, Default)]
pub struct RhaiRuntime {}

impl RhaiRuntime {
    /// Create a new [`RhaiRuntime`].
    pub const fn new() -> Self {
        Self {}
    }
}

impl SkillExecutor for RhaiRuntime {
    fn execute(&self, input: SkillInput) -> Result<SkillOutput, SkillError> {
        self.execute_with_context(input, None)
    }

    fn execute_with_context(
        &self,
        input: SkillInput,
        context: Option<&SkillContext>,
    ) -> Result<SkillOutput, SkillError> {
        let rhai_args: RhaiArgs = serde_json::from_value(input.args)
            .map_err(|e| SkillError::InvalidManifest(format!("invalid rhai runtime args: {e}")))?;

        debug!(
            script_len = rhai_args.script.len(),
            has_context = context.is_some(),
            "evaluating rhai script"
        );

        let mut engine = Engine::new();
        let mut scope = Scope::new();

        if let Some(ctx) = context {
            register_ctx_type(&mut engine);
            register_compat_fns(&mut engine, ctx);

            let rhai_ctx = RhaiCtx {
                root: Arc::new(ctx.project_root.clone()),
                allowed_tools: ctx.available_tools.clone(),
                tool_provider: ctx.tool_provider.clone(),
                sampling: ctx.sampling.clone(),
                progress_tx: ctx.progress_tx.clone(),
            };
            let _ = scope.push("ctx", rhai_ctx);
        }

        let dynamic_input =
            ::rhai::serde::to_dynamic(rhai_args.input).map_err(|e| SkillError::Runtime {
                runtime: "rhai".to_owned(),
                message: format!("failed to convert input to dynamic: {e}"),
            })?;
        let _ = scope.push_dynamic("input", dynamic_input);

        let result = engine
            .eval_with_scope::<Dynamic>(&mut scope, &rhai_args.script)
            .map_err(|e| SkillError::Runtime {
                runtime: "rhai".to_owned(),
                message: format!("{e}"),
            })?;

        let json_result =
            ::rhai::serde::from_dynamic(&result).map_err(|e| SkillError::Runtime {
                runtime: "rhai".to_owned(),
                message: format!("failed to convert result to JSON: {e}"),
            })?;

        Ok(SkillOutput {
            result: json_result,
        })
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
    fn simple_arithmetic() {
        let out = RhaiRuntime::new()
            .execute(SkillInput {
                args: serde_json::json!({"script": "let x = input; x * 2", "input": 21}),
            })
            .expect("arithmetic");
        assert_eq!(out.result, serde_json::json!(42));
    }

    #[test]
    fn access_input_fields() {
        let out = RhaiRuntime::new()
            .execute(SkillInput {
                args: serde_json::json!({
                    "script": "input.a + input.b",
                    "input": {"a": 10, "b": 32}
                }),
            })
            .expect("field access");
        assert_eq!(out.result, serde_json::json!(42));
    }

    #[test]
    fn syntax_error_returns_runtime_error() {
        let err = RhaiRuntime::new()
            .execute(SkillInput {
                args: serde_json::json!({"script": "let x = !!!;", "input": null}),
            })
            .expect_err("should fail");
        assert!(matches!(err, SkillError::Runtime { .. }));
    }

    #[test]
    fn missing_script_returns_manifest_error() {
        let err = RhaiRuntime::new()
            .execute(SkillInput {
                args: serde_json::json!({"input": 1}),
            })
            .expect_err("should fail");
        assert!(matches!(err, SkillError::InvalidManifest(_)));
    }

    #[test]
    fn read_file_via_ctx() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("hello.txt"), "world").expect("write");
        let ctx = SkillContext {
            project_root: dir.path().to_path_buf(),
            ..Default::default()
        };
        let out = RhaiRuntime::new()
            .execute_with_context(
                SkillInput {
                    args: serde_json::json!({"script": "ctx.read_file(\"hello.txt\")", "input": null}),
                },
                Some(&ctx),
            )
            .expect("ctx read_file");
        assert_eq!(out.result, serde_json::json!("world"));
    }

    #[test]
    fn read_file_compat_global() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("hello.txt"), "world").expect("write");
        let ctx = SkillContext {
            project_root: dir.path().to_path_buf(),
            ..Default::default()
        };
        let out = RhaiRuntime::new()
            .execute_with_context(
                SkillInput {
                    args: serde_json::json!({"script": "read_file(\"hello.txt\")", "input": null}),
                },
                Some(&ctx),
            )
            .expect("compat read_file");
        assert_eq!(out.result, serde_json::json!("world"));
    }

    #[test]
    fn list_dir_via_ctx() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("a.txt"), "").expect("a");
        std::fs::write(dir.path().join("b.txt"), "").expect("b");
        let ctx = SkillContext {
            project_root: dir.path().to_path_buf(),
            ..Default::default()
        };
        let out = RhaiRuntime::new()
            .execute_with_context(
                SkillInput {
                    args: serde_json::json!({
                        "script": "let e = ctx.list_dir(\".\"); e.sort(); e",
                        "input": null
                    }),
                },
                Some(&ctx),
            )
            .expect("list_dir");
        let arr = out.result.as_array().expect("array");
        let names: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
        assert!(names.contains(&"a.txt"));
        assert!(names.contains(&"b.txt"));
    }

    #[test]
    fn head_and_tail() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("lines.txt"), "a\nb\nc\nd\ne").expect("write");
        let ctx = SkillContext {
            project_root: dir.path().to_path_buf(),
            ..Default::default()
        };
        let rt = RhaiRuntime::new();

        let head = rt.execute_with_context(
            SkillInput { args: serde_json::json!({"script": "ctx.head(\"lines.txt\", 2)", "input": null}) },
            Some(&ctx),
        ).expect("head");
        assert_eq!(head.result, serde_json::json!("a\nb"));

        let tail = rt.execute_with_context(
            SkillInput { args: serde_json::json!({"script": "ctx.tail(\"lines.txt\", 2)", "input": null}) },
            Some(&ctx),
        ).expect("tail");
        assert_eq!(tail.result, serde_json::json!("d\ne"));
    }

    #[test]
    fn stat_is_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("f.txt"), "hi").expect("write");
        let ctx = SkillContext {
            project_root: dir.path().to_path_buf(),
            ..Default::default()
        };
        let out = RhaiRuntime::new()
            .execute_with_context(
                SkillInput {
                    args: serde_json::json!({"script": "ctx.stat(\"f.txt\").is_file", "input": null}),
                },
                Some(&ctx),
            )
            .expect("stat");
        assert_eq!(out.result, serde_json::json!(true));
    }

    #[test]
    fn grep_finds_match() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("code.rs"), "fn main() {}\nlet x = 1;").expect("write");
        let ctx = SkillContext {
            project_root: dir.path().to_path_buf(),
            ..Default::default()
        };
        let out = RhaiRuntime::new()
            .execute_with_context(
                SkillInput {
                    args: serde_json::json!({
                        "script": "ctx.grep(\"fn main\", \"code.rs\").len()",
                        "input": null
                    }),
                },
                Some(&ctx),
            )
            .expect("grep");
        assert_eq!(out.result, serde_json::json!(1));
    }

    #[test]
    fn path_traversal_rejected_in_rhai() {
        let dir = tempfile::tempdir().expect("temp dir");
        let ctx = SkillContext {
            project_root: dir.path().to_path_buf(),
            ..Default::default()
        };
        let out = RhaiRuntime::new()
            .execute_with_context(
                SkillInput {
                    args: serde_json::json!({
                        "script": "ctx.read_file(\"../../../etc/passwd\")",
                        "input": null
                    }),
                },
                Some(&ctx),
            )
            .expect("should not crash");
        assert!(out.result.as_str().unwrap_or("").starts_with("ERROR:"));
    }

    #[test]
    fn write_denied_without_permission() {
        let dir = tempfile::tempdir().expect("temp dir");
        let ctx = SkillContext {
            project_root: dir.path().to_path_buf(),
            ..Default::default()
        };
        let out = RhaiRuntime::new()
            .execute_with_context(
                SkillInput {
                    args: serde_json::json!({"script": "ctx.write(\"out.txt\", \"x\")", "input": null}),
                },
                Some(&ctx),
            )
            .expect("no crash");
        assert!(out.result.as_str().unwrap_or("").starts_with("ERROR:"));
    }

    #[test]
    fn write_succeeds_with_permission() {
        let dir = tempfile::tempdir().expect("temp dir");
        let ctx = SkillContext {
            project_root: dir.path().to_path_buf(),
            available_tools: vec!["write".to_owned()],
            ..Default::default()
        };
        let out = RhaiRuntime::new()
            .execute_with_context(
                SkillInput {
                    args: serde_json::json!({"script": "ctx.write(\"out.txt\", \"hello\")", "input": null}),
                },
                Some(&ctx),
            )
            .expect("write");
        assert_eq!(out.result.as_str().unwrap_or(""), "ok");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("out.txt")).expect("read"),
            "hello"
        );
    }
}
