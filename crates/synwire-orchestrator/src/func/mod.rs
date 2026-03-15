//! Functional API utilities.
//!
//! Helpers for creating node functions from closures.

use crate::error::GraphError;
use crate::graph::state::{NodeFn, State};

/// Creates a [`NodeFn<S>`] from a synchronous closure.
///
/// The closure receives the current state and returns the updated state.
/// Useful for simple transformations that do not require async I/O.
pub fn sync_node<S, F>(f: F) -> NodeFn<S>
where
    S: State,
    F: Fn(S) -> Result<S, GraphError> + Send + Sync + 'static,
{
    Box::new(move |state| {
        let result = f(state);
        Box::pin(async move { result })
    })
}
