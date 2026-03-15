//! XML output parser.

use std::collections::HashMap;

use synwire_core::error::{ParseError, SynwireError};
use synwire_core::output_parsers::OutputParser;

/// Parses simple XML-tagged output into a key-value map.
///
/// Extracts content from XML-style tags like `<tag>content</tag>`.
/// This is a lightweight parser; it does not handle nested tags,
/// attributes, or CDATA sections.
///
/// # Examples
///
/// ```
/// use synwire::output_parsers::XmlOutputParser;
/// use synwire_core::output_parsers::OutputParser;
///
/// let parser = XmlOutputParser::new(vec!["name".into(), "age".into()]);
/// let result = parser.parse("<name>Alice</name>\n<age>30</age>").unwrap();
/// assert_eq!(result["name"], "Alice");
/// assert_eq!(result["age"], "30");
/// ```
pub struct XmlOutputParser {
    tags: Vec<String>,
}

impl XmlOutputParser {
    /// Creates a new XML parser that extracts the given tags.
    pub const fn new(tags: Vec<String>) -> Self {
        Self { tags }
    }
}

impl OutputParser for XmlOutputParser {
    type Output = HashMap<String, String>;

    fn parse(&self, text: &str) -> Result<HashMap<String, String>, SynwireError> {
        let mut result = HashMap::new();

        for tag in &self.tags {
            let open = format!("<{tag}>");
            let close = format!("</{tag}>");

            let start = text.find(&open).ok_or_else(|| {
                SynwireError::from(ParseError::ParseFailed {
                    message: format!("missing opening tag <{tag}>"),
                })
            })?;

            let content_start = start + open.len();
            let end = text[content_start..].find(&close).ok_or_else(|| {
                SynwireError::from(ParseError::ParseFailed {
                    message: format!("missing closing tag </{tag}>"),
                })
            })?;

            let content = text[content_start..content_start + end].trim().to_owned();
            let _ = result.insert(tag.clone(), content);
        }

        Ok(result)
    }

    fn get_format_instructions(&self) -> String {
        let tag_examples: Vec<String> = self
            .tags
            .iter()
            .map(|t| format!("<{t}>value</{t}>"))
            .collect();
        format!(
            "Respond with XML tags for each field:\n{}",
            tag_examples.join("\n")
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_xml() {
        let parser = XmlOutputParser::new(vec!["name".into(), "city".into()]);
        let result = parser
            .parse("<name>Alice</name>\n<city>London</city>")
            .unwrap();
        assert_eq!(result["name"], "Alice");
        assert_eq!(result["city"], "London");
    }

    #[test]
    fn trims_content_whitespace() {
        let parser = XmlOutputParser::new(vec!["val".into()]);
        let result = parser.parse("<val>  hello  </val>").unwrap();
        assert_eq!(result["val"], "hello");
    }

    #[test]
    fn missing_tag_returns_error() {
        let parser = XmlOutputParser::new(vec!["name".into(), "missing".into()]);
        let result = parser.parse("<name>Alice</name>");
        assert!(result.is_err());
    }

    #[test]
    fn missing_closing_tag_returns_error() {
        let parser = XmlOutputParser::new(vec!["name".into()]);
        let result = parser.parse("<name>Alice");
        assert!(result.is_err());
    }

    #[test]
    fn format_instructions_contain_tags() {
        let parser = XmlOutputParser::new(vec!["a".into(), "b".into()]);
        let instructions = parser.get_format_instructions();
        assert!(instructions.contains("<a>"));
        assert!(instructions.contains("</b>"));
    }
}
