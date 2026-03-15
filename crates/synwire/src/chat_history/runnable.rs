//! Runnable wrapper that injects chat message history.

use std::sync::Arc;

use synwire_core::BoxFuture;
use synwire_core::error::SynwireError;
use synwire_core::messages::Message;
use synwire_core::runnables::RunnableConfig;
use synwire_core::runnables::core::RunnableCore;

use super::traits::ChatMessageHistory;

/// A factory function that returns a [`ChatMessageHistory`] for a given session ID.
pub type HistoryFactory = Arc<dyn Fn(&str) -> Arc<dyn ChatMessageHistory> + Send + Sync>;

/// Wraps a [`RunnableCore`] with chat message history management.
///
/// Before invoking the inner runnable, this wrapper:
/// 1. Retrieves the session's history via the factory
/// 2. Prepends historical messages to the input
/// 3. After invocation, stores both the input and output messages
///
/// The input `serde_json::Value` must contain a `"session_id"` string field
/// and a `"messages"` array field.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use synwire::chat_history::{
///     InMemoryChatMessageHistory, RunnableWithMessageHistory,
/// };
///
/// // Use a simple factory that always returns the same history
/// let history = Arc::new(InMemoryChatMessageHistory::new());
/// let history_clone = history.clone();
/// let factory: synwire::chat_history::HistoryFactory =
///     Arc::new(move |_session_id| history_clone.clone());
/// ```
pub struct RunnableWithMessageHistory {
    inner: Arc<dyn RunnableCore>,
    history_factory: HistoryFactory,
}

impl RunnableWithMessageHistory {
    /// Creates a new `RunnableWithMessageHistory`.
    pub fn new(inner: Arc<dyn RunnableCore>, history_factory: HistoryFactory) -> Self {
        Self {
            inner,
            history_factory,
        }
    }
}

impl RunnableCore for RunnableWithMessageHistory {
    fn invoke<'a>(
        &'a self,
        input: serde_json::Value,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<serde_json::Value, SynwireError>> {
        Box::pin(async move {
            let obj = input.as_object().ok_or_else(|| SynwireError::Prompt {
                message: "RunnableWithMessageHistory input must be a JSON object".into(),
            })?;

            let session_id = obj
                .get("session_id")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| SynwireError::Prompt {
                    message: "input must contain a \"session_id\" string field".into(),
                })?;

            let history = (self.history_factory)(session_id);

            // Get existing history messages
            let history_msgs = history.get_messages().await?;

            // Parse input messages
            let input_messages: Vec<Message> = obj
                .get("messages")
                .map(|v| serde_json::from_value(v.clone()))
                .transpose()
                .map_err(|e| SynwireError::Prompt {
                    message: format!("failed to parse input messages: {e}"),
                })?
                .unwrap_or_default();

            // Combine history + new messages
            let mut all_messages = history_msgs;
            all_messages.extend(input_messages.clone());

            // Store new input messages in history
            for msg in &input_messages {
                history.add_message(msg.clone()).await?;
            }

            // Build the combined input for the inner runnable
            let combined_input = serde_json::json!({
                "messages": all_messages,
            });

            // Invoke the inner runnable
            let output = self.inner.invoke(combined_input, config).await?;

            // If the output contains an AI message, store it in history
            if let Some(output_messages) = output.get("messages") {
                let out_msgs: Vec<Message> =
                    serde_json::from_value(output_messages.clone()).unwrap_or_default();
                for msg in &out_msgs {
                    if msg.message_type() == "ai" {
                        history.add_message(msg.clone()).await?;
                    }
                }
            }

            Ok(output)
        })
    }

    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "RunnableWithMessageHistory"
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::chat_history::InMemoryChatMessageHistory;

    /// A simple echo runnable that returns the messages it receives.
    struct EchoRunnable;

    impl RunnableCore for EchoRunnable {
        fn invoke<'a>(
            &'a self,
            input: serde_json::Value,
            _config: Option<&'a RunnableConfig>,
        ) -> BoxFuture<'a, Result<serde_json::Value, SynwireError>> {
            Box::pin(async move { Ok(input) })
        }
    }

    #[tokio::test]
    async fn injects_history_into_input() {
        let history = Arc::new(InMemoryChatMessageHistory::new());
        history
            .add_message(Message::human("Previous"))
            .await
            .unwrap();

        let history_clone = history.clone();
        let factory: HistoryFactory = Arc::new(move |_| history_clone.clone());

        let runnable = RunnableWithMessageHistory::new(Arc::new(EchoRunnable), factory);

        let input = serde_json::json!({
            "session_id": "test",
            "messages": [Message::human("Current")],
        });

        let output = runnable.invoke(input, None).await.unwrap();
        let messages: Vec<Message> = serde_json::from_value(output["messages"].clone()).unwrap();

        // Should contain both previous + current
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content().as_text(), "Previous");
        assert_eq!(messages[1].content().as_text(), "Current");
    }

    #[tokio::test]
    async fn tracks_sessions_independently() {
        let sessions: Arc<Mutex<HashMap<String, Arc<InMemoryChatMessageHistory>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let sessions_clone = sessions.clone();
        let factory: HistoryFactory = Arc::new(move |session_id| {
            let mut map = sessions_clone.lock().expect("lock");
            map.entry(session_id.to_owned())
                .or_insert_with(|| Arc::new(InMemoryChatMessageHistory::new()))
                .clone()
        });

        let runnable = RunnableWithMessageHistory::new(Arc::new(EchoRunnable), factory);

        // Session A
        let input_a = serde_json::json!({
            "session_id": "A",
            "messages": [Message::human("Hello A")],
        });
        let _output = runnable.invoke(input_a, None).await.unwrap();

        // Session B
        let input_b = serde_json::json!({
            "session_id": "B",
            "messages": [Message::human("Hello B")],
        });
        let _output = runnable.invoke(input_b, None).await.unwrap();

        // Verify sessions are independent -- clone Arcs before dropping the lock
        let (history_a, history_b) = {
            let map = sessions.lock().expect("lock");
            (
                Arc::clone(map.get("A").unwrap()),
                Arc::clone(map.get("B").unwrap()),
            )
        };

        let a_msgs = history_a.get_messages().await.unwrap();
        let b_msgs = history_b.get_messages().await.unwrap();
        assert_eq!(a_msgs.len(), 1);
        assert_eq!(b_msgs.len(), 1);
        assert_eq!(a_msgs[0].content().as_text(), "Hello A");
        assert_eq!(b_msgs[0].content().as_text(), "Hello B");
    }

    use std::sync::Mutex;
}
