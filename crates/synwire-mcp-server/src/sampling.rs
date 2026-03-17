//! MCP `sampling/createMessage` delegation.
//!
//! When running as a client-connected MCP server, sampling requests are
//! forwarded to the host LLM via the MCP `sampling/createMessage` method.
//!
//! Full implementation requires the MCP host to expose
//! `sampling/createMessage`. Currently returns [`SamplingError::NotAvailable`]
//! to trigger graceful degradation. Full support requires bidirectional
//! transport (MCP 2024-11-05 sampling capability).

use synwire_core::{BoxFuture, SamplingError, SamplingProvider, SamplingRequest, SamplingResponse};

/// An MCP-based sampling provider that delegates to the connected host.
///
/// In the current v0.1 implementation sampling is always disabled because
/// full MCP client-to-host sampling requires bidirectional transport. All
/// calls return [`SamplingError::NotAvailable`] and callers should degrade
/// gracefully.
pub struct McpSampling {
    enabled: bool,
}

impl McpSampling {
    /// Create a new [`McpSampling`] provider.
    ///
    /// The provider is disabled by default; `enabled` will be set to `true`
    /// once bidirectional MCP transport is implemented.
    #[must_use]
    pub const fn new() -> Self {
        Self { enabled: false }
    }
}

impl Default for McpSampling {
    fn default() -> Self {
        Self::new()
    }
}

impl SamplingProvider for McpSampling {
    fn is_available(&self) -> bool {
        self.enabled
    }

    fn sample(
        &self,
        _request: SamplingRequest,
    ) -> BoxFuture<'_, Result<SamplingResponse, SamplingError>> {
        Box::pin(async { Err(SamplingError::NotAvailable) })
    }
}
