//! `LanceDB`-backed [`VectorStore`] implementation.

use std::collections::HashMap;
use std::sync::Arc;

use arrow_array::types::Float32Type;
use arrow_array::{
    Array, FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures_util::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use serde_json::Value;
use tracing::debug;
use uuid::Uuid;

use synwire_core::BoxFuture;
use synwire_core::documents::Document;
use synwire_core::embeddings::Embeddings;
use synwire_core::error::SynwireError;
use synwire_core::vectorstores::VectorStore;

use crate::error::LanceDbError;

/// Column name for the distance score returned by `LanceDB` vector search.
const DISTANCE_COL: &str = "_distance";

/// Column name for document identifiers.
const ID_COL: &str = "id";

/// Column name for document text content.
const TEXT_COL: &str = "text";

/// Column name for the embedding vector.
const VECTOR_COL: &str = "vector";

/// Column name for JSON-serialised document metadata.
const METADATA_COL: &str = "metadata";

/// `LanceDB`-backed vector store.
///
/// Stores document chunks with embedding vectors in a `LanceDB` table for
/// persistent semantic search. The table is created automatically if it does
/// not already exist.
///
/// # Schema
///
/// | Column     | Arrow type                     | Description                        |
/// |------------|--------------------------------|------------------------------------|
/// | `id`       | Utf8                           | UUID string identifier             |
/// | `text`     | Utf8                           | Document page content              |
/// | `vector`   | FixedSizeList(Float32, `dims`) | Embedding vector                   |
/// | `metadata` | Utf8                           | JSON-serialised metadata `HashMap` |
///
/// # Example
///
/// ```no_run
/// use synwire_vectorstore_lancedb::LanceDbVectorStore;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Open (or create) a table with 384-dimensional embeddings.
/// let store = LanceDbVectorStore::open("/tmp/my-db", "documents", 384).await?;
/// # Ok(())
/// # }
/// ```
pub struct LanceDbVectorStore {
    table: lancedb::Table,
    dims: usize,
}

impl LanceDbVectorStore {
    /// Open or create a `LanceDB` vector store at the given path.
    ///
    /// If a table named `table_name` already exists inside the database at
    /// `path` it is opened directly; otherwise an empty table with the
    /// appropriate schema is created.
    ///
    /// `dims` is the embedding dimensionality (e.g. `384` for
    /// `bge-small-en-v1.5`). The value must match the output of the
    /// [`Embeddings`] implementation that will be used with this store.
    ///
    /// # Errors
    ///
    /// Returns [`LanceDbError`] if the connection or table creation fails.
    pub async fn open(path: &str, table_name: &str, dims: usize) -> Result<Self, LanceDbError> {
        let conn = lancedb::connect(path).execute().await?;

        let existing = conn.table_names().execute().await?;

        let table = if existing.contains(&table_name.to_owned()) {
            debug!(table = table_name, "opening existing `LanceDB` table");
            conn.open_table(table_name).execute().await?
        } else {
            debug!(table = table_name, dims, "creating new `LanceDB` table");
            let schema = Arc::new(build_schema(dims));
            conn.create_empty_table(table_name, schema)
                .execute()
                .await?
        };

        Ok(Self { table, dims })
    }

    /// Add rows to the underlying `LanceDB` table.
    ///
    /// Validates that every embedding produced by `embeddings` has exactly
    /// `dims` dimensions. On success, returns the list of IDs that were
    /// assigned to the persisted documents.
    async fn add_documents_inner(
        &self,
        documents: &[Document],
        embeddings: &dyn Embeddings,
    ) -> Result<Vec<String>, LanceDbError> {
        if documents.is_empty() {
            return Ok(Vec::new());
        }

        let texts: Vec<String> = documents.iter().map(|d| d.page_content.clone()).collect();

        let vectors = embeddings
            .embed_documents(&texts)
            .await
            .map_err(|e| LanceDbError::Embedding(e.to_string()))?;

        // Assign IDs — reuse the document's existing ID when available.
        let ids: Vec<String> = documents
            .iter()
            .map(|d| d.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string()))
            .collect();

        let schema = Arc::new(build_schema(self.dims));

        // Validate dimensions and build Arrow arrays.
        let id_array = StringArray::from(ids.clone());
        let text_array = StringArray::from(
            documents
                .iter()
                .map(|d| d.page_content.as_str())
                .collect::<Vec<_>>(),
        );

        let mut all_floats: Vec<Option<Vec<Option<f32>>>> = Vec::with_capacity(documents.len());
        for (i, vec) in vectors.iter().enumerate() {
            if vec.len() != self.dims {
                return Err(LanceDbError::DimensionMismatch {
                    expected: self.dims,
                    actual: vec.len(),
                });
            }
            let row: Vec<Option<f32>> = vec.iter().map(|&v| Some(v)).collect();
            let _ = i; // suppress unused-variable warning
            all_floats.push(Some(row));
        }

        let vector_array = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
            all_floats,
            i32::try_from(self.dims).unwrap_or(i32::MAX),
        );

        let metadata_strs: Result<Vec<String>, serde_json::Error> = documents
            .iter()
            .map(|d| serde_json::to_string(&d.metadata))
            .collect();
        let metadata_array = StringArray::from(
            metadata_strs?
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        );

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(id_array) as Arc<dyn Array>,
                Arc::new(text_array) as Arc<dyn Array>,
                Arc::new(vector_array) as Arc<dyn Array>,
                Arc::new(metadata_array) as Arc<dyn Array>,
            ],
        )?;

        let reader = RecordBatchIterator::new(vec![batch].into_iter().map(Ok), schema);

        let _ = self.table.add(reader).execute().await?;

        debug!(count = ids.len(), "added documents to `LanceDB`");

        Ok(ids)
    }

    /// Perform a vector similarity search and return scored results.
    async fn similarity_search_with_score_inner(
        &self,
        query: &str,
        k: usize,
        embeddings: &dyn Embeddings,
    ) -> Result<Vec<(Document, f32)>, LanceDbError> {
        let query_vec: Vec<f32> = embeddings
            .embed_query(query)
            .await
            .map_err(|e| LanceDbError::Embedding(e.to_string()))?;

        if query_vec.len() != self.dims {
            return Err(LanceDbError::DimensionMismatch {
                expected: self.dims,
                actual: query_vec.len(),
            });
        }

        let stream = self
            .table
            .vector_search(query_vec.as_slice())?
            .limit(k)
            .execute()
            .await?;

        let batches: Vec<RecordBatch> = stream.try_collect().await?;

        let mut results: Vec<(Document, f32)> = Vec::new();

        for batch in &batches {
            let n_rows = batch.num_rows();

            let id_col = batch
                .column_by_name(ID_COL)
                .ok_or_else(|| LanceDbError::MissingColumn(ID_COL.to_owned()))?
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| LanceDbError::MissingColumn(ID_COL.to_owned()))?;

            let text_col = batch
                .column_by_name(TEXT_COL)
                .ok_or_else(|| LanceDbError::MissingColumn(TEXT_COL.to_owned()))?
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| LanceDbError::MissingColumn(TEXT_COL.to_owned()))?;

            let metadata_col = batch
                .column_by_name(METADATA_COL)
                .ok_or_else(|| LanceDbError::MissingColumn(METADATA_COL.to_owned()))?
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| LanceDbError::MissingColumn(METADATA_COL.to_owned()))?;

            let distance_col = batch
                .column_by_name(DISTANCE_COL)
                .ok_or_else(|| LanceDbError::MissingColumn(DISTANCE_COL.to_owned()))?
                .as_any()
                .downcast_ref::<Float32Array>()
                .ok_or_else(|| LanceDbError::MissingColumn(DISTANCE_COL.to_owned()))?;

            for i in 0..n_rows {
                let id = id_col.value(i).to_owned();
                let text = text_col.value(i).to_owned();
                let metadata_json = metadata_col.value(i);
                let distance = distance_col.value(i);

                let metadata: HashMap<String, Value> = serde_json::from_str(metadata_json)?;

                let doc = Document {
                    id: Some(id),
                    page_content: text,
                    metadata,
                };

                results.push((doc, distance));
            }
        }

        // `LanceDB` returns results sorted by distance ascending; the trait
        // contract expects descending similarity score. We convert distance
        // to a cosine-style similarity by negating so that lower distances
        // sort higher (most similar first). The raw score preserved here is
        // the raw L2/cosine distance value as returned by `LanceDB`.
        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results)
    }

    /// Delete documents by a SQL predicate string.
    async fn delete_inner(&self, ids: &[String]) -> Result<(), LanceDbError> {
        if ids.is_empty() {
            return Ok(());
        }

        // Build a SQL IN clause: id IN ('id1','id2',...)
        let quoted: Vec<String> = ids.iter().map(|id| format!("'{id}'")).collect();
        let predicate = format!("{ID_COL} IN ({})", quoted.join(", "));

        debug!(predicate = %predicate, "deleting documents from `LanceDB`");

        let _ = self.table.delete(&predicate).await?;

        Ok(())
    }
}

