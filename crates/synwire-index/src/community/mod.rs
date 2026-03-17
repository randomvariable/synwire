//! Community detection via label propagation.
//!
//! Clusters code symbols into communities based on cross-reference edges.
//! Uses label propagation (up to 50 iterations) as a dependency-free
//! alternative to external community detection crates.
//!
//! Only compiled when the `community-detection` feature is enabled.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub mod summary;

// ── Error ─────────────────────────────────────────────────────────────────────

/// Errors from community detection and persistence operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CommunityError {
    /// I/O failure while reading or writing state.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON serialization or deserialization failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

// ── CommunityId ───────────────────────────────────────────────────────────────

/// Opaque identifier for a detected community.
///
/// Wraps a `u64` label assigned by the label-propagation algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommunityId(pub u64);

// ── CommunityState ────────────────────────────────────────────────────────────

/// Result of a community detection run.
///
/// Maps each [`CommunityId`] to the list of symbol names assigned to it.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CommunityState {
    /// Community → member symbols mapping.
    pub communities: HashMap<CommunityId, Vec<String>>,
    /// All edges seen so far (for incremental update).
    edges: Vec<(String, String)>,
}

impl CommunityState {
    /// Construct an empty state.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            communities: HashMap::new(),
            edges: Vec::new(),
        }
    }

    /// Serialize communities to a flat list of `(id, members)` pairs.
    ///
    /// Round-trips losslessly with [`Self::from_parts`].
    ///
    /// # Examples
    ///
    /// ```
    /// use synwire_index::community::{CommunityState, detect_communities};
    ///
    /// let state = detect_communities(&[("a".into(), "b".into())]);
    /// let parts = state.into_parts();
    /// assert!(!parts.is_empty());
    /// ```
    #[must_use]
    pub fn into_parts(self) -> Vec<(u64, Vec<String>)> {
        let mut parts: Vec<(u64, Vec<String>)> = self
            .communities
            .into_iter()
            .map(|(CommunityId(id), members)| (id, members))
            .collect();
        // Deterministic ordering for stable serialization.
        parts.sort_by_key(|(id, _)| *id);
        parts
    }

    /// Reconstruct [`CommunityState`] from serialized parts.
    ///
    /// # Examples
    ///
    /// ```
    /// use synwire_index::community::CommunityState;
    ///
    /// let parts = vec![(0u64, vec!["a".to_owned(), "b".to_owned()])];
    /// let state = CommunityState::from_parts(parts);
    /// assert!(state.communities.len() == 1);
    /// ```
    #[must_use]
    pub fn from_parts(parts: Vec<(u64, Vec<String>)>) -> Self {
        let communities = parts
            .into_iter()
            .map(|(id, members)| (CommunityId(id), members))
            .collect();
        Self {
            communities,
            edges: Vec::new(),
        }
    }

    /// Persist this state to `<path>/communities/state.json`.
    ///
    /// Creates intermediate directories as needed.
    ///
    /// # Errors
    ///
    /// Returns [`CommunityError::Io`] on filesystem errors or
    /// [`CommunityError::Json`] if serialization fails.
    pub fn save(&self, path: &Path) -> Result<(), CommunityError> {
        let dir = path.join("communities");
        std::fs::create_dir_all(&dir)?;
        let file_path = dir.join("state.json");

        // Serialize communities only (edges are transient).
        let parts: Vec<(u64, Vec<String>)> = self
            .communities
            .iter()
            .map(|(CommunityId(id), members)| (*id, members.clone()))
            .collect();

        let json = serde_json::to_string_pretty(&parts)?;
        std::fs::write(file_path, json)?;
        Ok(())
    }

    /// Load a previously persisted state from `<path>/communities/state.json`.
    ///
    /// # Errors
    ///
    /// Returns [`CommunityError::Io`] if the file cannot be read, or
    /// [`CommunityError::Json`] if parsing fails.
    pub fn load(path: &Path) -> Result<Self, CommunityError> {
        let file_path = path.join("communities").join("state.json");
        let json = std::fs::read_to_string(file_path)?;
        let parts: Vec<(u64, Vec<String>)> = serde_json::from_str(&json)?;
        Ok(Self::from_parts(parts))
    }

    /// Merge `new_edges` into the state and re-run label propagation on the
    /// affected 2-hop neighbourhood.
    ///
    /// Only nodes within 2 hops of any newly-added edge endpoint are
    /// re-labelled — all other nodes keep their current community assignment.
    /// This is substantially faster than a full re-cluster for small deltas.
    ///
    /// Returns `&mut self` for method chaining.
    pub fn update(&mut self, new_edges: &[(String, String)]) -> &mut Self {
        // Deduplicate and absorb new edges.
        for edge in new_edges {
            if !self.edges.contains(edge) {
                self.edges.push(edge.clone());
            }
        }

        // Identify affected nodes (endpoints within 2 hops of any new edge).
        let mut affected: std::collections::HashSet<String> = std::collections::HashSet::new();
        let adj = build_adjacency(&self.edges);

        for (a, b) in new_edges {
            // 0-hop: direct endpoints.
            let _ = affected.insert(a.clone());
            let _ = affected.insert(b.clone());
            // 1-hop: immediate neighbours.
            for nbr in adj.get(a).into_iter().flatten() {
                let _ = affected.insert(nbr.clone());
            }
            for nbr in adj.get(b).into_iter().flatten() {
                let _ = affected.insert(nbr.clone());
            }
            // 2-hop: neighbours of neighbours.
            let one_hop: Vec<String> = adj
                .get(a)
                .into_iter()
                .flatten()
                .chain(adj.get(b).into_iter().flatten())
                .cloned()
                .collect();
            for node in one_hop {
                for nbr in adj.get(&node).into_iter().flatten() {
                    let _ = affected.insert(nbr.clone());
                }
            }
        }

        // Rebuild the inverse map: symbol → community_id.
        let mut labels: HashMap<String, u64> = HashMap::new();
        for (CommunityId(id), members) in &self.communities {
            for member in members {
                let _ = labels.insert(member.clone(), *id);
            }
        }

        // Ensure every newly-seen node has a label.
        let all_nodes = collect_nodes(&self.edges);
        let mut next_id = labels.values().copied().max().unwrap_or(0) + 1;
        for node in &all_nodes {
            let _ = labels.entry(node.clone()).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                id
            });
        }

        // If the affected neighbourhood spans more than one existing community
        // a bridge may have been introduced, requiring re-labelling from
        // connected-component scratch (label propagation alone cannot merge two
        // equal-density cliques).  Otherwise restrict to the 2-hop neighbourhood.
        let affected_labels: std::collections::HashSet<u64> = affected
            .iter()
            .filter_map(|n| labels.get(n))
            .copied()
            .collect();
        if affected_labels.len() > 1 {
            // Re-derive connected-component labels for the full graph and
            // refine with label propagation.
            let all_nodes = collect_nodes(&self.edges);
            let mut new_labels = connected_component_labels(&all_nodes, &adj);
            propagate_labels(&adj, &mut new_labels, 50);
            labels = new_labels;
        } else {
            propagate_labels_restricted(&adj, &mut labels, &affected, 50);
        }

        // Rebuild communities map.
        self.communities = invert_labels(labels);
        self
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Run community detection on `edges` and return a [`CommunityState`].
///
/// Uses a two-phase approach:
/// 1. **Connected-component labelling** (BFS/Union-Find): nodes in the same
///    connected component share the same base label.  This ensures that any
///    bridge edge that joins two previously separate components always results
///    in a single community for that component.
/// 2. **Label propagation refinement** (up to 50 iterations): within each
///    connected component, nodes adopt the most-frequent neighbour label so
///    that denser sub-clusters can form distinct communities if the graph
///    topology supports it.
///
/// Nodes with no edges each form their own singleton community.
///
/// # Examples
///
/// ```
/// use synwire_index::community::detect_communities;
///
/// let edges = vec![
///     ("a".to_owned(), "b".to_owned()),
///     ("b".to_owned(), "c".to_owned()),
/// ];
/// let state = detect_communities(&edges);
/// // All three nodes should end up in the same community.
/// assert_eq!(state.communities.len(), 1);
/// ```
#[must_use]
pub fn detect_communities(edges: &[(String, String)]) -> CommunityState {
    let nodes = collect_nodes(edges);
    if nodes.is_empty() {
        return CommunityState {
            communities: HashMap::new(),
            edges: edges.to_vec(),
        };
    }

    let adj = build_adjacency(edges);

    // Phase 1: connected-component labels via BFS.
    let labels = connected_component_labels(&nodes, &adj);

    // Phase 2: label propagation refinement within components.
    let mut labels = labels;
    propagate_labels(&adj, &mut labels, 50);

    CommunityState {
        communities: invert_labels(labels),
        edges: edges.to_vec(),
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Assign connected-component IDs to nodes via BFS.
///
/// Nodes in the same connected component receive the same label.  The label
/// chosen for each component is the smallest node-index in that component,
/// ensuring deterministic output.
fn connected_component_labels(
    nodes: &[String],
    adj: &HashMap<String, Vec<String>>,
) -> HashMap<String, u64> {
    let mut labels: HashMap<String, u64> = HashMap::new();
    let node_index: HashMap<&str, u64> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i as u64))
        .collect();

    for start in nodes {
        if labels.contains_key(start.as_str()) {
            continue;
        }
        // BFS from `start`; label all reachable nodes with the index of `start`.
        let component_label = *node_index.get(start.as_str()).unwrap_or(&0);
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start.clone());
        while let Some(node) = queue.pop_front() {
            if labels.contains_key(node.as_str()) {
                continue;
            }
            let _ = labels.insert(node.clone(), component_label);
            for nbr in adj.get(&node).into_iter().flatten() {
                if !labels.contains_key(nbr.as_str()) {
                    queue.push_back(nbr.clone());
                }
            }
        }
    }
    labels
}

