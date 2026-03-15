//! Chat prompt template with message-level variable substitution.

use std::collections::HashMap;

use crate::error::SynwireError;
use crate::messages::Message;
use crate::prompts::PromptValue;

/// A template for a single message in a chat prompt.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MessageTemplate {
    /// A system message template.
    System(String),
    /// A human message template.
    Human(String),
    /// An AI message template.
    AI(String),
    /// A placeholder for dynamic messages (e.g., chat history).
    Placeholder(String),
}

/// A template that formats messages with variable substitution.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use synwire_core::prompts::{ChatPromptTemplate, MessageTemplate};
///
/// let tpl = ChatPromptTemplate::from_messages(vec![
///     MessageTemplate::System("You are {role}".into()),
///     MessageTemplate::Human("{question}".into()),
/// ]);
/// let mut vars = HashMap::new();
/// vars.insert("role".into(), "a helpful assistant".into());
/// vars.insert("question".into(), "What is Rust?".into());
/// let messages = tpl.format_messages(&vars).unwrap();
/// assert_eq!(messages.len(), 2);
/// ```
#[derive(Debug, Clone)]
pub struct ChatPromptTemplate {
    messages: Vec<MessageTemplate>,
    input_variables: Vec<String>,
}

/// Extracts `{variable}` names from a template string.
fn extract_variables(template: &str) -> Vec<String> {
    let mut vars = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        rest = &rest[start + 1..];
        if let Some(end) = rest.find('}') {
            let var = &rest[..end];
            if !var.is_empty() {
                vars.push(var.to_owned());
            }
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }
    vars
}

/// Performs `{variable}` substitution on a template string.
fn substitute(template: &str, variables: &HashMap<String, String>) -> Result<String, SynwireError> {
    let mut result = template.to_owned();
    for var in &extract_variables(template) {
        let value = variables.get(var).ok_or_else(|| SynwireError::Prompt {
            message: format!("missing required variable '{var}'"),
        })?;
        result = result.replace(&format!("{{{var}}}"), value);
    }
    Ok(result)
}

impl ChatPromptTemplate {
    /// Creates a `ChatPromptTemplate` from a list of message templates.
    ///
    /// Input variables are automatically extracted from template strings.
    pub fn from_messages(messages: Vec<MessageTemplate>) -> Self {
        let mut seen = std::collections::HashSet::new();
        let mut input_variables = Vec::new();
        for msg in &messages {
            let tpl = match msg {
                MessageTemplate::System(t) | MessageTemplate::Human(t) | MessageTemplate::AI(t) => {
                    t.as_str()
                }
                MessageTemplate::Placeholder(_) => continue,
            };
            for var in extract_variables(tpl) {
                if seen.insert(var.clone()) {
                    input_variables.push(var);
                }
            }
        }
        Self {
            messages,
            input_variables,
        }
    }

    /// Returns the input variables required by this template.
    pub fn input_variables(&self) -> &[String] {
        &self.input_variables
    }

    /// Formats all message templates into concrete [`Message`] values.
    ///
    /// `Placeholder` templates are skipped in the current implementation.
    ///
    /// # Errors
    ///
    /// Returns [`SynwireError::Prompt`] if a required variable is missing.
    pub fn format_messages(
        &self,
        variables: &HashMap<String, String>,
    ) -> Result<Vec<Message>, SynwireError> {
        let mut result = Vec::with_capacity(self.messages.len());
        for msg in &self.messages {
            match msg {
                MessageTemplate::System(tpl) => {
                    let text = substitute(tpl, variables)?;
                    result.push(Message::system(text));
                }
                MessageTemplate::Human(tpl) => {
                    let text = substitute(tpl, variables)?;
                    result.push(Message::human(text));
                }
                MessageTemplate::AI(tpl) => {
                    let text = substitute(tpl, variables)?;
                    result.push(Message::ai(text));
                }
                MessageTemplate::Placeholder(_) => {
                    // Placeholder expansion is not yet implemented; skip.
                }
            }
        }
        Ok(result)
    }

    /// Formats messages and wraps them in a [`PromptValue::Messages`].
    ///
    /// # Errors
    ///
    /// Returns [`SynwireError::Prompt`] if a required variable is missing.
    pub fn to_prompt_value(
        &self,
        variables: &HashMap<String, String>,
    ) -> Result<PromptValue, SynwireError> {
        let messages = self.format_messages(variables)?;
        Ok(PromptValue::Messages(messages))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_prompt_template_format_messages() {
        let tpl = ChatPromptTemplate::from_messages(vec![
            MessageTemplate::System("You are {role}".into()),
            MessageTemplate::Human("{question}".into()),
        ]);
        let mut vars = HashMap::new();
        let _ = vars.insert("role".into(), "a helpful assistant".into());
        let _ = vars.insert("question".into(), "What is Rust?".into());

        let messages = tpl.format_messages(&vars).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].message_type(), "system");
        assert_eq!(
            messages[0].content().as_text(),
            "You are a helpful assistant"
        );
        assert_eq!(messages[1].message_type(), "human");
        assert_eq!(messages[1].content().as_text(), "What is Rust?");
    }

    #[test]
    fn test_chat_prompt_template_to_prompt_value() {
        let tpl =
            ChatPromptTemplate::from_messages(vec![MessageTemplate::Human("Hello {name}".into())]);
        let mut vars = HashMap::new();
        let _ = vars.insert("name".into(), "World".into());
        let pv = tpl.to_prompt_value(&vars).unwrap();
        let messages = pv.to_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content().as_text(), "Hello World");
    }

    #[test]
    fn test_chat_prompt_template_missing_variable() {
        let tpl =
            ChatPromptTemplate::from_messages(vec![MessageTemplate::Human("{question}".into())]);
        let vars = HashMap::new();
        let err = tpl.format_messages(&vars).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("question"),
            "error should mention the missing variable, got: {msg}"
        );
    }

    #[test]
    fn test_extract_variables() {
        let vars = extract_variables("Hello {name}, you are {age} years old");
        assert_eq!(vars, vec!["name", "age"]);
    }

    #[test]
    fn test_extract_variables_empty() {
        let vars = extract_variables("No variables here");
        assert!(vars.is_empty());
    }

    #[test]
    fn test_input_variables_auto_extracted() {
        let tpl = ChatPromptTemplate::from_messages(vec![
            MessageTemplate::System("You are {role}".into()),
            MessageTemplate::Human("{question} about {topic}".into()),
        ]);
        assert_eq!(tpl.input_variables(), &["role", "question", "topic"]);
    }

    #[test]
    fn test_placeholder_skipped() {
        let tpl = ChatPromptTemplate::from_messages(vec![
            MessageTemplate::System("Hello".into()),
            MessageTemplate::Placeholder("history".into()),
            MessageTemplate::Human("{question}".into()),
        ]);
        let mut vars = HashMap::new();
        let _ = vars.insert("question".into(), "Hi".into());
        let messages = tpl.format_messages(&vars).unwrap();
        // Placeholder is skipped, so only System + Human
        assert_eq!(messages.len(), 2);
    }
}
