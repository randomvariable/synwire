//! Cross-project symbol reference graph.
//!
//! During indexing, resolves imports against locally-indexed projects'
//! symbol tables to produce inter-project edges.  Stored in
//! `StorageLayout.global_dependency_db()` (shared `SQLite`).

/// A directed edge in the cross-project reference graph.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct XrefEdge {
    /// Source project root.
    pub source_project: String,
    /// Source symbol (qualified name).
    pub source_symbol: String,
    /// Target project root.
    pub target_project: String,
    /// Target symbol.
    pub target_symbol: String,
    /// Whether this edge is stale (source or target re-indexed).
    pub is_stale: bool,
}

impl XrefEdge {
    /// Construct a non-stale [`XrefEdge`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use synwire_index::XrefEdge;
    ///
    /// let edge = XrefEdge::new("proj_a", "proj_a::Foo", "proj_b", "proj_b::Bar");
    /// assert!(!edge.is_stale);
    /// ```
    #[must_use]
    pub fn new(
        source_project: impl Into<String>,
        source_symbol: impl Into<String>,
        target_project: impl Into<String>,
        target_symbol: impl Into<String>,
    ) -> Self {
        Self {
            source_project: source_project.into(),
            source_symbol: source_symbol.into(),
            target_project: target_project.into(),
            target_symbol: target_symbol.into(),
            is_stale: false,
        }
    }
}

/// Direction for [`XrefGraph::xref_query`] lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum XrefDirection {
    /// References made by the symbol (outgoing edges where `source_symbol` matches).
    Outgoing,
    /// References to the symbol (incoming edges where `target_symbol` matches).
    Incoming,
    /// Both outgoing and incoming edges.
    Both,
}

/// In-memory cross-project symbol reference graph.
///
/// Edges can be added incrementally; stale edges are marked and pruned when a
/// project is re-indexed via [`rebuild_project_xrefs`].
pub struct XrefGraph {
    edges: Vec<XrefEdge>,
}

impl XrefGraph {
    /// Create a new, empty cross-project reference graph.
    #[must_use]
    pub const fn new() -> Self {
        Self { edges: Vec::new() }
    }

    /// Add a cross-project reference edge.
    pub fn add_edge(&mut self, edge: XrefEdge) {
        self.edges.push(edge);
    }

    /// Mark all edges whose `source_project` or `target_project` matches
    /// `project` as stale.
    pub fn mark_stale(&mut self, project: &str) {
        for edge in &mut self.edges {
            if edge.source_project == project || edge.target_project == project {
                edge.is_stale = true;
            }
        }
    }

    /// Remove all edges that have been marked stale.
    pub fn prune_stale(&mut self) {
        self.edges.retain(|e| !e.is_stale);
    }

    /// Query cross-project references for a symbol.
    ///
    /// Only non-stale edges are returned.
    #[must_use]
    pub fn xref_query(&self, symbol: &str, direction: XrefDirection) -> Vec<&XrefEdge> {
        self.edges
            .iter()
            .filter(|e| !e.is_stale)
            .filter(|e| match direction {
                XrefDirection::Outgoing => e.source_symbol == symbol,
                XrefDirection::Incoming => e.target_symbol == symbol,
                XrefDirection::Both => e.source_symbol == symbol || e.target_symbol == symbol,
            })
            .collect()
    }

    /// Return the total number of edges (including stale edges).
    #[must_use]
    pub fn len(&self) -> usize {
        self.edges.len()
    }

    /// Return `true` if the graph contains no edges.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}

impl Default for XrefGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Query cross-project references using a shared xref graph.
///
/// This is the main entry point for xref lookup from external crates.
///
/// # Example
///
/// ```rust
/// use synwire_index::{XrefGraph, XrefEdge, XrefDirection, xref_query};
///
/// let mut graph = XrefGraph::new();
/// graph.add_edge(XrefEdge::new("proj_a", "proj_a::Foo", "proj_b", "proj_b::Bar"));
///
/// let results = xref_query(&graph, "proj_b::Bar", XrefDirection::Incoming);
/// assert_eq!(results.len(), 1);
/// ```
pub fn xref_query<'a>(
    graph: &'a XrefGraph,
    symbol: &str,
    direction: XrefDirection,
) -> Vec<&'a XrefEdge> {
    graph.xref_query(symbol, direction)
}

/// Invalidate and rebuild xrefs for a given project.
///
/// Marks all edges from or to `project` as stale, prunes them, then inserts
/// `new_edges`.  Returns the number of new edges added.
///
/// # Example
///
/// ```rust
/// use synwire_index::{XrefGraph, XrefEdge, XrefDirection, rebuild_project_xrefs};
///
/// let mut graph = XrefGraph::new();
/// graph.add_edge(XrefEdge::new("proj_a", "fn_old", "proj_b", "fn_b"));
///
/// let new_edges = vec![XrefEdge::new("proj_a", "fn_new", "proj_b", "fn_b")];
/// let added = rebuild_project_xrefs(&mut graph, "proj_a", new_edges);
/// assert_eq!(added, 1);
/// assert!(graph.xref_query("fn_old", XrefDirection::Outgoing).is_empty());
/// ```
pub fn rebuild_project_xrefs(
    graph: &mut XrefGraph,
    project: &str,
    new_edges: Vec<XrefEdge>,
) -> usize {
    graph.mark_stale(project);
    graph.prune_stale();
    let count = new_edges.len();
    for edge in new_edges {
        graph.add_edge(edge);
    }
    count
}

