//! MCP elicitation callbacks.
//!
//! Elicitation allows an MCP server to request additional information from the
//! user mid-call (e.g. credentials, confirmations).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::BoxFuture;
use crate::agents::error::AgentError;

// ---------------------------------------------------------------------------
// Elicitation types
// ---------------------------------------------------------------------------

/// A request from an MCP server for additional user input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationRequest {
    /// Server-assigned request ID.
    pub request_id: String,
    /// Human-readable prompt shown to the user.
    pub message: String,
    /// JSON Schema describing the expected response shape.
    pub response_schema: Value,
    /// Whether the request is required (vs. optional / cancellable).
    pub required: bool,
}

/// The user's response to an elicitation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ElicitationResult {
    /// User provided a value matching the schema.
    Provided {
        /// Request ID echoed back.
        request_id: String,
        /// User-supplied value.
        value: Value,
    },
    /// User cancelled the elicitation.
    Cancelled {
        /// Request ID echoed back.
        request_id: String,
    },
}

// ---------------------------------------------------------------------------
// OnElicitation callback trait
// ---------------------------------------------------------------------------

/// Receives elicitation requests from MCP servers and returns user responses.
pub trait OnElicitation: Send + Sync {
    /// Handle an elicitation request.
    fn elicit(
        &self,
        request: ElicitationRequest,
    ) -> BoxFuture<'_, Result<ElicitationResult, AgentError>>;
}

/// Default elicitation handler that cancels all requests.
#[derive(Debug, Default)]
pub struct CancelAllElicitations;

impl OnElicitation for CancelAllElicitations {
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
