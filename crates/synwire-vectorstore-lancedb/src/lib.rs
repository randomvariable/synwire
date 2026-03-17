//! `LanceDB`-backed vector store for Synwire semantic search.
//!
//! [`LanceDbVectorStore`] implements the `VectorStore` trait using `LanceDB`
//! for persistent vector storage and similarity search.
//!
//! # Example
//!
//! ```no_run
//! use synwire_vectorstore_lancedb::LanceDbVectorStore;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let store = LanceDbVectorStore::open("/tmp/my-db", "documents", 384).await?;
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]

mod error;
mod store;

pub use error::LanceDbError;
pub use store::LanceDbVectorStore;
