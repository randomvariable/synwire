//! # synwire-llm-openai
//!
//! `OpenAI` provider for Synwire.
//!
//! Provides [`ChatOpenAI`] (implementing [`BaseChatModel`](synwire_core::language_models::BaseChatModel)),
//! and [`BaseChatOpenAI`] as a shared base for `OpenAI`-compatible providers.

#![deny(unsafe_code)]

/// Shared base type for OpenAI-compatible providers.
pub mod base;
/// `ChatOpenAI` implementation.
pub mod chat;
/// `OpenAI` embeddings provider.
pub mod embeddings;
/// OpenAI-specific error types.
pub mod error;
/// `OpenAI` content moderation middleware.
pub mod moderation;

pub use base::BaseChatOpenAI;
pub use chat::{ChatOpenAI, ChatOpenAIBuilder};
pub use embeddings::{OpenAIEmbeddings, OpenAIEmbeddingsBuilder};
pub use error::OpenAIError;
