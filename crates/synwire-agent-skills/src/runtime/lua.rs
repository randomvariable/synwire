//! Lua scripting runtime.
//!
//! Executes Lua scripts using [`mlua`]. The script receives an `input` global
//! and, when a [`SkillContext`] is provided, a `ctx` userdata object exposing
//! all VFS, tool, sampling, and logging capabilities.
//!
//! # Context API (`ctx`)
//!
//! | Method | Signature | Description |
//! |--------|-----------|-------------|
//! | `read_file` | `(path) → string` | Read file contents |
//! | `list_dir` | `(path) → string[]` | List directory entries |
//! | `head` | `(path, n) → string` | First *n* lines |
//! | `tail` | `(path, n) → string` | Last *n* lines |
//! | `stat` | `(path) → table` | File metadata |
//! | `grep` | `(pattern, path) → table[]` | Substring search |
//! | `glob` | `(pattern) → string[]` | Glob file paths |
//! | `find` | `(path, name_pat) → string[]` | Name-pattern walk |
//! | `tree` | `(path[, depth]) → table[]` | Directory tree |
//! | `write` | `(path, content)` | Write file (requires `write` in allowed\_tools) |
//! | `append` | `(path, content)` | Append to file (requires `write`) |
//! | `mkdir` | `(path)` | Create directory (requires `write`) |
//! | `log` | `(level, msg)` | Emit tracing log |
//! | `progress` | `(msg)` | Emit progress message |
//! | `tool` | `(name, args) → string` | Invoke a tool (requires tool\_provider) |
//! | `sample` | `(system, prompt[, max_tokens]) → string` | LLM sampling |
//!
//! This module is gated behind the `lua-runtime` feature flag.
//!
//! # Example input
//!
//! ```json
//! {
//!     "script": "return ctx:read_file('README.md'):len()",
//!     "input": {}
//! }
//! ```

use std::sync::Arc;

use mlua::{Lua, LuaSerdeExt as _, UserData, UserDataMethods};
use serde::Deserialize;
use tracing::debug;

use crate::error::SkillError;
use crate::runtime::path_safety::safe_resolve;
use crate::runtime::{SkillContext, SkillExecutor, SkillInput, SkillOutput, block_on_result};

/// Parameters extracted from the input JSON for Lua script execution.
#[derive(Debug, Deserialize)]
struct LuaArgs {
    /// The Lua source code to evaluate.
    script: String,
    /// The value to bind as the `input` global variable.
    #[serde(default)]
    input: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Context userdata
// ---------------------------------------------------------------------------

/// Userdata object exposed to Lua scripts as the `ctx` global.
struct LuaCtx {
    root: std::path::PathBuf,
    allowed_tools: Vec<String>,
    tool_provider: Option<Arc<dyn synwire_core::tools::ToolProvider>>,
    sampling: Option<Arc<dyn synwire_core::agents::sampling::SamplingProvider>>,
    progress_tx: Option<tokio::sync::mpsc::Sender<String>>,
}

impl LuaCtx {
    fn allows_write(&self) -> bool {
        self.allowed_tools
            .iter()
            .any(|t| t == "write" || t == "vfs.write" || t.contains("write"))
    }
}

impl UserData for LuaCtx {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        register_vfs_basic_read_methods(methods);
        register_vfs_search_methods(methods);
        register_vfs_write_methods(methods);
        register_utility_methods(methods);
        register_tool_and_sampling_methods(methods);
    }
}

