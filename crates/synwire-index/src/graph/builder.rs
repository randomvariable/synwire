//! Builds the code dependency graph from source files.

use crate::graph::{
    storage::GraphStorage,
    types::{Edge, EdgeKind, NodeId},
};

/// Extracts edges from a source file using simple pattern matching.
///
/// A full implementation would use tree-sitter ASTs for accurate extraction.
/// This implementation uses line-by-line heuristics for the common cases.
#[derive(Debug, Default)]
pub struct GraphBuilder;

impl GraphBuilder {
    /// Create a new builder.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Extract edges from `file` (relative path) with `content`.
    /// Inserts them into `storage`.
    pub fn extract(&self, file: &str, content: &str, storage: &mut GraphStorage) {
        // Remove old edges for this file first (incremental update).
        storage.remove_file_edges(file);

        let file_node = NodeId {
            file: file.to_owned(),
            symbol: String::new(),
        };

        for (lineno, line) in content.lines().enumerate() {
            let line = line.trim();
            let line_num = u32::try_from(lineno + 1).unwrap_or(u32::MAX);

            // Rust `use` / Python `import` / JS `import`
            if let Some(imported) = extract_import(line) {
                let target = NodeId {
                    file: imported,
                    symbol: String::new(),
                };
                storage.insert_edge(&Edge {
                    from: file_node.clone(),
                    to: target,
                    kind: EdgeKind::Imports,
                    line: line_num,
                });
            }
        }
    }
}

fn extract_import(line: &str) -> Option<String> {
    // Rust: `use foo::bar;`
    if line.starts_with("use ") && line.ends_with(';') {
        let inner = line.trim_start_matches("use ").trim_end_matches(';');
        return Some(inner.replace("::", "/"));
    }
    // Python: `import foo` or `from foo import bar`
    if line.starts_with("import ") {
        return Some(
            line.trim_start_matches("import ")
                .split_whitespace()
                .next()?
                .to_owned(),
        );
    }
    if line.starts_with("from ") && line.contains(" import ") {
        let module = line.trim_start_matches("from ").split(" import ").next()?;
        return Some(module.to_owned());
    }
    // JS/TS: `import ... from 'foo'`
    if line.starts_with("import ") && line.contains("from '") {
        let after = line.split("from '").nth(1)?;
        let module = after.trim_end_matches("';").trim_end_matches('\'');
        return Some(module.to_owned());
    }
    None
}
