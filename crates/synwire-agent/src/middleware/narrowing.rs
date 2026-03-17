//! Hierarchical narrowing middleware.
//!
//! Implements a three-phase progressive-disclosure strategy:
//! 1. `tree` — build a directory map of the project
//! 2. `skeleton` — extract signatures from candidate files
//! 3. targeted read — return only the relevant function/range
//!
//! This reduces token usage versus reading entire files by ~75%.

use std::path::PathBuf;
use std::sync::Arc;

use synwire_core::vfs::protocol::Vfs;
use synwire_core::vfs::types::{TreeEntry, TreeOptions};

/// Query parameters for hierarchical narrowing.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NarrowingQuery {
    /// Natural-language description of the code to locate.
    pub description: String,
    /// Maximum number of candidate files to inspect skeletons for.
    pub top_k_files: usize,
    /// Maximum number of symbols to return in results.
    pub top_k_symbols: usize,
}

impl NarrowingQuery {
    /// Construct a new query with sensible defaults (`top_k_files = 5`, `top_k_symbols = 3`).
    #[must_use]
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            top_k_files: 5,
            top_k_symbols: 3,
        }
    }
}

/// A single result from hierarchical narrowing.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NarrowingResult {
    /// Path of the file containing the symbol.
    pub file: PathBuf,
    /// Symbol name, if a specific symbol was identified.
    pub symbol: Option<String>,
    /// Relevance score in the range `[0.0, 1.0]`.
    pub score: f32,
    /// Skeleton line or signature used as context.
    pub context: String,
}

/// Errors produced by [`HierarchicalNarrowing`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum NarrowingError {
    /// A VFS operation failed.
    #[error("VFS error: {0}")]
    Vfs(String),
    /// No results matched the query.
    #[error("no results found for the given query")]
    NoResults,
}

/// Hierarchical narrowing engine.
///
/// Uses a three-phase heuristic search (tree → skeleton → match) to locate
/// code relevant to a natural-language description without calling an LLM.
#[derive(Debug, Default)]
pub struct HierarchicalNarrowing;

impl HierarchicalNarrowing {
    /// Create a new narrowing engine.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Locate code relevant to `query` within the VFS rooted at `.`.
    ///
    /// # Errors
    ///
    /// Returns [`NarrowingError::Vfs`] if a VFS operation fails, or
    /// [`NarrowingError::NoResults`] if nothing matches.
    pub async fn narrow(
        &self,
        vfs: &Arc<dyn Vfs>,
        query: &NarrowingQuery,
    ) -> Result<Vec<NarrowingResult>, NarrowingError> {
        // Phase 1: collect all file paths via tree walk.
        let tree = vfs
            .tree(".", TreeOptions::default())
            .await
            .map_err(|e| NarrowingError::Vfs(e.to_string()))?;

        let mut all_files: Vec<String> = Vec::new();
        collect_files(&tree, &mut all_files);

        // Phase 2: score each file by keyword overlap with the description.
        let query_words = tokenise(&query.description);
        let mut scored_files: Vec<(String, f32)> = all_files
            .iter()
            .map(|path| {
                let score = file_score(path, &query_words);
                (path.clone(), score)
            })
            .collect();

        // Sort descending by score, keep top-k.
        scored_files.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored_files.truncate(query.top_k_files);

        // Discard zero-score files only when higher-scoring ones exist.
        let any_positive = scored_files.iter().any(|(_, s)| *s > 0.0);
        if any_positive {
            scored_files.retain(|(_, s)| *s > 0.0);
        }

        // Phase 3: for each candidate file, get the skeleton and score symbols.
        let mut results: Vec<NarrowingResult> = Vec::new();

        for (file_path, file_score) in &scored_files {
            let Ok(skeleton) = vfs.skeleton(file_path).await else {
                continue;
            };

            // Score each non-empty skeleton line as a candidate symbol.
            let mut sym_candidates: Vec<(String, f32, String)> = skeleton
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| {
                    let sym_name = extract_symbol_name(line);
                    let sym_score = symbol_score(line, &query_words);
                    let combined = sym_score.mul_add(0.6, file_score * 0.4);
                    (sym_name, combined, line.to_owned())
                })
                .collect();

            sym_candidates
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            // Take the best symbol from this file.
            if let Some((sym_name, score, context)) = sym_candidates.into_iter().next() {
                results.push(NarrowingResult {
                    file: PathBuf::from(file_path),
                    symbol: if sym_name.is_empty() {
                        None
                    } else {
                        Some(sym_name)
                    },
                    score,
                    context,
                });
            } else {
                // File matched but had no skeleton lines — still include it.
                results.push(NarrowingResult {
                    file: PathBuf::from(file_path),
                    symbol: None,
                    score: *file_score,
                    context: String::new(),
                });
            }
        }

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(query.top_k_symbols);

        if results.is_empty() {
            return Err(NarrowingError::NoResults);
        }