/// Register basic read-only VFS methods (read, list, head, tail, stat).
fn register_vfs_basic_read_methods<M: UserDataMethods<LuaCtx>>(methods: &mut M) {
    methods.add_method("read_file", |_, this, path: String| {
        let resolved = safe_resolve(&this.root, &path).map_err(mlua::Error::external)?;
        std::fs::read_to_string(&resolved).map_err(mlua::Error::external)
    });

    methods.add_method("list_dir", |lua, this, path: String| {
        let resolved = safe_resolve(&this.root, &path).map_err(mlua::Error::external)?;
        let table = lua.create_table()?;
        let mut idx = 1i64;
        for entry in std::fs::read_dir(&resolved).map_err(mlua::Error::external)? {
            let entry = entry.map_err(mlua::Error::external)?;
            table.set(idx, entry.file_name().to_string_lossy().to_string())?;
            idx += 1;
        }
        Ok(table)
    });

    methods.add_method("head", |_, this, (path, n): (String, usize)| {
        let resolved = safe_resolve(&this.root, &path).map_err(mlua::Error::external)?;
        let content = std::fs::read_to_string(&resolved).map_err(mlua::Error::external)?;
        Ok(content.lines().take(n).collect::<Vec<_>>().join("\n"))
    });

    methods.add_method("tail", |_, this, (path, n): (String, usize)| {
        let resolved = safe_resolve(&this.root, &path).map_err(mlua::Error::external)?;
        let content = std::fs::read_to_string(&resolved).map_err(mlua::Error::external)?;
        let all: Vec<&str> = content.lines().collect();
        let start = all.len().saturating_sub(n);
        Ok(all[start..].join("\n"))
    });

    methods.add_method("stat", |lua, this, path: String| {
        let resolved = safe_resolve(&this.root, &path).map_err(mlua::Error::external)?;
        let meta = std::fs::metadata(&resolved).map_err(mlua::Error::external)?;
        let t = lua.create_table()?;
        t.set("size", meta.len())?;
        t.set("is_dir", meta.is_dir())?;
        t.set("is_file", meta.is_file())?;
        t.set("readonly", meta.permissions().readonly())?;
        Ok(t)
    });
}

/// Register search VFS methods (grep, glob, find, tree).
fn register_vfs_search_methods<M: UserDataMethods<LuaCtx>>(methods: &mut M) {
    methods.add_method("grep", |lua, this, (pattern, path): (String, String)| {
        let resolved = safe_resolve(&this.root, &path).map_err(mlua::Error::external)?;
        let mut hits: Vec<(String, usize, String)> = Vec::new();
        let search = |fp: &std::path::Path, pat: &str, acc: &mut Vec<_>| {
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
            search(&resolved, &pattern, &mut hits);
        } else {
            for e in walkdir::WalkDir::new(&resolved).follow_links(false) {
                let Ok(e) = e else { continue };
                if e.file_type().is_file() {
                    search(e.path(), &pattern, &mut hits);
                }
            }
        }
        let results = lua.create_table()?;
        for (i, (file, line, content)) in hits.into_iter().enumerate() {
            let row = lua.create_table()?;
            row.set("file", file)?;
            row.set("line", line)?;
            row.set("content", content)?;
            results.set(i + 1, row)?;
        }
        Ok(results)
    });

    methods.add_method("glob", |lua, this, pattern: String| {
        let matcher = globset::GlobBuilder::new(&pattern)
            .literal_separator(true)
            .build()
            .map_err(|e| mlua::Error::runtime(e.to_string()))?
            .compile_matcher();
        let results = lua.create_table()?;
        let mut idx = 1i64;
        for e in walkdir::WalkDir::new(&this.root).follow_links(false) {
            let Ok(e) = e else { continue };
            let rel = e
                .path()
                .strip_prefix(&this.root)
                .unwrap_or_else(|_| e.path());
            if matcher.is_match(rel) {
                results.set(idx, rel.to_string_lossy().to_string())?;
                idx += 1;
            }
        }
        Ok(results)
    });

    methods.add_method("find", |lua, this, (path, name_pat): (String, String)| {
        let resolved = safe_resolve(&this.root, &path).map_err(mlua::Error::external)?;
        let results = lua.create_table()?;
        let mut idx = 1i64;
        for e in walkdir::WalkDir::new(&resolved).follow_links(false) {
            let Ok(e) = e else { continue };
            if e.file_name().to_string_lossy().contains(name_pat.as_str()) {
                let rel = e
                    .path()
                    .strip_prefix(&this.root)
                    .unwrap_or_else(|_| e.path());
                results.set(idx, rel.to_string_lossy().to_string())?;
                idx += 1;
            }
        }
        Ok(results)
    });

    methods.add_method(
        "tree",
        |lua, this, (path, depth): (String, Option<usize>)| {
            let resolved = safe_resolve(&this.root, &path).map_err(mlua::Error::external)?;
            let results = lua.create_table()?;
            let mut idx = 1i64;
            for e in walkdir::WalkDir::new(&resolved)
                .max_depth(depth.unwrap_or(3))
                .follow_links(false)
            {
                let Ok(e) = e else { continue };
                let rel = e
                    .path()
                    .strip_prefix(&this.root)
                    .unwrap_or_else(|_| e.path());
                let row = lua.create_table()?;
                row.set("path", rel.to_string_lossy().to_string())?;
                row.set("is_dir", e.file_type().is_dir())?;
                results.set(idx, row)?;
                idx += 1;
            }
            Ok(results)
        },
    );
}

