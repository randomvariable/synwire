//! Passthrough runnable that forwards input unchanged.

use crate::BoxFuture;
use crate::error::SynwireError;
use crate::runnables::config::RunnableConfig;
use crate::runnables::core::RunnableCore;
use serde_json::Value;

/// A runnable that passes its input through unchanged.
///
/// Useful as an identity element in parallel compositions where one
/// branch should preserve the original input.
pub struct RunnablePassthrough;

impl RunnableCore for RunnablePassthrough {
    fn invoke<'a>(
        &'a self,
        input: Value,
        _config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Value, SynwireError>> {
        Box::pin(async move { Ok(input) })
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "RunnablePassthrough"
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_passthrough_forwards_input() {
        let passthrough = RunnablePassthrough;
        let input = serde_json::json!({"key": "value", "num": 42});
        let result = passthrough.invoke(input.clone(), None).await.unwrap();
        assert_eq!(result, input);
    }

    #[tokio::test]
    async fn test_passthrough_name() {
        assert_eq!(RunnablePassthrough.name(), "RunnablePassthrough");
    }
}
