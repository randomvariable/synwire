//! Process tracking plugin, command execution tools, and shell session tools.
//!
//! [`ProcessPlugin`] contributes five management tools (`list_processes`,
//! `kill_process`, `process_stats`, `wait_for_process`, `read_process_output`).
//!
//! `CommandPlugin` contributes four execution tools (`run_command`,
//! `open_shell`, `shell_write`, `shell_read`) that spawn and interact with
//! sandboxed processes.

pub mod command_tools;
pub mod context;
pub mod expect_engine;
pub mod process_plugin;
pub mod tools;

pub use context::SandboxContext;
pub use process_plugin::{ProcessPlugin, ProcessPluginKey, ProcessPluginState};
