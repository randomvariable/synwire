//! Community summary generation and caching.
//!
//! Generates and caches natural-language summaries for code communities.
//! When a [`SamplingProvider`] is available the summary is produced by the LLM;
//! otherwise the fallback is a comma-joined member list.
//!
//! Summaries are stored as JSON files under
//! `<base>/communities/summaries/<community_id>.json`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use synwire_core::agents::sampling::{SamplingProvider, SamplingRequest};

use super::{CommunityError, CommunityId};

// ── CommunitySummary ──────────────────────────────────────────────────────────

/// A cached natural-language summary for one community.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CommunitySummary {
    /// The community this summary describes.
    pub community_id: CommunityId,
    /// Generated summary text.
    pub summary: String,
    /// When the summary was generated.
    pub generated_at: DateTime<Utc>,
    /// `true` if the community's membership has changed since generation.
    pub is_stale: bool,
}

impl CommunitySummary {
    fn new(community_id: CommunityId, summary: String) -> Self {
        Self {
            community_id,
            summary,
            generated_at: Utc::now(),
            is_stale: false,
        }
    }
}

// ── SummaryCache ──────────────────────────────────────────────────────────────

/// On-disk cache for community summaries.
///
/// Each entry is stored as `<base_dir>/communities/summaries/<id>.json`.
/// The in-memory index (`entries`) mirrors the on-disk state to avoid
/// repeated filesystem reads.
#[derive(Debug)]
pub struct SummaryCache {
    base_dir: PathBuf,
    /// In-memory mirror of on-disk entries.
    entries: HashMap<CommunityId, CommunitySummary>,
}

impl SummaryCache {
    /// Create a new cache rooted at `base_dir`.
    ///
    /// The directory `<base_dir>/communities/summaries/` is created if it does
    /// not already exist.
    ///
    /// # Errors
    ///
    /// Returns [`CommunityError::Io`] if the directory cannot be created.
    pub fn new(base_dir: &Path) -> Result<Self, CommunityError> {
        let summaries_dir = base_dir.join("communities").join("summaries");
        std::fs::create_dir_all(&summaries_dir)?;
        Ok(Self {
            base_dir: base_dir.to_path_buf(),
            entries: HashMap::new(),
        })
    }

    fn summary_path(&self, id: CommunityId) -> PathBuf {
        self.base_dir
            .join("communities")
            .join("summaries")
            .join(format!("{}.json", id.0))
    }

    /// Insert or replace a summary in the cache (memory + disk).
    ///
    /// # Errors
    ///
    /// Returns [`CommunityError`] on serialization or write failures.
    pub fn set(&mut self, summary: CommunitySummary) -> Result<(), CommunityError> {
        let path = self.summary_path(summary.community_id);
        let json = serde_json::to_string_pretty(&summary)?;
        std::fs::write(&path, json)?;
        let _ = self.entries.insert(summary.community_id, summary);
        Ok(())
    }

    /// Return a reference to the cached summary for `id`, loading from disk if
    /// not yet in memory.
    ///
    /// # Errors
    ///
    /// Returns [`CommunityError`] if the file cannot be read or parsed.
    pub fn get(&mut self, id: CommunityId) -> Result<Option<&CommunitySummary>, CommunityError> {
        if self.entries.contains_key(&id) {
            return Ok(self.entries.get(&id));
        }
        let path = self.summary_path(id);
        if !path.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(&path)?;
        let summary: CommunitySummary = serde_json::from_str(&json)?;
        let _ = self.entries.insert(id, summary);
        Ok(self.entries.get(&id))
    }

    /// Mark a community's summary as stale (both in memory and on disk).
    ///
    /// A no-op if no summary exists for `community_id`.
    ///
    /// # Errors
    ///
    /// Returns [`CommunityError`] on write failure.
    pub fn mark_stale(&mut self, community_id: CommunityId) -> Result<(), CommunityError> {
        if let Some(entry) = self.entries.get_mut(&community_id) {
            entry.is_stale = true;
            // Collect path and serialized form before the mutable borrow ends.
            let path = self
                .base_dir
                .join("communities")
                .join("summaries")
                .join(format!("{}.json", community_id.0));
            let json = serde_json::to_string_pretty(entry)?;
            std::fs::write(path, json)?;
        } else {
            // Try loading from disk.
            let path = self.summary_path(community_id);
            if path.exists() {
                let json = std::fs::read_to_string(&path)?;
                let mut summary: CommunitySummary = serde_json::from_str(&json)?;
                summary.is_stale = true;
                let updated = serde_json::to_string_pretty(&summary)?;
                std::fs::write(&path, updated)?;
                let _ = self.entries.insert(community_id, summary);
            }
        }
        Ok(())
    }
}

