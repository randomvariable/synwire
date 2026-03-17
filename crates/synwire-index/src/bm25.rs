//! BM25 text index for hybrid keyword+vector search.
//!
//! Built alongside the semantic embedding index during indexing.
//! Provides fast keyword recall to complement semantic similarity.
//!
//! # Example
//!
//! ```no_run
//! use synwire_index::Bm25Index;
//! use std::path::Path;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let dir = std::env::temp_dir().join("bm25-example");
//! let mut idx = Bm25Index::create(&dir)?;
//! idx.add_document("doc1", "authentication login password", "auth.rs", None)?;
//! idx.commit()?;
//! let results = idx.search("authentication", 5)?;
//! # Ok(())
//! # }
//! ```

use std::path::Path;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Schema, TEXT, TextFieldIndexing, TextOptions, Value};
use tantivy::{Index, IndexWriter, ReloadPolicy, Term, doc};

/// A single BM25 search result.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct Bm25Result {
    /// Document identifier (matches the id used during indexing).
    pub id: String,
    /// BM25 relevance score.
    pub score: f32,
}

/// Errors from BM25 index operations.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Bm25Error {
    /// Failed to open or create the index.
    #[error("failed to open BM25 index: {0}")]
    Open(String),
    /// Failed to write to the index.
    #[error("BM25 write error: {0}")]
    Write(String),
    /// Failed to execute a search query.
    #[error("BM25 search error: {0}")]
    Search(String),
}

/// BM25 keyword index backed by tantivy.
///
/// Wraps a tantivy [`Index`] with an [`IndexWriter`] and provides a simple
/// API for adding documents, committing, searching, and deleting by id.
///
/// # Thread safety
///
/// `Bm25Index` is **not** `Sync` — tantivy's `IndexWriter` requires exclusive
/// access.  Wrap in a `Mutex` or `tokio::sync::Mutex` if shared across tasks.
pub struct Bm25Index {
    index: Index,
    writer: IndexWriter,
    id_field: tantivy::schema::Field,
    content_field: tantivy::schema::Field,
    file_field: tantivy::schema::Field,
    symbol_field: tantivy::schema::Field,
}

impl std::fmt::Debug for Bm25Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bm25Index").finish_non_exhaustive()
    }
}

fn build_schema() -> (
    Schema,
    tantivy::schema::Field,
    tantivy::schema::Field,
    tantivy::schema::Field,
    tantivy::schema::Field,
) {
    let mut schema_builder = Schema::builder();

    // id: stored + indexed as a single token for exact match deletes
    let id_indexing = TextFieldIndexing::default()
        .set_tokenizer("raw")
        .set_index_option(tantivy::schema::IndexRecordOption::Basic);
    let id_opts = TextOptions::default()
        .set_indexing_options(id_indexing)
        .set_stored();
    let id_field = schema_builder.add_text_field("id", id_opts);

    // content: full-text indexed for BM25 scoring, not stored (large)
    let content_field = schema_builder.add_text_field("content", TEXT);

    // file: stored + raw token for exact retrieval
    let file_indexing = TextFieldIndexing::default()
        .set_tokenizer("raw")
        .set_index_option(tantivy::schema::IndexRecordOption::Basic);
    let file_opts = TextOptions::default()
        .set_indexing_options(file_indexing)
        .set_stored();
    let file_field = schema_builder.add_text_field("file", file_opts);

    // symbol: stored + raw token, optional
    let symbol_indexing = TextFieldIndexing::default()
        .set_tokenizer("raw")
        .set_index_option(tantivy::schema::IndexRecordOption::Basic);
    let symbol_opts = TextOptions::default()
        .set_indexing_options(symbol_indexing)
        .set_stored();
    let symbol_field = schema_builder.add_text_field("symbol", symbol_opts);

    let schema = schema_builder.build();
    (schema, id_field, content_field, file_field, symbol_field)
}

