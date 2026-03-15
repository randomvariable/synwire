//! Combining output parser that merges results from multiple parsers.

use std::collections::HashMap;

use synwire_core::error::SynwireError;
use synwire_core::output_parsers::OutputParser;

/// Combines multiple `OutputParser<Output = HashMap<String, String>>` parsers,
/// merging their outputs into a single map.
///
/// Each inner parser is run on the same input text, and their results
/// are merged. Later parsers' keys overwrite earlier ones on conflict.
///
/// # Examples
///
/// ```
/// use synwire::output_parsers::CombiningOutputParser;
/// use synwire::output_parsers::XmlOutputParser;
///
/// let parser1 = XmlOutputParser::new(vec!["name".into()]);
/// let parser2 = XmlOutputParser::new(vec!["age".into()]);
///
/// let combined = CombiningOutputParser::new(vec![
///     Box::new(parser1),
///     Box::new(parser2),
/// ]);
/// ```
pub struct CombiningOutputParser {
    parsers: Vec<Box<dyn OutputParser<Output = HashMap<String, String>>>>,
}

impl CombiningOutputParser {
    /// Creates a new combining parser from a list of inner parsers.
    pub fn new(parsers: Vec<Box<dyn OutputParser<Output = HashMap<String, String>>>>) -> Self {
        Self { parsers }
    }
}

impl OutputParser for CombiningOutputParser {
    type Output = HashMap<String, String>;

    fn parse(&self, text: &str) -> Result<HashMap<String, String>, SynwireError> {
        let mut combined = HashMap::new();
        for parser in &self.parsers {
            let result = parser.parse(text)?;
            combined.extend(result);
        }
        Ok(combined)
    }

    fn get_format_instructions(&self) -> String {
        self.parsers
            .iter()
            .map(|p| p.get_format_instructions())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::output_parsers::XmlOutputParser;

    #[test]
    fn combines_results_from_multiple_parsers() {
        let p1 = XmlOutputParser::new(vec!["name".into()]);
        let p2 = XmlOutputParser::new(vec!["age".into()]);

        let combined = CombiningOutputParser::new(vec![Box::new(p1), Box::new(p2)]);

        let result = combined.parse("<name>Alice</name>\n<age>30</age>").unwrap();
        assert_eq!(result["name"], "Alice");
        assert_eq!(result["age"], "30");
    }

    #[test]
    fn error_propagated_from_inner_parser() {
        let p1 = XmlOutputParser::new(vec!["missing".into()]);
        let combined = CombiningOutputParser::new(vec![Box::new(p1)]);

        let result = combined.parse("<name>Alice</name>");
        assert!(result.is_err());
    }

    #[test]
    fn format_instructions_combined() {
        let p1 = XmlOutputParser::new(vec!["a".into()]);
        let p2 = XmlOutputParser::new(vec!["b".into()]);
        let combined = CombiningOutputParser::new(vec![Box::new(p1), Box::new(p2)]);

        let instructions = combined.get_format_instructions();
        assert!(instructions.contains("<a>"));
        assert!(instructions.contains("<b>"));
    }

    #[test]
    fn empty_parsers_returns_empty_map() {
        let combined = CombiningOutputParser::new(vec![]);
        let result = combined.parse("anything").unwrap();
        assert!(result.is_empty());
    }
}
