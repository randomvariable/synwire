//! Runtime execution layer for agent skills.
//!
//! Each runtime variant implements the [`SkillExecutor`] trait. The `external`
//! and `sequence` runtimes are always available. The `rhai`, `lua`, and `wasm`
//! runtimes are gated behind their respective feature flags.

pub mod external;
pub mod sequence;

#[cfg(feature = "rhai-runtime")]
pub mod rhai;

#[cfg(feature = "lua-runtime")]
pub mod lua;

#[cfg(feature = "wasm-runtime")]
pub mod wasm;

pub(crate) mod path_safety;

use std::sync::Arc;

use crate::error::SkillError;
use synwire_core::agents::sampling::SamplingProvider;
use synwire_core::tools::ToolProvider;

/// A skill input payload passed to the executor.
#[derive(Debug, Clone)]
pub struct SkillInput {
    /// Arbitrary JSON-encoded arguments supplied by the caller.
    pub args: serde_json::Value,
}

/// A skill output payload returned by the executor.
#[derive(Debug, Clone)]
pub struct SkillOutput {
    /// The result produced by the skill, as a JSON value.
    pub result: serde_json::Value,
}

/// Execution context providing VFS and tool access to skill runtimes.
///
/// When set, runtimes can expose filesystem operations and tool invocation
/// to scripts. When `None`, only pure computation is available.
#[derive(Clone)]
pub struct SkillContext {
    /// Project root for VFS operations.
    pub project_root: std::path::PathBuf,
    /// Available tool names that the skill can invoke.
    pub available_tools: Vec<String>,
    /// Tool provider for `ctx.tool()` invocation. `None` means tool calls are unavailable.
    pub tool_provider: Option<Arc<dyn ToolProvider>>,
    /// Sampling provider for `ctx.sample()` LLM access. `None` means sampling is unavailable.
    pub sampling: Option<Arc<dyn SamplingProvider>>,
    /// Channel for emitting progress messages.
    pub progress_tx: Option<tokio::sync::mpsc::Sender<String>>,
}

impl Default for SkillContext {
    fn default() -> Self {
        Self {
            project_root: std::path::PathBuf::new(),
            available_tools: Vec::new(),
            tool_provider: None,
            sampling: None,
            progress_tx: None,
        }
    }
}

impl std::fmt::Debug for SkillContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkillContext")
            .field("project_root", &self.project_root)
            .field("available_tools", &self.available_tools)
            .field("tool_provider", &self.tool_provider.is_some())
            .field("sampling", &self.sampling.is_some())
            .field("progress_tx", &self.progress_tx.is_some())
            .finish()
    }
}

/// Block on an async future that returns `Result<T, E>` from a synchronous
/// context, bridging into the caller's existing tokio runtime.
///
/// Uses [`tokio::task::block_in_place`] so it is safe to call from a
/// multi-threaded executor. Returns [`SkillError::Runtime`] if no runtime
/// handle is available.
#[cfg(any(feature = "rhai-runtime", feature = "lua-runtime"))]
pub(crate) fn block_on_result<F, T, E>(fut: F) -> Result<T, SkillError>
where
    F: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    match tokio::runtime::Handle::try_current() {
        Err(_) => Err(SkillError::Runtime {
            runtime: "async_bridge".to_owned(),
            message: "no tokio runtime available for async operations".to_owned(),
        }),
        Ok(handle) => {
            if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::CurrentThread {
                return Err(SkillError::Runtime {
                    runtime: "async_bridge".to_owned(),
                    message: "block_in_place not supported on current-thread runtime".to_owned(),
                });
            }
            tokio::task::block_in_place(|| handle.block_on(fut)).map_err(|e| SkillError::Runtime {
                runtime: "async_bridge".to_owned(),
                message: e.to_string(),
            })
        }
    }
}

/// Common interface implemented by all skill runtimes.
pub trait SkillExecutor: Send + Sync {
    /// Execute the skill synchronously.
    ///
    /// # Errors
    ///
    /// Returns a [`SkillError`] variant appropriate to the runtime if
    /// execution fails.
    fn execute(&self, input: SkillInput) -> Result<SkillOutput, SkillError>;

    /// Execute with a VFS/tool context.
    ///
    /// When a [`SkillContext`] is provided, runtimes that support it will
    /// expose filesystem operations (`read_file`, `list_dir`, etc.) scoped to
    /// the project root. The default implementation delegates to
    /// [`execute`](SkillExecutor::execute), ignoring the context.
    ///
    /// # Errors
    ///
    /// Returns a [`SkillError`] variant appropriate to the runtime if
    /// execution fails.
    fn execute_with_context(
        &self,
        input: SkillInput,
        _context: Option<&SkillContext>,
    ) -> Result<SkillOutput, SkillError> {
        self.execute(input)
    }
}
