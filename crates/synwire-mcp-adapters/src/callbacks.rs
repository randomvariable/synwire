//! MCP callback slots for logging, progress, and elicitation.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use synwire_core::BoxFuture;
use synwire_core::agents::error::AgentError;
use synwire_core::mcp::elicitation::{ElicitationRequest, ElicitationResult, OnElicitation};

// ---------------------------------------------------------------------------
// Logging callback
// ---------------------------------------------------------------------------

/// Log level as reported by an MCP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum McpLogLevel {
    /// Debug-level message.
    Debug,
    /// Informational message.
    Info,
    /// Warning-level message.
    Warning,
    /// Error-level message.
    Error,
}

/// A logging message emitted by an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpLoggingMessage {
    /// Severity level of the message.
    pub level: McpLogLevel,
    /// The logger name or server component that produced the message.
    pub logger: Option<String>,
    /// Message data (may be a string or structured JSON).
    pub data: Value,
}

/// Callback invoked when an MCP server emits a logging message.
pub trait OnMcpLogging: Send + Sync {
    /// Handle a logging message from an MCP server.
    fn on_log(&self, server_name: &str, message: McpLoggingMessage);
}

// ---------------------------------------------------------------------------
// Progress callback
// ---------------------------------------------------------------------------

/// A progress notification from an MCP server during a long-running operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpProgressNotification {
    /// Opaque token identifying the operation.
    pub progress_token: String,
    /// Number of units completed so far.
    pub progress: u64,
    /// Total number of units (if known).
    pub total: Option<u64>,
    /// Human-readable description of current activity.
    pub message: Option<String>,
}

/// Callback invoked when an MCP server reports progress on an operation.
pub trait OnMcpProgress: Send + Sync {
    /// Handle a progress notification from an MCP server.
    fn on_progress(&self, server_name: &str, notification: McpProgressNotification);
}

// ---------------------------------------------------------------------------
// Default no-op implementations
// ---------------------------------------------------------------------------

/// A logging callback that discards all messages.
#[derive(Debug, Default, Clone)]
pub struct DiscardLogging;

impl OnMcpLogging for DiscardLogging {
    fn on_log(&self, _server_name: &str, _message: McpLoggingMessage) {}
}

/// A progress callback that discards all notifications.
#[derive(Debug, Default, Clone)]
pub struct DiscardProgress;

impl OnMcpProgress for DiscardProgress {
    fn on_progress(&self, _server_name: &str, _notification: McpProgressNotification) {}
}

/// A logging callback that forwards messages to the `tracing` framework.
#[derive(Debug, Default, Clone)]
pub struct TracingLogging;

impl OnMcpLogging for TracingLogging {
    fn on_log(&self, server_name: &str, message: McpLoggingMessage) {
        match message.level {
            McpLogLevel::Debug => {
                tracing::debug!(server = %server_name, logger = ?message.logger, data = ?message.data, "MCP log");
            }
            McpLogLevel::Info => {
                tracing::info!(server = %server_name, logger = ?message.logger, data = ?message.data, "MCP log");
            }
            McpLogLevel::Warning => {
                tracing::warn!(server = %server_name, logger = ?message.logger, data = ?message.data, "MCP log");
            }
            McpLogLevel::Error => {
                tracing::error!(server = %server_name, logger = ?message.logger, data = ?message.data, "MCP log");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// McpCallbacks bundle
// ---------------------------------------------------------------------------

/// Bundle of callback handlers for MCP server events.
///
/// All fields have default no-op implementations so only the handlers you
/// care about need to be provided.
pub struct McpCallbacks {
    /// Handler for log messages emitted by MCP servers.
    pub logging: Arc<dyn OnMcpLogging>,
    /// Handler for progress notifications from MCP servers.
    pub progress: Arc<dyn OnMcpProgress>,
    /// Handler for elicitation requests from MCP servers.
    pub elicitation: Arc<dyn OnElicitation>,
}

impl std::fmt::Debug for McpCallbacks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpCallbacks")
            .field("logging", &"<handler>")
            .field("progress", &"<handler>")
            .field("elicitation", &"<handler>")
            .finish()
    }
}

impl Default for McpCallbacks {
    fn default() -> Self {
        Self {
            logging: Arc::new(DiscardLogging),
            progress: Arc::new(DiscardProgress),
            elicitation: Arc::new(CancelAllElicitationsAdapter),
        }
    }
}

impl McpCallbacks {
    /// Creates a new `McpCallbacks` with all handlers set to the defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the logging callback.
    #[must_use]
    pub fn with_logging(mut self, logging: Arc<dyn OnMcpLogging>) -> Self {
        self.logging = logging;
        self
    }

    /// Sets the progress callback.
    #[must_use]
    pub fn with_progress(mut self, progress: Arc<dyn OnMcpProgress>) -> Self {
        self.progress = progress;
        self
    }

    /// Sets the elicitation callback.
    #[must_use]
    pub fn with_elicitation(mut self, elicitation: Arc<dyn OnElicitation>) -> Self {
        self.elicitation = elicitation;
        self
    }
}

// ---------------------------------------------------------------------------
// Adapter: synwire-core's CancelAllElicitations
// ---------------------------------------------------------------------------

/// Adapter that wraps the `CancelAllElicitations` default from synwire-core.
#[derive(Debug)]
struct CancelAllElicitationsAdapter;

impl OnElicitation for CancelAllElicitationsAdapter {
    fn elicit(
        &self,
        request: ElicitationRequest,
    ) -> BoxFuture<'_, Result<ElicitationResult, AgentError>> {
        Box::pin(async move {
            Ok(ElicitationResult::Cancelled {
                request_id: request.request_id,
            })
        })
    }
}
