//! Session management implementations.

pub mod manager;
pub mod mounted_repos;

pub use manager::InMemorySessionManager;
pub use mounted_repos::MountedRepo;
