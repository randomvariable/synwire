//! Ollama-specific error types.

use synwire_core::error::ModelError;

/// Errors specific to the Ollama provider.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OllamaError {
    /// HTTP request failed.
    #[error("HTTP error (status: {status:?}): {message}")]
    Http {
        /// HTTP status code, if available.
        status: Option<u16>,
        /// Error message.
        message: String,
    },
    /// Response parsing failed.
    #[error("response parse error: {0}")]
    ResponseParse(String),
    /// The requested model was not found on the Ollama server.
    #[error("model not found: {0}")]
    ModelNotFound(String),
    /// Configuration error.
    #[error("configuration error: {0}")]
    Configuration(String),
}

impl From<OllamaError> for ModelError {
    fn from(err: OllamaError) -> Self {
        match &err {
            OllamaError::Http {
                status: Some(404), ..
            } => Self::InvalidRequest {
                message: err.to_string(),
            },
            OllamaError::Http {
                status: Some(401 | 403),
                ..
            } => Self::AuthenticationFailed {
                message: err.to_string(),
            },
            OllamaError::Http {
                status: Some(408), ..
            } => Self::Timeout,
            OllamaError::ModelNotFound(name) => Self::InvalidRequest {
                message: format!("model not found: {name}"),
            },
            OllamaError::Configuration(msg) => Self::InvalidRequest {
                message: msg.clone(),
            },
            _ => Self::Other {
                message: err.to_string(),
            },
        }
    }
}

impl From<OllamaError> for synwire_core::error::SynwireError {
    fn from(err: OllamaError) -> Self {
        Self::Model(ModelError::from(err))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn http_404_maps_to_invalid_request() {
        let err = OllamaError::Http {
            status: Some(404),
            message: "not found".into(),
        };
        let model_err: ModelError = err.into();
        assert!(matches!(model_err, ModelError::InvalidRequest { .. }));
    }

    #[test]
    fn http_401_maps_to_auth_failed() {
        let err = OllamaError::Http {
            status: Some(401),
            message: "unauthorized".into(),
        };
        let model_err: ModelError = err.into();
        assert!(matches!(model_err, ModelError::AuthenticationFailed { .. }));
    }

    #[test]
    fn model_not_found_maps_to_invalid_request() {
        let err = OllamaError::ModelNotFound("llama99".into());
        let model_err: ModelError = err.into();
        assert!(matches!(model_err, ModelError::InvalidRequest { .. }));
    }

    #[test]
    fn generic_http_maps_to_other() {
        let err = OllamaError::Http {
            status: Some(500),
            message: "internal error".into(),
        };
        let model_err: ModelError = err.into();
        assert!(matches!(model_err, ModelError::Other { .. }));
    }

    #[test]
    fn response_parse_maps_to_other() {
        let err = OllamaError::ResponseParse("bad json".into());
        let model_err: ModelError = err.into();
        assert!(matches!(model_err, ModelError::Other { .. }));
    }

    #[test]
    fn ollama_error_converts_to_synwire_error() {
        let err = OllamaError::Http {
            status: Some(500),
            message: "fail".into(),
        };
        let synwire_err: synwire_core::error::SynwireError = err.into();
        assert!(matches!(
            synwire_err,
            synwire_core::error::SynwireError::Model(_)
        ));
    }
}
