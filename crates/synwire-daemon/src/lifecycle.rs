//! Daemon lifecycle management: PID file, Unix domain socket, grace period,
//! signal handling, and cleanup.
//!
//! This module contains all the logic for the daemon's lifecycle:
//!
//! - **PID file** at `StorageLayout::daemon_pid_file()` prevents duplicate
//!   instances and allows clients to detect a running daemon.
//! - **Unix domain socket** at `StorageLayout::daemon_socket()` accepts
//!   connections from MCP server proxies.
//! - **Grace period** (5 minutes) triggers automatic shutdown when no clients
//!   are connected.
//! - **Signal handling** catches SIGTERM/SIGINT for clean shutdown.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::Notify;
use tracing::{error, info, warn};

/// Duration of the grace period before the daemon shuts down after the last
/// client disconnects.
pub const GRACE_PERIOD: std::time::Duration = std::time::Duration::from_secs(5 * 60);

// ── Main entry point ────────────────────────────────────────────────────────

/// Run the daemon's accept loop and grace-period shutdown logic.
///
/// Binds a Unix domain socket at `sock_path`, accepts client connections, and
/// monitors for shutdown conditions (signal or grace-period expiry). Returns
/// `0` on clean shutdown, `1` on error.
pub async fn run_daemon(sock_path: &Path, pid_path: &Path) -> i32 {
    // Remove a stale socket from a previous run.
    remove_stale_socket(sock_path);

    let Some(listener) = bind_listener(sock_path) else {
        cleanup(pid_path, sock_path);
        return 1;
    };

    info!(path = %sock_path.display(), "listening on Unix domain socket");

    let client_count = Arc::new(AtomicUsize::new(0));
    let client_changed = Arc::new(Notify::new());

    // Spawn the accept loop.
    let accept_clients = client_count.clone();
    let accept_notify = client_changed.clone();
    let accept_handle = tokio::spawn(async move {
        accept_loop(listener, accept_clients, accept_notify).await;
    });

    // Wait for a shutdown signal or the grace-period to expire.
    let reason = shutdown_monitor(client_count, client_changed).await;
    info!(reason = %reason, "initiating shutdown");

    // Cancel the accept loop.
    accept_handle.abort();
    let _ = accept_handle.await;

    0
}

// ── Accept loop ─────────────────────────────────────────────────────────────

/// Accept incoming Unix-socket connections, tracking the active client count.
#[cfg(unix)]
async fn accept_loop(
    listener: tokio::net::UnixListener,
    client_count: Arc<AtomicUsize>,
    client_changed: Arc<Notify>,
) {
    loop {
        let (stream, _addr) = match listener.accept().await {
            Ok(pair) => pair,
            Err(e) => {
                warn!("Failed to accept connection: {e}");
                continue;
            }
        };

        let count = client_count.clone();
        let notify = client_changed.clone();

        let prev = count.fetch_add(1, Ordering::SeqCst);
        info!(clients = prev + 1, "client connected");
        notify.notify_waiters();

        // Spawn a task per client.  For now the client simply holds the
        // connection open -- no IPC protocol is implemented yet (future phase).
        let _handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            handle_client(stream).await;

            let prev = count.fetch_sub(1, Ordering::SeqCst);
            info!(clients = prev - 1, "client disconnected");
            notify.notify_waiters();
        });
    }
}

/// Placeholder per-client handler.  Waits until the remote end closes the
/// connection (or the task is cancelled).
#[cfg(unix)]
async fn handle_client(stream: tokio::net::UnixStream) {
    // Wait for the readable half to signal EOF (remote close).
    let _ = stream.readable().await;
    // Drain -- the stream may become readable multiple times before true EOF.
    let mut buf = [0u8; 1024];
    loop {
        match stream.try_read(&mut buf) {
            // EOF or error (including WouldBlock) -- treat as disconnect.
            Ok(0) | Err(_) => break,
            Ok(_) => {
                // Discard data; no IPC protocol yet.
                let _ = stream.readable().await;
            }
        }
    }
}

/// Placeholder accept loop on non-Unix platforms (no-op, just waits forever).
#[cfg(not(unix))]
async fn accept_loop(_listener: (), _client_count: Arc<AtomicUsize>, _client_changed: Arc<Notify>) {
    // No UDS on this platform; the daemon will shut down via signal only.
    std::future::pending::<()>().await;
}

// ── Shutdown monitoring ─────────────────────────────────────────────────────