/// Register write/mutation VFS methods on the Lua `ctx` object.
fn register_vfs_write_methods<M: UserDataMethods<LuaCtx>>(methods: &mut M) {
    methods.add_method("write", |_, this, (path, content): (String, String)| {
        if !this.allows_write() {
            return Err(mlua::Error::runtime("'write' not in allowed_tools"));
        }
        let resolved = safe_resolve(&this.root, &path).map_err(mlua::Error::external)?;
        if let Some(p) = resolved.parent() {
            std::fs::create_dir_all(p).map_err(mlua::Error::external)?;
        }
        std::fs::write(&resolved, content).map_err(mlua::Error::external)
    });

    methods.add_method("append", |_, this, (path, content): (String, String)| {
        use std::io::Write as _;
        if !this.allows_write() {
            return Err(mlua::Error::runtime("'write' not in allowed_tools"));
        }
        let resolved = safe_resolve(&this.root, &path).map_err(mlua::Error::external)?;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&resolved)
            .map_err(mlua::Error::external)?;
        f.write_all(content.as_bytes())
            .map_err(mlua::Error::external)
    });

    methods.add_method("mkdir", |_, this, path: String| {
        if !this.allows_write() {
            return Err(mlua::Error::runtime("'write' not in allowed_tools"));
        }
        let resolved = safe_resolve(&this.root, &path).map_err(mlua::Error::external)?;
        std::fs::create_dir_all(&resolved).map_err(mlua::Error::external)
    });
}

/// Register logging and progress methods on the Lua `ctx` object.
fn register_utility_methods<M: UserDataMethods<LuaCtx>>(methods: &mut M) {
    methods.add_method("log", |_, _, (level, msg): (String, String)| {
        match level.to_lowercase().as_str() {
            "error" => tracing::error!("{}", msg),
            "warn" => tracing::warn!("{}", msg),
            "debug" => tracing::debug!("{}", msg),
            "trace" => tracing::trace!("{}", msg),
            _ => tracing::info!("{}", msg),
        }
        Ok(())
    });

    methods.add_method("progress", |_, this, msg: String| {
        if let Some(tx) = &this.progress_tx {
            let tx = tx.clone();
            // Fire-and-forget; ignore send errors.
            let _ = block_on_result(async move { tx.send(msg).await.map_err(|e| e.to_string()) });
        }
        Ok(())
    });
}

/// Register tool invocation and LLM sampling methods on the Lua `ctx` object.
fn register_tool_and_sampling_methods<M: UserDataMethods<LuaCtx>>(methods: &mut M) {
    methods.add_method("tool", |lua, this, (name, args): (String, mlua::Value)| {
        let provider = Arc::clone(
            this.tool_provider
                .as_ref()
                .ok_or_else(|| mlua::Error::runtime("tool provider not configured"))?,
        );
        let args_json: serde_json::Value = lua.from_value(args).map_err(mlua::Error::external)?;
        let content = block_on_result(async move {
            let tool = provider
                .get_tool(&name)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("tool '{name}' not found"))?;
            tool.invoke(args_json)
                .await
                .map(|o| o.content)
                .map_err(|e| e.to_string())
        })
        .map_err(mlua::Error::external)?;
        lua.to_value(&content)
    });

    methods.add_method(
        "sample",
        |_, this, (system, prompt, max_tokens): (Option<String>, String, Option<u32>)| {
            use synwire_core::agents::sampling::SamplingRequest;
            let provider = Arc::clone(
                this.sampling
                    .as_ref()
                    .ok_or_else(|| mlua::Error::runtime("sampling provider not configured"))?,
            );
            let mut req = SamplingRequest::new(prompt);
            if let Some(s) = system {
                req = req.with_system(s);
            }
            if let Some(mt) = max_tokens {
                req = req.with_max_tokens(mt);
            }
            let text = block_on_result(async move {
                provider
                    .sample(req)
                    .await
                    .map(|r| r.text)
                    .map_err(|e| e.to_string())
            })
            .map_err(mlua::Error::external)?;
            Ok(text)
        },
    );
}

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

