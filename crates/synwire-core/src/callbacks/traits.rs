//! Callback handler trait for observability hooks.

use crate::BoxFuture;
use crate::messages::Message;
use serde_json::Value;

/// Trait for receiving callback events during execution.
///
/// All methods have default no-op implementations, so consumers only need
/// to override the events they care about. The `ignore_*` methods allow
/// filtering entire categories of events.
///
/// # Example
///
/// ```
/// use synwire_core::callbacks::CallbackHandler;
/// use synwire_core::BoxFuture;
///
/// struct LoggingCallback;
///
/// impl CallbackHandler for LoggingCallback {
///     fn on_tool_start<'a>(
///         &'a self,
///         _tool_name: &'a str,
///         _input: &'a serde_json::Value,
///     ) -> BoxFuture<'a, ()> {
///         Box::pin(async {
///             // log tool start
///         })
///     }
/// }
/// ```
pub trait CallbackHandler: Send + Sync {
    /// Called when an LLM invocation starts.
    fn on_llm_start<'a>(
        &'a self,
        _model_type: &'a str,
        _messages: &'a [Message],
    ) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    /// Called when an LLM invocation ends.
    fn on_llm_end<'a>(&'a self, _response: &'a Value) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    /// Called when a tool invocation starts.
    fn on_tool_start<'a>(&'a self, _tool_name: &'a str, _input: &'a Value) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    /// Called when a tool invocation ends.
    fn on_tool_end<'a>(&'a self, _tool_name: &'a str, _output: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    /// Called on tool error.
    fn on_tool_error<'a>(&'a self, _tool_name: &'a str, _error: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    /// Called on retry.
    fn on_retry<'a>(&'a self, _attempt: u32, _error: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    /// Called when a chain starts.
    fn on_chain_start<'a>(&'a self, _chain_name: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    /// Called when a chain ends.
    fn on_chain_end<'a>(&'a self, _output: &'a Value) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }

    /// Whether to ignore tool callbacks.
    fn ignore_tool(&self) -> bool {
        false
    }

    /// Whether to ignore LLM callbacks.
    fn ignore_llm(&self) -> bool {
        false
    }
}
