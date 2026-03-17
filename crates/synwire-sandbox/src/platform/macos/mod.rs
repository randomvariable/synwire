//! macOS sandbox backends.
//!
//! Dispatches between:
//! - [`seatbelt`] — `sandbox-exec` with a generated Seatbelt SBPL profile (light)
//! - [`container`] — Apple Container / Docker / Podman / Colima (strong)

pub mod container;
pub mod seatbelt;

pub use container::{ContainerRuntime, detect_container_runtime, spawn_with_runtime};
pub use seatbelt::SeatbeltProfile;
