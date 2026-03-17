//! MCP (Model Context Protocol) integration traits and types.

pub mod config;
pub mod elicitation;
pub mod traits;

pub use config::McpServerConfig;
pub use elicitation::{
    CancelAllElicitations, ElicitationRequest, ElicitationResult, OnElicitation,
};
pub use traits::{McpConnectionState, McpServerStatus, McpToolDescriptor, McpTransport};
