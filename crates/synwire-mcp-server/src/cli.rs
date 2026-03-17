//! CLI argument parsing for the Synwire MCP server.
//!
//! The server accepts arguments directly or via a TOML/JSON config file
//! (`--config`). Explicit flags override config file values.

use clap::Parser;
use std::path::PathBuf;

/// Synwire MCP server — expose Synwire tools via stdio MCP transport.
///
/// Configure Claude Code by adding to `.claude/mcp.json`:
/// ```json
/// { "synwire": { "command": "synwire-mcp-server", "args": ["--project", "."] } }
/// ```
#[derive(Debug, Parser)]
#[command(name = "synwire-mcp-server", version, about)]
pub struct Cli {
    /// Root directory of the project to index and serve.
    #[arg(long, short = 'p', env = "SYNWIRE_PROJECT")]
    pub project: Option<PathBuf>,

    /// Product name used to scope persistent storage paths.
    #[arg(long, default_value = "synwire", env = "SYNWIRE_PRODUCT")]
    pub product_name: String,

    /// Language server command to launch (e.g., `rust-analyzer`).
    ///
    /// When not specified and `--project` is set, auto-detection selects the
    /// best available server for the project's primary language.
    #[arg(long, env = "SYNWIRE_LSP")]
    pub lsp: Option<String>,

    /// Disable LSP auto-detection and all LSP tools.
    #[arg(long, default_value_t = false)]
    pub no_lsp: bool,

    /// Debug adapter command to launch (e.g., `codelldb`).
    ///
    /// When not specified and `--project` is set, auto-detection selects the
    /// best available adapter for the project's primary language.
    #[arg(long, env = "SYNWIRE_DAP")]
    pub dap: Option<String>,

    /// Disable DAP auto-detection and all DAP tools.
    #[arg(long, default_value_t = false)]
    pub no_dap: bool,

    /// Embedding model identifier (fastembed model name).
    #[arg(
        long,
        default_value = "bge-small-en-v1.5",
        env = "SYNWIRE_EMBEDDING_MODEL"
    )]
    pub embedding_model: String,

    /// Log verbosity level (`error`, `warn`, `info`, `debug`, `trace`).
    #[arg(long, default_value = "info", env = "RUST_LOG")]
    pub log_level: String,

    /// Path to a TOML or JSON config file. CLI flags override config values.
    #[arg(long, short = 'c')]
    pub config: Option<PathBuf>,
}
