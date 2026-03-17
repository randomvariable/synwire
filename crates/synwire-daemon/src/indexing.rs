//! Daemon-side indexing coordinator.
//!
//! Wires the [`DependencyIndex`] (from `synwire-storage`) and the
//! [`XrefGraph`] (from `synwire-index`) into a single coordinator that the
//! daemon uses to manage global dependency tracking and cross-project
//! symbol references.

#![forbid(unsafe_code)]

use std::path::Path;
use std::sync::Mutex;

use synwire_index::{XrefDirection, XrefEdge, XrefGraph, rebuild_project_xrefs, xref_query};
use synwire_storage::{
    DependencyEntry, DependencyIndex, DependencyIndexError, StorageLayout, WorktreeId,
};

/// Errors produced by [`IndexingCoordinator`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum IndexingError {
    /// An error from the underlying storage layer.
    #[error("storage error: {0}")]
    Storage(#[from] synwire_storage::StorageError),
    /// An error from the dependency index database.
    #[error("dependency index error: {0}")]
    DependencyIndex(#[from] DependencyIndexError),
    /// An error from the cross-project reference graph.
    #[error("xref error: {0}")]
    Xref(String),
    /// An I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Daemon-side coordinator for global dependency indexing and cross-project
/// symbol reference tracking.
///
/// Owns a [`DependencyIndex`] (backed by `SQLite` at
/// `StorageLayout::global_dependency_db()`) and an in-memory [`XrefGraph`]
/// for cross-project symbol edges.
///
/// # Thread safety
///
/// `IndexingCoordinator` is `Send + Sync` -- the [`DependencyIndex`] uses an
/// internal `Mutex<Connection>` and the [`XrefGraph`] is wrapped in a
/// `Mutex` here.
pub struct IndexingCoordinator {
    /// The storage layout used to resolve paths.
    layout: StorageLayout,
    /// Global cross-project dependency index.
    dep_index: DependencyIndex,
    /// In-memory cross-project symbol reference graph.
    xref_graph: Mutex<XrefGraph>,
}

impl IndexingCoordinator {
    /// Create a new indexing coordinator.
    ///
    /// Opens (or creates) the dependency index database at
    /// `layout.global_dependency_db()` and initialises an empty cross-project
    /// reference graph.
    ///
    /// # Errors
    ///
    /// Returns [`IndexingError::DependencyIndex`] if the database cannot be
    /// opened or initialised.
    pub fn new(layout: &StorageLayout) -> Result<Self, IndexingError> {
        let dep_db_path = layout.global_dependency_db();
        let dep_index = DependencyIndex::open(&dep_db_path)?;

        Ok(Self {
            layout: layout.clone(),
            dep_index,
            xref_graph: Mutex::new(XrefGraph::new()),
        })
    }

    /// Parse project manifests and index their dependencies into the global
    /// dependency database.
    ///
    /// Detects the manifest type (`Cargo.toml`, `go.mod`, `package.json`,
    /// `pyproject.toml`) from files present in `project_root` and inserts all
    /// discovered dependencies.  Returns the count of dependencies indexed.
    ///
    /// # Errors
    ///
    /// Returns [`IndexingError::DependencyIndex`] if the manifest cannot be
    /// read, parsed, or written to the database.
    pub fn index_project_deps(&self, project_root: &Path) -> Result<usize, IndexingError> {
        let count = self.dep_index.index_project(project_root)?;
        Ok(count)
    }

    /// Rebuild cross-project symbol references for the given worktree.
    ///
    /// Marks existing edges for this project as stale, prunes them, and
    /// inserts the provided `new_edges`.  Returns the number of new edges
    /// added.
    ///
    /// The project identifier used for the xref graph is derived from the
    /// worktree's graph directory path.
    ///
    /// # Errors
    ///
    /// Returns [`IndexingError::Xref`] if the internal mutex is poisoned.
    #[allow(clippy::significant_drop_tightening)]
    pub fn rebuild_xrefs(
        &self,
        worktree_id: &WorktreeId,
        new_edges: Vec<XrefEdge>,
    ) -> Result<usize, IndexingError> {
        let graph_dir = self.layout.graph_dir(worktree_id);
        let project_key = graph_dir.to_string_lossy().into_owned();

        let mut graph = self
            .xref_graph
            .lock()
            .map_err(|e| IndexingError::Xref(format!("xref graph lock poisoned: {e}")))?;

        let count = rebuild_project_xrefs(&mut graph, &project_key, new_edges);
        Ok(count)
    }

    /// Query cross-project symbol references for a given symbol within a
    /// worktree context.
    ///
    /// Returns all non-stale [`XrefEdge`]s that reference `symbol` in either
    /// direction (incoming and outgoing).
    ///
    /// # Errors
    ///
    /// Returns [`IndexingError::Xref`] if the internal mutex is poisoned.
    #[allow(clippy::significant_drop_tightening)]
    pub fn query_xrefs(
        &self,
        symbol: &str,
        _worktree_id: &WorktreeId,
    ) -> Result<Vec<XrefEdge>, IndexingError> {
        let graph = self
            .xref_graph
            .lock()
            .map_err(|e| IndexingError::Xref(format!("xref graph lock poisoned: {e}")))?;

        let edges = xref_query(&graph, symbol, XrefDirection::Both);
        Ok(edges.into_iter().cloned().collect())
    }

    /// Query which projects depend on the named library.
    ///
    /// Delegates to [`DependencyIndex::projects_using`].
    ///
    /// # Errors
    ///
    /// Returns [`IndexingError::DependencyIndex`] if the database query fails.
    pub fn projects_using_dep(
        &self,
        dep_name: &str,
    ) -> Result<Vec<DependencyEntry>, IndexingError> {
        let entries = self.dep_index.projects_using(dep_name)?;
        Ok(entries)
    }

    /// Query all dependencies of a given project.
    ///
    /// Delegates to [`DependencyIndex::dependencies_of`].
    ///
    /// # Errors
    ///
    /// Returns [`IndexingError::DependencyIndex`] if the database query fails.
    pub fn project_dependencies(
        &self,
        project_path: &str,
    ) -> Result<Vec<DependencyEntry>, IndexingError> {
        let entries = self.dep_index.dependencies_of(project_path)?;
        Ok(entries)
    }
}

// Ensure the public API is `Send + Sync`.
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    const fn check() {
        assert_send_sync::<IndexingCoordinator>();
    }
    let _ = check;
};

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_layout(dir: &Path) -> StorageLayout {
        StorageLayout::with_root(dir, "synwire")
    }

    fn dummy_worktree() -> WorktreeId {
        use synwire_storage::identity::RepoId;
        WorktreeId::from_parts(
            RepoId::from_string("abc123"),
            "def456789012".to_owned(),
            "myrepo@main".to_owned(),
        )
    }

    #[test]
    fn coordinator_indexes_cargo_project() {
        let dir = tempdir().expect("tempdir");
        let layout = test_layout(dir.path());
        let coordinator = IndexingCoordinator::new(&layout).expect("new");

        // Create a project with a Cargo.toml.
        let project_dir = dir.path().join("my-project");
        std::fs::create_dir_all(&project_dir).expect("create dir");
        std::fs::write(
            project_dir.join("Cargo.toml"),
            "[package]\nname = \"test\"\n\n[dependencies]\nserde = \"1\"\ntokio = \"1\"\n",
        )
        .expect("write Cargo.toml");

        let count = coordinator
            .index_project_deps(&project_dir)
            .expect("index_project_deps");
        assert!(count >= 2);
    }

    #[test]
    fn coordinator_queries_deps() {
        let dir = tempdir().expect("tempdir");
        let layout = test_layout(dir.path());
        let coordinator = IndexingCoordinator::new(&layout).expect("new");

        let project_dir = dir.path().join("proj-a");
        std::fs::create_dir_all(&project_dir).expect("create dir");
        std::fs::write(
            project_dir.join("Cargo.toml"),
            "[package]\nname = \"a\"\n\n[dependencies]\nserde = \"1\"\n",
        )
        .expect("write Cargo.toml");

        let _ = coordinator.index_project_deps(&project_dir).expect("index");

        let projects = coordinator.projects_using_dep("serde").expect("query");
        assert!(!projects.is_empty());

        let deps = coordinator
            .project_dependencies(&project_dir.to_string_lossy())
            .expect("deps");
        assert!(!deps.is_empty());
    }

    #[test]
    fn coordinator_rebuilds_and_queries_xrefs() {
        let dir = tempdir().expect("tempdir");
        let layout = test_layout(dir.path());
        let coordinator = IndexingCoordinator::new(&layout).expect("new");
        let wid = dummy_worktree();

        let edges = vec![
            XrefEdge::new("proj_a", "proj_a::Foo", "proj_b", "proj_b::Bar"),
            XrefEdge::new("proj_a", "proj_a::Baz", "proj_c", "proj_c::Qux"),
        ];

        let count = coordinator
            .rebuild_xrefs(&wid, edges)
            .expect("rebuild_xrefs");
        assert_eq!(count, 2);

        let results = coordinator
            .query_xrefs("proj_b::Bar", &wid)
            .expect("query_xrefs");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source_symbol, "proj_a::Foo");
    }

    #[test]
    fn coordinator_xref_rebuild_replaces_old_edges() {
        let dir = tempdir().expect("tempdir");
        let layout = test_layout(dir.path());
        let coordinator = IndexingCoordinator::new(&layout).expect("new");
        let wid = dummy_worktree();

        // The project key used internally is the graph_dir path for this
        // worktree.  Edges must reference this key as source/target project
        // for `rebuild_project_xrefs` to correctly mark them stale.
        let project_key = layout.graph_dir(&wid).to_string_lossy().into_owned();

        // First build: one edge.
        let edges1 = vec![XrefEdge::new(
            &project_key,
            "proj_a::OldSym",
            "proj_b",
            "proj_b::Target",
        )];
        let _ = coordinator.rebuild_xrefs(&wid, edges1).expect("rebuild 1");

        // Second build: replace with a new edge.
        let edges2 = vec![XrefEdge::new(
            &project_key,
            "proj_a::NewSym",
            "proj_b",
            "proj_b::Target",
        )];
        let count = coordinator.rebuild_xrefs(&wid, edges2).expect("rebuild 2");
        assert_eq!(count, 1);

        // Old edge should be gone.
        let old = coordinator
            .query_xrefs("proj_a::OldSym", &wid)
            .expect("query old");
        assert!(old.is_empty());

        // New edge should be present.
        let new = coordinator
            .query_xrefs("proj_a::NewSym", &wid)
            .expect("query new");
        assert_eq!(new.len(), 1);
    }

    #[test]
    fn coordinator_indexes_go_mod_project() {
        let dir = tempdir().expect("tempdir");
        let layout = test_layout(dir.path());
        let coordinator = IndexingCoordinator::new(&layout).expect("new");

        let project_dir = dir.path().join("go-project");
        std::fs::create_dir_all(&project_dir).expect("create dir");
        std::fs::write(
            project_dir.join("go.mod"),
            "module example.com/app\n\ngo 1.21\n\nrequire (\n\tgithub.com/gin-gonic/gin v1.9.1\n)\n",
        )
        .expect("write go.mod");

        let count = coordinator.index_project_deps(&project_dir).expect("index");
        assert_eq!(count, 1);

        let projects = coordinator
            .projects_using_dep("github.com/gin-gonic/gin")
            .expect("query");
        assert!(!projects.is_empty());
    }

    #[test]
    fn coordinator_indexes_package_json_project() {
        let dir = tempdir().expect("tempdir");
        let layout = test_layout(dir.path());
        let coordinator = IndexingCoordinator::new(&layout).expect("new");

        let project_dir = dir.path().join("node-project");
        std::fs::create_dir_all(&project_dir).expect("create dir");
        std::fs::write(
            project_dir.join("package.json"),
            r#"{"name":"app","dependencies":{"react":"^18.0.0","axios":"^1.0.0"}}"#,
        )
        .expect("write package.json");

        let count = coordinator.index_project_deps(&project_dir).expect("index");
        assert_eq!(count, 2);
    }
}
