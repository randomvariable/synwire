//! # synwire
//!
//! Convenience re-exports and reference implementations for Synwire.
//!
//! This crate provides ready-to-use implementations for common patterns:
//! chat history management, embedding cache, few-shot prompts,
//! text splitters, and additional output parsers.

#![deny(unsafe_code)]

pub use synwire_core as core;

/// Embedding cache backed by moka.
pub mod cache;

/// Chat message history traits and implementations.
pub mod chat_history;

/// Few-shot prompt templates and example selectors.
pub mod prompts;

/// Text splitter implementations for chunking documents.
pub mod text_splitters;

/// Additional output parser implementations.
pub mod output_parsers;
