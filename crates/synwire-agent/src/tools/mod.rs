//! Pre-built tool providers for common coding agent patterns.
//!
//! Compose these with [`CompositeToolProvider`] to assemble a full tool suite.
//! Each sub-module builds a `StaticToolProvider` containing namespaced tools
//! for a specific domain.
//!
//! # Provider composition
//!
//! VFS, LSP, and DAP tool providers live in their respective crates
//! (`synwire-core::vfs`, `synwire-lsp`, `synwire-dap`) because they require
//! runtime dependencies (VFS instance, LSP client, DAP client). The
//! [`default_tool_provider`] assembles only the providers that need no external
//! runtime state; consumers add VFS/LSP/DAP providers at startup.
//!
//! ```rust,no_run
//! use synwire_agent::tools::{DefaultToolConfig, default_tool_provider};
//!
//! let provider = default_tool_provider(DefaultToolConfig::new().with_meta());
//! ```

pub mod code;
pub mod index;
pub mod meta;

use synwire_core::tools::CompositeToolProvider;

/// Configuration for [`default_tool_provider`].
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct DefaultToolConfig {
    /// Whether to include `meta.*` tools (tool search and listing).
    pub include_meta: bool,
}

impl DefaultToolConfig {
    /// Create a new configuration with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            include_meta: false,
        }
    }

    /// Enable `meta.*` tools (tool search and listing).
    #[must_use]
    pub const fn with_meta(mut self) -> Self {
        self.include_meta = true;
        self
    }
}

/// Assemble the default tool suite for a coding agent.
///
/// Includes `code.*` and `index.*` tool providers. Optionally includes
/// `meta.*` tools when [`DefaultToolConfig::include_meta`] is set.
///
/// VFS, LSP, and DAP providers are **not** included because they require
/// runtime dependencies. Add them via [`CompositeToolProvider`] at startup.
///
/// # Errors
///
/// Returns [`synwire_core::error::SynwireError`] if any tool fails validation.
pub fn default_tool_provider(
    config: DefaultToolConfig,
) -> Result<CompositeToolProvider, synwire_core::error::SynwireError> {
    let mut providers: Vec<Box<dyn synwire_core::tools::ToolProvider>> =
        vec![code::code_tool_provider()?, index::index_tool_provider()?];
    if config.include_meta {
        providers.push(meta::meta_tool_provider()?);
    }
    Ok(CompositeToolProvider::with_keep_first(providers))
}