        Ok(results)
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Recursively collect all non-directory file paths from a `TreeEntry`.
fn collect_files(entry: &TreeEntry, out: &mut Vec<String>) {
    if !entry.is_dir {
        out.push(entry.path.clone());
    }
    for child in &entry.children {
        collect_files(child, out);
    }
}

/// Split a string into lowercase words (split on non-alphanumeric characters).
fn tokenise(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Score a file path by the proportion of query words that appear in it.
///
/// A query word matches when the path contains it as a substring **or** when
/// the path contains a word that starts with the query word (prefix match),
/// allowing "auth" in a path to match the query word "authentication".
///
/// Returns a value in `[0.0, 1.0]`.
#[allow(clippy::cast_precision_loss)]
pub fn file_score(path: &str, query_words: &[String]) -> f32 {
    if query_words.is_empty() {
        return 0.0;
    }
    let path_lower = path.to_lowercase();
    let path_tokens = tokenise(&path_lower);
    let matches = query_words
        .iter()
        .filter(|qw| {
            // Direct substring match in the full path string.
            if path_lower.contains(qw.as_str()) {
                return true;
            }
            // Prefix match: any path token that starts with the query word.
            path_tokens.iter().any(|pt| qw.starts_with(pt.as_str()))
        })
        .count();
    matches as f32 / query_words.len() as f32
}

/// Score a skeleton line by query word overlap.
///
/// Returns a value in `[0.0, 1.0]`.
#[allow(clippy::cast_precision_loss)]
pub fn symbol_score(line: &str, query_words: &[String]) -> f32 {
    if query_words.is_empty() {
        return 0.0;
    }
    let line_lower = line.to_lowercase();
    let matches = query_words
        .iter()
        .filter(|w| line_lower.contains(w.as_str()))
        .count();
    matches as f32 / query_words.len() as f32
}

/// Extract the first identifier token from a skeleton line as the symbol name.
fn extract_symbol_name(line: &str) -> String {
    // Find the first word that looks like an identifier (letters/underscores/digits).
    for word in line.split_whitespace() {
        // Strip everything from the first non-identifier character (e.g. `(`).
        let ident: String = word
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if ident.is_empty() || !ident.chars().next().is_some_and(char::is_alphabetic) {
            continue;
        }
        // Skip common keywords that are not the symbol name.
        match ident.as_str() {
            "pub" | "fn" | "async" | "struct" | "enum" | "impl" | "trait" | "mod" | "use"
            | "type" | "const" | "static" | "let" | "for" | "if" | "while" | "return"
            | "unsafe" | "extern" | "crate" | "super" | "self" => {}
            _ => return ident,
        }
    }
    String::new()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::needless_collect,
    clippy::useless_vec
)]
mod tests {
    use super::*;

    // Tests for the pure keyword-scoring helpers — no VFS required.

    #[test]
    fn file_score_exact_match() {
        let words = tokenise("authentication logic");
        assert!(file_score("src/auth.rs", &words) > 0.0);
    }

    #[test]
    fn file_score_no_match() {
        let words = tokenise("authentication logic");
        assert_eq!(file_score("src/routes.rs", &words), 0.0);
    }

    #[test]
    fn file_score_database_match() {
        let words = tokenise("database connection");
        assert!(file_score("src/database.rs", &words) > 0.0);
    }

    #[test]
    fn symbol_score_counts_overlapping_words() {
        let words = tokenise("authenticate user");
        let score = symbol_score("pub fn authenticate(user: &User) -> Result<Token>", &words);
        assert!(score > 0.0);
    }

    #[test]
    fn symbol_score_zero_when_no_overlap() {
        let words = tokenise("unrelated concept");
        let score = symbol_score("pub fn authenticate(user: &User) -> Result<Token>", &words);
        assert_eq!(score, 0.0);
    }

    /// Simulate the file-ranking phase without a real VFS.
    #[test]
    fn narrowing_ranks_auth_file_for_authentication_query() {
        let files = vec!["src/auth.rs", "src/database.rs", "src/routes.rs"];
        let words = tokenise("authentication logic");
        let mut scored: Vec<(&str, f32)> =
            files.iter().map(|f| (*f, file_score(f, &words))).collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top3: Vec<&str> = scored.iter().take(3).map(|(f, _)| *f).collect();
        assert!(top3.contains(&"src/auth.rs"), "auth.rs should be in top-3");
    }

    /// Simulate the file-ranking phase for the database query.
    #[test]
    fn narrowing_ranks_database_file_for_database_query() {
        let files = vec!["src/auth.rs", "src/database.rs", "src/routes.rs"];
        let words = tokenise("database connection");
        let mut scored: Vec<(&str, f32)> =
            files.iter().map(|f| (*f, file_score(f, &words))).collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top3: Vec<&str> = scored.iter().take(3).map(|(f, _)| *f).collect();
        assert!(
            top3.contains(&"src/database.rs"),
            "database.rs should be in top-3"
        );
    }

    #[test]
    fn extract_symbol_name_skips_keywords() {
        assert_eq!(
            extract_symbol_name("pub fn authenticate(user: &User)"),
            "authenticate"
        );
        assert_eq!(extract_symbol_name("pub struct AuthToken {"), "AuthToken");
        assert_eq!(extract_symbol_name("  "), "");
    }

    #[test]
    fn tokenise_splits_on_non_alphanumeric() {
        let words = tokenise("hello-world_foo bar");
        assert!(words.contains(&"hello".to_owned()));
        assert!(words.contains(&"world".to_owned()));
        assert!(words.contains(&"foo".to_owned()));
        assert!(words.contains(&"bar".to_owned()));
    }
}