impl Bm25Index {
    /// Create a new BM25 index in `dir`.
    ///
    /// `dir` will be created if it does not already exist.
    ///
    /// # Errors
    ///
    /// Returns [`Bm25Error::Open`] if the directory cannot be created or the
    /// index cannot be initialised.
    pub fn create(dir: &Path) -> Result<Self, Bm25Error> {
        std::fs::create_dir_all(dir).map_err(|e| Bm25Error::Open(e.to_string()))?;

        let (schema, id_field, content_field, file_field, symbol_field) = build_schema();

        let index =
            Index::create_in_dir(dir, schema).map_err(|e| Bm25Error::Open(e.to_string()))?;

        // 50 MB write buffer
        let writer = index
            .writer(50_000_000)
            .map_err(|e| Bm25Error::Open(e.to_string()))?;

        Ok(Self {
            index,
            writer,
            id_field,
            content_field,
            file_field,
            symbol_field,
        })
    }

    /// Open an existing BM25 index from `dir`.
    ///
    /// # Errors
    ///
    /// Returns [`Bm25Error::Open`] if the directory does not contain a valid
    /// tantivy index or the writer cannot be acquired.
    pub fn open(dir: &Path) -> Result<Self, Bm25Error> {
        let index = Index::open_in_dir(dir).map_err(|e| Bm25Error::Open(e.to_string()))?;

        let schema = index.schema();
        let id_field = schema
            .get_field("id")
            .map_err(|e| Bm25Error::Open(e.to_string()))?;
        let content_field = schema
            .get_field("content")
            .map_err(|e| Bm25Error::Open(e.to_string()))?;
        let file_field = schema
            .get_field("file")
            .map_err(|e| Bm25Error::Open(e.to_string()))?;
        let symbol_field = schema
            .get_field("symbol")
            .map_err(|e| Bm25Error::Open(e.to_string()))?;

        let writer = index
            .writer(50_000_000)
            .map_err(|e| Bm25Error::Open(e.to_string()))?;

        Ok(Self {
            index,
            writer,
            id_field,
            content_field,
            file_field,
            symbol_field,
        })
    }

    /// Add (or replace) a document in the index.
    ///
    /// Any existing document with the same `id` is deleted first, enabling
    /// incremental updates without duplicates.  Call [`commit`](Self::commit)
    /// to flush the pending writes to disk.
    ///
    /// # Errors
    ///
    /// Returns [`Bm25Error::Write`] if the document cannot be added.
    pub fn add_document(
        &mut self,
        id: &str,
        content: &str,
        file: &str,
        symbol: Option<&str>,
    ) -> Result<(), Bm25Error> {
        // Delete any existing document with this id for idempotent upsert.
        let _ = self
            .writer
            .delete_term(Term::from_field_text(self.id_field, id));

        let mut doc = doc!(
            self.id_field => id,
            self.content_field => content,
            self.file_field => file,
        );
        if let Some(sym) = symbol {
            doc.add_text(self.symbol_field, sym);
        }
        let _ = self
            .writer
            .add_document(doc)
            .map_err(|e| Bm25Error::Write(e.to_string()))?;

        Ok(())
    }

    /// Commit pending writes to disk.
    ///
    /// # Errors
    ///
    /// Returns [`Bm25Error::Write`] if the commit fails.
    pub fn commit(&mut self) -> Result<(), Bm25Error> {
        let _ = self
            .writer
            .commit()
            .map_err(|e| Bm25Error::Write(e.to_string()))?;
        Ok(())
    }

    /// Delete a document by `id`.
    ///
    /// The deletion is buffered — call [`commit`](Self::commit) afterwards to
    /// persist it.
    ///
    /// # Errors
    ///
    /// This method currently cannot fail; the `Result` type is kept for API
    /// consistency with other mutating methods.
    pub fn delete_document(&mut self, id: &str) -> Result<(), Bm25Error> {
        let _ = self
            .writer
            .delete_term(Term::from_field_text(self.id_field, id));
        Ok(())
    }

