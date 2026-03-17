//! Conformance test harnesses for core traits.
//!
//! Call these functions from `#[tokio::test]` blocks to validate that an
//! implementation satisfies the trait contract.

pub mod session;
pub mod vfs;