// Ensure the public API is `Send + Sync`.
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    const fn check() {
        assert_send_sync::<XrefGraph>();
    }
    let _ = check;
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xref_query_finds_cross_project_callers() {
        let mut graph = XrefGraph::new();
        graph.add_edge(XrefEdge {
            source_project: "/home/user/proj-a".to_owned(),
            source_symbol: "proj_a::fetch_user".to_owned(),
            target_project: "/home/user/proj-b".to_owned(),
            target_symbol: "proj_b::User".to_owned(),
            is_stale: false,
        });

        let results = graph.xref_query("proj_b::User", XrefDirection::Incoming);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source_symbol, "proj_a::fetch_user");
    }

    #[test]
    fn incremental_xref_matches_full() {
        let mut graph = XrefGraph::new();
        graph.add_edge(XrefEdge {
            source_project: "proj_a".to_owned(),
            source_symbol: "fn_old".to_owned(),
            target_project: "proj_b".to_owned(),
            target_symbol: "fn_b".to_owned(),
            is_stale: false,
        });

        let new_edges = vec![XrefEdge {
            source_project: "proj_a".to_owned(),
            source_symbol: "fn_new".to_owned(),
            target_project: "proj_b".to_owned(),
            target_symbol: "fn_b".to_owned(),
            is_stale: false,
        }];
        let added = rebuild_project_xrefs(&mut graph, "proj_a", new_edges);

        assert_eq!(added, 1);
        assert!(
            graph
                .xref_query("fn_old", XrefDirection::Outgoing)
                .is_empty()
        );
        assert!(
            !graph
                .xref_query("fn_new", XrefDirection::Outgoing)
                .is_empty()
        );
    }

    #[test]
    fn stale_edges_excluded_from_query() {
        let mut graph = XrefGraph::new();
        graph.add_edge(XrefEdge {
            source_project: "proj_a".to_owned(),
            source_symbol: "sym_a".to_owned(),
            target_project: "proj_b".to_owned(),
            target_symbol: "sym_b".to_owned(),
            is_stale: false,
        });
        graph.mark_stale("proj_a");

        let results = graph.xref_query("sym_a", XrefDirection::Outgoing);
        assert!(results.is_empty());
    }

    #[test]
    fn prune_stale_removes_only_stale() {
        let mut graph = XrefGraph::new();
        graph.add_edge(XrefEdge {
            source_project: "proj_a".to_owned(),
            source_symbol: "sym_a".to_owned(),
            target_project: "proj_b".to_owned(),
            target_symbol: "sym_b".to_owned(),
            is_stale: false,
        });
        graph.add_edge(XrefEdge {
            source_project: "proj_c".to_owned(),
            source_symbol: "sym_c".to_owned(),
            target_project: "proj_b".to_owned(),
            target_symbol: "sym_b".to_owned(),
            is_stale: false,
        });
        graph.mark_stale("proj_a");
        graph.prune_stale();

        assert_eq!(graph.len(), 1);
        assert!(
            !graph
                .xref_query("sym_c", XrefDirection::Outgoing)
                .is_empty()
        );
    }

    #[test]
    fn both_direction_returns_outgoing_and_incoming() {
        let mut graph = XrefGraph::new();
        graph.add_edge(XrefEdge {
            source_project: "a".to_owned(),
            source_symbol: "target_sym".to_owned(),
            target_project: "b".to_owned(),
            target_symbol: "other".to_owned(),
            is_stale: false,
        });
        graph.add_edge(XrefEdge {
            source_project: "c".to_owned(),
            source_symbol: "caller".to_owned(),
            target_project: "d".to_owned(),
            target_symbol: "target_sym".to_owned(),
            is_stale: false,
        });

        let results = graph.xref_query("target_sym", XrefDirection::Both);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn standalone_xref_query_fn_delegates_to_method() {
        let mut graph = XrefGraph::new();
        graph.add_edge(XrefEdge {
            source_project: "a".to_owned(),
            source_symbol: "sym".to_owned(),
            target_project: "b".to_owned(),
            target_symbol: "tgt".to_owned(),
            is_stale: false,
        });

        let via_fn = xref_query(&graph, "sym", XrefDirection::Outgoing);
        let via_method = graph.xref_query("sym", XrefDirection::Outgoing);
        assert_eq!(via_fn.len(), via_method.len());
    }
}
