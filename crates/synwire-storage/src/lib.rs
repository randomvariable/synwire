//! Configurable persistent storage layout, project identity, and migration for Synwire.
//!
//! ## Overview
//!
//! This crate provides three core abstractions:
//!
//! - **[`StorageLayout`]**: computes all Synwire storage paths for a given
//!   product name, respecting the platform data/cache directory conventions
//!   (XDG on Linux, `~/Library/…` on macOS, `%APPDATA%` on Windows).
//!
//! - **[`RepoId`] + [`WorktreeId`]**: stable two-level project identity.
//!   `RepoId` is derived from the Git first-commit hash (shared across clones
//!   and worktrees).  `WorktreeId` further discriminates by worktree root path.
//!
//! - **[`StorageMigration`]**: per-subsystem schema version tracking and
//!   incremental copy-then-swap migrations.
//!
//! - **[`ProjectRegistry`]**: global registry of indexed projects with
//!   last-access timestamps and user tags.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use synwire_storage::{StorageLayout, WorktreeId};
//! use std::path::Path;
//!
//! let layout = StorageLayout::new("synwire").expect("storage layout");
//! let worktree = WorktreeId::for_path(Path::new(".")).expect("worktree id");
//!
//! let index_path = layout.index_cache(&worktree);
//! println!("Index cache: {}", index_path.display());
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod concurrency;
pub mod dependency_index;
pub mod error;
pub mod identity;
pub mod layout;
pub mod migration;
pub mod registry;

pub use concurrency::{atomic_write, ensure_wal_mode, open_wal_database};
pub use dependency_index::{DependencyEntry, DependencyIndex, DependencyIndexError};
pub use error::StorageError;
pub use identity::{RepoId, WorktreeId};
pub use layout::{StorageConfig, StorageLayout};
pub use migration::{MigrationStep, NoOpMigrationStep, StorageMigration, VersionFile};
pub use registry::{ProjectRegistry, RegistryEntry};