/// Executor that evaluates Lua 5.4 scripts.
///
/// A new [`Lua`] VM is created for each execution call to ensure isolation
/// between skill invocations.
#[derive(Debug, Default)]
pub struct LuaRuntime {}

impl LuaRuntime {
    /// Create a new [`LuaRuntime`].
    pub const fn new() -> Self {
        Self {}
    }
}

#[allow(clippy::needless_pass_by_value)]
fn map_lua_error(e: mlua::Error) -> SkillError {
    SkillError::Runtime {
        runtime: "lua".to_owned(),
        message: e.to_string(),
    }
}

impl SkillExecutor for LuaRuntime {
    fn execute(&self, input: SkillInput) -> Result<SkillOutput, SkillError> {
        self.execute_with_context(input, None)
    }

    fn execute_with_context(
        &self,
        input: SkillInput,
        context: Option<&SkillContext>,
    ) -> Result<SkillOutput, SkillError> {
        let lua_args: LuaArgs = serde_json::from_value(input.args)
            .map_err(|e| SkillError::InvalidManifest(format!("invalid lua runtime args: {e}")))?;

        debug!(
            script_len = lua_args.script.len(),
            has_context = context.is_some(),
            "evaluating lua script"
        );

        let lua = Lua::new();
        let globals = lua.globals();

        if let Some(ctx) = context {
            globals
                .set(
                    "ctx",
                    LuaCtx {
                        root: ctx.project_root.clone(),
                        allowed_tools: ctx.available_tools.clone(),
                        tool_provider: ctx.tool_provider.clone(),
                        sampling: ctx.sampling.clone(),
                        progress_tx: ctx.progress_tx.clone(),
                    },
                )
                .map_err(map_lua_error)?;

            // Backward-compat top-level globals.
            let root = ctx.project_root.clone();
            globals
                .set(
                    "read_file",
                    lua.create_function(move |_, path: String| {
                        let r = safe_resolve(&root, &path)
                            .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
                        std::fs::read_to_string(&r).map_err(mlua::Error::external)
                    })
                    .map_err(map_lua_error)?,
                )
                .map_err(map_lua_error)?;

            let root2 = ctx.project_root.clone();
            globals
                .set(
                    "list_dir",
                    lua.create_function(move |lua_inner, path: String| {
                        let r = safe_resolve(&root2, &path)
                            .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
                        let t = lua_inner.create_table()?;
                        let mut i = 1i64;
                        for entry in std::fs::read_dir(&r).map_err(mlua::Error::external)? {
                            let entry = entry.map_err(mlua::Error::external)?;
                            t.set(i, entry.file_name().to_string_lossy().to_string())?;
                            i += 1;
                        }
                        Ok(t)
                    })
                    .map_err(map_lua_error)?,
                )
                .map_err(map_lua_error)?;
        }

        let lua_input = lua.to_value(&lua_args.input).map_err(map_lua_error)?;
        globals.set("input", lua_input).map_err(map_lua_error)?;

        let result: mlua::Value = lua.load(&lua_args.script).eval().map_err(map_lua_error)?;
        let json_result: serde_json::Value = lua.from_value(result).map_err(map_lua_error)?;

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
        let runtime = LuaRuntime::new();
        let out = runtime
            .execute(SkillInput {
                args: serde_json::json!({"script": "return input * 2", "input": 21}),
            })
            .expect("arithmetic");
        assert_eq!(out.result, serde_json::json!(42));
    }

    #[test]
    fn access_table_fields() {
        let runtime = LuaRuntime::new();
        let out = runtime
            .execute(SkillInput {
                args: serde_json::json!({
                    "script": "return input.a + input.b",
                    "input": {"a": 10, "b": 32}
                }),
            })
            .expect("table access");
        assert_eq!(out.result, serde_json::json!(42));
    }

    #[test]
    fn syntax_error_returns_runtime_error() {
        let err = LuaRuntime::new()
            .execute(SkillInput {
                args: serde_json::json!({"script": "return !!!invalid", "input": null}),
            })
            .expect_err("should fail");
        assert!(matches!(err, SkillError::Runtime { .. }));
    }

    #[test]
    fn missing_script_returns_manifest_error() {
        let err = LuaRuntime::new()
            .execute(SkillInput {
                args: serde_json::json!({"input": 1}),
            })
            .expect_err("should fail");
        assert!(matches!(err, SkillError::InvalidManifest(_)));
    }