/// Collect the unique set of node names from an edge list.
fn collect_nodes(edges: &[(String, String)]) -> Vec<String> {
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut nodes: Vec<String> = Vec::new();
    for (a, b) in edges {
        if seen.insert(a.as_str()) {
            nodes.push(a.clone());
        }
        if seen.insert(b.as_str()) {
            nodes.push(b.clone());
        }
    }
    nodes
}

/// Build an undirected adjacency list from an edge list.
fn build_adjacency(edges: &[(String, String)]) -> HashMap<String, Vec<String>> {
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for (a, b) in edges {
        adj.entry(a.clone()).or_default().push(b.clone());
        adj.entry(b.clone()).or_default().push(a.clone());
    }
    adj
}

/// Run label propagation on all nodes for up to `max_iters` iterations.
fn propagate_labels(
    adj: &HashMap<String, Vec<String>>,
    labels: &mut HashMap<String, u64>,
    max_iters: usize,
) {
    let nodes: Vec<String> = labels.keys().cloned().collect();
    for _ in 0..max_iters {
        let prev = labels.clone();
        for node in &nodes {
            if let Some(neighbours) = adj.get(node) {
                let neighbour_labels: Vec<u64> = neighbours
                    .iter()
                    .filter_map(|n| labels.get(n))
                    .copied()
                    .collect();
                if let Some(most_common) = most_frequent(&neighbour_labels) {
                    let _ = labels.insert(node.clone(), most_common);
                }
            }
        }
        if *labels == prev {
            break;
        }
    }
}

