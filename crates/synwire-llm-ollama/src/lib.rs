//! # synwire-llm-ollama
//!
//! Ollama provider for Synwire.
//!
//! Provides [`ChatOllama`] (implementing `BaseChatModel`) and
//! [`OllamaEmbeddings`] (implementing `Embeddings`) for local
//! LLM inference via the Ollama API.
//!
//! # Examples
//!
//! ```no_run
//! use synwire_llm_ollama::ChatOllama;
//!
//! let model = ChatOllama::builder()
//!     .model("llama3.2")
//!     .build()
//!     .unwrap();
//! ```

#![deny(unsafe_code)]

pub mod chat;
pub mod embeddings;
pub mod error;

pub use chat::{ChatOllama, ChatOllamaBuilder};
pub use embeddings::{OllamaEmbeddings, OllamaEmbeddingsBuilder};
pub use error::OllamaError;
