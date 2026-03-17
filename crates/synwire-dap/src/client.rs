//! High-level DAP client wrapping the low-level transport.
//!
//! Provides typed methods for the full DAP lifecycle: initialization,
//! breakpoints, stepping, evaluation, and teardown.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::config::DapAdapterConfig;
use crate::error::DapError;
use crate::transport::{DapTransport, EventHandler};

/// Current state of a debug session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum DapSessionState {
    /// No session has been started.
    #[default]
    NotStarted,
    /// The `initialize` handshake is in progress.
    Initializing,
    /// `configurationDone` has been sent.
    Configured,
    /// The debuggee is executing.
    Running,
    /// The debuggee has hit a breakpoint or been paused.
    Stopped,
    /// The debug session has terminated.
    Terminated,
}

impl std::fmt::Display for DapSessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotStarted => write!(f, "not_started"),
            Self::Initializing => write!(f, "initializing"),
            Self::Configured => write!(f, "configured"),
            Self::Running => write!(f, "running"),
            Self::Stopped => write!(f, "stopped"),
            Self::Terminated => write!(f, "terminated"),
        }
    }
}

/// High-level DAP client managing a single debug session.
///
/// Wraps [`DapTransport`] and tracks session state, capabilities,
/// and active breakpoints.
pub struct DapClient {
    transport: Arc<DapTransport>,
    capabilities: RwLock<Option<serde_json::Value>>,
    status: Arc<RwLock<DapSessionState>>,
    active_breakpoints: Arc<RwLock<HashMap<String, Vec<serde_json::Value>>>>,
}

