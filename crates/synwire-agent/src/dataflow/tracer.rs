//! Heuristic dataflow tracer using assignment pattern matching.

/// A dataflow origin: where a value comes from.
#[non_exhaustive]
pub struct DataflowOrigin {
    /// Source file.
    pub file: String,
    /// Line number (1-based).
    pub line: u32,
    /// Kind of origin: `"definition"`, `"assignment"`, `"parameter"`, or `"return"`.
    pub kind: String,
    /// Code snippet at this origin.
    pub snippet: String,
}

/// A hop in the dataflow trace.
#[non_exhaustive]
pub struct DataflowHop {
    /// Origin at this hop.
    pub origin: DataflowOrigin,
    /// Hop number (0 = immediate assignment site).
    pub depth: u32,
}

/// Traces data flow for a variable up to `max_hops` hops backward.
///
/// Uses heuristic pattern matching (assignment operators, `let` bindings).
/// For a full implementation, integrate with LSP type inference.
pub struct DataflowTracer {
    /// Maximum number of backward hops to trace.
    pub max_hops: u32,
}

impl DataflowTracer {
    /// Create a tracer with the given hop limit.
    pub const fn new(max_hops: u32) -> Self {
        Self { max_hops }
    }

    /// Trace dataflow for a variable in the given source text.
    ///
    /// Returns origins found within [`DataflowTracer::max_hops`] backward hops.
    ///
    /// # Examples
    ///
    /// ```
    /// use synwire_agent::dataflow::DataflowTracer;
    /// let tracer = DataflowTracer::new(5);
    /// let source = "let x = 5;\nlet y = x + 1;\nx = compute();\n";
    /// let hops = tracer.trace(source, "x", "test.rs");
    /// assert!(!hops.is_empty());
    /// ```
    pub fn trace(&self, source: &str, variable: &str, file: &str) -> Vec<DataflowHop> {
        let mut hops = Vec::new();
        let assign_pattern1 = format!("{variable} =");
        let assign_pattern2 = format!("{variable}=");
        let let_pattern = format!("let {variable}");

        for (line_idx, line) in source.lines().enumerate() {
            if line.contains(&assign_pattern1)
                || line.contains(&assign_pattern2)
                || line.contains(&let_pattern)
            {
                let kind = if line.contains("let ") {
                    "definition".to_owned()
                } else {
                    "assignment".to_owned()
                };
                hops.push(DataflowHop {
                    origin: DataflowOrigin {
                        file: file.to_owned(),
                        line: u32::try_from(line_idx + 1).unwrap_or(u32::MAX),
                        kind,
                        snippet: line.trim().to_owned(),
                    },
                    depth: 0,
                });
            }
            if hops.len() >= self.max_hops as usize {
                break;
            }
        }
        hops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dataflow_finds_assignment() {
        let tracer = DataflowTracer::new(5);
        let source = "let x = 5;\nlet y = x + 1;\nx = compute();\n";
        let hops = tracer.trace(source, "x", "test.rs");
        assert!(!hops.is_empty());
        assert!(hops.iter().any(|h| h.origin.kind == "definition"));
    }

    #[test]
    fn dataflow_respects_max_hops() {
        let tracer = DataflowTracer::new(2);
        let source = "x = 1;\nx = 2;\nx = 3;\nx = 4;\n";
        let hops = tracer.trace(source, "x", "test.rs");
        assert!(hops.len() <= 2);
    }

    #[test]
    fn dataflow_no_match_returns_empty() {
        let tracer = DataflowTracer::new(5);
        let source = "let y = 10;\nz = 20;\n";
        let hops = tracer.trace(source, "nonexistent_var", "test.rs");
        assert!(hops.is_empty());
    }
}
