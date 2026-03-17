//! Call graph node and edge types with cycle detection.

use std::collections::{HashMap, HashSet};

/// A node in the call graph.
#[non_exhaustive]
pub struct CallNode {
    /// Qualified symbol name.
    pub name: String,
    /// File containing this symbol.
    pub file: String,
    /// Line number.
    pub line: u32,
}

/// A directed call graph built incrementally via LSP goto-definition requests.
pub struct DynamicCallGraph {
    edges: Vec<(String, String)>,
}

impl DynamicCallGraph {
    /// Create an empty call graph.
    pub const fn new() -> Self {
        Self { edges: Vec::new() }
    }

    /// Add a caller -> callee edge.
    #[allow(clippy::similar_names)]
    pub fn add_edge(&mut self, caller: &str, callee: &str) {
        self.edges.push((caller.to_owned(), callee.to_owned()));
    }

    /// Get callees of a given caller (direct calls).
    pub fn callees(&self, caller: &str) -> Vec<&str> {
        self.edges
            .iter()
            .filter(|(c, _)| c == caller)
            .map(|(_, callee)| callee.as_str())
            .collect()
    }

    /// Get callers of a given callee.
    pub fn callers(&self, callee: &str) -> Vec<&str> {
        self.edges
            .iter()
            .filter(|(_, t)| t == callee)
            .map(|(caller, _)| caller.as_str())
            .collect()
    }

    /// Detect cycles using depth-first search.
    ///
    /// Returns `true` if the graph contains at least one cycle.
    pub fn has_cycle(&self) -> bool {
        fn dfs<'a>(
            node: &'a str,
            adj: &HashMap<&'a str, Vec<&'a str>>,
            visited: &mut HashSet<&'a str>,
            in_stack: &mut HashSet<&'a str>,
        ) -> bool {
            if in_stack.contains(node) {
                return true;
            }
            if visited.contains(node) {
                return false;
            }
            let _ = visited.insert(node);
            let _ = in_stack.insert(node);
            if let Some(neighbors) = adj.get(node) {
                for &neighbor in neighbors {
                    if dfs(neighbor, adj, visited, in_stack) {
                        return true;
                    }
                }
            }
            let _ = in_stack.remove(node);
            false
        }

        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for (from, to) in &self.edges {
            adj.entry(from.as_str()).or_default().push(to.as_str());
        }

        let mut visited: HashSet<&str> = HashSet::new();
        let mut in_stack: HashSet<&str> = HashSet::new();

        let all_nodes: Vec<&str> = adj.keys().copied().collect();
        for node in all_nodes {
            if dfs(node, &adj, &mut visited, &mut in_stack) {
                return true;
            }
        }
        false
    }
}

impl Default for DynamicCallGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_detection_finds_cycle() {
        let mut g = DynamicCallGraph::new();
        g.add_edge("a", "b");
        g.add_edge("b", "c");
        g.add_edge("c", "a"); // cycle
        assert!(g.has_cycle());
    }

    #[test]
    fn cycle_detection_no_false_positive() {
        let mut g = DynamicCallGraph::new();
        g.add_edge("a", "b");
        g.add_edge("b", "c");
        assert!(!g.has_cycle());
    }

    #[test]
    fn callees_and_callers() {
        let mut g = DynamicCallGraph::new();
        g.add_edge("main", "parse");
        g.add_edge("main", "execute");
        assert_eq!(g.callees("main").len(), 2);
        assert_eq!(g.callers("parse"), vec!["main"]);
    }
}