    #[test]
    fn return_table_as_json() {
        let out = LuaRuntime::new()
            .execute(SkillInput {
                args: serde_json::json!({
                    "script": "return { x = input + 1, y = \"hello\" }",
                    "input": 41
                }),
            })
            .expect("table return");
        assert_eq!(out.result["x"], serde_json::json!(42));
        assert_eq!(out.result["y"], serde_json::json!("hello"));
    }

    #[test]
    fn read_file_via_ctx() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("hello.txt"), "world").expect("write");
        let ctx = SkillContext {
            project_root: dir.path().to_path_buf(),
            ..Default::default()
        };
        let out = LuaRuntime::new()
            .execute_with_context(
                SkillInput {
                    args: serde_json::json!({"script": "return ctx:read_file('hello.txt')", "input": null}),
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
        let out = LuaRuntime::new()
            .execute_with_context(
                SkillInput {
                    args: serde_json::json!({"script": "return read_file('hello.txt')", "input": null}),
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
        let out = LuaRuntime::new()
            .execute_with_context(
                SkillInput {
                    args: serde_json::json!({
                        "script": "local e = ctx:list_dir('.'); table.sort(e); return e",
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
        let rt = LuaRuntime::new();

        let head = rt
            .execute_with_context(
                SkillInput {
                    args: serde_json::json!({"script": "return ctx:head('lines.txt', 2)", "input": null}),
                },
                Some(&ctx),
            )
            .expect("head");
        assert_eq!(head.result, serde_json::json!("a\nb"));

        let tail = rt
            .execute_with_context(
                SkillInput {
                    args: serde_json::json!({"script": "return ctx:tail('lines.txt', 2)", "input": null}),
                },
                Some(&ctx),
            )
            .expect("tail");
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
        let out = LuaRuntime::new()
            .execute_with_context(
                SkillInput {
                    args: serde_json::json!({"script": "return ctx:stat('f.txt').is_file", "input": null}),
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
        let out = LuaRuntime::new()
            .execute_with_context(
                SkillInput {
                    args: serde_json::json!({
                        "script": "return #ctx:grep('fn main', 'code.rs')",
                        "input": null
                    }),
                },
                Some(&ctx),
            )
            .expect("grep");
        assert_eq!(out.result, serde_json::json!(1));
    }

    #[test]
    fn write_denied_without_permission() {
        let dir = tempfile::tempdir().expect("temp dir");
        let ctx = SkillContext {
            project_root: dir.path().to_path_buf(),
            ..Default::default()
        };
        let err = LuaRuntime::new()
            .execute_with_context(
                SkillInput {
                    args: serde_json::json!({"script": "ctx:write('out.txt', 'x')", "input": null}),
                },
                Some(&ctx),
            )
            .expect_err("write without perm");
        assert!(matches!(err, SkillError::Runtime { .. }));
    }

    #[test]
    fn write_succeeds_with_permission() {
        let dir = tempfile::tempdir().expect("temp dir");
        let ctx = SkillContext {
            project_root: dir.path().to_path_buf(),
            available_tools: vec!["write".to_owned()],
            ..Default::default()
        };
        let _ = LuaRuntime::new()
            .execute_with_context(
                SkillInput {
                    args: serde_json::json!({"script": "ctx:write('out.txt', 'hello')", "input": null}),
                },
                Some(&ctx),
            )
            .expect("write");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("out.txt")).expect("read"),
            "hello"
        );
    }

    #[test]
    fn path_traversal_rejected() {
        let dir = tempfile::tempdir().expect("temp dir");
        let ctx = SkillContext {
            project_root: dir.path().to_path_buf(),
            ..Default::default()
        };
        let err = LuaRuntime::new()
            .execute_with_context(
                SkillInput {
                    args: serde_json::json!({
                        "script": "return ctx:read_file('../../../etc/passwd')",
                        "input": null
                    }),
                },
                Some(&ctx),
            )
            .expect_err("traversal");
        assert!(matches!(err, SkillError::Runtime { .. }));
    }

    #[test]
    fn no_ctx_without_context() {
        let err = LuaRuntime::new()
            .execute(SkillInput {
                args: serde_json::json!({"script": "return read_file('test.txt')", "input": null}),
            })
            .expect_err("no ctx");
        assert!(matches!(err, SkillError::Runtime { .. }));
    }
}
