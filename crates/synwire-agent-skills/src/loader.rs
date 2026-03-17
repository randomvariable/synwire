//! Directory scanner that discovers `SKILL.md` files and produces [`SkillEntry`] values.

use std::path::{Path, PathBuf};

use tokio::fs;
use tracing::debug;

use crate::{
    error::SkillError,
    manifest::{SkillManifest, parse_skill_md},
};

/// A fully-loaded skill entry, combining the parsed manifest with the raw body
/// text and the directory it was loaded from.
#[derive(Debug, Clone)]
pub struct SkillEntry {
    /// The parsed manifest from the SKILL.md frontmatter.
    pub manifest: SkillManifest,
    /// The full SKILL.md content (instructions after the frontmatter).
    pub body: String,
    /// The directory that contains `SKILL.md`.
    pub skill_dir: PathBuf,
}

/// Scans directories for `SKILL.md` files and loads them as [`SkillEntry`]
/// values.
#[derive(Debug, Default)]
pub struct SkillLoader {}

impl SkillLoader {
    /// Create a new [`SkillLoader`].
    pub const fn new() -> Self {
        Self {}
    }

    /// Scan `dir` for immediate child directories that contain a `SKILL.md`
    /// file, parse each manifest, and return the resulting entries.
    ///
    /// Only one level of subdirectories is examined — nested skill trees are
    /// not walked recursively.
    ///
    /// # Errors
    ///
    /// Returns [`SkillError::Io`] if `dir` cannot be read.
    /// Returns [`SkillError::InvalidManifest`] or [`SkillError::Yaml`] if a
    /// `SKILL.md` file is malformed.
    pub async fn scan(&self, dir: &Path) -> Result<Vec<SkillEntry>, SkillError> {
        let mut entries: Vec<SkillEntry> = Vec::new();

        let mut read_dir = fs::read_dir(dir).await?;
        while let Some(child) = read_dir.next_entry().await? {
            let child_path = child.path();
            if !child_path.is_dir() {
                continue;
            }
            let skill_file = child_path.join("SKILL.md");
            if !skill_file.exists() {
                continue;
            }

            debug!(path = %skill_file.display(), "loading skill");
            let content = fs::read_to_string(&skill_file).await?;
            let manifest = parse_skill_md(&content)?;
            let body = extract_body(&content);

            let entry = SkillEntry {
                manifest,
                body: body.to_owned(),
                skill_dir: child_path,
            };

            self.validate(&entry)?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// Validate a [`SkillEntry`] against structural invariants.
    ///
    /// Currently enforced:
    /// - The skill directory name must match `manifest.name`.
    ///
    /// # Errors
    ///
    /// Returns [`SkillError::InvalidManifest`] if any constraint is violated.
    pub fn validate(&self, entry: &SkillEntry) -> Result<(), SkillError> {
        let dir_name = entry
            .skill_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if dir_name != entry.manifest.name {
            return Err(SkillError::InvalidManifest(format!(
                "directory name '{}' does not match skill name '{}'",
                dir_name, entry.manifest.name
            )));
        }

        Ok(())
    }
}

/// Extract the body text from a SKILL.md file (everything after the closing
/// `---` delimiter).
fn extract_body(content: &str) -> &str {
    // Skip the opening `---\n`
    let Some(after_open) = content
        .strip_prefix("---")
        .and_then(|s| s.strip_prefix('\n').or_else(|| s.strip_prefix("\r\n")))
    else {
        return content;
    };

    // Find the closing `\n---`
    after_open.find("\n---").map_or("", |pos| {
        let remainder = &after_open[pos + 4..]; // skip `\n---`
        // Skip the optional newline after the closing delimiter
        remainder
            .strip_prefix('\n')
            .or_else(|| remainder.strip_prefix("\r\n"))
            .unwrap_or(remainder)
    })
}
