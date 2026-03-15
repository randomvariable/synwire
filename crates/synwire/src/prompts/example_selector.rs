//! Example selector traits and implementations.

use std::collections::HashMap;

use synwire_core::BoxFuture;
use synwire_core::error::SynwireError;

/// Trait for selecting relevant examples for few-shot prompts.
///
/// Implementations choose the most relevant examples from a pool
/// based on the input query.
pub trait ExampleSelector: Send + Sync {
    /// Selects examples relevant to the given input variables.
    fn select_examples<'a>(
        &'a self,
        input_variables: &'a HashMap<String, String>,
    ) -> BoxFuture<'a, Result<Vec<HashMap<String, String>>, SynwireError>>;

    /// Adds an example to the selector's pool.
    fn add_example(
        &self,
        example: HashMap<String, String>,
    ) -> BoxFuture<'_, Result<(), SynwireError>>;
}

/// A stub example selector that returns all examples regardless of input.
///
/// In a full implementation this would use embeddings to find semantically
/// similar examples, but for now it returns everything.
///
/// # Examples
///
/// ```
/// use synwire::prompts::SemanticSimilarityExampleSelector;
///
/// let selector = SemanticSimilarityExampleSelector::new();
/// ```
pub struct SemanticSimilarityExampleSelector {
    examples: std::sync::Mutex<Vec<HashMap<String, String>>>,
}

impl SemanticSimilarityExampleSelector {
    /// Creates a new empty selector.
    pub const fn new() -> Self {
        Self {
            examples: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl Default for SemanticSimilarityExampleSelector {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to convert a mutex poison error to a `SynwireError`.
fn lock_err(e: impl std::fmt::Display) -> SynwireError {
    SynwireError::Other(Box::new(std::io::Error::other(e.to_string())))
}

impl ExampleSelector for SemanticSimilarityExampleSelector {
    fn select_examples<'a>(
        &'a self,
        _input_variables: &'a HashMap<String, String>,
    ) -> BoxFuture<'a, Result<Vec<HashMap<String, String>>, SynwireError>> {
        Box::pin(async move {
            let guard = self.examples.lock().map_err(lock_err)?;
            Ok(guard.clone())
        })
    }

    fn add_example(
        &self,
        example: HashMap<String, String>,
    ) -> BoxFuture<'_, Result<(), SynwireError>> {
        Box::pin(async move {
            self.examples.lock().map_err(lock_err)?.push(example);
            Ok(())
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn selector_returns_all_examples() {
        let selector = SemanticSimilarityExampleSelector::new();

        let mut ex1 = HashMap::new();
        let _ = ex1.insert("input".into(), "hello".into());
        let _ = ex1.insert("output".into(), "world".into());
        selector.add_example(ex1).await.unwrap();

        let mut ex2 = HashMap::new();
        let _ = ex2.insert("input".into(), "foo".into());
        let _ = ex2.insert("output".into(), "bar".into());
        selector.add_example(ex2).await.unwrap();

        let input = HashMap::new();
        let examples = selector.select_examples(&input).await.unwrap();
        assert_eq!(examples.len(), 2);
    }

    #[tokio::test]
    async fn empty_selector_returns_empty() {
        let selector = SemanticSimilarityExampleSelector::new();
        let input = HashMap::new();
        let examples = selector.select_examples(&input).await.unwrap();
        assert!(examples.is_empty());
    }
}
