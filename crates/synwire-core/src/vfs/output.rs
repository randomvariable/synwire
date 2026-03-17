//! Output serialization for VFS responses.
//!
//! VFS operations return Rust types that derive `Serialize`.  Before passing
//! them to an LLM, they must be serialized to a text format the model can
//! consume.  Two formats are supported:
//!
//! - **JSON** — standard `serde_json` serialization.
//! - **TOON** — [Token-Oriented Object Notation](https://github.com/toon-format/spec),
//!   a compact format that reduces token usage by 30–60% for tabular data.
//!
//! The format can be set as a default on the LLM provider and overridden
//! per-call.

use serde::Serialize;

/// Serialization format for VFS output returned to LLMs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum OutputFormat {
    /// Standard JSON (`serde_json`).
    #[default]
    Json,
    /// Compact JSON — no pretty-printing.
    JsonCompact,
    /// TOON — token-efficient format for LLM consumption.
    ///
    /// Requires the `toon` feature.  Falls back to `Json` if not enabled.
    Toon,
}

/// Serialize any `Serialize` value to a string in the given format.
///
/// # Errors
///
/// Returns an error string if serialization fails.
pub fn format_output<T: Serialize>(value: &T, format: OutputFormat) -> Result<String, String> {
    match format {
        OutputFormat::Json => serde_json::to_string_pretty(value).map_err(|e| e.to_string()),
        OutputFormat::JsonCompact => serde_json::to_string(value).map_err(|e| e.to_string()),
        OutputFormat::Toon => format_toon(value),
    }
}

#[cfg(feature = "toon")]
fn format_toon<T: Serialize>(value: &T) -> Result<String, String> {
    let json = serde_json::to_value(value).map_err(|e| e.to_string())?;
    Ok(toon::encode(&json, None))
}

#[cfg(not(feature = "toon"))]
fn format_toon<T: Serialize>(value: &T) -> Result<String, String> {
    // Fallback to pretty JSON when toon feature is not enabled.
    serde_json::to_string_pretty(value).map_err(|e| e.to_string())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::vfs::types::DirEntry;

    #[test]
    fn test_json_format() {
        let entry = DirEntry {
            name: "hello.txt".to_string(),
            path: "/hello.txt".to_string(),
            is_dir: false,
            size: Some(42),
            modified: None,
            permissions: None,
            is_symlink: false,
        };
        let out = format_output(&entry, OutputFormat::Json).expect("json");
        assert!(out.contains("hello.txt"));
        assert!(out.contains('\n')); // pretty-printed
    }

    #[test]
    fn test_json_compact_format() {
        let entry = DirEntry {
            name: "hello.txt".to_string(),
            path: "/hello.txt".to_string(),
            is_dir: false,
            size: Some(42),
            modified: None,
            permissions: None,
            is_symlink: false,
        };
        let out = format_output(&entry, OutputFormat::JsonCompact).expect("json compact");
        assert!(out.contains("hello.txt"));
        assert!(!out.contains('\n')); // not pretty-printed
    }

    #[cfg(feature = "toon")]
    #[test]
    fn test_toon_format() {
        let entries = vec![
            DirEntry {
                name: "a.rs".to_string(),
                path: "/a.rs".to_string(),
                is_dir: false,
                size: Some(100),
                modified: None,
                permissions: None,
                is_symlink: false,
            },
            DirEntry {
                name: "b.rs".to_string(),
                path: "/b.rs".to_string(),
                is_dir: false,
                size: Some(200),
                modified: None,
                permissions: None,
                is_symlink: false,
            },
        ];
        let out = format_output(&entries, OutputFormat::Toon).expect("toon");
        // TOON should be more compact than JSON for uniform arrays.
        let json = format_output(&entries, OutputFormat::Json).expect("json");
        assert!(
            out.len() <= json.len(),
            "TOON should be <= JSON for tabular data"
        );
    }
}