    /// Search the index for `query`, returning up to `top_k` results.
    ///
    /// Uses tantivy's default BM25 scorer over the `content` field.
    ///
    /// # Errors
    ///
    /// Returns [`Bm25Error::Search`] if the query cannot be parsed or the
    /// search fails.
    pub fn search(&self, query: &str, top_k: usize) -> Result<Vec<Bm25Result>, Bm25Error> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()
            .map_err(|e: tantivy::TantivyError| Bm25Error::Search(e.to_string()))?;

        let searcher = reader.searcher();

        let query_parser = QueryParser::for_index(&self.index, vec![self.content_field]);
        let parsed = query_parser
            .parse_query(query)
            .map_err(|e| Bm25Error::Search(e.to_string()))?;

        let top_docs = searcher
            .search(&parsed, &TopDocs::with_limit(top_k))
            .map_err(|e| Bm25Error::Search(e.to_string()))?;

        let mut results = Vec::with_capacity(top_docs.len());
        for (score, doc_address) in top_docs {
            let retrieved: tantivy::TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| Bm25Error::Search(e.to_string()))?;
            let id = retrieved
                .get_first(self.id_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            results.push(Bm25Result { id, score });
        }

        Ok(results)
    }
}

#[cfg(all(test, feature = "hybrid-search"))]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn bm25_search_basic() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut idx = Bm25Index::create(dir.path()).expect("create");
        idx.add_document("doc1", "authentication login password", "auth.rs", None)
            .expect("add doc1");
        idx.add_document("doc2", "database connection pool query", "db.rs", None)
            .expect("add doc2");
        idx.commit().expect("commit");

        let results = idx.search("authentication", 5).expect("search");
        assert!(!results.is_empty(), "expected at least one result");
        assert_eq!(results[0].id, "doc1");
    }

    #[test]
    fn bm25_delete_document() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut idx = Bm25Index::create(dir.path()).expect("create");
        idx.add_document("doc1", "authentication login", "auth.rs", None)
            .expect("add");
        idx.commit().expect("commit");

        idx.delete_document("doc1").expect("delete");
        idx.commit().expect("commit after delete");

        let results = idx
            .search("authentication", 5)
            .expect("search after delete");
        assert!(results.is_empty(), "document should have been deleted");
    }

    #[test]
    fn bm25_upsert_replaces_existing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut idx = Bm25Index::create(dir.path()).expect("create");
        idx.add_document("doc1", "authentication login", "auth.rs", None)
            .expect("add first");
        idx.commit().expect("commit first");

        // Re-add doc1 with different content — should replace, not duplicate.
        idx.add_document("doc1", "database connection pool", "db.rs", None)
            .expect("upsert");
        idx.commit().expect("commit upsert");

        let auth_results = idx.search("authentication", 5).expect("search auth");
        assert!(
            auth_results.is_empty(),
            "old content should no longer match"
        );

        let db_results = idx.search("database", 5).expect("search database");
        assert!(!db_results.is_empty(), "new content should match");
        assert_eq!(db_results[0].id, "doc1");
    }

    #[test]
    fn hybrid_alpha_pure_bm25() {
        use crate::hybrid::{HybridSearchConfig, hybrid_search};

        let dir = tempfile::tempdir().expect("tempdir");
        let mut idx = Bm25Index::create(dir.path()).expect("create");
        idx.add_document("doc1", "authentication login", "auth.rs", None)
            .expect("add doc1");
        idx.add_document("doc2", "database connection", "db.rs", None)
            .expect("add doc2");
        idx.commit().expect("commit");

        // alpha=1.0 means purely BM25 — vector scores are irrelevant.
        // Provide a vector score that strongly favours doc2 to prove it is ignored.
        let vector_results = vec![
            ("doc2".to_string(), 0.99_f32),
            ("doc1".to_string(), 0.01_f32),
        ];
        let config = HybridSearchConfig {
            alpha: 1.0,
            top_k: 5,
        };

        let results =
            hybrid_search(&idx, &vector_results, "authentication", &config).expect("hybrid_search");
        // BM25 should put doc1 first despite the poor vector score.
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "doc1");
    }
}
