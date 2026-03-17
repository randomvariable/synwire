//! Daemon proxy for forwarding requests over a Unix domain socket.
//!
//! The MCP server handles VFS file operations (`fs.*`) locally but proxies
//! index, search, and graph requests to the long-running `synwire-daemon`
//! process via a JSON-RPC 2.0 connection over a Unix domain socket.
//!
//! # Architecture
//!
//! ```text
//! MCP server (stdio)
//!   |
//!   |-- local tools: fs.read, fs.write, fs.edit, fs.grep, fs.skeleton, ...
//!   |-- remote tools (via UDS) -----> synwire-daemon
//!       index.build, code.search, code.dependencies, ...
//! ```
//!
//! The daemon is auto-launched as a detached process if it is not already
//! running.  See [`DaemonProxy::ensure_daemon_running`].

use serde_json::Value;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use synwire_storage::StorageLayout;

/// Tool names that must be proxied to the daemon rather than handled locally.
///
/// These correspond to operations that require the daemon's long-lived state
/// (embedding models, vector indices, code graphs, cloned repositories).
pub const REMOTE_METHODS: &[&str] = &[
    "index.build",
    "index.status",
    "index.search_docs",
    "index.search_docs_hybrid",
    "code.search",
    "code.search_hybrid",
    "code.dependencies",
    "code.community_members",
    "vcs.clone",
];

/// Returns `true` if `name` is a tool that should be forwarded to the daemon.
pub fn is_remote_tool(name: &str) -> bool {
    REMOTE_METHODS.contains(&name)
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur when communicating with the daemon.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProxyError {
    /// The daemon is not running and could not be reached.
    ///
    /// Returned on non-Unix platforms where UDS is unavailable.
    #[error("daemon is not running")]
    #[cfg_attr(unix, allow(dead_code))]
    NotRunning,

    /// Failed to connect to the daemon's Unix domain socket.
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    /// Failed to write a request to the socket.
    #[error("send failed: {0}")]
    SendFailed(String),

    /// Failed to read a response from the socket.
    #[error("receive failed: {0}")]
    ReceiveFailed(String),

    /// The response was not valid JSON-RPC or could not be parsed.
    #[error("protocol error: {0}")]
    Protocol(String),

    /// The daemon returned a JSON-RPC error response.
    #[error("daemon error ({code}): {message}")]
    DaemonError {
        /// JSON-RPC error code from the daemon.
        code: i32,
        /// Human-readable error message from the daemon.
        message: String,
    },

    /// Failed to spawn the daemon process.
    #[error("failed to spawn daemon: {0}")]
    SpawnFailed(String),
}

// ---------------------------------------------------------------------------
// DaemonProxy
// ---------------------------------------------------------------------------

/// Proxy that forwards JSON-RPC requests to the synwire-daemon over a Unix
/// domain socket.
///
/// Each call to [`send_request`](DaemonProxy::send_request) opens a fresh
/// connection, sends one request, and reads one response.  This keeps the proxy
/// stateless and avoids connection-pool complexity; the overhead is negligible
/// for the expected request rate (a few requests per user interaction).
#[derive(Debug)]
pub struct DaemonProxy {
    /// Path to the daemon's Unix domain socket.
    socket_path: PathBuf,
    /// Monotonically increasing request ID.
    next_id: AtomicU64,
}

impl DaemonProxy {
    /// Create a new proxy targeting the given socket path.
    pub const fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            next_id: AtomicU64::new(1),
        }
    }

    /// Returns `true` if the daemon socket file exists on disk.
    ///
    /// This is a necessary but not sufficient condition for the daemon to be
    /// reachable -- the socket may be stale.  A definitive check requires
    /// attempting a connection.
    pub fn is_available(&self) -> bool {
        self.socket_path.exists()
    }

    /// Send a JSON-RPC 2.0 request to the daemon and return the result.
    ///
    /// # Errors
    ///
    /// Returns a [`ProxyError`] if the daemon is unreachable, the request
    /// cannot be sent, or the response is invalid.
    pub async fn send_request(&self, method: &str, params: Value) -> Result<Value, ProxyError> {
        self.send_request_impl(method, params).await
    }

    /// Check whether the daemon is running (via its PID file) and spawn it as
    /// a detached process if not.
    ///
    /// The daemon binary is expected to be on `$PATH` as
    /// `synwire-daemon`.  It receives the product name so it can locate its
    /// own [`StorageLayout`].
    ///
    /// # Errors
    ///
    /// Returns [`ProxyError::SpawnFailed`] if the daemon cannot be started.
    pub fn ensure_daemon_running(layout: &StorageLayout) -> Result<(), ProxyError> {
        // If the PID file exists and the process is alive, nothing to do.
        let pid_path = layout.daemon_pid_file();
        if pid_path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&pid_path) {
                if let Ok(pid) = contents.trim().parse::<u32>() {
                    if is_process_alive(pid) {
                        return Ok(());
                    }
                }
            }
            // Stale PID file -- remove it so the daemon can recreate it.
            let _removed = std::fs::remove_file(&pid_path);
        }

        spawn_daemon(layout)
    }

    /// Blocking wrapper around [`send_request`](Self::send_request) for use
    /// in synchronous contexts.
    ///
    /// Creates a single-threaded tokio runtime, sends the request, and blocks
    /// until the response arrives.
    ///
    /// # Errors
    ///
    /// Returns a [`ProxyError`] on connection, protocol, or daemon errors.
    pub fn send_request_blocking(&self, method: &str, params: Value) -> Result<Value, ProxyError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| ProxyError::ConnectionFailed(e.to_string()))?;
        rt.block_on(self.send_request(method, params))
    }

    // -- convenience wrappers ------------------------------------------------
    //
    // Typed helpers that provide compile-time-checked parameter names rather
    // than raw JSON.  Both async and blocking variants are provided so they
    // can be called from the synchronous MCP stdio loop.

    /// Request the daemon to index (or re-index) a worktree.
    pub async fn index(&self, worktree_root: &str) -> Result<Value, ProxyError> {
        self.send_request(
            "index",
            serde_json::json!({ "worktree_root": worktree_root }),
        )
        .await
    }

    /// Blocking variant of [`index`](Self::index).
    ///
    /// # Errors
    ///
    /// Returns a [`ProxyError`] on connection, protocol, or daemon errors.
    pub fn index_blocking(&self, worktree_root: &str) -> Result<Value, ProxyError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| ProxyError::ConnectionFailed(e.to_string()))?;
        rt.block_on(self.index(worktree_root))
    }

    /// Query the code dependency graph for a worktree.
    pub async fn graph_query(&self, query: &str, worktree_id: &str) -> Result<Value, ProxyError> {
        self.send_request(
            "graph_query",
            serde_json::json!({ "query": query, "worktree_id": worktree_id }),
        )
        .await
    }

    /// Blocking variant of [`graph_query`](Self::graph_query).
    ///
    /// # Errors
    ///
    /// Returns a [`ProxyError`] on connection, protocol, or daemon errors.
    pub fn graph_query_blocking(
        &self,
        query: &str,
        worktree_id: &str,
    ) -> Result<Value, ProxyError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| ProxyError::ConnectionFailed(e.to_string()))?;
        rt.block_on(self.graph_query(query, worktree_id))
    }

    /// Ask the daemon to clone a repository into its cache.
    pub async fn clone_repo(&self, url: &str, dest: &str) -> Result<Value, ProxyError> {
        self.send_request(
            "clone_repo",
            serde_json::json!({ "url": url, "dest": dest }),
        )
        .await
    }

    /// Blocking variant of [`clone_repo`](Self::clone_repo).
    ///
    /// # Errors
    ///
    /// Returns a [`ProxyError`] on connection, protocol, or daemon errors.
    pub fn clone_repo_blocking(&self, url: &str, dest: &str) -> Result<Value, ProxyError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| ProxyError::ConnectionFailed(e.to_string()))?;
        rt.block_on(self.clone_repo(url, dest))
    }
}

