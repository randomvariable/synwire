//! Directory walking with include/exclude/size filters and agentic ignore.

use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::{Path, PathBuf};
use synwire_core::vfs::IndexOptions;
use synwire_core::vfs::agentic_ignore::AgenticIgnore;
use tracing::warn;

/// Build a [`GlobSet`] from a list of patterns.
///
/// Returns `(globset, has_patterns)` where `has_patterns` indicates whether
/// any valid patterns were added (an empty `GlobSet` matches nothing, not everything).
fn build_globset(patterns: &[String]) -> (GlobSet, bool) {
    if patterns.is_empty() {
        return (GlobSet::empty(), false);
    }
    let mut builder = GlobSetBuilder::new();
    let mut added = 0usize;
    for pat in patterns {
        match Glob::new(pat) {
            Ok(g) => {
                let _ = builder.add(g);
                added += 1;
            }
            Err(e) => warn!("Invalid glob pattern {pat:?}: {e}"),
        }
    }
    let set = builder.build().unwrap_or_else(|e| {
        warn!("Failed to build GlobSet: {e}");
        GlobSet::empty()
    });
    (set, added > 0)
}

/// Collect all files under `root` matching the filter options.
///
/// Files matched by agentic ignore files (`.cursorignore`, `.aiignore`,
/// `.claudeignore`, `.gitignore`, etc.) found in `root` or any ancestor
/// directory are excluded automatically.
///
/// Returns a list of absolute paths to files that should be indexed.
pub fn walk(root: &Path, opts: &IndexOptions) -> Vec<PathBuf> {
    let (include_set, has_includes) = build_globset(&opts.include);
    let (exclude_set, has_excludes) = build_globset(&opts.exclude);
    let max_size = opts.max_file_size.unwrap_or(1024 * 1024); // 1 MiB default
    let agentic = AgenticIgnore::discover(root);

    let mut files = Vec::new();

    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| match e {
            Ok(e) => Some(e),
            Err(err) => {
                warn!("Walk error: {err}");
                None
            }
        })
    {
        let path = entry.path();

        // Agentic ignore check — applies to both files and directories.
        // Skipping ignored directories avoids descending into them needlessly,
        // but walkdir doesn't support per-entry skip, so we just filter.
        if agentic.is_ignored(path, entry.file_type().is_dir()) {
            continue;
        }

        if !entry.file_type().is_file() {
            continue;
        }
        let rel = path.strip_prefix(root).unwrap_or(path);
        let rel_str = rel.to_string_lossy();

        // Exclude check — only applies when exclude patterns were provided.
        if has_excludes && exclude_set.is_match(rel_str.as_ref()) {
            continue;
        }
        // Include check — only applies when include patterns were provided.
        // When none are provided, all files pass.
        if has_includes && !include_set.is_match(rel_str.as_ref()) {
            continue;
        }
        // Size check
        if let Ok(meta) = std::fs::metadata(path)
            && meta.len() > max_size
        {
            continue;
        }

        files.push(path.to_path_buf());
    }

    files
}
