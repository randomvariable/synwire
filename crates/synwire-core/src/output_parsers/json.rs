//! JSON output parser that deserialises text to `serde_json::Value`.

use crate::error::{ParseError, SynwireError};
use crate::output_parsers::OutputParser;

/// Parser that deserialises JSON text to `serde_json::Value`.
///
/// Useful when you need structured data from a model but do not have
/// a specific Rust type to deserialise into.
///
/// # Examples
///
/// ```
/// use synwire_core::output_parsers::{OutputParser, JsonOutputParser};
///
/// let parser = JsonOutputParser;
/// let result = parser.parse(r#"{"key": "value"}"#).unwrap();
/// assert_eq!(result["key"], "value");
/// ```
pub struct JsonOutputParser;

impl OutputParser for JsonOutputParser {
    type Output = serde_json::Value;

    fn parse(&self, text: &str) -> Result<serde_json::Value, SynwireError> {
        serde_json::from_str(text).map_err(|e| {
            SynwireError::from(ParseError::ParseFailed {
                message: format!("Failed to parse JSON: {e}"),
            })
        })
    }

    fn get_format_instructions(&self) -> String {
        "Respond with valid JSON.".to_string()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_json_parser_valid() {
        let parser = JsonOutputParser;
        let result = parser.parse(r#"{"name": "Alice", "age": 30}"#).unwrap();
        assert_eq!(result["name"], "Alice");
        assert_eq!(result["age"], 30);
    }

    #[test]
    fn test_json_parser_invalid() {
        let parser = JsonOutputParser;
        let result = parser.parse("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_json_parser_array() {
        let parser = JsonOutputParser;
        let result = parser.parse(r"[1, 2, 3]").unwrap();
        assert_eq!(result.as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_json_parser_format_instructions() {
        let parser = JsonOutputParser;
        assert_eq!(parser.get_format_instructions(), "Respond with valid JSON.");
    }
}
