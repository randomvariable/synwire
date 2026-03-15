//! Language model traits and types.

/// Batch processing trait for provider-level batch APIs.
#[cfg(feature = "batch-api")]
pub mod batch;
/// Fake model for testing.
pub mod fake;
/// Model profile registry.
pub mod registry;
/// Language model trait definitions.
pub mod traits;
mod types;

pub use fake::FakeChatModel;
pub use registry::{InMemoryModelProfileRegistry, ModelProfile, ModelProfileRegistry};
pub use traits::{BaseChatModel, BaseLLM};
pub use types::{ChatChunk, ChatResult, CostEstimate, Generation, LLMResult, ToolCallChunk};
