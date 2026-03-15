//! Comma-separated list output parser.

use synwire_core::error::SynwireError;
use synwire_core::output_parsers::OutputParser;

/// Parses comma-separated text into a `Vec<String>`.
///
/// Each item is trimmed of whitespace. Empty items are excluded.
///
/// # Examples
///
/// ```
/// use synwire::output_parsers::CommaSeparatedListOutputParser;
/// use synwire_core::output_parsers::OutputParser;
///
/// let parser = CommaSeparatedListOutputParser;
/// let result = parser.parse("apple, banana, cherry").unwrap();
/// assert_eq!(result, vec!["apple", "banana", "cherry"]);
/// ```
pub struct CommaSeparatedListOutputParser;

impl OutputParser for CommaSeparatedListOutputParser {
    type Output = Vec<String>;

    fn parse(&self, text: &str) -> Result<Vec<String>, SynwireError> {
        Ok(text
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect())
    }

    fn get_format_instructions(&self) -> String {
        "Your output should be a comma-separated list of items, e.g.: item1, item2, item3"
            .to_owned()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_list() {
        let parser = CommaSeparatedListOutputParser;
        let result = parser.parse("one, two, three").unwrap();
        assert_eq!(result, vec!["one", "two", "three"]);
    }

    #[test]
    fn handles_whitespace() {
        let parser = CommaSeparatedListOutputParser;
        let result = parser.parse("  a ,  b  , c  ").unwrap();
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn filters_empty_items() {
        let parser = CommaSeparatedListOutputParser;
        let result = parser.parse("a,,b,,,c").unwrap();
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn empty_input() {
        let parser = CommaSeparatedListOutputParser;
        let result = parser.parse("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn single_item() {
        let parser = CommaSeparatedListOutputParser;
        let result = parser.parse("only_one").unwrap();
        assert_eq!(result, vec!["only_one"]);
    }

    #[test]
    fn format_instructions() {
        let parser = CommaSeparatedListOutputParser;
        assert!(!parser.get_format_instructions().is_empty());
    }
}