// ---------------------------------------------------------------------------
// Platform-specific UDS implementation
// ---------------------------------------------------------------------------

#[cfg(unix)]
impl DaemonProxy {
    /// Platform-specific implementation that connects via `UnixStream`.
    async fn send_request_impl(&self, method: &str, params: Value) -> Result<Value, ProxyError> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::UnixStream;

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let mut payload =
            serde_json::to_vec(&request).map_err(|e| ProxyError::Protocol(e.to_string()))?;
        payload.push(b'\n');

        let stream = UnixStream::connect(&self.socket_path)
            .await
            .map_err(|e| ProxyError::ConnectionFailed(e.to_string()))?;

        let (reader, mut writer) = stream.into_split();

        writer
            .write_all(&payload)
            .await
            .map_err(|e| ProxyError::SendFailed(e.to_string()))?;
        writer
            .shutdown()
            .await
            .map_err(|e| ProxyError::SendFailed(e.to_string()))?;

        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();
        let _bytes_read = buf_reader
            .read_line(&mut line)
            .await
            .map_err(|e| ProxyError::ReceiveFailed(e.to_string()))?;

        if line.is_empty() {
            return Err(ProxyError::ReceiveFailed(
                "daemon closed connection without responding".to_owned(),
            ));
        }

        let response: Value =
            serde_json::from_str(line.trim()).map_err(|e| ProxyError::Protocol(e.to_string()))?;

        // Check for JSON-RPC error.
        if let Some(err_obj) = response.get("error") {
            #[allow(clippy::cast_possible_truncation)]
            let code = err_obj.get("code").and_then(Value::as_i64).unwrap_or(-1) as i32;
            let message = err_obj
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
                .to_owned();
            return Err(ProxyError::DaemonError { code, message });
        }

