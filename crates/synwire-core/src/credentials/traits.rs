//! Credential provider traits.

use crate::BoxFuture;
use crate::credentials::SecretValue;
use crate::error::SynwireError;

/// Provider of credentials (API keys, tokens, etc).
///
/// Implementors supply credentials to model providers and other components
/// that require authentication. The trait supports both static and
/// refreshable credential sources.
///
/// # Example
///
/// ```
/// use synwire_core::credentials::{CredentialProvider, SecretValue, StaticCredentialProvider};
///
/// let provider = StaticCredentialProvider::new(SecretValue::new("sk-test-key"));
/// ```
pub trait CredentialProvider: Send + Sync {
    /// Get the current credential value.
    fn get_credential(&self) -> BoxFuture<'_, Result<SecretValue, SynwireError>>;

    /// Refresh the credential (e.g., after a 401/403 response).
    ///
    /// Default implementation delegates to [`get_credential`](Self::get_credential).
    fn refresh_credential(&self) -> BoxFuture<'_, Result<SecretValue, SynwireError>> {
        self.get_credential()
    }
}
