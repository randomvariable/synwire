//! # synwire-test-utils
//!
//! Test utilities, fake models, and proptest strategies for Synwire.
//!
//! Provides `FakeChatModel`, `FakeEmbeddings`, proptest `Strategy`
//! implementations for core types, and test fixture builders.

#![deny(unsafe_code)]

pub mod fixtures;
pub mod strategies;

// Re-export commonly used items for convenience in test code.
pub use strategies::channels;
pub use strategies::checkpoints;
pub use strategies::documents;
pub use strategies::embeddings;
pub use strategies::graphs;
pub use strategies::messages;
pub use strategies::prompts;
pub use strategies::tools;

pub use fixtures::builders::{
    DocumentBuilder, MessageBuilder, PromptTemplateBuilder, ToolSchemaBuilder,
};
