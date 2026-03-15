//! String output parser that returns raw text unchanged.

use crate::error::SynwireError;
use crate::output_parsers::OutputParser;

/// Parser that returns raw text unchanged.
///
/// This is the simplest output parser, useful when no structured parsing
/// is needed and the model's raw text output is sufficient.
///
/// # Examples
///
/// ```
/// use synwire_core::output_parsers::{OutputParser, StrOutputParser};
///
/// let parser = StrOutputParser;
/// let result = parser.parse("Hello, world!").unwrap();
/// assert_eq!(result, "Hello, world!");
/// ```
pub struct StrOutputParser;

impl OutputParser for StrOutputParser {
    type Output = String;

    fn parse(&self, text: &str) -> Result<String, SynwireError> {
        Ok(text.to_string())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_str_parser_returns_unchanged() {
        let parser = StrOutputParser;
        let input = "Hello, world!";
        let result = parser.parse(input).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_str_parser_empty_string() {
        let parser = StrOutputParser;
        let result = parser.parse("").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_str_parser_preserves_whitespace() {
        let parser = StrOutputParser;
        let input = "  hello\n  world  ";
        let result = parser.parse(input).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_str_parser_format_instructions_empty() {
        let parser = StrOutputParser;
        assert!(parser.get_format_instructions().is_empty());
    }
}