impl DapClient {
    /// Spawn an adapter process and create a new client.
    ///
    /// This only starts the child process. Call [`initialize`](Self::initialize)
    /// afterwards to perform the DAP handshake.
    ///
    /// # Errors
    ///
    /// Returns [`DapError::BinaryNotFound`] if the command is not on `PATH`,
    /// or [`DapError::Io`] if the process fails to spawn.
    pub fn start(config: &DapAdapterConfig, event_handler: EventHandler) -> Result<Self, DapError> {
        // Verify the binary exists.
        drop(
            which::which(&config.command).map_err(|_| DapError::BinaryNotFound {
                binary: config.command.clone(),
            })?,
        );

        let transport =
            DapTransport::spawn(&config.command, &config.args, &config.env, event_handler)?;

        Ok(Self {
            transport: Arc::new(transport),
            capabilities: RwLock::new(None),
            status: Arc::new(RwLock::new(DapSessionState::NotStarted)),
            active_breakpoints: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Perform the DAP `initialize` handshake.
    ///
    /// Sends `initialize`, stores the adapter's capabilities, then sends
    /// `configurationDone`.
    ///
    /// # Errors
    ///
    /// Returns [`DapError::InitializationFailed`] if the handshake fails.
    pub async fn initialize(&self) -> Result<(), DapError> {
        self.set_status(DapSessionState::Initializing).await;

        let args = serde_json::json!({
            "clientID": "synwire",
            "clientName": "Synwire Agent",
            "adapterID": "synwire",
            "linesStartAt1": true,
            "columnsStartAt1": true,
            "pathFormat": "path",
            "supportsRunInTerminalRequest": false,
            "supportsVariableType": true,
            "supportsVariablePaging": false,
        });

        let response = self
            .transport
            .send_request("initialize", args)
            .await
            .map_err(|e| DapError::InitializationFailed(e.to_string()))?;

        // Store capabilities from the response body.
        if let Some(body) = response.get("body") {
            *self.capabilities.write().await = Some(body.clone());
        }

        tracing::debug!("DAP initialize handshake complete");

        Ok(())
    }

    /// Launch a program under the debugger.
    ///
    /// The adapter must have been initialized first. Sends `launch` followed
    /// by `configurationDone`.
    ///
    /// # Errors
    ///
    /// Returns [`DapError`] if the request fails.
    pub async fn launch(&self, config: serde_json::Value) -> Result<(), DapError> {
        self.ensure_initialized().await?;

        let _ = self.transport.send_request("launch", config).await?;
        let _ = self
            .transport
            .send_request("configurationDone", serde_json::json!({}))
            .await?;

        self.set_status(DapSessionState::Running).await;
        tracing::debug!("DAP launch complete");
        Ok(())
    }

    /// Attach to a running process.
    ///
    /// # Errors
    ///
    /// Returns [`DapError`] if the request fails.
    pub async fn attach(&self, config: serde_json::Value) -> Result<(), DapError> {
        self.ensure_initialized().await?;

        let _ = self.transport.send_request("attach", config).await?;
        let _ = self
            .transport
            .send_request("configurationDone", serde_json::json!({}))
            .await?;

        self.set_status(DapSessionState::Running).await;
        tracing::debug!("DAP attach complete");
        Ok(())
    }

    /// Disconnect from the debug session.
    ///
    /// # Errors
    ///
    /// Returns [`DapError`] if the request fails.
    pub async fn disconnect(&self) -> Result<(), DapError> {
        let _ = self
            .transport
            .send_request(
                "disconnect",
                serde_json::json!({
                    "restart": false,
                    "terminateDebuggee": true,
                }),
            )
            .await?;

        self.set_status(DapSessionState::Terminated).await;
        tracing::debug!("DAP disconnected");
        Ok(())
    }

    /// Terminate the debuggee.
    ///
    /// # Errors
    ///
    /// Returns [`DapError`] if the request fails.
    pub async fn terminate(&self) -> Result<(), DapError> {
        let _ = self
            .transport
            .send_request("terminate", serde_json::json!({ "restart": false }))
            .await?;

        self.set_status(DapSessionState::Terminated).await;
        tracing::debug!("DAP terminated");
        Ok(())
    }

    /// Set breakpoints for a source file.
    ///
    /// Returns the list of verified breakpoint objects from the adapter.
    ///
    /// # Errors
    ///
    /// Returns [`DapError`] if the request fails.
    pub async fn set_breakpoints(
        &self,
        source_path: &str,
        lines: &[i64],
    ) -> Result<Vec<serde_json::Value>, DapError> {
        let breakpoints: Vec<serde_json::Value> = lines
            .iter()
            .map(|&line| serde_json::json!({ "line": line }))
            .collect();

        let args = serde_json::json!({
            "source": { "path": source_path },
            "breakpoints": breakpoints,
        });

        let response = self.transport.send_request("setBreakpoints", args).await?;

        let result = response
            .get("body")
            .and_then(|b| b.get("breakpoints"))
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();

        // Track active breakpoints.
        let _ = self
            .active_breakpoints
            .write()
            .await
            .insert(source_path.to_string(), result.clone());

        Ok(result)
    }

    /// Set function breakpoints by name.
    ///
    /// # Errors
    ///
    /// Returns [`DapError`] if the request fails.
    pub async fn set_function_breakpoints(
        &self,
        names: &[String],
    ) -> Result<Vec<serde_json::Value>, DapError> {
        let breakpoints: Vec<serde_json::Value> = names
            .iter()
            .map(|name| serde_json::json!({ "name": name }))
            .collect();

        let args = serde_json::json!({ "breakpoints": breakpoints });

        let response = self
            .transport
            .send_request("setFunctionBreakpoints", args)
            .await?;

        let result = response
            .get("body")
            .and_then(|b| b.get("breakpoints"))
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();

        Ok(result)
    }

    /// Set exception breakpoints by filter ID.
    ///
    /// # Errors
    ///
    /// Returns [`DapError`] if the request fails.
    pub async fn set_exception_breakpoints(&self, filters: &[String]) -> Result<(), DapError> {
        let _ = self
            .transport
            .send_request(
                "setExceptionBreakpoints",
                serde_json::json!({ "filters": filters }),
            )
            .await?;
        Ok(())
    }

    /// Continue execution of the specified thread.
    ///
    /// # Errors
    ///
    /// Returns [`DapError`] if the request fails.
    pub async fn continue_execution(&self, thread_id: i64) -> Result<(), DapError> {
        let _ = self
            .transport
            .send_request("continue", serde_json::json!({ "threadId": thread_id }))
            .await?;
        self.set_status(DapSessionState::Running).await;
        Ok(())
    }

    /// Step over (next) for the specified thread.
    ///
    /// # Errors
    ///
    /// Returns [`DapError`] if the request fails.
    pub async fn next(&self, thread_id: i64) -> Result<(), DapError> {
        let _ = self
            .transport
            .send_request("next", serde_json::json!({ "threadId": thread_id }))
            .await?;
        self.set_status(DapSessionState::Running).await;
        Ok(())
    }

    /// Step into for the specified thread.
    ///
    /// # Errors
    ///
    /// Returns [`DapError`] if the request fails.
    pub async fn step_in(&self, thread_id: i64) -> Result<(), DapError> {
        let _ = self
            .transport
            .send_request("stepIn", serde_json::json!({ "threadId": thread_id }))
            .await?;
        self.set_status(DapSessionState::Running).await;
        Ok(())
    }

    /// Step out for the specified thread.
    ///
    /// # Errors
    ///
    /// Returns [`DapError`] if the request fails.
    pub async fn step_out(&self, thread_id: i64) -> Result<(), DapError> {
        let _ = self
            .transport
            .send_request("stepOut", serde_json::json!({ "threadId": thread_id }))
            .await?;
        self.set_status(DapSessionState::Running).await;
        Ok(())
    }

    /// Pause the specified thread.
    ///
    /// # Errors
    ///
    /// Returns [`DapError`] if the request fails.
    pub async fn pause(&self, thread_id: i64) -> Result<(), DapError> {
        let _ = self
            .transport
            .send_request("pause", serde_json::json!({ "threadId": thread_id }))
            .await?;
        self.set_status(DapSessionState::Stopped).await;
        Ok(())
    }

    /// List all threads in the debuggee.
    ///
    /// # Errors
    ///
    /// Returns [`DapError`] if the request fails.
    pub async fn threads(&self) -> Result<Vec<serde_json::Value>, DapError> {
        let response = self
            .transport
            .send_request("threads", serde_json::json!({}))
            .await?;
        Ok(extract_array(&response, "threads"))
    }

    /// Get the stack trace for a thread.
    ///
    /// # Errors
    ///
    /// Returns [`DapError`] if the request fails.
    pub async fn stack_trace(&self, thread_id: i64) -> Result<Vec<serde_json::Value>, DapError> {
        let response = self
            .transport
            .send_request("stackTrace", serde_json::json!({ "threadId": thread_id }))
            .await?;
        Ok(extract_array(&response, "stackFrames"))
    }

    /// Get scopes for a stack frame.
    ///
    /// # Errors
    ///
    /// Returns [`DapError`] if the request fails.
    pub async fn scopes(&self, frame_id: i64) -> Result<Vec<serde_json::Value>, DapError> {
        let response = self
            .transport
            .send_request("scopes", serde_json::json!({ "frameId": frame_id }))
            .await?;
        Ok(extract_array(&response, "scopes"))
    }

    /// Get variables for a variables reference (from a scope or structured variable).
    ///
    /// # Errors
    ///
    /// Returns [`DapError`] if the request fails.
    pub async fn variables(&self, variables_ref: i64) -> Result<Vec<serde_json::Value>, DapError> {
        let response = self
            .transport
            .send_request(
                "variables",
                serde_json::json!({ "variablesReference": variables_ref }),
            )
            .await?;
        Ok(extract_array(&response, "variables"))
    }

    /// Evaluate an expression in the debuggee context.
    ///
    /// # Errors
    ///
    /// Returns [`DapError`] if the request fails.
    pub async fn evaluate(
        &self,
        expression: &str,
        frame_id: Option<i64>,
    ) -> Result<serde_json::Value, DapError> {
        let mut args = serde_json::json!({
            "expression": expression,
            "context": "repl",
        });

        if let Some(fid) = frame_id
            && let Some(obj) = args.as_object_mut()
        {
            let _ = obj.insert("frameId".to_string(), serde_json::json!(fid));
        }

        let response = self.transport.send_request("evaluate", args).await?;

        Ok(response
            .get("body")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({})))
    }

    /// Return the current session state.
    pub async fn status(&self) -> DapSessionState {
        *self.status.read().await
    }

    /// Return the adapter capabilities received during initialization.
    pub async fn capabilities(&self) -> Option<serde_json::Value> {
        self.capabilities.read().await.clone()
    }

    /// Return a reference to the underlying transport.
    #[must_use]
    pub const fn transport(&self) -> &Arc<DapTransport> {
        &self.transport
    }

    /// Return active breakpoints keyed by source path.
    pub async fn active_breakpoints(&self) -> HashMap<String, Vec<serde_json::Value>> {
        self.active_breakpoints.read().await.clone()
    }

    /// Update session state.
    pub(crate) async fn set_status(&self, new_state: DapSessionState) {
        *self.status.write().await = new_state;
    }

    /// Verify the adapter has been initialized.
    async fn ensure_initialized(&self) -> Result<(), DapError> {
        let state = *self.status.read().await;
        if state == DapSessionState::NotStarted {
            return Err(DapError::NotReady {
                state: state.to_string(),
            });
        }
        Ok(())
    }
}

/// Extract an array field from a DAP response body.
fn extract_array(response: &serde_json::Value, field: &str) -> Vec<serde_json::Value> {
    response
        .get("body")
        .and_then(|b| b.get(field))
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
}
