//! `OpenAI`-specific error types.

use synwire_core::error::ModelError;

/// Errors specific to the `OpenAI` provider.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OpenAIError {
    /// HTTP request failed.
    #[error("HTTP error: {message}")]
    Http {
        /// HTTP status code.
        status: Option<u16>,
        /// Error message.
        message: String,
    },
    /// Response parsing failed.
    #[error("response parse error: {0}")]
    ResponseParse(String),
    /// Configuration error.
    #[error("configuration error: {0}")]
    Configuration(String),
}

impl From<OpenAIError> for ModelError {
    fn from(err: OpenAIError) -> Self {
        match err {
            OpenAIError::Http {
                status: Some(429), ..
            } => Self::RateLimit { retry_after: None },
            OpenAIError::Http {
                status: Some(401 | 403),
                message,
            } => Self::AuthenticationFailed { message },
            OpenAIError::Http {
                status: Some(400),
                message,
            } => Self::InvalidRequest { message },
            OpenAIError::Http {
                status: Some(408), ..
            } => Self::Timeout,
            OpenAIError::Http { message, .. } => Self::Other { message },
            OpenAIError::ResponseParse(msg) => Self::Other { message: msg },
            OpenAIError::Configuration(msg) => Self::InvalidRequest { message: msg },
        }
    }
}

impl From<OpenAIError> for synwire_core::error::SynwireError {
    fn from(err: OpenAIError) -> Self {
        Self::Model(ModelError::from(err))
    }
}