impl VectorStore for LanceDbVectorStore {
    fn add_documents<'a>(
        &'a self,
        documents: &'a [Document],
        embeddings: &'a dyn Embeddings,
    ) -> BoxFuture<'a, Result<Vec<String>, SynwireError>> {
        Box::pin(async move {
            self.add_documents_inner(documents, embeddings)
                .await
                .map_err(SynwireError::from)
        })
    }

    fn similarity_search<'a>(
        &'a self,
        query: &'a str,
        k: usize,
        embeddings: &'a dyn Embeddings,
    ) -> BoxFuture<'a, Result<Vec<Document>, SynwireError>> {
        Box::pin(async move {
            let results = self
                .similarity_search_with_score_inner(query, k, embeddings)
                .await
                .map_err(SynwireError::from)?;
            Ok(results.into_iter().map(|(doc, _score)| doc).collect())
        })
    }

    fn similarity_search_with_score<'a>(
        &'a self,
        query: &'a str,
        k: usize,
        embeddings: &'a dyn Embeddings,
    ) -> BoxFuture<'a, Result<Vec<(Document, f32)>, SynwireError>> {
        Box::pin(async move {
            self.similarity_search_with_score_inner(query, k, embeddings)
                .await
                .map_err(SynwireError::from)
        })
    }

    fn delete<'a>(&'a self, ids: &'a [String]) -> BoxFuture<'a, Result<(), SynwireError>> {
        Box::pin(async move { self.delete_inner(ids).await.map_err(SynwireError::from) })
    }
}

