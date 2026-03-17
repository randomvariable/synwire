//! Global cross-project dependency index.
//!
//! Parses project manifests (`Cargo.toml`, `go.mod`, `package.json`,
//! `pyproject.toml`) to build a graph of which projects depend on which
//! libraries.  Stored in `global/dependencies/deps.db` via `SQLite`.

use rusqlite::params;
use std::path::Path;

/// A single project→dependency edge in the dependency index.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DependencyEntry {
    /// Project root path.
    pub project_path: String,
    /// Dependency name.
    pub dependency: String,
    /// Dependency version requirement (e.g., `"^1.2.0"`).
    pub version_req: String,
    /// Ecosystem: `"cargo"`, `"go"`, `"npm"`, or `"python"`.
    pub ecosystem: String,
}

/// Errors produced by [`DependencyIndex`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DependencyIndexError {
    /// `SQLite` operation failed.
    #[error("sqlite error: {0}")]
    Sqlite(String),
    /// I/O operation failed.
    #[error("io error: {0}")]
    Io(String),
    /// Manifest parse error.
    #[error("parse error: {0}")]
    Parse(String),
}

impl From<rusqlite::Error> for DependencyIndexError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e.to_string())
    }
}

impl From<std::io::Error> for DependencyIndexError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

/// Global cross-project dependency index backed by `SQLite`.
///
/// # Thread safety
///
/// The underlying [`rusqlite::Connection`] is guarded by a `Mutex` so
/// `DependencyIndex` is `Send + Sync`.
pub struct DependencyIndex {
    conn: std::sync::Mutex<rusqlite::Connection>,
}

