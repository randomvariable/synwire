//! Property test: Resources are properly cleaned up after vector store operations.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;
use synwire_core::documents::Document;
use synwire_core::embeddings::FakeEmbeddings;
use synwire_core::vectorstores::{InMemoryVectorStore, VectorStore};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// After deleting all documents from a vector store, similarity search
    /// should return an empty result set.
    #[test]
    fn deleted_documents_not_searchable(doc_count in 1usize..=8) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let store = InMemoryVectorStore::new();
            let embeddings = FakeEmbeddings::new(32);

            let docs: Vec<Document> = (0..doc_count)
                .map(|i| Document::new(format!("resource cleanup test doc {i}")))
                .collect();
            let ids = store.add_documents(&docs, &embeddings).await.unwrap();

            // Delete all documents.
            store.delete(&ids).await.unwrap();

            // Search should return nothing.
            let results = store
                .similarity_search("resource cleanup", 100, &embeddings)
                .await
                .unwrap();
            assert!(
                results.is_empty(),
                "expected no results after deleting all docs, got {}",
                results.len()
            );
        });
    }

    /// Deleting non-existent IDs should not cause an error.
    #[test]
    fn delete_nonexistent_is_noop(fake_id in "[a-z0-9]{8,16}") {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let store = InMemoryVectorStore::new();
            let result = store.delete(&[fake_id]).await;
            assert!(result.is_ok(), "deleting non-existent ID should not error");
        });
    }
}
