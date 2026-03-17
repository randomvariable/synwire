//! Sandbox implementations for command execution, process management, and archive handling.
//!
//! These are distinct from VFS providers — they handle operations that go beyond
//! filesystem abstraction (shell execution, process lifecycle, archive manipulation).

pub mod archive;
pub mod pipeline;
pub mod process;
pub mod shell;
pub mod threshold_gate;

pub use archive::ArchiveManager;
pub use pipeline::PipelineExecutor;
pub use process::ProcessManager;
pub use shell::Shell;
pub use threshold_gate::ThresholdGate;
