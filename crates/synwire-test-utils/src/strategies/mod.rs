//! Proptest strategies for Synwire types.
//!
//! Provides reusable strategies for generating arbitrary instances of core
//! types such as [`Message`](synwire_core::messages::Message),
//! [`Document`](synwire_core::documents::Document), and checkpoint data.

pub mod agents;
pub mod channels;
pub mod checkpoints;
pub mod documents;
pub mod embeddings;
pub mod graphs;
pub mod messages;
pub mod prompts;
pub mod tools;
