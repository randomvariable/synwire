//! Core output parser trait.

use crate::error::SynwireError;

/// Trait for parsing model output into structured types.
///
/// Output parsers transform raw text from language models into structured data.
/// Implement this trait to create custom parsers for specific output formats.
///
/// # Examples
///
/// ```
/// use synwire_core::output_parsers::OutputParser;
/// use synwire_core::error::SynwireError;
///
/// struct UpperCaseParser;
///
/// impl OutputParser for UpperCaseParser {
///     type Output = String;
///
///     fn parse(&self, text: &str) -> Result<String, SynwireError> {
///         Ok(text.to_uppercase())
///     }
/// }
/// ```
pub trait OutputParser: Send + Sync {
    /// The output type this parser produces.
    type Output;

    /// Parse raw text output from a model.
    ///
    /// # Errors
    ///
    /// Returns `SynwireError` if the text cannot be parsed into the expected type.
    fn parse(&self, text: &str) -> Result<Self::Output, SynwireError>;

    /// Get format instructions to include in prompts.
    ///
    /// These instructions guide the model to produce output in the format
    /// expected by this parser. Returns an empty string by default.
    fn get_format_instructions(&self) -> String {
        String::new()
    }
}