/// Wait for either a termination signal or grace-period expiry.
///
/// Returns a human-readable reason string describing why shutdown was
/// initiated.
async fn shutdown_monitor(
    client_count: Arc<AtomicUsize>,
    client_changed: Arc<Notify>,
) -> &'static str {
    // Start with the grace period running (no clients yet).
    loop {
        let clients = client_count.load(Ordering::SeqCst);

        if clients > 0 {
            // While clients are connected, just wait for a signal or a client
            // change event.
            tokio::select! {
                () = shutdown_signal() => return "received shutdown signal",
                () = client_changed.notified() => {}
            }
        } else {
            // No clients -- start the grace-period timer.
            info!(
                grace_secs = GRACE_PERIOD.as_secs(),
                "no active clients, grace period started"
            );

            tokio::select! {
                () = shutdown_signal() => return "received shutdown signal",
                () = tokio::time::sleep(GRACE_PERIOD) => {
                    // Recheck: a client might have connected just before the
                    // timer fired.
                    if client_count.load(Ordering::SeqCst) == 0 {
                        return "grace period expired with no clients";
                    }
                    // Clients reconnected -- loop around.
                }
                () = client_changed.notified() => {}
            }
        }
    }
}

/// Wait for SIGTERM or SIGINT (Unix) / Ctrl-C (all platforms).
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm = signal(SignalKind::terminate()).unwrap_or_else(|e| {
            // We cannot recover from this, but we also cannot panic.
            error!("Failed to register SIGTERM handler: {e}");
            std::process::exit(1);
        });
        let mut sigint = signal(SignalKind::interrupt()).unwrap_or_else(|e| {
            error!("Failed to register SIGINT handler: {e}");
            std::process::exit(1);
        });
        tokio::select! {
            () = async { let _ = sigterm.recv().await; } => {}
            () = async { let _ = sigint.recv().await; } => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

// ── PID file helpers ────────────────────────────────────────────────────────

/// Check whether an existing PID file points to a running process.
///
/// Returns `Ok(())` if no daemon is running (or the PID file is stale).
/// Returns `Err(message)` if another daemon is still alive.
pub fn check_existing_daemon(pid_path: &Path) -> Result<(), String> {
    let contents = match std::fs::read_to_string(pid_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(format!(
                "Unable to read PID file {}: {e}",
                pid_path.display()
            ));
        }
    };

    let Ok(pid) = contents.trim().parse::<u32>() else {
        warn!(
            path = %pid_path.display(),
            contents = %contents.trim(),
            "Removing PID file with invalid contents"
        );
        let _ = std::fs::remove_file(pid_path);
        return Ok(());
    };

    if is_process_alive(pid) {
        return Err(format!(
            "Another synwire-daemon is already running (PID {pid})"
        ));
    }

    // Stale PID file -- the process is gone.
    warn!(pid, "Removing stale PID file for dead process");
    let _ = std::fs::remove_file(pid_path);
    Ok(())
}

/// Write the current process ID to the PID file.
pub fn write_pid_file(pid_path: &Path) -> std::io::Result<()> {
    std::fs::write(pid_path, format!("{}\n", std::process::id()))
}

/// Check whether a process with the given PID is alive.
#[cfg(unix)]
fn is_process_alive(pid: u32) -> bool {
    // kill(pid, 0) checks existence without sending a signal.
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(i32::try_from(pid).unwrap_or(0)),
        None,
    )
    .is_ok()
}

/// Check whether a process with the given PID is alive (non-Unix stub).
#[cfg(not(unix))]
fn is_process_alive(_pid: u32) -> bool {
    // Conservative: assume it might still be running. The user can manually
    // delete the PID file if needed.
    true
}

// ── Socket helpers ──────────────────────────────────────────────────────────

/// Remove a leftover socket file from a previous daemon run.
fn remove_stale_socket(sock_path: &Path) {
    if sock_path.exists() {
        warn!(path = %sock_path.display(), "Removing stale daemon socket");
        let _ = std::fs::remove_file(sock_path);
    }
}

/// Bind a `UnixListener` on the given path, returning `None` on failure.
#[cfg(unix)]
fn bind_listener(sock_path: &Path) -> Option<tokio::net::UnixListener> {
    match tokio::net::UnixListener::bind(sock_path) {
        Ok(l) => {
            // Restrict socket permissions to owner only.
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            if let Err(e) = std::fs::set_permissions(sock_path, perms) {
                warn!(
                    path = %sock_path.display(),
                    "Failed to restrict socket permissions: {e}"
                );
            }
            Some(l)
        }
        Err(e) => {
            error!(path = %sock_path.display(), "Failed to bind Unix socket: {e}");
            None
        }
    }
}

/// On non-Unix platforms, UDS is not available.
#[cfg(not(unix))]
fn bind_listener(sock_path: &Path) -> Option<()> {
    warn!(
        path = %sock_path.display(),
        "Unix domain sockets are not supported on this platform; running without IPC listener"
    );
    Some(())
}

// ── Cleanup ─────────────────────────────────────────────────────────────────

/// Best-effort removal of the PID file and socket.
pub fn cleanup(pid_path: &Path, sock_path: &Path) {
    if let Err(e) = std::fs::remove_file(pid_path)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        warn!(path = %pid_path.display(), "Failed to remove PID file: {e}");
    }
    if let Err(e) = std::fs::remove_file(sock_path)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        warn!(path = %sock_path.display(), "Failed to remove socket: {e}");
    }
}