impl DependencyIndex {
    /// Open or create the dependency index at the given path.
    ///
    /// Creates parent directories if they do not exist, opens the database in
    /// WAL mode, and initialises the schema.
    ///
    /// # Errors
    ///
    /// Returns [`DependencyIndexError::Io`] if the parent directory cannot be
    /// created, or [`DependencyIndexError::Sqlite`] if the database cannot be
    /// opened or the schema cannot be initialised.
    pub fn open(path: &Path) -> Result<Self, DependencyIndexError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DependencyIndexError::Io(e.to_string()))?;
        }

        let conn = rusqlite::Connection::open(path)
            .map_err(|e| DependencyIndexError::Sqlite(e.to_string()))?;

        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;",
        )
        .map_err(|e| DependencyIndexError::Sqlite(e.to_string()))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS dependencies (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                project_path TEXT NOT NULL,
                dependency   TEXT NOT NULL,
                version_req  TEXT NOT NULL,
                ecosystem    TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_dep  ON dependencies(dependency);
             CREATE INDEX IF NOT EXISTS idx_proj ON dependencies(project_path);",
        )
        .map_err(|e| DependencyIndexError::Sqlite(e.to_string()))?;

        Ok(Self {
            conn: std::sync::Mutex::new(conn),
        })
    }

    /// Parse a project manifest file and index its dependencies.
    ///
    /// Detects the manifest type from files present in `project_root` and
    /// inserts all discovered dependencies.  Returns the count of rows
    /// inserted.
    ///
    /// # Errors
    ///
    /// Returns [`DependencyIndexError::Io`] if the manifest cannot be read, or
    /// [`DependencyIndexError::Parse`] if the manifest is malformed, or
    /// [`DependencyIndexError::Sqlite`] if a database write fails.
    pub fn index_project(&self, project_root: &Path) -> Result<usize, DependencyIndexError> {
        let project_path = project_root.to_string_lossy().into_owned();

        let mut entries: Vec<(String, String, String)> = Vec::new();

        let cargo_path = project_root.join("Cargo.toml");
        let gomod_path = project_root.join("go.mod");
        let pkg_json_path = project_root.join("package.json");
        let pyproject_path = project_root.join("pyproject.toml");

        if cargo_path.exists() {
            let content = std::fs::read_to_string(&cargo_path)
                .map_err(|e| DependencyIndexError::Io(e.to_string()))?;
            Self::parse_cargo_toml(&content, &mut entries)
                .map_err(|e| DependencyIndexError::Parse(e.to_string()))?;
        } else if gomod_path.exists() {
            let content = std::fs::read_to_string(&gomod_path)
                .map_err(|e| DependencyIndexError::Io(e.to_string()))?;
            Self::parse_go_mod(&content, &mut entries);
        } else if pkg_json_path.exists() {
            let content = std::fs::read_to_string(&pkg_json_path)
                .map_err(|e| DependencyIndexError::Io(e.to_string()))?;
            Self::parse_package_json(&content, &mut entries)
                .map_err(|e| DependencyIndexError::Parse(e.to_string()))?;
        } else if pyproject_path.exists() {
            let content = std::fs::read_to_string(&pyproject_path)
                .map_err(|e| DependencyIndexError::Io(e.to_string()))?;
            Self::parse_pyproject_toml(&content, &mut entries)
                .map_err(|e| DependencyIndexError::Parse(e.to_string()))?;
        }

        let count = entries.len();
        for (name, version_req, ecosystem) in entries {
            self.insert_dep(&project_path, &name, &version_req, &ecosystem)?;
        }
        Ok(count)
    }

    /// Query: which projects depend on the given library?
    ///
    /// # Errors
    ///
    /// Returns [`DependencyIndexError::Sqlite`] if the query fails.
    pub fn projects_using(
        &self,
        dependency: &str,
    ) -> Result<Vec<DependencyEntry>, DependencyIndexError> {
        self.query_entries(
            "SELECT project_path, dependency, version_req, ecosystem \
             FROM dependencies WHERE dependency = ?1",
            dependency,
        )
    }

    /// Query: what does the given project depend on?
    ///
    /// # Errors
    ///
    /// Returns [`DependencyIndexError::Sqlite`] if the query fails.
    pub fn dependencies_of(
        &self,
        project_path: &str,
    ) -> Result<Vec<DependencyEntry>, DependencyIndexError> {
        self.query_entries(
            "SELECT project_path, dependency, version_req, ecosystem \
             FROM dependencies WHERE project_path = ?1",
            project_path,
        )
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Execute a single-parameter SELECT and collect the result rows.
    #[allow(clippy::significant_drop_tightening)]
    fn query_entries(
        &self,
        sql: &str,
        param: &str,
    ) -> Result<Vec<DependencyEntry>, DependencyIndexError> {
        let guard = self
            .conn
            .lock()
            .map_err(|e| DependencyIndexError::Sqlite(e.to_string()))?;
        let mut stmt = guard
            .prepare(sql)
            .map_err(|e| DependencyIndexError::Sqlite(e.to_string()))?;
        // stmt borrows guard, so we collect eagerly to release the lock as
        // soon as the scope ends.
        stmt.query_map(params![param], |row| {
            Ok(DependencyEntry {
                project_path: row.get(0)?,
                dependency: row.get(1)?,
                version_req: row.get(2)?,
                ecosystem: row.get(3)?,
            })
        })
        .map_err(|e| DependencyIndexError::Sqlite(e.to_string()))?
        .map(|r| r.map_err(|e| DependencyIndexError::Sqlite(e.to_string())))
        .collect()
    }

    #[allow(clippy::significant_drop_tightening)]
    fn insert_dep(
        &self,
        project_path: &str,
        dependency: &str,
        version_req: &str,
        ecosystem: &str,
    ) -> Result<(), DependencyIndexError> {
        let guard = self
            .conn
            .lock()
            .map_err(|e| DependencyIndexError::Sqlite(e.to_string()))?;
        // The guard is held for the duration of this call; the lint suggests
        // dropping it earlier but stmt borrows it so it cannot be released sooner.
        let _rows = guard
            .execute(
                "INSERT INTO dependencies (project_path, dependency, version_req, ecosystem) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![project_path, dependency, version_req, ecosystem],
            )
            .map_err(|e| DependencyIndexError::Sqlite(e.to_string()))?;
        Ok(())
    }

    /// Parse `[dependencies]` and `[dev-dependencies]` from a `Cargo.toml`.
    fn parse_cargo_toml(
        content: &str,
        out: &mut Vec<(String, String, String)>,
    ) -> Result<(), toml::de::Error> {
        let value: toml::Value = toml::from_str(content)?;
        for table_key in &["dependencies", "dev-dependencies"] {
            if let Some(deps) = value.get(table_key).and_then(|v| v.as_table()) {
                for (name, spec) in deps {
                    let version_req = match spec {
                        toml::Value::String(s) => s.clone(),
                        toml::Value::Table(t) => t
                            .get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("*")
                            .to_owned(),
                        _ => "*".to_owned(),
                    };
                    out.push((name.clone(), version_req, "cargo".to_owned()));
                }
            }
        }
        Ok(())
    }

    /// Parse `require` blocks from a `go.mod`.
    fn parse_go_mod(content: &str, out: &mut Vec<(String, String, String)>) {
        let mut in_require_block = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == "require (" {
                in_require_block = true;
                continue;
            }
            if in_require_block {
                if trimmed == ")" {
                    in_require_block = false;
                    continue;
                }
                // Expected format inside block: <module> v<version>
                Self::push_go_dep(trimmed, out);
            } else if let Some(rest) = trimmed.strip_prefix("require ") {
                // Single-line require: require <module> v<version>
                let rest = rest.trim();
                if !rest.starts_with('(') {
                    Self::push_go_dep(rest, out);
                }
            }
        }
    }

    /// Parse a single `<module> v<version>` line and push to `out`.
    fn push_go_dep(line: &str, out: &mut Vec<(String, String, String)>) {
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() == 2 {
            let module = parts[0].to_owned();
            let version = parts[1].trim_start_matches('v').to_owned();
            out.push((module, version, "go".to_owned()));
        }
    }

    /// Parse `dependencies` and `devDependencies` from a `package.json`.
    fn parse_package_json(
        content: &str,
        out: &mut Vec<(String, String, String)>,
    ) -> Result<(), serde_json::Error> {
        let json: serde_json::Value = serde_json::from_str(content)?;
        for key in &["dependencies", "devDependencies"] {
            if let Some(deps) = json.get(key).and_then(|v| v.as_object()) {
                for (name, version) in deps {
                    let version_req = version.as_str().unwrap_or("*").to_owned();
                    out.push((name.clone(), version_req, "npm".to_owned()));
                }
            }
        }
        Ok(())
    }

    /// Parse `[tool.poetry.dependencies]` or `[project.dependencies]` from a
    /// `pyproject.toml`.
    fn parse_pyproject_toml(
        content: &str,
        out: &mut Vec<(String, String, String)>,
    ) -> Result<(), toml::de::Error> {
        let value: toml::Value = toml::from_str(content)?;

        // PEP 517 / setuptools: [project.dependencies] is a list of strings.
        if let Some(deps) = value
            .get("project")
            .and_then(|v| v.get("dependencies"))
            .and_then(|v| v.as_array())
        {
            for dep in deps {
                if let Some(s) = dep.as_str() {
                    // e.g. "requests>=2.28"
                    let name = s
                        .split(['>', '<', '=', '!', '~'])
                        .next()
                        .unwrap_or(s)
                        .trim()
                        .to_owned();
                    let version_req = if name.len() < s.len() {
                        s[name.len()..].trim().to_owned()
                    } else {
                        "*".to_owned()
                    };
                    out.push((name, version_req, "python".to_owned()));
                }
            }
        }

        // Poetry: [tool.poetry.dependencies] is a table.
        if let Some(deps) = value
            .get("tool")
            .and_then(|v| v.get("poetry"))
            .and_then(|v| v.get("dependencies"))
            .and_then(|v| v.as_table())
        {
            for (name, spec) in deps {
                if name == "python" {
                    continue;
                }
                let version_req = match spec {
                    toml::Value::String(s) => s.clone(),
                    toml::Value::Table(t) => t
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("*")
                        .to_owned(),
                    _ => "*".to_owned(),
                };
                out.push((name.clone(), version_req, "python".to_owned()));
            }
        }

        Ok(())
    }
}

