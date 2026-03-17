//! # synwire
//!
//! Convenience re-exports and reference implementations for Synwire.
//!
//! This crate provides ready-to-use implementations for common patterns:
//! chat history management, embedding cache, few-shot prompts,
//! text splitters, and additional output parsers.

#![deny(unsafe_code)]

pub use synwire_core as core;

/// Agent core re-exports — types needed to build and run agents.
pub mod agent {
    /// Commonly used agent types — glob-import for quick access.
    pub mod prelude {
        pub use synwire_core::agents::agent_node::{
            Agent, AgentNode, ModelErrorAction, OutputMode, RunContext,
        };
        pub use synwire_core::agents::directive::{Directive, DirectiveResult};
        pub use synwire_core::agents::error::AgentError;
        pub use synwire_core::agents::hooks::{HookMatcher, HookRegistry, HookResult};
        pub use synwire_core::agents::runner::Runner;
        pub use synwire_core::agents::session::{Session, SessionManager, SessionMetadata};
        pub use synwire_core::agents::streaming::{AgentEvent, TerminationReason};
        pub use synwire_core::agents::usage::Usage;
    }
}

/// Embedding cache backed by moka.
pub mod cache;

/// Chat message history traits and implementations.
pub mod chat_history;

/// Few-shot prompt templates and example selectors.
pub mod prompts;

/// Text splitter implementations for chunking documents.
pub mod text_splitters;

/// Additional output parser implementations.
pub mod output_parsers;

/// Sandbox integration for agent tool calling.
///
/// Provides `SandboxedAgent::with_sandbox()` to wire up process tracking
/// and the `ProcessPlugin` automatically when a `SandboxConfig` is set.
///
/// Requires the `sandbox` feature: `synwire = { features = ["sandbox"] }`.
#[cfg(feature = "sandbox")]
pub mod sandbox;

/// Language Server Protocol integration.
///
/// Provides [`LspPlugin`](synwire_lsp::plugin::LspPlugin) for adding LSP
/// tools (go-to-definition, hover, diagnostics, etc.) to agents, plus a
/// [`LanguageServerRegistry`](synwire_lsp::registry::LanguageServerRegistry)
/// with built-in entries for popular language servers.
///
/// Requires the `lsp` feature: `synwire = { features = ["lsp"] }`.
#[cfg(feature = "lsp")]
pub use synwire_lsp as lsp;

/// Debug Adapter Protocol integration.
///
/// Provides [`DapPlugin`](synwire_dap::plugin::DapPlugin) for adding debug
/// tools (breakpoints, stepping, variable inspection) to agents, plus a
/// [`DebugAdapterRegistry`](synwire_dap::registry::DebugAdapterRegistry)
/// with built-in entries for popular debug adapters.
///
/// Requires the `dap` feature: `synwire = { features = ["dap"] }`.
#[cfg(feature = "dap")]
pub use synwire_dap as dap;
