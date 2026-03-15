//! Structured output parser that deserialises JSON to a typed struct.

use std::marker::PhantomData;

use serde::de::DeserializeOwned;

use crate::error::{ParseError, SynwireError};
use crate::output_parsers::OutputParser;

/// Parser that deserialises JSON text to a typed struct.
///
/// Use this when you have a specific Rust type annotated with `#[derive(Deserialize)]`
/// that you want to parse model output into.
///
/// # Examples
///
/// ```
/// use serde::Deserialize;
/// use synwire_core::output_parsers::{OutputParser, StructuredOutputParser};
///
/// #[derive(Deserialize, Debug, PartialEq)]
/// struct Person {
///     name: String,
///     age: u32,
/// }
///
/// let parser = StructuredOutputParser::<Person>::new();
/// let result = parser.parse(r#"{"name": "Alice", "age": 30}"#).unwrap();
/// assert_eq!(result.name, "Alice");
/// assert_eq!(result.age, 30);
/// ```
pub struct StructuredOutputParser<T: DeserializeOwned> {
    _marker: PhantomData<T>,
}

impl<T: DeserializeOwned> StructuredOutputParser<T> {
    /// Create a new structured output parser.
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<T: DeserializeOwned + Send + Sync> StructuredOutputParser<T> {
    /// Parse with a validation error message for retry.
    ///
    /// On success, returns the parsed value. On failure, returns both the
    /// original error and a formatted context string suitable for inclusion
    /// in a retry prompt.
    ///
    /// # Errors
    ///
    /// Returns a tuple of (`SynwireError`, retry context `String`) when
    /// parsing fails.
    pub fn parse_with_retry_context(&self, text: &str) -> Result<T, (SynwireError, String)> {
        match self.parse(text) {
            Ok(v) => Ok(v),
            Err(e) => {
                let context = format!(
                    "Previous attempt failed with error: {e}\nPlease fix the output and try again."
                );
                Err((e, context))
            }
        }
    }
}

impl<T: DeserializeOwned> Default for StructuredOutputParser<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: DeserializeOwned + Send + Sync> OutputParser for StructuredOutputParser<T> {
    type Output = T;

    fn parse(&self, text: &str) -> Result<T, SynwireError> {
        serde_json::from_str(text).map_err(|e| {
            SynwireError::from(ParseError::ParseFailed {
                message: format!("Failed to parse structured output: {e}"),
            })
        })
    }

    fn get_format_instructions(&self) -> String {
        "Respond with valid JSON matching the expected schema.".to_string()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestPerson {
        name: String,
        age: u32,
    }

    #[test]
    fn test_structured_parser() {
        let parser = StructuredOutputParser::<TestPerson>::new();
        let result = parser.parse(r#"{"name": "Alice", "age": 30}"#).unwrap();
        assert_eq!(
            result,
            TestPerson {
                name: "Alice".to_string(),
                age: 30,
            }
        );
    }

    #[test]
    fn test_structured_parser_invalid() {
        let parser = StructuredOutputParser::<TestPerson>::new();
        let result = parser.parse(r#"{"name": "Alice"}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_structured_parser_format_instructions() {
        let parser = StructuredOutputParser::<TestPerson>::new();
        assert_eq!(
            parser.get_format_instructions(),
            "Respond with valid JSON matching the expected schema."
        );
    }

    #[test]
    fn test_parse_with_retry_context_success() {
        let parser = StructuredOutputParser::<TestPerson>::new();
        let result = parser
            .parse_with_retry_context(r#"{"name": "Alice", "age": 30}"#)
            .unwrap();
        assert_eq!(result.name, "Alice");
        assert_eq!(result.age, 30);
    }

    #[test]
    fn test_parse_with_retry_context_failure() {
        let parser = StructuredOutputParser::<TestPerson>::new();
        let result = parser.parse_with_retry_context(r#"{"name": "Alice"}"#);
        assert!(result.is_err());
        let (err, context) = result.unwrap_err();
        assert!(err.to_string().contains("Failed to parse"));
        assert!(context.contains("Previous attempt failed"));
        assert!(context.contains("Please fix the output"));
    }

    #[test]
    fn test_structured_parser_default() {
        let parser = StructuredOutputParser::<TestPerson>::default();
        let result = parser.parse(r#"{"name": "Bob", "age": 25}"#).unwrap();
        assert_eq!(result.name, "Bob");
    }
}
