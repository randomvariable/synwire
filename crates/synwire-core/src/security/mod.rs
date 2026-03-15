//! Security primitives including SSRF protection and path traversal guards.

mod path;

pub use path::validate_tool_path;