// ── generate_summary ──────────────────────────────────────────────────────────

/// Generate a summary string for the given community members.
///
/// If `sampling` is provided and [`SamplingProvider::is_available`] returns
/// `true`, a prompt is submitted to the LLM and its response is returned.
/// On any sampling failure, or when no provider is configured, the fallback
/// is a comma-joined member list.
///
/// This function is `async` so it can await the sampling future without
/// blocking the executor.
///
/// # Examples
///
/// ```
/// # use synwire_core::agents::sampling::{NoopSamplingProvider, SamplingProvider};
/// # use synwire_index::community::summary::generate_summary;
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let members = vec!["parse_expr".to_owned(), "parse_stmt".to_owned()];
/// let p = NoopSamplingProvider;
/// let summary = generate_summary(&members, Some(&p as &dyn SamplingProvider)).await;
/// assert!(summary.contains("parse_expr"));
/// # });
/// ```
pub async fn generate_summary(
    members: &[String],
    sampling: Option<&dyn SamplingProvider>,
) -> String {
    let fallback = || format!("Members: {}", members.join(", "));

    let Some(provider) = sampling else {
        return fallback();
    };
    if !provider.is_available() {
        return fallback();
    }

    let prompt = format!(
        "Summarise this code community in one sentence. Members: {}",
        members.join(", ")
    );
    let request = SamplingRequest::new(prompt)
        .with_system(
            "You are a code analysis assistant. \
             Produce a concise one-sentence summary of a group of related code symbols.",
        )
        .with_max_tokens(128)
        .with_temperature(0.3);

    match provider.sample(request).await {
        Ok(response) => response.text,
        Err(_) => fallback(),
    }
}

/// Generate a summary and insert it into `cache` for `community_id`.
///
/// If a non-stale cached entry already exists it is returned unchanged.
///
/// # Errors
///
/// Returns [`CommunityError`] on cache write failure.
pub async fn generate_and_cache(
    cache: &mut SummaryCache,
    community_id: CommunityId,
    members: &[String],
    sampling: Option<&dyn SamplingProvider>,
) -> Result<String, CommunityError> {
    // Return cached non-stale entry immediately.
    if let Some(entry) = cache.get(community_id)? {
        if !entry.is_stale {
            return Ok(entry.summary.clone());
        }
    }

    let text = generate_summary(members, sampling).await;
    let summary = CommunitySummary::new(community_id, text.clone());
    cache.set(summary)?;
    Ok(text)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use synwire_core::agents::sampling::NoopSamplingProvider;

    #[tokio::test]
    async fn fallback_when_no_provider() {
        let members = vec!["fn_a".to_owned(), "fn_b".to_owned()];
        let s = generate_summary(&members, None).await;
        assert!(s.contains("fn_a"));
        assert!(s.contains("fn_b"));
    }

    #[tokio::test]
    async fn fallback_when_noop_provider() {
        let members = vec!["fn_a".to_owned()];
        let p = NoopSamplingProvider;
        let s = generate_summary(&members, Some(&p)).await;
        assert!(s.contains("fn_a"));
    }

    #[test]
    fn mark_stale_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let mut cache = SummaryCache::new(dir.path()).unwrap();

        let id = CommunityId(42);
        let summary = CommunitySummary::new(id, "test summary".to_owned());
        cache.set(summary).unwrap();

        cache.mark_stale(id).unwrap();

        let entry = cache.get(id).unwrap().unwrap();
        assert!(entry.is_stale);
    }

    #[tokio::test]
    async fn generate_and_cache_stores_entry() {
        let dir = tempfile::tempdir().unwrap();
        let mut cache = SummaryCache::new(dir.path()).unwrap();

        let members = vec!["sym_a".to_owned(), "sym_b".to_owned()];
        let id = CommunityId(1);
        let text = generate_and_cache(&mut cache, id, &members, None)
            .await
            .unwrap();
        assert!(text.contains("sym_a"));

        // Second call should hit the cache without re-generating.
        let text2 = generate_and_cache(&mut cache, id, &members, None)
            .await
            .unwrap();
        assert_eq!(text, text2);
    }
}