// Ensure the public API is `Send + Sync`.
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    const fn check() {
        assert_send_sync::<DependencyIndex>();
    }
    let _ = check;
};

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn dependency_index_cargo() {
        let dir = tempdir().expect("tempdir");
        let cargo_toml =
            "[package]\nname = \"test\"\n\n[dependencies]\nserde = \"1\"\ntokio = \"1\"\n";
        std::fs::write(dir.path().join("Cargo.toml"), cargo_toml).expect("write Cargo.toml");

        let db_path = dir.path().join("deps.db");
        let idx = DependencyIndex::open(&db_path).expect("open");
        let count = idx.index_project(dir.path()).expect("index_project");
        assert!(count >= 2);

        let projects = idx.projects_using("serde").expect("projects_using");
        assert!(!projects.is_empty());
    }

    #[test]
    fn dependency_index_go_mod() {
        let dir = tempdir().expect("tempdir");
        let go_mod = "module example.com/myapp\n\ngo 1.21\n\nrequire (\n\tgithub.com/gin-gonic/gin v1.9.1\n\tgolang.org/x/net v0.20.0\n)\n";
        std::fs::write(dir.path().join("go.mod"), go_mod).expect("write go.mod");

        let db_path = dir.path().join("deps.db");
        let idx = DependencyIndex::open(&db_path).expect("open");
        let count = idx.index_project(dir.path()).expect("index_project");
        assert_eq!(count, 2);

        let projects = idx
            .projects_using("github.com/gin-gonic/gin")
            .expect("projects_using");
        assert!(!projects.is_empty());
        assert_eq!(projects[0].ecosystem, "go");
    }

    #[test]
    fn dependency_index_package_json() {
        let dir = tempdir().expect("tempdir");
        let pkg_json = r#"{"name":"myapp","dependencies":{"react":"^18.0.0","axios":"^1.0.0"}}"#;
        std::fs::write(dir.path().join("package.json"), pkg_json).expect("write package.json");

        let db_path = dir.path().join("deps.db");
        let idx = DependencyIndex::open(&db_path).expect("open");
        let count = idx.index_project(dir.path()).expect("index_project");
        assert_eq!(count, 2);

        let projects = idx.projects_using("react").expect("projects_using");
        assert!(!projects.is_empty());
        assert_eq!(projects[0].ecosystem, "npm");
    }

    #[test]
    fn dependencies_of_returns_all_for_project() {
        let dir = tempdir().expect("tempdir");
        let cargo_toml =
            "[package]\nname = \"x\"\n\n[dependencies]\na = \"1\"\nb = \"2\"\nc = \"3\"\n";
        std::fs::write(dir.path().join("Cargo.toml"), cargo_toml).expect("write");

        let db_path = dir.path().join("deps.db");
        let idx = DependencyIndex::open(&db_path).expect("open");
        let _ = idx.index_project(dir.path()).expect("index");

        let deps = idx
            .dependencies_of(&dir.path().to_string_lossy())
            .expect("dependencies_of");
        assert_eq!(deps.len(), 3);
    }
}
