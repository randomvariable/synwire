//! Error types for the agent skills crate.

/// Errors that can occur when working with agent skills.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SkillError {
    /// The skill manifest is invalid.
    #[error("Invalid skill manifest: {0}")]
    InvalidManifest(String),

    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A YAML parse error occurred while reading the frontmatter.
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// The requested runtime has not been implemented yet.
    #[error("Runtime not implemented: {0}")]
    RuntimeNotImplemented(String),

    /// A runtime execution error.
    #[error("Runtime error ({runtime}): {message}")]
    Runtime {
        /// The runtime that produced the error.
        runtime: String,
        /// A human-readable description of the error.
        message: String,
    },
}
