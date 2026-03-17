//! Session state for repositories that have been cloned and mounted into the VFS.

use std::path::PathBuf;

/// Records a repository that was cloned and mounted during a session.
///
/// Stored in session state so that cloned repositories can be re-mounted
/// automatically when a session is resumed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct MountedRepo {
    /// Remote URL the repository was cloned from.
    pub url: String,
    /// Local path to the cloned repository on disk.
    pub local_path: PathBuf,
    /// Whether the repository has been semantically indexed.
    pub indexed: bool,
}

impl MountedRepo {
    /// Construct a new `MountedRepo` record.
    #[must_use]
    pub fn new(url: impl Into<String>, local_path: impl Into<PathBuf>) -> Self {
        Self {
            url: url.into(),
            local_path: local_path.into(),
            indexed: false,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn mounted_repo_round_trips_through_json() {
        let repo = MountedRepo::new(
            "https://github.com/example/repo.git",
            "/home/user/.cache/synwire/repos/example/repo",
        );
        let json = serde_json::to_string(&repo).expect("serialize");
        let de: MountedRepo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(de.url, repo.url);
        assert_eq!(de.local_path, repo.local_path);
        assert!(!de.indexed);
    }
}
