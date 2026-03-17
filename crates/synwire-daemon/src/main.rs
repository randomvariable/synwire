//! Synwire daemon -- singleton background process per product.
//!
//! The daemon manages all repositories and worktrees for a product, owning the
//! embedding model, file watchers, indexing pipelines, and the global tier
//! (registry, dependency index, cross-references, experience pool).
//!
//! MCP servers connect via a Unix domain socket at
//! `StorageLayout::daemon_socket()` as thin stdio-to-UDS proxies.
//!
//! # Auto-launch
//!
//! The daemon is spawned as a detached process by the first MCP server
//! instance.  It exits after a 5-minute grace period with no active clients.

#![forbid(unsafe_code)]

use std::process;

use synwire_storage::StorageLayout;
use tracing::{error, info};

mod lifecycle;

#[tokio::main]
async fn main() {
    tracing_subscriber_init();

    let product_name = std::env::var("SYNWIRE_PRODUCT").unwrap_or_else(|_| "synwire".to_owned());

    let layout = match StorageLayout::new(&product_name) {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to initialise storage layout: {e}");
            process::exit(1);
        }
    };

    // Ensure the data directory exists before writing the PID file / socket.
    if let Err(e) = layout.ensure_dir(layout.data_home()) {
        error!(path = %layout.data_home().display(), "Failed to create data directory: {e}");
        process::exit(1);
    }

    let pid_path = layout.daemon_pid_file();
    let sock_path = layout.daemon_socket();

    // ── Check for an existing daemon ───────────────────────────────────
    if let Err(msg) = lifecycle::check_existing_daemon(&pid_path) {
        error!("{msg}");
        process::exit(1);
    }

    // ── Write PID file ─────────────────────────────────────────────────
    if let Err(e) = lifecycle::write_pid_file(&pid_path) {
        error!(path = %pid_path.display(), "Failed to write PID file: {e}");
        process::exit(1);
    }

    info!(
        product = %product_name,
        pid = std::process::id(),
        data_home = %layout.data_home().display(),
        "synwire-daemon starting"
    );

    // ── Run the main loop ──────────────────────────────────────────────
    let exit_code = lifecycle::run_daemon(&sock_path, &pid_path).await;

    // ── Cleanup (best-effort) ──────────────────────────────────────────
    lifecycle::cleanup(&pid_path, &sock_path);

    info!("synwire-daemon exiting");
    process::exit(exit_code);
}

/// Initialise the `tracing` subscriber from the `RUST_LOG` environment
/// variable.
fn tracing_subscriber_init() {
    use tracing_subscriber::EnvFilter;
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();
}
