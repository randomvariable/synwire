//! Direct model sampling provider.
//!
//! Uses the configured chat model directly for standalone (non-MCP) mode.
//! When running the agent without an MCP host this provider is used in place
//! of `McpSampling` from the MCP server crate.

use std::sync::Arc;

use synwire_core::{BoxFuture, SamplingError, SamplingProvider, SamplingRequest, SamplingResponse};

/// Type alias for the callback used to invoke a language model.
///
/// The function receives a [`SamplingRequest`] and returns a boxed future that
/// resolves to a [`SamplingResponse`] or a [`SamplingError`].
pub type SamplingFn = Arc<
    dyn Fn(SamplingRequest) -> BoxFuture<'static, Result<SamplingResponse, SamplingError>>
        + Send
        + Sync,
>;

/// A sampling provider backed by direct model invocation.
///
/// Used when running the agent without an MCP host. Wraps a caller-supplied
/// callback that performs the actual model call. When no callback is provided
/// (via [`DirectModelSampling::unavailable`]), all sampling calls return
/// [`SamplingError::NotAvailable`] to trigger graceful degradation in callers.
pub struct DirectModelSampling {
    invoke: Option<SamplingFn>,
}

impl DirectModelSampling {
    /// Create a new provider with the given model invocation callback.
    ///
    /// The callback is wrapped in an `Arc` so the provider remains `Clone`-friendly
    /// and can be shared across tasks.
    #[must_use]
    pub fn new(
        invoke: impl Fn(SamplingRequest) -> BoxFuture<'static, Result<SamplingResponse, SamplingError>>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            invoke: Some(Arc::new(invoke)),
        }
    }

    /// Create a provider that always reports sampling as unavailable.
    ///
    /// [`SamplingProvider::is_available`] will return `false` and all calls to
    /// [`SamplingProvider::sample`] will return [`SamplingError::NotAvailable`].
    #[must_use]
    pub const fn unavailable() -> Self {
        Self { invoke: None }
    }
}

impl SamplingProvider for DirectModelSampling {
    fn is_available(&self) -> bool {
        self.invoke.is_some()
    }

    fn sample(
        &self,
        request: SamplingRequest,
    ) -> BoxFuture<'_, Result<SamplingResponse, SamplingError>> {
        match &self.invoke {
            Some(invoke) => invoke(request),
            None => Box::pin(async { Err(SamplingError::NotAvailable) }),
        }
    }
}
