//! Synwire MCP server — a standalone binary exposing Synwire tools via stdio.
//!
//! # Quick start (Claude Code)
//!
//! Add to `.claude/mcp.json`:
//! ```json
//! {
//!   "synwire": {
//!     "command": "synwire-mcp-server",
//!     "args": ["--project", "/path/to/your/project"]
//!   }
//! }
//! ```
//!
//! Then install: `cargo install synwire-mcp-server`

#![forbid(unsafe_code)]

mod cli;
mod config;
mod proxy;
mod sampling;
mod server;
mod tools;

use clap::Parser;
use cli::Cli;
use config::ServerConfig;
use server::{McpServer, ServerOptions};
use std::path::Path;
use std::process;
use tracing::error;
#[cfg(any(feature = "lsp", feature = "dap"))]
use tracing::info;

fn main() {
    let cli = Cli::parse();

    // Initialise logging to stderr (never stdout — that's reserved for MCP).
    // Compute the logs directory from StorageLayout so log files land alongside
    // other product data. Falls back to stderr-only if the layout cannot be
    // resolved.
    let log_layout = synwire_storage::StorageLayout::new(&cli.product_name);
    let logs_dir_buf = log_layout
        .as_ref()
        .ok()
        .map(synwire_storage::StorageLayout::logs_dir);
    init_logging(&cli.log_level, logs_dir_buf.as_deref());

    // Load config file if specified, then merge with CLI flags.
    let file_cfg = cli
        .config
        .as_ref()
        .map_or_else(
            ServerConfig::default,
            |config_path| match ServerConfig::load(config_path) {
                Ok(c) => c,
                Err(e) => {
                    error!(
                        "Failed to load config file {}: {}",
                        config_path.display(),
                        e
                    );
                    process::exit(1);
                }
            },
        );

    // CLI flags take precedence over config file.
    let project = cli.project.or(file_cfg.project);

    // Resolve LSP: explicit flag > config file > auto-detect (unless --no-lsp).
    let lsp = resolve_lsp(
        cli.lsp.as_deref(),
        file_cfg.lsp.as_deref(),
        cli.no_lsp,
        project.as_deref(),
    );

    // Resolve DAP: explicit flag > config file > auto-detect (unless --no-dap).
    let dap = resolve_dap(
        cli.dap.as_deref(),
        file_cfg.dap.as_deref(),
        cli.no_dap,
        project.as_deref(),
    );

    let options = ServerOptions {
        project,
        product_name: file_cfg.product_name.unwrap_or(cli.product_name),
        embedding_model: file_cfg.embedding_model.unwrap_or(cli.embedding_model),
        lsp,
        dap,
    };

    let server = match McpServer::new(options) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to initialise MCP server: {e}");
            process::exit(1);
        }
    };

    if let Err(e) = server.serve() {
        error!("MCP server error: {e}");
        process::exit(1);
    }
}

/// Initialise structured logging to stderr and optionally to a daily-rotating
/// log file under `logs_dir`.
///
/// All output goes to **stderr** — stdout is reserved for the MCP protocol.
/// The log level is read from `RUST_LOG` if set, otherwise `level` is used.
fn init_logging(level: &str, logs_dir: Option<&std::path::Path>) {
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::prelude::*;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    let stderr_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);

    if let Some(dir) = logs_dir
        && std::fs::create_dir_all(dir).is_ok()
    {
        let file_appender = tracing_appender::rolling::daily(dir, "synwire-mcp-server.log");
        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(file_appender)
            .with_ansi(false);
        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(stderr_layer)
            .with(file_layer)
            .try_init();
        return;
    }

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(stderr_layer)
        .try_init();
}

/// Resolve the LSP server to use.
///
/// Priority: explicit CLI flag > config file value > auto-detect from project.
/// Returns `None` when disabled via `--no-lsp` or when no server can be found.
#[cfg(feature = "lsp")]
fn resolve_lsp(
    cli_lsp: Option<&str>,
    cfg_lsp: Option<&str>,
    no_lsp: bool,
    project: Option<&Path>,
) -> Option<String> {
    if no_lsp {
        return None;
    }

    // Explicit value takes priority.
    if let Some(lsp) = cli_lsp.or(cfg_lsp) {
        return Some(lsp.to_owned());
    }

    // Auto-detect from project directory.
    let project_root = project?;
    let registry = synwire_lsp::registry::LanguageServerRegistry::default_registry();
    let detected = registry.detect_for_project(project_root);
    if detected.is_empty() {
        info!("No language servers auto-detected for project");
        return None;
    }

    let names: Vec<&str> = detected.iter().map(|e| e.name.as_str()).collect();
    info!(servers = ?names, "Auto-detected language servers");

    // Return the first (primary) detected server.
    detected.first().map(|e| e.name.clone())
}

/// Resolve the LSP server to use (stub when `lsp` feature is disabled).
#[cfg(not(feature = "lsp"))]
fn resolve_lsp(
    cli_lsp: Option<&str>,
    cfg_lsp: Option<&str>,
    _no_lsp: bool,
    _project: Option<&Path>,
) -> Option<String> {
    cli_lsp.or(cfg_lsp).map(str::to_owned)
}

/// Resolve the DAP adapter to use.
///
/// Priority: explicit CLI flag > config file value > auto-detect from project.
/// Returns `None` when disabled via `--no-dap` or when no adapter can be found.
#[cfg(feature = "dap")]
fn resolve_dap(
    cli_dap: Option<&str>,
    cfg_dap: Option<&str>,
    no_dap: bool,
    project: Option<&Path>,
) -> Option<String> {
    if no_dap {
        return None;
    }

    // Explicit value takes priority.
    if let Some(dap) = cli_dap.or(cfg_dap) {
        return Some(dap.to_owned());
    }

    // Auto-detect from project directory.
    let project_root = project?;
    let registry = synwire_dap::DebugAdapterRegistry::with_builtins();
    let detected = registry.detect_for_project(project_root);
    if detected.is_empty() {
        info!("No debug adapters auto-detected for project");
        return None;
    }

    let names: Vec<&str> = detected.iter().map(|e| e.name.as_str()).collect();
    info!(adapters = ?names, "Auto-detected debug adapters");

    // Return the first (primary) detected adapter.
    detected.first().map(|e| e.name.clone())
}

/// Resolve the DAP adapter to use (stub when `dap` feature is disabled).
#[cfg(not(feature = "dap"))]
fn resolve_dap(
    cli_dap: Option<&str>,
    cfg_dap: Option<&str>,
    _no_dap: bool,
    _project: Option<&Path>,
) -> Option<String> {
    cli_dap.or(cfg_dap).map(str::to_owned)
}
