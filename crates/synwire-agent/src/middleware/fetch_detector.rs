//! Repository fetch detector middleware.
//!
//! Monitors web fetch calls for GitHub raw content URLs.
//! After 3+ fetches from the same repository, emits a `PromptSuggestion`
//! recommending the user clone the repository for faster access.

use std::collections::HashMap;

/// Detects repeated HTTP fetches from the same GitHub repository and
/// suggests cloning it locally once the fetch count exceeds a threshold.
#[derive(Debug, Clone)]
pub struct RepoFetchDetector {
    /// Minimum number of fetches before a suggestion is emitted.
    pub threshold: usize,
    /// Per-repository fetch counts keyed by `"owner/repo"`.
    counts: HashMap<String, usize>,
}

impl RepoFetchDetector {
    /// Construct a detector with the given `threshold`.
    ///
    /// A threshold of `3` means a suggestion is emitted on the third fetch
    /// from the same repository.
    #[must_use]
    pub fn new(threshold: usize) -> Self {
        Self {
            threshold,
            counts: HashMap::new(),
        }
    }

    /// Record a single HTTP fetch from `url`.
    ///
    /// Recognised URL patterns:
    /// - `raw.githubusercontent.com/<owner>/<repo>/…`
    /// - `github.com/<owner>/<repo>/blob/…`
    ///
    /// Unrecognised URLs are silently ignored.
    pub fn record_fetch(&mut self, url: &str) {
        if let Some(owner_repo) = parse_github_owner_repo(url) {
            *self.counts.entry(owner_repo).or_insert(0) += 1;
        }
    }

    /// Return `true` if the fetch count for `owner_repo` meets or exceeds the threshold.
    #[must_use]
    pub fn should_suggest(&self, owner_repo: &str) -> bool {
        self.counts
            .get(owner_repo)
            .is_some_and(|&c| c >= self.threshold)
    }

    /// Return clone suggestions for every repository that has reached the threshold.
    #[must_use]
    pub fn suggestions(&self) -> Vec<String> {
        let mut suggestions: Vec<String> = self
            .counts
            .iter()
            .filter(|&(_, &count)| count >= self.threshold)
            .map(|(owner_repo, _)| format!("Consider cloning {owner_repo} for faster access"))
            .collect();
        suggestions.sort();
        suggestions
    }
}

/// Parse a GitHub URL and return `"owner/repo"` if the pattern matches.
fn parse_github_owner_repo(url: &str) -> Option<String> {
    // Pattern: raw.githubusercontent.com/<owner>/<repo>/
    if let Some(rest) = url
        .strip_prefix("https://raw.githubusercontent.com/")
        .or_else(|| url.strip_prefix("http://raw.githubusercontent.com/"))
        .or_else(|| url.strip_prefix("raw.githubusercontent.com/"))
    {
        return extract_two_segments(rest);
    }

    // Pattern: github.com/<owner>/<repo>/blob/
    if let Some(rest) = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
        .or_else(|| url.strip_prefix("github.com/"))
    {
        return extract_two_segments(rest);
    }

    None
}

/// Extract the first two path segments as `"seg1/seg2"` from a URL suffix.
fn extract_two_segments(rest: &str) -> Option<String> {
    let mut parts = rest.splitn(3, '/');
    let owner = parts.next().filter(|s| !s.is_empty())?;
    let repo = parts.next().filter(|s| !s.is_empty())?;
    Some(format!("{owner}/{repo}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn three_fetches_from_raw_url_triggers_suggestion() {
        let mut det = RepoFetchDetector::new(3);
        for _ in 0..3 {
            det.record_fetch("https://raw.githubusercontent.com/owner/repo/main/file.txt");
        }
        assert!(det.should_suggest("owner/repo"));
        let sug = det.suggestions();
        assert!(!sug.is_empty());
        assert!(sug[0].contains("owner/repo"));
    }

    #[test]
    fn two_fetches_do_not_trigger_suggestion() {
        let mut det = RepoFetchDetector::new(3);
        for _ in 0..2 {
            det.record_fetch("https://raw.githubusercontent.com/owner/repo/main/file.txt");
        }
        assert!(!det.should_suggest("owner/repo"));
        assert!(det.suggestions().is_empty());
    }

    #[test]
    fn github_blob_url_is_recognised() {
        let mut det = RepoFetchDetector::new(1);
        det.record_fetch("https://github.com/acme/widget/blob/main/src/lib.rs");
        assert!(det.should_suggest("acme/widget"));
    }

    #[test]
    fn unrecognised_url_is_ignored() {
        let mut det = RepoFetchDetector::new(1);
        det.record_fetch("https://example.com/some/path/file.txt");
        assert!(det.suggestions().is_empty());
    }

    #[test]
    fn threshold_boundary_exactly_at_threshold() {
        let mut det = RepoFetchDetector::new(2);
        det.record_fetch("https://raw.githubusercontent.com/x/y/main/a.txt");
        assert!(!det.should_suggest("x/y"), "1 fetch should not trigger");
        det.record_fetch("https://raw.githubusercontent.com/x/y/main/b.txt");
        assert!(det.should_suggest("x/y"), "2 fetches should trigger");
    }
}
