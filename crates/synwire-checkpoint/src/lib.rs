//! # synwire-checkpoint
//!
//! Checkpoint traits and in-memory implementation for Synwire.
//!
//! Provides [`BaseCheckpointSaver`](base::BaseCheckpointSaver) for persisting graph state,
//! [`BaseStore`](store::base::BaseStore) for key-value storage,
//! and [`InMemoryCheckpointSaver`](memory::InMemoryCheckpointSaver) as the default implementation.

#![deny(unsafe_code)]

pub mod base;
pub mod memory;
pub mod serde;
pub mod store;
pub mod types;
