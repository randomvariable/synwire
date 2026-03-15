//! Property test: `InMemoryVectorStore` `similarity_search` returns at most k results.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;
use synwire_core::documents::Document;
use synwire_core::embeddings::FakeEmbeddings;
use synwire_core::vectorstores::{InMemoryVectorStore, VectorStore};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// similarity_search should never return more results than k.
    #[test]
    fn search_count_at_most_k(
        doc_count in 1usize..=10,
        k in 1usize..=10,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let store = InMemoryVectorStore::new();
            let embeddings = FakeEmbeddings::new(32);

            let docs: Vec<Document> = (0..doc_count)
                .map(|i| Document::new(format!("document number {i}")))
                .collect();
            let _ = store.add_documents(&docs, &embeddings).await.unwrap();

            let results = store
                .similarity_search("query", k, &embeddings)
                .await
                .unwrap();

            assert!(
                results.len() <= k,
                "expected at most {k} results, got {}",
                results.len()
            );
            assert!(
                results.len() <= doc_count,
                "cannot return more results than documents"
            );
        });
    }

    /// similarity_search_with_score results should be in descending score order.
    #[test]
    fn search_scores_descending(doc_count in 2usize..=8) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let store = InMemoryVectorStore::new();
            let embeddings = FakeEmbeddings::new(32);

            let docs: Vec<Document> = (0..doc_count)
                .map(|i| Document::new(format!("doc {i} with content")))
                .collect();
            let _ = store.add_documents(&docs, &embeddings).await.unwrap();

            let results = store
                .similarity_search_with_score("query", doc_count, &embeddings)
                .await
                .unwrap();

            for window in results.windows(2) {
                assert!(
                    window[0].1 >= window[1].1,
                    "scores should be descending: {} >= {}",
                    window[0].1,
                    window[1].1
                );
            }
        });
    }
}
