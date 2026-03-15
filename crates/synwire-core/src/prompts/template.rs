//! String prompt template with variable substitution.

use std::collections::HashMap;

use crate::error::SynwireError;
use crate::prompts::PromptValue;

/// Template format variants.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub enum TemplateFormat {
    /// Simple `{variable}` substitution (default).
    #[default]
    FString,
}

/// A template that formats a string with variable substitution.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use synwire_core::prompts::PromptTemplate;
///
/// let tpl = PromptTemplate::new("Hello {name}!", vec!["name".into()]);
/// let mut vars = HashMap::new();
/// vars.insert("name".into(), "World".into());
/// assert_eq!(tpl.format(&vars).unwrap(), "Hello World!");
/// ```
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    template: String,
    input_variables: Vec<String>,
    _template_format: TemplateFormat,
}

impl PromptTemplate {
    /// Creates a new `PromptTemplate` with the `FString` format.
    pub fn new(template: impl Into<String>, input_variables: Vec<String>) -> Self {
        Self {
            template: template.into(),
            input_variables,
            _template_format: TemplateFormat::default(),
        }
    }

    /// Returns the input variables required by this template.
    pub fn input_variables(&self) -> &[String] {
        &self.input_variables
    }

    /// Substitutes `{var}` placeholders with values from the provided map.
    ///
    /// # Errors
    ///
    /// Returns [`SynwireError::Prompt`] if a required variable is missing.
    pub fn format(&self, variables: &HashMap<String, String>) -> Result<String, SynwireError> {
        let mut result = self.template.clone();
        for var in &self.input_variables {
            let value = variables.get(var).ok_or_else(|| SynwireError::Prompt {
                message: format!("missing required variable '{var}'"),
            })?;
            result = result.replace(&format!("{{{var}}}"), value);
        }
        Ok(result)
    }

    /// Formats the template and wraps the result in a [`PromptValue::String`].
    ///
    /// # Errors
    ///
    /// Returns [`SynwireError::Prompt`] if a required variable is missing.
    pub fn to_prompt_value(
        &self,
        variables: &HashMap<String, String>,
    ) -> Result<PromptValue, SynwireError> {
        let text = self.format(variables)?;
        Ok(PromptValue::String(text))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_template_format() {
        let tpl = PromptTemplate::new("Hello {name}", vec!["name".into()]);
        let mut vars = HashMap::new();
        let _ = vars.insert("name".into(), "World".into());
        assert_eq!(tpl.format(&vars).unwrap(), "Hello World");
    }

    #[test]
    fn test_prompt_template_format_multiple_vars() {
        let tpl = PromptTemplate::new(
            "Hello {name}, you are {age}",
            vec!["name".into(), "age".into()],
        );
        let mut vars = HashMap::new();
        let _ = vars.insert("name".into(), "Alice".into());
        let _ = vars.insert("age".into(), "30".into());
        assert_eq!(tpl.format(&vars).unwrap(), "Hello Alice, you are 30");
    }

    #[test]
    fn test_prompt_template_missing_variable() {
        let tpl = PromptTemplate::new("Hello {name}", vec!["name".into()]);
        let vars = HashMap::new();
        let err = tpl.format(&vars).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("name"),
            "error should mention the missing variable, got: {msg}"
        );
    }

    #[test]
    fn test_prompt_template_to_prompt_value() {
        let tpl = PromptTemplate::new("Hello {name}", vec!["name".into()]);
        let mut vars = HashMap::new();
        let _ = vars.insert("name".into(), "World".into());
        let pv = tpl.to_prompt_value(&vars).unwrap();
        assert_eq!(pv.to_text(), "Hello World");
    }

    #[test]
    fn test_input_variables_getter() {
        let tpl = PromptTemplate::new("Hi {a} {b}", vec!["a".into(), "b".into()]);
        assert_eq!(tpl.input_variables(), &["a", "b"]);
    }
}
