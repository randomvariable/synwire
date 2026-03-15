//! Error types for the orchestrator crate.

use std::fmt;

/// Errors that can occur during graph construction, compilation, or execution.
#[derive(Debug)]
#[non_exhaustive]
pub enum GraphError {
    /// The execution exceeded the configured recursion limit.
    RecursionLimit {
        /// The limit that was exceeded.
        limit: usize,
    },
    /// An invalid state update was attempted.
    InvalidUpdate {
        /// Description of the invalid update.
        message: String,
    },
    /// The graph execution was interrupted.
    Interrupt {
        /// Description of the interrupt.
        message: String,
    },
    /// The input provided to the graph was empty.
    EmptyInput,
    /// A referenced task (node) was not found.
    TaskNotFound {
        /// Name of the missing task.
        name: String,
    },
    /// A channel had no value when one was required.
    EmptyChannel {
        /// Name of the empty channel.
        name: String,
    },
    /// An error occurred during graph compilation.
    CompileError {
        /// Description of the compilation error.
        message: String,
    },
    /// An error occurred during checkpointing.
    Checkpoint {
        /// Description of the checkpoint error.
        message: String,
    },
    /// An error occurred during store operations.
    Store {
        /// Description of the store error.
        message: String,
    },
    /// An error propagated from the core crate.
    Core(synwire_core::error::SynwireError),
    /// A duplicate node name was detected.
    DuplicateNode {
        /// Name of the duplicate node.
        name: String,
    },
    /// No entry point was set for the graph.
    NoEntryPoint,
    /// A channel received multiple values when only one was expected.
    MultipleValues {
        /// Name of the channel.
        channel: String,
    },
    /// A tool referenced in a tool call was not found.
    ToolNotFound {
        /// Name of the missing tool.
        name: String,
    },
    /// A tool invocation failed.
    ToolInvocation {
        /// Tool name.
        tool: String,
        /// Error message.
        message: String,
    },
    /// An HTTP request failed.
    HttpRequest {
        /// Error message.
        message: String,
    },
    /// A node exceeded its maximum iteration count.
    MaxIterations {
        /// The limit that was exceeded.
        limit: usize,
    },
    /// A validation check failed.
    Validation {
        /// Description of the validation failure.
        message: String,
    },
    /// A template rendering error.
    Template {
        /// Error message.
        message: String,
    },
    /// A channel value could not be deserialised into the expected field type.
    DeserializationError {
        /// Name of the field that failed to deserialise.
        field: String,
        /// Description of the deserialisation error.
        message: String,
    },
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecursionLimit { limit } => {
                write!(f, "recursion limit of {limit} reached")
            }
            Self::InvalidUpdate { message } => {
                write!(f, "invalid update: {message}")
            }
            Self::Interrupt { message } => {
                write!(f, "graph interrupted: {message}")
            }
            Self::EmptyInput => write!(f, "empty input"),
            Self::TaskNotFound { name } => {
                write!(f, "task not found: {name}")
            }
            Self::EmptyChannel { name } => {
                write!(f, "empty channel: {name}")
            }
            Self::CompileError { message } => {
                write!(f, "compile error: {message}")
            }
            Self::Checkpoint { message } => {
                write!(f, "checkpoint error: {message}")
            }
            Self::Store { message } => {
                write!(f, "store error: {message}")
            }
            Self::Core(err) => write!(f, "{err}"),
            Self::DuplicateNode { name } => {
                write!(f, "duplicate node: {name}")
            }
            Self::NoEntryPoint => write!(f, "no entry point set"),
            Self::MultipleValues { channel } => {
                write!(f, "multiple values for channel: {channel}")
            }
            Self::ToolNotFound { name } => {
                write!(f, "tool not found: {name}")
            }
            Self::ToolInvocation { tool, message } => {
                write!(f, "tool '{tool}' invocation failed: {message}")
            }
            Self::HttpRequest { message } => {
                write!(f, "HTTP request failed: {message}")
            }
            Self::MaxIterations { limit } => {
                write!(f, "maximum iterations ({limit}) exceeded")
            }
            Self::Validation { message } => {
                write!(f, "validation failed: {message}")
            }
            Self::Template { message } => {
                write!(f, "template error: {message}")
            }
            Self::DeserializationError { field, message } => {
                write!(f, "failed to deserialise field '{field}': {message}")
            }
        }
    }
}

impl std::error::Error for GraphError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Core(err) => Some(err),
            _ => None,
        }
    }
}

impl From<synwire_core::error::SynwireError> for GraphError {
    fn from(err: synwire_core::error::SynwireError) -> Self {
        Self::Core(err)
    }
}
