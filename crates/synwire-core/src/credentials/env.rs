//! Environment variable credential provider.

use crate::BoxFuture;
use crate::credentials::SecretValue;
use crate::credentials::traits::CredentialProvider;
use crate::error::SynwireError;

/// Reads a credential from an environment variable on each call.
///
/// # Example
///
/// ```
/// use synwire_core::credentials::EnvCredentialProvider;
///
/// let provider = EnvCredentialProvider::new("OPENAI_API_KEY");
/// ```
#[derive(Debug, Clone)]
pub struct EnvCredentialProvider {
    env_var: String,
}

impl EnvCredentialProvider {
    /// Creates a new provider that reads from the given environment variable.
    pub fn new(env_var: impl Into<String>) -> Self {
        Self {
            env_var: env_var.into(),
        }
    }

    /// Returns the environment variable name this provider reads from.
    pub fn env_var(&self) -> &str {
        &self.env_var
    }
}

impl CredentialProvider for EnvCredentialProvider {
    fn get_credential(&self) -> BoxFuture<'_, Result<SecretValue, SynwireError>> {
        Box::pin(async {
            let value = std::env::var(&self.env_var).map_err(|_| SynwireError::Credential {
                message: format!("environment variable {} not set", self.env_var),
            })?;
            Ok(SecretValue::new(value))
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn missing_env_var_returns_error() {
        // Use a variable name that will not exist in the environment.
        let provider = EnvCredentialProvider::new("SYNWIRE_NONEXISTENT_VAR_82a9f3c1d4e6b7");
        let result = provider.get_credential().await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("not set"));
    }

    #[tokio::test]
    async fn get_credential_from_existing_env() {
        // PATH is always set on Linux, so use it as a safe read-only test.
        let provider = EnvCredentialProvider::new("PATH");
        let result = provider.get_credential().await;
        assert!(result.is_ok());
        assert!(!result.unwrap().expose().is_empty());
    }

    #[test]
    fn env_var_accessor() {
        let provider = EnvCredentialProvider::new("MY_KEY");
        assert_eq!(provider.env_var(), "MY_KEY");
    }
}
