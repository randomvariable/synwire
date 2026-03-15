//! Callback handler traits for observability hooks.

mod traits;

pub use traits::CallbackHandler;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::significant_drop_tightening)]
mod tests;
