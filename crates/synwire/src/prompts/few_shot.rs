//! Few-shot prompt templates.

use std::collections::HashMap;
use std::sync::Arc;

use synwire_core::BoxFuture;
use synwire_core::error::SynwireError;
use synwire_core::messages::Message;
use synwire_core::prompts::PromptTemplate;

use super::example_selector::ExampleSelector;

/// A few-shot prompt template that formats examples using a string template.
///
/// Given examples (as `HashMap<String, String>`) and an example template,
/// produces a prompt string with all examples formatted and joined.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use synwire::prompts::FewShotPromptTemplate;
/// use synwire_core::prompts::PromptTemplate;
///
/// let example_template = PromptTemplate::new(
///     "Input: {input}\nOutput: {output}",
///     vec!["input".into(), "output".into()],
/// );
///
/// let examples = vec![{
///     let mut m = HashMap::new();
///     m.insert("input".into(), "hello".into());
///     m.insert("output".into(), "world".into());
///     m
/// }];
///
/// let tpl = FewShotPromptTemplate::new(
///     examples,
///     example_template,
///     "Answer the question.\n\n{examples}\n\nInput: {input}\nOutput:",
///     vec!["input".into()],
/// );
/// ```
pub struct FewShotPromptTemplate {
    examples: Vec<HashMap<String, String>>,
    example_template: PromptTemplate,
    prefix_suffix_template: String,
    input_variables: Vec<String>,
    example_separator: String,
    example_selector: Option<Arc<dyn ExampleSelector>>,
}

impl FewShotPromptTemplate {
    /// Creates a new few-shot prompt template with static examples.
    ///
    /// `template` should contain `{examples}` as a placeholder where
    /// formatted examples will be inserted.
    pub fn new(
        examples: Vec<HashMap<String, String>>,
        example_template: PromptTemplate,
        template: &str,
        input_variables: Vec<String>,
    ) -> Self {
        Self {
            examples,
            example_template,
            prefix_suffix_template: template.to_owned(),
            input_variables,
            example_separator: "\n\n".to_owned(),
            example_selector: None,
        }
    }

    /// Creates a new few-shot prompt template with a dynamic example selector.
    pub fn with_selector(
        selector: Arc<dyn ExampleSelector>,
        example_template: PromptTemplate,
        template: &str,
        input_variables: Vec<String>,
    ) -> Self {
        Self {
            examples: Vec::new(),
            example_template,
            prefix_suffix_template: template.to_owned(),
            input_variables,
            example_separator: "\n\n".to_owned(),
            example_selector: Some(selector),
        }
    }

    /// Sets the separator between formatted examples.
    #[must_use]
    pub fn with_separator(mut self, sep: impl Into<String>) -> Self {
        self.example_separator = sep.into();
        self
    }

    /// Formats the prompt with the given input variables.
    ///
    /// If an example selector is configured, it is used to choose examples
    /// dynamically. Otherwise, all static examples are used.
    pub fn format<'a>(
        &'a self,
        variables: &'a HashMap<String, String>,
    ) -> BoxFuture<'a, Result<String, SynwireError>> {
        Box::pin(async move {
            let examples = if let Some(ref selector) = self.example_selector {
                selector.select_examples(variables).await?
            } else {
                self.examples.clone()
            };

            let formatted_examples: Result<Vec<String>, SynwireError> = examples
                .iter()
                .map(|ex| self.example_template.format(ex))
                .collect();
            let examples_text = formatted_examples?.join(&self.example_separator);

            let mut all_vars = variables.clone();
            let _ = all_vars.insert("examples".into(), examples_text);

            let mut result = self.prefix_suffix_template.clone();
            for var in &self.input_variables {
                let value = all_vars.get(var).ok_or_else(|| SynwireError::Prompt {
                    message: format!("missing required variable '{var}'"),
                })?;
                result = result.replace(&format!("{{{var}}}"), value);
            }
            // Also substitute the examples placeholder
            #[allow(clippy::literal_string_with_formatting_args)]
            let examples_placeholder = "{examples}";
            if let Some(examples_val) = all_vars.get("examples") {
                result = result.replace(examples_placeholder, examples_val);
            }

            Ok(result)
        })
    }
}

/// A few-shot prompt template that produces chat messages.
///
/// Each example is formatted as a pair of human/AI messages.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use synwire::prompts::FewShotChatMessagePromptTemplate;
///
/// let examples = vec![{
///     let mut m = HashMap::new();
///     m.insert("input".into(), "hello".into());
///     m.insert("output".into(), "world".into());
///     m
/// }];
///
/// let tpl = FewShotChatMessagePromptTemplate::new(
///     examples,
///     "input",
///     "output",
/// );
/// ```
pub struct FewShotChatMessagePromptTemplate {
    examples: Vec<HashMap<String, String>>,
    input_key: String,
    output_key: String,
    example_selector: Option<Arc<dyn ExampleSelector>>,
}

