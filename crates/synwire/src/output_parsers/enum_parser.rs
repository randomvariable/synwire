//! Enum output parser.

use synwire_core::error::{ParseError, SynwireError};
use synwire_core::output_parsers::OutputParser;

/// Parses text into one of a set of allowed enum values.
///
/// The input text is trimmed and compared case-insensitively against
/// the allowed values.
///
/// # Examples
///
/// ```
/// use synwire::output_parsers::EnumOutputParser;
/// use synwire_core::output_parsers::OutputParser;
///
/// let parser = EnumOutputParser::new(vec![
///     "red".into(), "green".into(), "blue".into(),
/// ]);
/// let result = parser.parse("Green").unwrap();
/// assert_eq!(result, "green");
/// ```
pub struct EnumOutputParser {
    allowed_values: Vec<String>,
}

impl EnumOutputParser {
    /// Creates a new enum parser with the given allowed values.
    ///
    /// Values should be lowercase; matching is case-insensitive.
    pub const fn new(allowed_values: Vec<String>) -> Self {
        Self { allowed_values }
    }
}

impl OutputParser for EnumOutputParser {
    type Output = String;

    fn parse(&self, text: &str) -> Result<String, SynwireError> {
        let trimmed = text.trim();
        let lower = trimmed.to_lowercase();

        for value in &self.allowed_values {
            if value.to_lowercase() == lower {
                return Ok(value.clone());
            }
        }

        Err(SynwireError::from(ParseError::ParseFailed {
            message: format!(
                "'{trimmed}' is not one of the allowed values: {:?}",
                self.allowed_values
            ),
        }))
    }

    fn get_format_instructions(&self) -> String {
        format!(
            "Select one of the following options: {}",
            self.allowed_values.join(", ")
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn parser() -> EnumOutputParser {
        EnumOutputParser::new(vec!["red".into(), "green".into(), "blue".into()])
    }

    #[test]
    fn exact_match() {
        let result = parser().parse("red").unwrap();
        assert_eq!(result, "red");
    }

    #[test]
    fn case_insensitive() {
        let result = parser().parse("GREEN").unwrap();
        assert_eq!(result, "green");
    }

    #[test]
    fn trims_whitespace() {
        let result = parser().parse("  blue  ").unwrap();
        assert_eq!(result, "blue");
    }

    #[test]
    fn rejects_invalid_value() {
        let result = parser().parse("yellow");
        assert!(result.is_err());
    }

    #[test]
    fn format_instructions_lists_values() {
        let instructions = parser().get_format_instructions();
        assert!(instructions.contains("red"));
        assert!(instructions.contains("green"));
        assert!(instructions.contains("blue"));
    }
}
