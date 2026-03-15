//! Static credential provider.

use crate::BoxFuture;
use crate::credentials::SecretValue;
use crate::credentials::traits::CredentialProvider;
use crate::error::SynwireError;

/// A credential provider that returns a fixed secret value.
///
/// Useful for testing or when the API key is known at construction time.
///
/// # Example
///
/// ```
/// use synwire_core::credentials::{SecretValue, StaticCredentialProvider};
///
/// let provider = StaticCredentialProvider::new(SecretValue::new("sk-test"));
/// ```
#[derive(Debug, Clone)]
pub struct StaticCredentialProvider {
    secret: SecretValue,
}

impl StaticCredentialProvider {
    /// Creates a new provider with the given fixed secret.
    pub const fn new(secret: SecretValue) -> Self {
        Self { secret }
    }
}

impl CredentialProvider for StaticCredentialProvider {
    fn get_credential(&self) -> BoxFuture<'_, Result<SecretValue, SynwireError>> {
        let secret = self.secret.clone();
        Box::pin(async move { Ok(secret) })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_fixed_value() {
        let provider = StaticCredentialProvider::new(SecretValue::new("sk-test-key"));
        let cred = provider.get_credential().await.unwrap();
        assert_eq!(cred.expose(), "sk-test-key");
    }

    #[tokio::test]
    async fn refresh_returns_same_value() {
        let provider = StaticCredentialProvider::new(SecretValue::new("sk-test-key"));
        let cred = provider.refresh_credential().await.unwrap();
        assert_eq!(cred.expose(), "sk-test-key");
    }
}
