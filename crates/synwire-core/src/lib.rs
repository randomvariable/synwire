//! # synwire-core
//!
//! Core traits and types for the Synwire AI framework -- a Rust port of
//! `LangChain`'s core abstractions.
//!
//! This crate provides the foundational trait hierarchy for chat models,
//! embeddings, vector stores, runnables, tools, callbacks, output parsers,
//! retrievers, credentials, and security primitives.
//!
//! All I/O operations are async-first. Core crate compiles with zero `unsafe`.

#![forbid(unsafe_code)]

use std::future::Future;
use std::pin::Pin;

/// A boxed future that is `Send`.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A boxed stream that is `Send`.
pub type BoxStream<'a, T> = Pin<Box<dyn futures_core::Stream<Item = T> + Send + 'a>>;

pub mod agents;
pub mod callbacks;
pub mod credentials;
pub mod documents;
pub mod embeddings;
pub mod error;
pub mod language_models;
pub mod loaders;
pub mod messages;
pub mod output_parsers;
pub mod prompts;
pub mod rerankers;
pub mod retrievers;
pub mod runnables;
pub mod security;
pub mod tools;
pub mod vectorstores;

pub mod observability;

pub mod prelude;
