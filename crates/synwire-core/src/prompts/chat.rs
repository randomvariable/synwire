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

/// Expands a placeholder value into one or more messages.
///
/// If the value is a valid JSON array of objects with `role` and `content`
/// fields, each element is converted to the corresponding [`Message`] variant.
/// Recognised roles are `"system"`, `"human"` / `"user"`, and `"ai"` /
/// `"assistant"`; unrecognised roles are silently skipped.
///
/// If the value is not a valid JSON array, it is treated as a single human
/// message whose content is the raw string.
fn expand_placeholder(value: &str, out: &mut Vec<Message>) {
    if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(value) {
        for item in &arr {
            let Some(role) = item.get("role").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let Some(content) = item.get("content").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let msg = match role {
                "system" => Message::system(content),
                "human" | "user" => Message::human(content),
                "ai" | "assistant" => Message::ai(content),
                _ => continue,
            };
            out.push(msg);
        }
    } else {
        out.push(Message::human(value));
    }
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
    /// `Placeholder` templates are expanded by looking up their variable name
    /// in the provided map. If the value is a JSON array of `{role, content}`
    /// objects the corresponding messages are injected; otherwise the value is
    /// treated as a single human message. Missing placeholders are skipped.
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
                MessageTemplate::Placeholder(name) => {
                    if let Some(value) = variables.get(name.as_str()) {
                        expand_placeholder(value, &mut result);
                    }
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
    fn test_placeholder_missing_variable_skipped() {
        let tpl = ChatPromptTemplate::from_messages(vec![
            MessageTemplate::System("Hello".into()),
            MessageTemplate::Placeholder("history".into()),
            MessageTemplate::Human("{question}".into()),
        ]);
        let mut vars = HashMap::new();
        let _ = vars.insert("question".into(), "Hi".into());
        let messages = tpl.format_messages(&vars).unwrap();
        // Placeholder variable not provided, so only System + Human
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_placeholder_json_array_expansion() {
        let tpl = ChatPromptTemplate::from_messages(vec![
            MessageTemplate::System("You are helpful.".into()),
            MessageTemplate::Placeholder("history".into()),
            MessageTemplate::Human("{question}".into()),
        ]);
        let history = serde_json::json!([
            {"role": "human", "content": "What is 2+2?"},
            {"role": "ai", "content": "4"},
        ]);
        let mut vars = HashMap::new();
        let _ = vars.insert("history".into(), history.to_string());
        let _ = vars.insert("question".into(), "And 3+3?".into());
        let messages = tpl.format_messages(&vars).unwrap();
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0].message_type(), "system");
        assert_eq!(messages[1].message_type(), "human");
        assert_eq!(messages[1].content().as_text(), "What is 2+2?");
        assert_eq!(messages[2].message_type(), "ai");
        assert_eq!(messages[2].content().as_text(), "4");
        assert_eq!(messages[3].message_type(), "human");
        assert_eq!(messages[3].content().as_text(), "And 3+3?");
    }

    #[test]
    fn test_placeholder_plain_string_becomes_human_message() {
        let tpl =
            ChatPromptTemplate::from_messages(vec![MessageTemplate::Placeholder("input".into())]);
        let mut vars = HashMap::new();
        let _ = vars.insert("input".into(), "Tell me a joke".into());
        let messages = tpl.format_messages(&vars).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].message_type(), "human");
        assert_eq!(messages[0].content().as_text(), "Tell me a joke");
    }

    #[test]
    fn test_placeholder_recognises_user_and_assistant_roles() {
        let tpl =
            ChatPromptTemplate::from_messages(vec![MessageTemplate::Placeholder("history".into())]);
        let history = serde_json::json!([
            {"role": "user", "content": "Hello"},
            {"role": "assistant", "content": "Hi there"},
            {"role": "system", "content": "Be concise"},
        ]);
        let mut vars = HashMap::new();
        let _ = vars.insert("history".into(), history.to_string());
        let messages = tpl.format_messages(&vars).unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].message_type(), "human");
        assert_eq!(messages[1].message_type(), "ai");
        assert_eq!(messages[2].message_type(), "system");
    }

    #[test]
    fn test_placeholder_skips_items_with_unknown_role() {
        let tpl =
            ChatPromptTemplate::from_messages(vec![MessageTemplate::Placeholder("history".into())]);
        let history = serde_json::json!([
            {"role": "human", "content": "Hi"},
            {"role": "tool", "content": "result"},
            {"role": "ai", "content": "Done"},
        ]);
        let mut vars = HashMap::new();
        let _ = vars.insert("history".into(), history.to_string());
        let messages = tpl.format_messages(&vars).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].message_type(), "human");
        assert_eq!(messages[1].message_type(), "ai");
    }
}
