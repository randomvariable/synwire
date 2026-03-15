//! Message types and utilities for conversation modelling.

mod filter;
mod traits;
mod types;
mod utils;

pub use filter::MessageFilter;
pub use traits::MessageLike;
pub use types::{
    ContentBlock, InputTokenDetails, InvalidToolCall, Message, MessageContent, OutputTokenDetails,
    ToolCall, ToolStatus, UsageMetadata,
};
pub use utils::{TrimStrategy, merge_message_runs, trim_messages};
