//! Linux sandbox backends.
//!
//! Dispatches between:
//! - [`cgroup`] — cgroup v2 resource tracking + optional `AppArmor` enforcement
//! - [`namespace`] — full namespace isolation via OCI runtime (runc/crun)

pub mod cgroup;
pub mod namespace;

pub use cgroup::CgroupV2Manager;
pub use namespace::NamespaceContainer;
