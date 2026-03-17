//! Sampling provider trait for tool-internal LLM access.
//!
//! Allows tools and middleware to request LLM completions without taking
//! a hard dependency on a specific model or MCP transport. Zero LLM calls
//! happen during indexing — sampling is only invoked when explicitly needed
//! (e.g. community summary generation, hierarchical narrowing ranking).

use crate::BoxFuture;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// A request to the LLM for a text completion.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SamplingRequest {
    /// Optional system message.
    pub system: Option<String>,
    /// User message content.
    pub prompt: String,
    /// Maximum tokens to generate.
    pub max_tokens: Option<u32>,
    /// Temperature (0.0–1.0).
    pub temperature: Option<f32>,
}

impl SamplingRequest {
    /// Create a simple prompt-only request.
    ///
    /// # Examples
    ///
    /// ```
    /// use synwire_core::agents::sampling::SamplingRequest;
    ///
    /// let req = SamplingRequest::new("Summarise this code.");
    /// assert_eq!(req.prompt, "Summarise this code.");
    /// assert!(req.system.is_none());
    /// ```
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            system: None,
            prompt: prompt.into(),
            max_tokens: None,
            temperature: None,
        }
    }

    /// Set the system message.
    ///
    /// # Examples
    ///
    /// ```
    /// use synwire_core::agents::sampling::SamplingRequest;
    ///
    /// let req = SamplingRequest::new("Hello").with_system("You are a helpful assistant.");
    /// assert!(req.system.is_some());
    /// ```
    #[must_use]
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Set the maximum number of tokens to generate.
    #[must_use]
    pub const fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the sampling temperature.
    #[must_use]
    pub const fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }
}

/// A response from the LLM.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SamplingResponse {
    /// Generated text.
    pub text: String,
    /// Stop reason (`"end_turn"`, `"max_tokens"`, etc.)
    pub stop_reason: String,
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Error from a sampling call.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SamplingError {
    /// Provider not configured.
    #[error("no sampling provider configured")]
    NotAvailable,
    /// The model refused the request.
    #[error("model refused: {0}")]
    Refused(String),
    /// The sampling call timed out.
    #[error("sampling timed out")]
    Timeout,
    /// Any other error.
    #[error("sampling error: {0}")]
    Other(String),
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Provides LLM sampling for tool-internal use.
///
/// Implementations include MCP `sampling/createMessage` delegation and
/// direct model invocation.
///
/// The `sample` method returns a [`BoxFuture`] so that the trait remains
/// object-safe and can be used as `dyn SamplingProvider`.
pub trait SamplingProvider: Send + Sync {
    /// Returns `true` if sampling is available (provider configured).
    fn is_available(&self) -> bool;

    /// Request a completion from the LLM.
    ///
    /// Returns a boxed future to preserve object safety.
    fn sample(
        &self,
        request: SamplingRequest,
    ) -> BoxFuture<'_, Result<SamplingResponse, SamplingError>>;
}

// ---------------------------------------------------------------------------
// No-op implementation
// ---------------------------------------------------------------------------

/// A sampling provider that is always unavailable.
///
/// Used as a default when no provider is configured. All calls return
/// [`SamplingError::NotAvailable`], enabling graceful degradation in callers.
///
/// # Examples
///
/// ```
/// use synwire_core::agents::sampling::{NoopSamplingProvider, SamplingProvider, SamplingRequest};
///
/// let p = NoopSamplingProvider;
/// assert!(!p.is_available());
/// ```
pub struct NoopSamplingProvider;

impl SamplingProvider for NoopSamplingProvider {
    fn is_available(&self) -> bool {
        false
    }

    fn sample(
        &self,
        _request: SamplingRequest,
    ) -> BoxFuture<'_, Result<SamplingResponse, SamplingError>> {
        Box::pin(async { Err(SamplingError::NotAvailable) })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn noop_provider_returns_not_available() {
        let p = NoopSamplingProvider;
        assert!(!p.is_available());
        let result = p.sample(SamplingRequest::new("test")).await;
        assert!(matches!(result, Err(SamplingError::NotAvailable)));
    }

    #[test]
    fn sampling_request_builder() {
        let req = SamplingRequest::new("hello")
            .with_system("sys")
            .with_max_tokens(100)
            .with_temperature(0.7);
        assert_eq!(req.prompt, "hello");
        assert_eq!(req.system.as_deref(), Some("sys"));
        assert_eq!(req.max_tokens, Some(100));
        assert!((req.temperature.unwrap_or(0.0) - 0.7).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn noop_provider_is_object_safe() {
        let p: &dyn SamplingProvider = &NoopSamplingProvider;
        assert!(!p.is_available());
        let result = p.sample(SamplingRequest::new("test")).await;
        assert!(matches!(result, Err(SamplingError::NotAvailable)));
    }
}
