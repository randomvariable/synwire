//! Core trait for all runnable components.
//!
//! # Design Decision: `serde_json::Value` as Universal I/O
//!
//! `RunnableCore` uses `serde_json::Value` for input/output rather than generic
//! type parameters (`RunnableCore<I, O>`) because:
//!
//! - **Object safety**: traits with generic type parameters cannot be easily
//!   stored in `Vec<Box<dyn RunnableCore>>` for heterogeneous chains.
//! - **Composability**: any runnable can be chained with any other without
//!   explicit type conversion boilerplate.
//! - **Trade-off**: runtime type checking instead of compile-time, but this
//!   matches `LangChain` Python's dynamic typing model.
//! - **Alternative considered**: `Cow<'_, Value>` for zero-copy reads, but the
//!   ergonomic cost was deemed too high for the typical use case.

use crate::error::SynwireError;
use crate::runnables::config::RunnableConfig;
use crate::{BoxFuture, BoxStream};

/// Core trait for all runnable components in a chain.
///
/// Uses `serde_json::Value` as the universal input/output type
/// for composability between heterogeneous runnables.
///
/// # Default implementations
///
/// - [`batch`](RunnableCore::batch) invokes sequentially over each input.
/// - [`stream`](RunnableCore::stream) wraps [`invoke`](RunnableCore::invoke)
///   as a single-item stream.
///
/// # Lifetime parameter
///
/// All async methods share a single lifetime `'a` that ties together
/// `&'a self` and `Option<&'a RunnableConfig>`, ensuring the returned
/// future can borrow both.
///
/// # Cancel safety
///
/// [`invoke`](Self::invoke) and [`batch`](Self::batch) are **not
/// cancel-safe** in general -- the behaviour depends on the concrete
/// implementation. Dropping `batch` mid-execution may leave earlier
/// inputs processed but later inputs unprocessed. The [`BoxStream`]
/// returned by [`stream`](Self::stream) can be safely dropped at any
/// point.
pub trait RunnableCore: Send + Sync {
    /// Invoke the runnable with a single input.
    fn invoke<'a>(
        &'a self,
        input: serde_json::Value,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<serde_json::Value, SynwireError>>;

    /// Invoke on multiple inputs. Default implementation calls
    /// [`invoke`](RunnableCore::invoke) sequentially for each input.
    fn batch<'a>(
        &'a self,
        inputs: Vec<serde_json::Value>,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Vec<serde_json::Value>, SynwireError>> {
        Box::pin(async move {
            let mut results = Vec::with_capacity(inputs.len());
            for input in inputs {
                results.push(self.invoke(input, config).await?);
            }
            Ok(results)
        })
    }

    /// Stream results. Default implementation wraps [`invoke`](RunnableCore::invoke)
    /// as a single-item stream.
    fn stream<'a>(
        &'a self,
        input: serde_json::Value,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<BoxStream<'a, Result<serde_json::Value, SynwireError>>, SynwireError>>
    {
        Box::pin(async move {
            let result = self.invoke(input, config).await;
            let stream: BoxStream<'a, Result<serde_json::Value, SynwireError>> =
                Box::pin(futures_util::stream::iter(vec![result]));
            Ok(stream)
        })
    }

    /// Get the runnable's name for debugging.
    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "RunnableCore"
    }
}
