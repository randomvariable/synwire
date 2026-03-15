//! Errors specific to output parsing.

/// Errors specific to output parsing.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// Failed to parse output.
    #[error("parse failed: {message}")]
    ParseFailed {
        /// Error message.
        message: String,
    },
    /// Output did not match expected format.
    #[error("format mismatch: {message}")]
    FormatMismatch {
        /// Error message.
        message: String,
    },
    /// Other parse error.
    #[error("parse error: {message}")]
    Other {
        /// Error message.
        message: String,
    },
}
