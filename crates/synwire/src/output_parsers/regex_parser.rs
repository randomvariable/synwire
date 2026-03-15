//! Regex-based output parser.

use std::collections::HashMap;

use regex::Regex;
use synwire_core::error::{ParseError, SynwireError};
use synwire_core::output_parsers::OutputParser;

/// Parses text using a regex pattern with named capture groups.
///
/// Each named capture group in the regex becomes a key in the output map.
///
/// # Examples
///
/// ```
/// use synwire::output_parsers::RegexParser;
/// use synwire_core::output_parsers::OutputParser;
///
/// let parser = RegexParser::new(
///     r"Name: (?P<name>\w+), Age: (?P<age>\d+)",
///     vec!["name".into(), "age".into()],
/// ).unwrap();
/// let result = parser.parse("Name: Alice, Age: 30").unwrap();
/// assert_eq!(result["name"], "Alice");
/// assert_eq!(result["age"], "30");
/// ```
pub struct RegexParser {
    regex: Regex,
    output_keys: Vec<String>,
}

impl RegexParser {
    /// Creates a new regex parser.
    ///
    /// # Errors
    ///
    /// Returns an error if the regex pattern is invalid.
    pub fn new(pattern: &str, output_keys: Vec<String>) -> Result<Self, SynwireError> {
        let regex = Regex::new(pattern).map_err(|e| {
            SynwireError::from(ParseError::ParseFailed {
                message: format!("invalid regex pattern: {e}"),
            })
        })?;
        Ok(Self { regex, output_keys })
    }
}

impl OutputParser for RegexParser {
    type Output = HashMap<String, String>;

    fn parse(&self, text: &str) -> Result<HashMap<String, String>, SynwireError> {
        let captures = self.regex.captures(text).ok_or_else(|| {
            SynwireError::from(ParseError::ParseFailed {
                message: format!("regex did not match text: '{text}'"),
            })
        })?;

        let mut result = HashMap::new();
        for key in &self.output_keys {
            let value = captures
                .name(key)
                .map(|m| m.as_str().to_owned())
                .ok_or_else(|| {
                    SynwireError::from(ParseError::ParseFailed {
                        message: format!("named capture group '{key}' not found"),
                    })
                })?;
            let _ = result.insert(key.clone(), value);
        }

        Ok(result)
    }

    fn get_format_instructions(&self) -> String {
        format!(
            "Respond so the output matches this pattern with fields: {}",
            self.output_keys.join(", ")
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn parses_named_groups() {
        let parser = RegexParser::new(
            r"Name: (?P<name>\w+), Age: (?P<age>\d+)",
            vec!["name".into(), "age".into()],
        )
        .unwrap();
        let result = parser.parse("Name: Bob, Age: 25").unwrap();
        assert_eq!(result["name"], "Bob");
        assert_eq!(result["age"], "25");
    }

    #[test]
    fn no_match_returns_error() {
        let parser = RegexParser::new(r"(?P<num>\d+)", vec!["num".into()]).unwrap();
        let result = parser.parse("no numbers here");
        assert!(result.is_err());
    }

    #[test]
    fn invalid_regex_returns_error() {
        let result = RegexParser::new(r"(invalid[", vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn missing_named_group_returns_error() {
        let parser =
            RegexParser::new(r"(?P<name>\w+)", vec!["name".into(), "missing".into()]).unwrap();
        let result = parser.parse("Alice");
        assert!(result.is_err());
    }

    #[test]
    fn format_instructions_list_keys() {
        let parser = RegexParser::new(r".", vec!["x".into(), "y".into()]).unwrap();
        let instructions = parser.get_format_instructions();
        assert!(instructions.contains('x'));
        assert!(instructions.contains('y'));
    }
}
