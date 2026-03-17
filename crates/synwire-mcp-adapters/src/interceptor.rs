//! Tool call interceptor chain (onion/middleware pattern).
//!
//! Interceptors wrap MCP tool call invocations in an onion-layered chain.
//! Each interceptor can inspect, modify, short-circuit, or observe requests
//! and responses. The ordering is `A → B → C → tool → C → B → A`.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::McpAdapterError;

// ---------------------------------------------------------------------------
// Request and result types
// ---------------------------------------------------------------------------

/// An MCP tool call request passed through the interceptor chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCallRequest {
    /// Name of the tool (exposed name, may include prefix).
    pub tool_name: String,
    /// Server name that will handle the call.
    pub server_name: String,
    /// JSON arguments for the tool.
    pub arguments: Value,
}

/// The result of an MCP tool call after passing through the interceptor chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCallResult {
    /// Raw result value from the server.
    pub value: Value,
    /// Whether the result represents an error.
    pub is_error: bool,
}

// ---------------------------------------------------------------------------
// ToolCallInterceptor trait
// ---------------------------------------------------------------------------

/// Type alias for the `next` continuation in the interceptor chain.
///
/// Calling `next` invokes the remaining interceptors and ultimately the tool.
pub type InterceptorNext<'a> = Box<
    dyn FnOnce(
            McpToolCallRequest,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<McpToolCallResult, McpAdapterError>>
                    + Send
                    + 'a,
            >,
        > + Send
        + 'a,
>;

/// An interceptor that wraps MCP tool calls.
///
/// Implement this trait to inspect, modify, or short-circuit tool call
/// requests and responses. Implementations must be `Send + Sync`.
///
/// # Ordering
///
/// Interceptors are executed in the order they are added to the chain.
/// If you add interceptors A, B, C, the call order is:
/// `A.intercept → B.intercept → C.intercept → tool → C returns → B returns → A returns`
pub trait ToolCallInterceptor: Send + Sync {
    /// Intercept a tool call.
    ///
    /// Call `next(request)` to continue the chain, or return a result
    /// directly to short-circuit.
    fn intercept<'a>(
        &'a self,
        request: McpToolCallRequest,
        next: InterceptorNext<'a>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<McpToolCallResult, McpAdapterError>>
                + Send
                + 'a,
        >,
    >;
}

// ---------------------------------------------------------------------------
// Chain executor
// ---------------------------------------------------------------------------

/// Executes a list of interceptors in onion order, with a final handler.
///
/// Interceptors wrap the `inner` future in order; panics within any
/// interceptor are caught and converted to [`McpAdapterError::InterceptorPanic`].
///
/// # Panic safety
///
/// Each interceptor is wrapped with `catch_unwind`. Panics in the `inner`
/// function itself are **not** caught.
pub async fn run_interceptor_chain<F>(
    interceptors: &[Arc<dyn ToolCallInterceptor>],
    request: McpToolCallRequest,
    inner: F,
) -> Result<McpToolCallResult, McpAdapterError>
where
    F: FnOnce(
            McpToolCallRequest,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<McpToolCallResult, McpAdapterError>> + Send,
            >,
        > + Send
        + 'static,
{
    // Build the chain from right to left (innermost first).
    // We represent each level as an Arc<dyn Fn(...)> to allow cloning.
    // For a small number of interceptors this is acceptable.

    if interceptors.is_empty() {
        return inner(request).await;
    }

    // Recursively build the chain using an index.
    run_chain_from(interceptors, 0, request, inner).await
}

/// Recursive helper that builds the interceptor chain.
fn run_chain_from<'a, F>(
    interceptors: &'a [Arc<dyn ToolCallInterceptor>],
    index: usize,
    request: McpToolCallRequest,
    inner: F,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<McpToolCallResult, McpAdapterError>> + Send + 'a>,
>
where
    F: FnOnce(
            McpToolCallRequest,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<McpToolCallResult, McpAdapterError>> + Send,
            >,
        > + Send
        + 'static,
{
    if index >= interceptors.len() {
        return Box::pin(async move { inner(request).await });
    }

    let interceptor = Arc::clone(&interceptors[index]);
    let remaining = interceptors;
    let next_index = index + 1;

    Box::pin(async move {
        // Catch panics from the interceptor itself
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            interceptor.intercept(
                request,
                Box::new(move |req| run_chain_from(remaining, next_index, req, inner)),
            )
        }));

        match result {
            Ok(fut) => fut.await,
            Err(payload) => {
                let msg = payload
                    .downcast_ref::<&str>()
                    .map(|s| (*s).to_owned())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown panic".to_owned());
                Err(McpAdapterError::InterceptorPanic { message: msg })
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Built-in interceptors
// ---------------------------------------------------------------------------

/// An interceptor that logs tool call requests and results via `tracing`.
#[derive(Debug, Default)]
pub struct LoggingInterceptor;

impl ToolCallInterceptor for LoggingInterceptor {
    fn intercept<'a>(
        &'a self,
        request: McpToolCallRequest,
        next: InterceptorNext<'a>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<McpToolCallResult, McpAdapterError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            tracing::debug!(
                tool = %request.tool_name,
                server = %request.server_name,
                "MCP tool call intercepted"
            );
            let result = next(request).await;
            match &result {
                Ok(r) => tracing::debug!(is_error = r.is_error, "MCP tool call completed"),
                Err(e) => tracing::warn!(error = %e, "MCP tool call failed in interceptor chain"),
            }
            result
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    struct RecordingInterceptor {
        id: char,
        order: Arc<tokio::sync::Mutex<Vec<char>>>,
    }

    impl ToolCallInterceptor for RecordingInterceptor {
        fn intercept<'a>(
            &'a self,
            request: McpToolCallRequest,
            next: InterceptorNext<'a>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<McpToolCallResult, McpAdapterError>>
                    + Send
                    + 'a,
            >,
        > {
            let id = self.id;
            let order = Arc::clone(&self.order);
            Box::pin(async move {
                order.lock().await.push(id);
                let result = next(request).await;
                order.lock().await.push(id);
                result
            })
        }
    }

    struct ShortCircuitInterceptor;
    impl ToolCallInterceptor for ShortCircuitInterceptor {
        fn intercept<'a>(
            &'a self,
            _request: McpToolCallRequest,
            _next: InterceptorNext<'a>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<McpToolCallResult, McpAdapterError>>
                    + Send
                    + 'a,
            >,
        > {
            Box::pin(async {
                Ok(McpToolCallResult {
                    value: serde_json::json!({"short": "circuit"}),
                    is_error: false,
                })
            })
        }
    }

    fn make_inner() -> impl FnOnce(
        McpToolCallRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<McpToolCallResult, McpAdapterError>> + Send>,
    > + Send
    + 'static {
        |_req| {
            Box::pin(async {
                Ok(McpToolCallResult {
                    value: serde_json::json!({"result": "ok"}),
                    is_error: false,
                })
            })
        }
    }

    fn make_request() -> McpToolCallRequest {
        McpToolCallRequest {
            tool_name: "search".into(),
            server_name: "s1".into(),
            arguments: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn onion_ordering_abc_to_cba() {
        let order: Arc<tokio::sync::Mutex<Vec<char>>> =
            Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let interceptors: Vec<Arc<dyn ToolCallInterceptor>> = vec![
            Arc::new(RecordingInterceptor {
                id: 'A',
                order: Arc::clone(&order),
            }),
            Arc::new(RecordingInterceptor {
                id: 'B',
                order: Arc::clone(&order),
            }),
            Arc::new(RecordingInterceptor {
                id: 'C',
                order: Arc::clone(&order),
            }),
        ];
        let result = run_interceptor_chain(&interceptors, make_request(), make_inner()).await;
        assert!(result.is_ok());
        let sequence = order.lock().await.clone();
        // Enter: A B C, Exit: C B A
        assert_eq!(sequence, vec!['A', 'B', 'C', 'C', 'B', 'A']);
    }

    #[tokio::test]
    async fn short_circuit_interceptor_stops_chain() {
        let order: Arc<tokio::sync::Mutex<Vec<char>>> =
            Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let interceptors: Vec<Arc<dyn ToolCallInterceptor>> = vec![
            Arc::new(RecordingInterceptor {
                id: 'A',
                order: Arc::clone(&order),
            }),
            Arc::new(ShortCircuitInterceptor),
            Arc::new(RecordingInterceptor {
                id: 'C',
                order: Arc::clone(&order),
            }),
        ];
        let result = run_interceptor_chain(&interceptors, make_request(), make_inner()).await;
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.value["short"], "circuit");
        // Only A entered (and then returned after short-circuit)
        let sequence = order.lock().await.clone();
        assert_eq!(sequence, vec!['A', 'A']);
    }

    #[tokio::test]
    async fn no_interceptors_calls_inner() {
        let result = run_interceptor_chain(&[], make_request(), make_inner()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value["result"], "ok");
    }
}
