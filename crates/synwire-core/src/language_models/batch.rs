//! Batch processing trait for provider-level batch APIs.
//!
//! This module is gated behind the `batch-api` feature flag.

use crate::BoxFuture;
use crate::error::SynwireError;

/// Status of a batch job.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BatchStatus {
    /// Batch has been submitted and is pending.
    Pending,
    /// Batch is currently being processed.
    InProgress,
    /// Batch completed successfully.
    Completed,
    /// Batch failed.
    Failed {
        /// Error message.
        message: String,
    },
}

/// Result of a single item in a batch.
#[derive(Debug)]
pub struct BatchItemResult<T> {
    /// Index of the item in the original batch.
    pub index: usize,
    /// Result for this item.
    pub result: Result<T, SynwireError>,
}

/// Trait for provider-level batch APIs (e.g., `OpenAI` Batch API).
///
/// Unlike [`BaseChatModel::batch`](crate::language_models::BaseChatModel::batch)
/// which sends requests sequentially, `BatchProcessor` submits work to
/// the provider's asynchronous batch endpoint for cost-efficient processing.
///
/// # Type Parameters
///
/// - `T`: The output type for each batch item.
pub trait BatchProcessor<T: Send>: Send + Sync {
    /// Submits a batch of inputs and returns a batch job ID.
    fn submit_batch(
        &self,
        inputs: Vec<serde_json::Value>,
    ) -> BoxFuture<'_, Result<String, SynwireError>>;

    /// Checks the status of a batch job.
    fn batch_status<'a>(
        &'a self,
        batch_id: &'a str,
    ) -> BoxFuture<'a, Result<BatchStatus, SynwireError>>;

    /// Retrieves results of a completed batch.
    fn get_batch_results<'a>(
        &'a self,
        batch_id: &'a str,
    ) -> BoxFuture<'a, Result<Vec<BatchItemResult<T>>, SynwireError>>;

    /// Cancels a pending or in-progress batch.
    fn cancel_batch<'a>(&'a self, batch_id: &'a str) -> BoxFuture<'a, Result<(), SynwireError>>;
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Verify the trait compiles and a default implementation is constructable.
    struct FakeBatchProcessor;

    impl BatchProcessor<String> for FakeBatchProcessor {
        fn submit_batch(
            &self,
            _inputs: Vec<serde_json::Value>,
        ) -> BoxFuture<'_, Result<String, SynwireError>> {
            Box::pin(async { Ok("batch-123".to_owned()) })
        }

        fn batch_status<'a>(
            &'a self,
            _batch_id: &'a str,
        ) -> BoxFuture<'a, Result<BatchStatus, SynwireError>> {
            Box::pin(async { Ok(BatchStatus::Completed) })
        }

        fn get_batch_results<'a>(
            &'a self,
            _batch_id: &'a str,
        ) -> BoxFuture<'a, Result<Vec<BatchItemResult<String>>, SynwireError>> {
            Box::pin(async {
                Ok(vec![BatchItemResult {
                    index: 0,
                    result: Ok("result".to_owned()),
                }])
            })
        }

        fn cancel_batch<'a>(
            &'a self,
            _batch_id: &'a str,
        ) -> BoxFuture<'a, Result<(), SynwireError>> {
            Box::pin(async { Ok(()) })
        }
    }

    #[tokio::test]
    async fn batch_processor_compiles_and_works() {
        let processor = FakeBatchProcessor;
        let batch_id = processor
            .submit_batch(vec![serde_json::json!("test")])
            .await
            .unwrap();
        assert_eq!(batch_id, "batch-123");

        let status = processor.batch_status(&batch_id).await.unwrap();
        assert_eq!(status, BatchStatus::Completed);

        let results = processor.get_batch_results(&batch_id).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].index, 0);

        let cancel = processor.cancel_batch(&batch_id).await;
        assert!(cancel.is_ok());
    }

    #[test]
    fn batch_processor_is_object_safe() {
        // Ensure trait can be used as dyn
        fn takes_processor(p: &dyn BatchProcessor<String>) {
            let _ = p;
        }
        let p = FakeBatchProcessor;
        takes_processor(&p);
    }
}
