//! Secret value type backed by the secrecy crate.

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A secret value with memory zeroisation on drop.
///
/// Debug and Display both render as `***`. Serialization produces `null`.
#[derive(Clone)]
pub struct SecretValue {
    inner: SecretString,
}

impl SecretValue {
    /// Creates a new secret value.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            inner: SecretString::from(value.into()),
        }
    }

    /// Exposes the secret value for explicit access.
    pub fn expose(&self) -> &str {
        self.inner.expose_secret()
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretValue(***)")
    }
}

impl fmt::Display for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "***")
    }
}

impl Serialize for SecretValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_none()
    }
}

impl<'de> Deserialize<'de> for SecretValue {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // SecretValue deserializes from null as an empty secret
        Ok(Self::new(""))
    }
}

impl PartialEq for SecretValue {
    fn eq(&self, other: &Self) -> bool {
        self.expose() == other.expose()
    }
}

impl Eq for SecretValue {}

impl std::hash::Hash for SecretValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.expose().hash(state);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_debug_redaction() {
        let secret = SecretValue::new("my-api-key");
        let debug = format!("{secret:?}");
        assert!(!debug.contains("my-api-key"));
        assert!(debug.contains("***"));
    }

    #[test]
    fn test_secret_display_redaction() {
        let secret = SecretValue::new("my-api-key");
        let display = format!("{secret}");
        assert_eq!(display, "***");
    }

    #[test]
    fn test_secret_expose() {
        let secret = SecretValue::new("my-api-key");
        assert_eq!(secret.expose(), "my-api-key");
    }

    #[test]
    fn test_secret_serialize_as_null() {
        let secret = SecretValue::new("my-api-key");
        let json = serde_json::to_string(&secret).unwrap();
        assert_eq!(json, "null");
    }

    #[test]
    fn test_secret_equality() {
        let a = SecretValue::new("key");
        let b = SecretValue::new("key");
        let c = SecretValue::new("other");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_secret_clone() {
        let secret = SecretValue::new("key");
        let cloned = secret.clone();
        assert_eq!(secret.expose(), cloned.expose());
    }
}
