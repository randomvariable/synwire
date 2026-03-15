//! # synwire-checkpoint-sqlite
//!
//! `SQLite` checkpoint backend for Synwire.
//!
//! Provides [`SqliteSaver`](saver::SqliteSaver) implementing
//! [`BaseCheckpointSaver`](synwire_checkpoint::base::BaseCheckpointSaver)
//! with `SQLite` persistence, using mode 0600 file permissions
//! and configurable `max_checkpoint_size`.

#![deny(unsafe_code)]

pub mod saver;
pub mod schema;
