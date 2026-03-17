//! Storage for the code dependency graph.

use crate::graph::types::{Edge, EdgeKind, NodeId};
use std::collections::{HashMap, HashSet, VecDeque};

/// In-memory adjacency list storage for the code dependency graph.
///
/// For repositories at Linux kernel scale (1M+ edges), this can be
/// replaced with a SQLite-backed implementation.
#[derive(Debug, Default)]
pub struct GraphStorage {
    /// Forward adjacency: from -> [(to, kind, line)]
    forward: HashMap<NodeId, Vec<(NodeId, EdgeKind, u32)>>,
    /// Reverse adjacency: to -> [from] (for caller lookups)
    reverse: HashMap<NodeId, Vec<NodeId>>,
}

impl GraphStorage {
    /// Create an empty graph storage.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert an edge, deduplicating by (from, to, kind).
    pub fn insert_edge(&mut self, edge: &Edge) {
        let fwd = self.forward.entry(edge.from.clone()).or_default();
        // Deduplicate
        if !fwd
            .iter()
            .any(|(to, k, _)| *to == edge.to && *k == edge.kind)
        {
            fwd.push((edge.to.clone(), edge.kind, edge.line));
        }
        let rev = self.reverse.entry(edge.to.clone()).or_default();
        if !rev.contains(&edge.from) {
            rev.push(edge.from.clone());
        }
    }

    /// Remove all edges originating from the given file.
    pub fn remove_file_edges(&mut self, file: &str) {
        // Remove forward edges from this file's nodes.
        let to_remove: Vec<NodeId> = self
            .forward
            .keys()
            .filter(|n| n.file == file)
            .cloned()
            .collect();
        for node in &to_remove {
            if let Some(targets) = self.forward.remove(node) {
                // Clean up reverse edges.
                for (target, _, _) in &targets {
                    if let Some(revs) = self.reverse.get_mut(target) {
                        revs.retain(|n| n != node);
                    }
                }
            }
        }
        // Remove reverse entries whose `to` node is in this file.
        let rev_remove: Vec<NodeId> = self
            .reverse
            .keys()
            .filter(|n| n.file == file)
            .cloned()
            .collect();
        for node in &rev_remove {
            let _ = self.reverse.remove(node);
        }
    }

    /// Traverse outgoing edges up to `depth` hops from `root`.
    pub fn outgoing(&self, root: &NodeId, depth: u32) -> (Vec<NodeId>, Vec<Edge>) {
        self.bfs(root, depth, false)
    }

    /// Traverse incoming edges (callers) up to `depth` hops from `root`.
    pub fn incoming(&self, root: &NodeId, depth: u32) -> (Vec<NodeId>, Vec<Edge>) {
        self.bfs(root, depth, true)
    }

    fn bfs(&self, root: &NodeId, depth: u32, reverse: bool) -> (Vec<NodeId>, Vec<Edge>) {
        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut queue: VecDeque<(NodeId, u32)> = VecDeque::new();
        let mut result_nodes: Vec<NodeId> = Vec::new();
        let mut result_edges: Vec<Edge> = Vec::new();

        queue.push_back((root.clone(), 0));
        let _ = visited.insert(root.clone());

        while let Some((current, d)) = queue.pop_front() {
            result_nodes.push(current.clone());
            if d >= depth {
                continue;
            }
            if reverse {
                if let Some(froms) = self.reverse.get(&current) {
                    for from in froms {
                        if visited.insert(from.clone()) {
                            result_edges.push(Edge {
                                from: from.clone(),
                                to: current.clone(),
                                kind: EdgeKind::References,
                                line: 0,
                            });
                            queue.push_back((from.clone(), d + 1));
                        }
                    }
                }
            } else if let Some(targets) = self.forward.get(&current) {
                for (to, kind, line) in targets {
                    if visited.insert(to.clone()) {
                        result_edges.push(Edge {
                            from: current.clone(),
                            to: to.clone(),
                            kind: *kind,
                            line: *line,
                        });
                        queue.push_back((to.clone(), d + 1));
                    }
                }
            }
        }
        (result_nodes, result_edges)
    }

    /// Total number of edges in the graph.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.forward.values().map(Vec::len).sum()
    }
}