/// Construct the Arrow schema used for storing documents in `LanceDB`.
fn build_schema(dims: usize) -> Schema {
    Schema::new(vec![
        Field::new(ID_COL, DataType::Utf8, false),
        Field::new(TEXT_COL, DataType::Utf8, false),
        Field::new(
            VECTOR_COL,
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                i32::try_from(dims).unwrap_or(i32::MAX),
            ),
            true,
        ),
        Field::new(METADATA_COL, DataType::Utf8, false),
    ])
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::HashMap;

    use synwire_core::documents::Document;
    use synwire_core::embeddings::FakeEmbeddings;
    use synwire_core::vectorstores::VectorStore;
    use tempfile::tempdir;

    use super::LanceDbVectorStore;

    async fn make_store(dims: usize) -> LanceDbVectorStore {
        let dir = tempdir().unwrap();
        LanceDbVectorStore::open(dir.path().to_str().unwrap(), "test_docs", dims)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn open_creates_table() {
        let _store = make_store(32).await;
    }

    #[tokio::test]
    async fn add_documents_returns_ids() {
        let store = make_store(32).await;
        let embeddings = FakeEmbeddings::new(32);
        let docs = vec![Document::new("hello world"), Document::new("goodbye world")];
        let ids = store.add_documents(&docs, &embeddings).await.unwrap();
        assert_eq!(ids.len(), 2);
        assert_ne!(ids[0], ids[1]);
    }

    #[tokio::test]
    async fn preserves_document_ids() {
        let store = make_store(32).await;
        let embeddings = FakeEmbeddings::new(32);
        let mut doc = Document::new("with explicit id");
        doc.id = Some("explicit-id-001".to_owned());
        let ids = store.add_documents(&[doc], &embeddings).await.unwrap();
        assert_eq!(ids, vec!["explicit-id-001"]);
    }

    #[tokio::test]
    async fn similarity_search_returns_results() {
        let store = make_store(32).await;
        let embeddings = FakeEmbeddings::new(32);
        let docs = vec![
            Document::new("the cat sat on the mat"),
            Document::new("quantum mechanics and general relativity"),
            Document::new("the cat played with yarn"),
        ];
        let _ = store.add_documents(&docs, &embeddings).await.unwrap();
        let results = store
            .similarity_search("cat mat", 2, &embeddings)
            .await
            .unwrap();
        assert!(!results.is_empty());
        assert!(results.len() <= 2);
    }

    #[tokio::test]
    async fn similarity_search_with_score_returns_scores() {
        let store = make_store(32).await;
        let embeddings = FakeEmbeddings::new(32);
        let docs = vec![Document::new("alpha"), Document::new("beta")];
        let _ = store.add_documents(&docs, &embeddings).await.unwrap();
        let results = store
            .similarity_search_with_score("alpha", 2, &embeddings)
            .await
            .unwrap();
        assert!(!results.is_empty());
        // Scores are raw L2 distances, so they are non-negative.
        for (_, score) in &results {
            assert!(*score >= 0.0);
        }
    }

    #[tokio::test]
    async fn delete_removes_document() {
        let store = make_store(32).await;
        let embeddings = FakeEmbeddings::new(32);
        let docs = vec![Document::new("to be deleted")];
        let ids = store.add_documents(&docs, &embeddings).await.unwrap();
        store.delete(&ids).await.unwrap();
        // Verify the store does not panic with an empty results set.
        let results = store
            .similarity_search("to be deleted", 5, &embeddings)
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn metadata_roundtrips() {
        let store = make_store(32).await;
        let embeddings = FakeEmbeddings::new(32);
        let mut meta = HashMap::new();
        let _ = meta.insert(
            "source".to_owned(),
            serde_json::Value::String("test_file.txt".to_owned()),
        );
        let doc = Document::with_metadata("document with metadata", meta.clone());
        let ids = store.add_documents(&[doc], &embeddings).await.unwrap();
        let results = store
            .similarity_search("document with metadata", 1, &embeddings)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, Some(ids[0].clone()));
        assert_eq!(
            results[0].metadata.get("source"),
            Some(&serde_json::Value::String("test_file.txt".to_owned()))
        );
    }

    #[tokio::test]
    async fn add_empty_documents_returns_empty_ids() {
        let store = make_store(32).await;
        let embeddings = FakeEmbeddings::new(32);
        let ids = store.add_documents(&[], &embeddings).await.unwrap();
        assert!(ids.is_empty());
    }

    #[tokio::test]
    async fn delete_empty_slice_is_noop() {
        let store = make_store(32).await;
        store.delete(&[]).await.unwrap();
    }
}
