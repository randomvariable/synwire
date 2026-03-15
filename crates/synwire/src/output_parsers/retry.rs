//! Retry output parser that uses an LLM to fix parsing failures.

use std::sync::Arc;

use synwire_core::error::SynwireError;
use synwire_core::language_models::BaseChatModel;
use synwire_core::messages::Message;
use synwire_core::output_parsers::OutputParser;

/// Wraps an [`OutputParser`] with LLM-based retry on failure.
///
/// When the inner parser fails, this parser constructs a prompt including
/// the original text, the error message, and format instructions, then
/// asks the LLM to produce a corrected output.
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use synwire::output_parsers::RetryOutputParser;
/// use synwire_core::output_parsers::StrOutputParser;
/// use synwire_core::language_models::FakeChatModel;
///
/// let parser = Arc::new(StrOutputParser);
/// let model = Arc::new(FakeChatModel::new(vec!["corrected".into()]));
/// let retry_parser = RetryOutputParser::new(parser, model, 3);
/// ```
pub struct RetryOutputParser<P: OutputParser> {
    inner: Arc<P>,
    model: Arc<dyn BaseChatModel>,
    max_retries: usize,
}

impl<P: OutputParser> RetryOutputParser<P> {
    /// Creates a new retry output parser.
    pub fn new(inner: Arc<P>, model: Arc<dyn BaseChatModel>, max_retries: usize) -> Self {
        Self {
            inner,
            model,
            max_retries,
        }
    }

    /// Attempts to parse the text, retrying with the LLM on failure.
    ///
    /// # Errors
    ///
    /// Returns the last parsing error if all retries are exhausted.
    pub async fn parse_with_retry(&self, text: &str) -> Result<P::Output, SynwireError> {
        // First attempt
        let first_err = match self.inner.parse(text) {
            Ok(v) => return Ok(v),
            Err(e) => e,
        };

        let mut last_err = first_err;
        let mut current_text = text.to_owned();

        for _ in 0..self.max_retries {
            let format_instructions = self.inner.get_format_instructions();
            let retry_prompt = format!(
                "The following output failed to parse:\n\n{current_text}\n\n\
                 Error: {last_err}\n\n\
                 Please produce corrected output.\n{format_instructions}"
            );

            let messages = vec![Message::human(retry_prompt)];
            let result = self.model.invoke(&messages, None).await?;
            current_text = result.message.content().as_text();

            match self.inner.parse(&current_text) {
                Ok(v) => return Ok(v),
                Err(e) => last_err = e,
            }
        }

        Err(last_err)
    }
}

/// [`OutputParser`] implementation delegates to the inner parser directly
/// (without retry). Use [`parse_with_retry`](RetryOutputParser::parse_with_retry)
/// for retry behavior.
impl<P: OutputParser> OutputParser for RetryOutputParser<P> {
    type Output = P::Output;

    fn parse(&self, text: &str) -> Result<P::Output, SynwireError> {
        self.inner.parse(text)
    }

    fn get_format_instructions(&self) -> String {
        self.inner.get_format_instructions()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use synwire_core::language_models::FakeChatModel;
    use synwire_core::output_parsers::JsonOutputParser;

    #[tokio::test]
    async fn retry_succeeds_on_corrected_output() {
        // Model returns valid JSON on retry
        let model = Arc::new(FakeChatModel::new(vec![r#"{"name": "Alice"}"#.into()]));
        let parser = Arc::new(JsonOutputParser);
        let retry = RetryOutputParser::new(parser, model, 3);

        let result = retry.parse_with_retry("not json").await.unwrap();
        assert_eq!(result["name"], "Alice");
    }

    #[tokio::test]
    async fn retry_exhausted_returns_error() {
        // Model always returns invalid JSON
        let model = Arc::new(FakeChatModel::new(vec!["still not json".into()]));
        let parser = Arc::new(JsonOutputParser);
        let retry = RetryOutputParser::new(parser, model, 2);

        let result = retry.parse_with_retry("bad input").await;
        assert!(result.is_err());
    }

    #[test]
    fn direct_parse_delegates_to_inner() {
        let model = Arc::new(FakeChatModel::new(vec![]));
        let parser = Arc::new(JsonOutputParser);
        let retry = RetryOutputParser::new(parser, model, 1);

        let result = retry.parse(r#"{"key": "value"}"#).unwrap();
        assert_eq!(result["key"], "value");
    }

    #[tokio::test]
    async fn no_retry_needed_on_success() {
        let model = Arc::new(FakeChatModel::new(vec![]));
        let parser = Arc::new(JsonOutputParser);
        let retry = RetryOutputParser::new(parser, model, 3);

        let result = retry.parse_with_retry(r#"{"ok": true}"#).await.unwrap();
        assert_eq!(result["ok"], true);
    }
}
