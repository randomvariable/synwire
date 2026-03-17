//! Low-level DAP transport managing child process I/O and request correlation.
//!
//! Spawns a debug adapter as a child process, wraps its stdin/stdout in a
//! Content-Length framed codec, and correlates request/response pairs by
//! DAP sequence number.

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use futures_util::{SinkExt, StreamExt};
use tokio::process::{Child, ChildStdout, Command};
use tokio::sync::{Mutex, RwLock, oneshot};
use tokio_util::codec::{FramedRead, FramedWrite};

use crate::codec::ContentLengthCodec;
use crate::error::DapError;

/// Callback type for DAP events received from the adapter.
pub type EventHandler = Arc<dyn Fn(serde_json::Value) + Send + Sync>;

/// Map of pending request sequence numbers to their response channels.
type PendingMap = HashMap<i64, oneshot::Sender<serde_json::Value>>;

/// Low-level DAP transport managing child process I/O and request correlation.
///
/// The transport spawns a background task that reads framed messages from the
/// adapter's stdout. Responses are correlated to pending requests by their
/// `request_seq` field; events are forwarded to an [`EventHandler`] callback.
pub struct DapTransport {
    writer: Arc<Mutex<FramedWrite<tokio::process::ChildStdin, ContentLengthCodec>>>,
    pending: Arc<RwLock<PendingMap>>,
    event_handler: EventHandler,
    next_seq: AtomicI64,
    _child: Arc<Mutex<Child>>,
    _read_handle: tokio::task::JoinHandle<()>,
}

impl DapTransport {
    /// Spawn a debug adapter process and set up framed transport.
    ///
    /// # Errors
    ///
    /// Returns [`DapError::Io`] if the process cannot be spawned, or
    /// [`DapError::Transport`] if stdin/stdout cannot be captured.
    pub fn spawn(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
        event_handler: EventHandler,
    ) -> Result<Self, DapError> {
        let mut cmd = Command::new(command);
        let _ = cmd
            .args(args)
            .envs(env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        let mut child = cmd.spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| DapError::Transport("failed to capture adapter stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| DapError::Transport("failed to capture adapter stdout".into()))?;

        let writer = Arc::new(Mutex::new(FramedWrite::new(
            stdin,
            ContentLengthCodec::new(),
        )));
        let reader = FramedRead::new(stdout, ContentLengthCodec::new());

        let pending: Arc<RwLock<PendingMap>> = Arc::new(RwLock::new(HashMap::new()));
        let child_arc = Arc::new(Mutex::new(child));

        let read_handle =
            Self::spawn_reader(reader, Arc::clone(&pending), Arc::clone(&event_handler));

        Ok(Self {
            writer,
            pending,
            event_handler,
            next_seq: AtomicI64::new(1),
            _child: child_arc,
            _read_handle: read_handle,
        })
    }

    /// Send a DAP request and wait for the correlated response.
    ///
    /// # Errors
    ///
    /// Returns [`DapError::Transport`] if the write fails, [`DapError::Timeout`]
    /// if the response channel is dropped, or [`DapError::RequestFailed`] if
    /// the adapter returns `success: false`.
    pub async fn send_request(
        &self,
        command: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, DapError> {
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);

        let request = serde_json::json!({
            "seq": seq,
            "type": "request",
            "command": command,
            "arguments": arguments,
        });

        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending.write().await;
            let _ = pending.insert(seq, tx);
        }

        self.writer
            .lock()
            .await
            .send(request)
            .await
            .map_err(|e| DapError::Transport(format!("failed to send request: {e}")))?;

        tracing::debug!(seq, command, "DAP request sent");

        let response = rx.await.map_err(|_| DapError::Timeout)?;

        // Check for error responses.
        let success = response
            .get("success")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        if !success {
            let message = response
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown error")
                .to_string();
            return Err(DapError::RequestFailed {
                command: command.to_string(),
                message,
            });
        }

        Ok(response)
    }

    /// Access the event handler (used by plugin to register callbacks).
    #[must_use]
    pub fn event_handler(&self) -> &EventHandler {
        &self.event_handler
    }

    /// Spawn a background task to read messages from the adapter and dispatch them.
    fn spawn_reader(
        mut reader: FramedRead<ChildStdout, ContentLengthCodec>,
        pending: Arc<RwLock<PendingMap>>,
        event_handler: EventHandler,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(result) = reader.next().await {
                match result {
                    Ok(message) => {
                        let msg_type = message
                            .get("type")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("");

                        match msg_type {
                            "response" => {
                                let request_seq = message
                                    .get("request_seq")
                                    .and_then(serde_json::Value::as_i64)
                                    .unwrap_or(-1);

                                let mut pending_guard = pending.write().await;
                                if let Some(tx) = pending_guard.remove(&request_seq) {
                                    if tx.send(message).is_err() {
                                        tracing::warn!(request_seq, "Response receiver dropped");
                                    }
                                } else {
                                    tracing::warn!(request_seq, "No pending request for response");
                                }
                            }
                            "event" => {
                                let event_name = message
                                    .get("event")
                                    .and_then(serde_json::Value::as_str)
                                    .unwrap_or("unknown");
                                tracing::debug!(event = event_name, "DAP event received");
                                event_handler(message);
                            }
                            other => {
                                tracing::debug!(msg_type = other, "Unknown DAP message type");
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "DAP transport read error");
                        break;
                    }
                }
            }
            tracing::debug!("DAP reader task exiting");
        })
    }
}