impl FewShotChatMessagePromptTemplate {
    /// Creates a new few-shot chat message prompt template.
    pub fn new(examples: Vec<HashMap<String, String>>, input_key: &str, output_key: &str) -> Self {
        Self {
            examples,
            input_key: input_key.to_owned(),
            output_key: output_key.to_owned(),
            example_selector: None,
        }
    }

    /// Creates with a dynamic example selector.
    pub fn with_selector(
        selector: Arc<dyn ExampleSelector>,
        input_key: &str,
        output_key: &str,
    ) -> Self {
        Self {
            examples: Vec::new(),
            input_key: input_key.to_owned(),
            output_key: output_key.to_owned(),
            example_selector: Some(selector),
        }
    }

    /// Formats examples into a list of human/AI message pairs.
    pub fn format_messages<'a>(
        &'a self,
        variables: &'a HashMap<String, String>,
    ) -> BoxFuture<'a, Result<Vec<Message>, SynwireError>> {
        Box::pin(async move {
            let examples = if let Some(ref selector) = self.example_selector {
                selector.select_examples(variables).await?
            } else {
                self.examples.clone()
            };

            let mut messages = Vec::with_capacity(examples.len() * 2);
            for ex in &examples {
                let input = ex
                    .get(&self.input_key)
                    .ok_or_else(|| SynwireError::Prompt {
                        message: format!("example missing key '{}'", self.input_key),
                    })?;
                let output = ex
                    .get(&self.output_key)
                    .ok_or_else(|| SynwireError::Prompt {
                        message: format!("example missing key '{}'", self.output_key),
                    })?;
                messages.push(Message::human(input.clone()));
                messages.push(Message::ai(output.clone()));
            }

            Ok(messages)
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn make_examples() -> Vec<HashMap<String, String>> {
        let mut ex1 = HashMap::new();
        let _ = ex1.insert("input".into(), "2+2".into());
        let _ = ex1.insert("output".into(), "4".into());

        let mut ex2 = HashMap::new();
        let _ = ex2.insert("input".into(), "3+3".into());
        let _ = ex2.insert("output".into(), "6".into());

        vec![ex1, ex2]
    }

    #[tokio::test]
    async fn few_shot_prompt_template_formats_correctly() {
        let example_template = PromptTemplate::new(
            "Q: {input}\nA: {output}",
            vec!["input".into(), "output".into()],
        );
        let tpl = FewShotPromptTemplate::new(
            make_examples(),
            example_template,
            "Solve math problems.\n\n{examples}\n\nQ: {input}\nA:",
            vec!["input".into()],
        );

        let mut vars = HashMap::new();
        let _ = vars.insert("input".into(), "5+5".into());
        let result = tpl.format(&vars).await.unwrap();

        assert!(result.contains("Q: 2+2"));
        assert!(result.contains("A: 4"));
        assert!(result.contains("Q: 3+3"));
        assert!(result.contains("A: 6"));
        assert!(result.contains("Q: 5+5"));
    }

    #[tokio::test]
    async fn few_shot_chat_produces_message_pairs() {
        let tpl = FewShotChatMessagePromptTemplate::new(make_examples(), "input", "output");

        let vars = HashMap::new();
        let messages = tpl.format_messages(&vars).await.unwrap();

        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0].message_type(), "human");
        assert_eq!(messages[0].content().as_text(), "2+2");
        assert_eq!(messages[1].message_type(), "ai");
        assert_eq!(messages[1].content().as_text(), "4");
        assert_eq!(messages[2].message_type(), "human");
        assert_eq!(messages[2].content().as_text(), "3+3");
        assert_eq!(messages[3].message_type(), "ai");
        assert_eq!(messages[3].content().as_text(), "6");
    }

    #[tokio::test]
    async fn few_shot_with_selector() {
        let selector = Arc::new(crate::prompts::SemanticSimilarityExampleSelector::new());

        let mut ex = HashMap::new();
        let _ = ex.insert("input".into(), "hi".into());
        let _ = ex.insert("output".into(), "hello".into());
        selector.add_example(ex).await.unwrap();

        let tpl = FewShotChatMessagePromptTemplate::with_selector(selector, "input", "output");

        let vars = HashMap::new();
        let messages = tpl.format_messages(&vars).await.unwrap();
        assert_eq!(messages.len(), 2);
    }
}
