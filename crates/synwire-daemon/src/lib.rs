//! Synwire daemon library crate.
//!
//! Provides the daemon lifecycle logic (PID file, Unix domain socket, grace
//! period, signal handling) as testable modules. The binary entrypoint lives
//! in `main.rs` and delegates to this library.

#![forbid(unsafe_code)]

pub mod indexing;
pub mod ipc;
pub mod lifecycle;
pub mod manager;

pub use manager::{ManagerError, RepoManager, WorktreeHandle, WorktreeStatus};
