//! Hybrid BM25 + vector search combining keyword recall with semantic similarity.
//!
//! The [`hybrid_search`] function accepts pre-computed vector search results
//! alongside a [`Bm25Index`] and merges them using a weighted combination:
//!
//! ```text
//! score = alpha * bm25_score_normalised + (1 - alpha) * vector_score
//! ```
//!
//! Setting `alpha = 1.0` gives pure BM25; `alpha = 0.0` gives pure vector;
//! `alpha = 0.5` (the default) gives equal weight to both signals.

use std::collections::HashMap;

use crate::bm25::{Bm25Error, Bm25Index};

/// Configuration for hybrid BM25 + vector search.
///
/// `alpha` controls the interpolation between BM25 and vector scores:
/// - `1.0` → pure BM25 keyword recall
/// - `0.0` → pure vector semantic similarity
/// - `0.5` → balanced (default)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct HybridSearchConfig {
    /// Weight applied to the normalised BM25 score (must be in `[0.0, 1.0]`).
    pub alpha: f32,
    /// Maximum number of results to return.
    pub top_k: usize,
}

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            alpha: 0.5,
            top_k: 10,
        }
    }
}

/// A single result from a hybrid search.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct HybridResult {
    /// Document identifier.
    pub id: String,
    /// Source file path.
    pub file: String,
    /// Optional symbol name (function, class, etc.) if indexed.
    pub symbol: Option<String>,
    /// Document content snippet (not available from BM25 alone; populated from
    /// metadata when the caller enriches results).
    pub content: String,
    /// Combined relevance score.
    pub score: f32,
}

/// Combine BM25 and vector search results into a ranked list.
///
/// # Algorithm
///
/// 1. Run BM25 on `query` with `top_k * 2` candidates.
/// 2. Normalise BM25 scores to `[0, 1]` by dividing by the maximum score.
/// 3. Build a map of `id → vector_score` from `vector_results`.
/// 4. For each BM25 candidate: `score = alpha * bm25_norm + (1 - alpha) * vector_score_or_0`.
/// 5. Sort descending by combined score, return `top_k`.
///
/// # Errors
///
/// Propagates [`Bm25Error`] from the underlying BM25 search.
pub fn hybrid_search(
    bm25: &Bm25Index,
    vector_results: &[(String, f32)],
    query: &str,
    config: &HybridSearchConfig,
) -> Result<Vec<HybridResult>, Bm25Error> {
    let candidates = bm25.search(query, config.top_k.saturating_mul(2))?;

    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    // Normalise BM25 scores.
    let max_bm25 = candidates
        .iter()
        .map(|r| r.score)
        .fold(f32::NEG_INFINITY, f32::max);

    // Build vector score lookup.
    let vector_map: HashMap<&str, f32> = vector_results
        .iter()
        .map(|(id, score)| (id.as_str(), *score))
        .collect();

    // Combine scores.
    let alpha = config.alpha.clamp(0.0, 1.0);
    let mut combined: Vec<(String, f32)> = candidates
        .into_iter()
        .map(|r| {
            let norm = if max_bm25 > 0.0 {
                r.score / max_bm25
            } else {
                0.0
            };
            let vec_score = vector_map.get(r.id.as_str()).copied().unwrap_or(0.0);
            let score = alpha * norm + (1.0 - alpha) * vec_score;
            (r.id, score)
        })
        .collect();

    combined.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    combined.truncate(config.top_k);

    // We return placeholder content/file/symbol — the caller enriches these
    // from the vector store or chunk cache if needed.
    let results = combined
        .into_iter()
        .map(|(id, score)| HybridResult {
            id,
            file: String::new(),
            symbol: None,
            content: String::new(),
            score,
        })
        .collect();

    Ok(results)
}