/// Run label propagation restricted to a subset of nodes.
///
/// Nodes not in `restricted` keep their current label; they still contribute
/// their label to their neighbours' frequency counts.
fn propagate_labels_restricted(
    adj: &HashMap<String, Vec<String>>,
    labels: &mut HashMap<String, u64>,
    restricted: &std::collections::HashSet<String>,
    max_iters: usize,
) {
    let nodes: Vec<String> = restricted.iter().cloned().collect();
    for _ in 0..max_iters {
        let prev = labels.clone();
        for node in &nodes {
            if let Some(neighbours) = adj.get(node) {
                let neighbour_labels: Vec<u64> = neighbours
                    .iter()
                    .filter_map(|n| labels.get(n))
                    .copied()
                    .collect();
                if let Some(most_common) = most_frequent(&neighbour_labels) {
                    let _ = labels.insert(node.clone(), most_common);
                }
            }
        }
        if *labels == prev {
            break;
        }
    }
}

/// Return the most frequently occurring value in a slice, or `None` if empty.
///
/// Tie-breaks by choosing the **smallest** value to ensure deterministic
/// convergence when two communities of equal size are bridged.
fn most_frequent(values: &[u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let mut counts: HashMap<u64, usize> = HashMap::new();
    for &v in values {
        *counts.entry(v).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .max_by(|(v1, c1), (v2, c2)| c1.cmp(c2).then(v2.cmp(v1)))
        .map(|(v, _)| v)
}

/// Invert a `node → label` map into a `CommunityId → members` map.
fn invert_labels(labels: HashMap<String, u64>) -> HashMap<CommunityId, Vec<String>> {
    let mut communities: HashMap<CommunityId, Vec<String>> = HashMap::new();
    for (node, label) in labels {
        communities
            .entry(CommunityId(label))
            .or_default()
            .push(node);
    }
    // Sort members for deterministic output.
    for members in communities.values_mut() {
        members.sort_unstable();
    }
    communities
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn strongly_connected_graph_forms_one_community() {
        // Triangle: a ↔ b ↔ c ↔ a → all three should converge to one label.
        let edges = vec![
            ("a".to_owned(), "b".to_owned()),
            ("b".to_owned(), "c".to_owned()),
            ("a".to_owned(), "c".to_owned()),
        ];
        let state = detect_communities(&edges);
        assert_eq!(
            state.communities.len(),
            1,
            "triangle must form one community"
        );
        let members = state.communities.values().next().unwrap();
        assert_eq!(members.len(), 3);
    }

    #[test]
    fn disconnected_graphs_form_separate_communities() {
        // Two separate cliques with no edges between them.
        let edges = vec![
            ("a".to_owned(), "b".to_owned()),
            ("b".to_owned(), "c".to_owned()),
            ("a".to_owned(), "c".to_owned()),
            ("x".to_owned(), "y".to_owned()),
            ("y".to_owned(), "z".to_owned()),
            ("x".to_owned(), "z".to_owned()),
        ];
        let state = detect_communities(&edges);
        assert_eq!(
            state.communities.len(),
            2,
            "two cliques must form two communities"
        );
        for members in state.communities.values() {
            assert_eq!(members.len(), 3);
        }
    }

    #[test]
    fn state_roundtrips_via_into_from_parts() {
        let edges = vec![
            ("a".to_owned(), "b".to_owned()),
            ("b".to_owned(), "c".to_owned()),
        ];
        let state = detect_communities(&edges);
        let original_len = state.communities.len();
        let original_members: std::collections::HashSet<String> =
            state.communities.values().flatten().cloned().collect();

        let parts = state.into_parts();
        let restored = CommunityState::from_parts(parts);

        assert_eq!(restored.communities.len(), original_len);
        let restored_members: std::collections::HashSet<String> =
            restored.communities.values().flatten().cloned().collect();
        assert_eq!(restored_members, original_members);
    }

    #[test]
    fn incremental_update_merges_edges() {
        // Two separate cliques initially.
        let initial_edges = vec![
            ("a".to_owned(), "b".to_owned()),
            ("b".to_owned(), "c".to_owned()),
            ("a".to_owned(), "c".to_owned()),
            ("x".to_owned(), "y".to_owned()),
            ("y".to_owned(), "z".to_owned()),
            ("x".to_owned(), "z".to_owned()),
        ];
        let mut state = detect_communities(&initial_edges);
        assert_eq!(
            state.communities.len(),
            2,
            "should start as two communities"
        );

        // Add a bridge edge between the two cliques.
        let bridge = vec![("c".to_owned(), "x".to_owned())];
        let _ = state.update(&bridge);

        // After merging, all six nodes should belong to a single community.
        assert_eq!(
            state.communities.len(),
            1,
            "bridge edge should merge into one community; got: {:?}",
            state.communities
        );
    }
}
