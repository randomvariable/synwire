//! Observable runnable extension trait.

use crate::BoxStream;
use crate::error::SynwireError;
use crate::runnables::events::StreamEvent;

/// Extension trait for runnables that emit observable events.
///
/// This trait extends [`RunnableCore`](super::core::RunnableCore) with event
/// streaming capabilities. Full implementation will be available in the
/// streaming phase (US3).
pub trait ObservableRunnable: super::core::RunnableCore {
    /// Stream events produced during execution.
    ///
    /// Returns a boxed future that resolves to a stream of [`StreamEvent`]
    /// results. Each event describes a lifecycle step (start, chunk, end)
    /// or a custom user-dispatched event.
    fn stream_events(
        &self,
        input: serde_json::Value,
    ) -> crate::BoxFuture<'_, Result<BoxStream<'_, Result<StreamEvent, SynwireError>>, SynwireError>>;
}
