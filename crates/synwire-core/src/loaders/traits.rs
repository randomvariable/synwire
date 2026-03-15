//! Document loader trait definition.

use crate::BoxFuture;
use crate::documents::Document;
use crate::error::SynwireError;

/// Trait for document loaders.
///
/// A loader reads documents from an external source (file, URL, database, etc.)
/// and returns them as a `Vec<Document>`.
pub trait DocumentLoader: Send + Sync {
    /// Load documents from the source.
    fn load(&self) -> BoxFuture<'_, Result<Vec<Document>, SynwireError>>;
}