        response
            .get("result")
            .cloned()
            .ok_or_else(|| ProxyError::Protocol("response missing 'result' field".to_owned()))
    }
}

#[cfg(not(unix))]
impl DaemonProxy {
    /// Non-Unix fallback -- UDS is not available.
    async fn send_request_impl(&self, _method: &str, _params: Value) -> Result<Value, ProxyError> {
        Err(ProxyError::NotRunning)
    }
}

// ---------------------------------------------------------------------------
// Daemon lifecycle helpers
// ---------------------------------------------------------------------------

/// Spawn the daemon as a detached child process.
fn spawn_daemon(layout: &StorageLayout) -> Result<(), ProxyError> {
    #[cfg(unix)]
    {
        spawn_daemon_unix(layout)
    }

    #[cfg(not(unix))]
    {
        let _ = layout;
        Err(ProxyError::SpawnFailed(
            "daemon spawning is only supported on Unix".to_owned(),
        ))
    }
}

/// Unix-specific daemon spawn using `Command` with stdio redirected to
/// `/dev/null` so the child survives the parent's exit.
#[cfg(unix)]
fn spawn_daemon_unix(layout: &StorageLayout) -> Result<(), ProxyError> {
    use std::process::{Command, Stdio};

    // Ensure the data directory exists so the daemon can write its PID file
    // and socket.
    let _ensure = layout.ensure_dir(layout.data_home());

    let _child = Command::new("synwire-daemon")
        .arg("--product")
        .arg(layout.product_name())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| ProxyError::SpawnFailed(e.to_string()))?;

    // The daemon will write its own PID file once it's ready.
    // We do not wait for it here -- callers should retry if the socket is
    // not yet available.
    Ok(())
}

/// Check whether a process with the given PID is still alive.
fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        is_process_alive_unix(pid)
    }

    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

/// Send signal 0 to probe process liveness (Unix only).
#[cfg(unix)]
fn is_process_alive_unix(pid: u32) -> bool {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;

    // `kill(pid, None)` sends signal 0: it does not deliver a signal but
    // checks whether the process exists and the caller has permission to
    // signal it.  This is the standard POSIX liveness probe.
    #[allow(clippy::cast_possible_wrap)]
    let nix_pid = Pid::from_raw(pid as i32);
    kill(nix_pid, None).is_ok()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn remote_methods_contains_expected_entries() {
        assert!(is_remote_tool("index.build"));
        assert!(is_remote_tool("index.status"));
        assert!(is_remote_tool("index.search_docs"));
        assert!(is_remote_tool("index.search_docs_hybrid"));
        assert!(is_remote_tool("code.search"));
        assert!(is_remote_tool("code.search_hybrid"));
        assert!(is_remote_tool("code.dependencies"));
        assert!(is_remote_tool("code.community_members"));
        assert!(is_remote_tool("vcs.clone"));
    }

    #[test]
    fn local_tools_are_not_remote() {
        assert!(!is_remote_tool("fs.read"));
        assert!(!is_remote_tool("fs.write"));
        assert!(!is_remote_tool("fs.edit"));
        assert!(!is_remote_tool("fs.grep"));
        assert!(!is_remote_tool("fs.skeleton"));
        assert!(!is_remote_tool("meta.list"));
    }

    #[test]
    fn proxy_reports_unavailable_for_missing_socket() {
        let proxy = DaemonProxy::new(PathBuf::from("/nonexistent/daemon.sock"));
        assert!(!proxy.is_available());
    }

    #[tokio::test]
    async fn send_request_fails_when_socket_missing() {
        let proxy = DaemonProxy::new(PathBuf::from("/nonexistent/daemon.sock"));
        let result = proxy.send_request("index", serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[test]
    fn proxy_error_display() {
        let err = ProxyError::DaemonError {
            code: -32600,
            message: "invalid request".to_owned(),
        };
        let msg = err.to_string();
        assert!(msg.contains("-32600"));
        assert!(msg.contains("invalid request"));
    }

    #[cfg(unix)]
    #[test]
    fn is_process_alive_returns_true_for_self() {
        let pid = std::process::id();
        assert!(is_process_alive(pid));
    }

    #[cfg(unix)]
    #[test]
    fn is_process_alive_returns_false_for_nonexistent() {
        // PID 4_000_000 is almost certainly not in use.
        assert!(!is_process_alive(4_000_000));
    }

    #[test]
    fn ensure_daemon_running_with_fresh_layout() {
        let dir = tempfile::tempdir().unwrap();
        let layout = StorageLayout::with_root(dir.path(), "synwire-test");
        // No PID file exists, and synwire-daemon is not on PATH in tests,
        // so this should return SpawnFailed.
        let result = DaemonProxy::ensure_daemon_running(&layout);
        assert!(matches!(result, Err(ProxyError::SpawnFailed(_))));
    }
}
