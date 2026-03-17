//! Ochiai coefficient for SBFL ranking.

use std::collections::HashMap;

/// Coverage data for a single line in a source file.
///
/// Field naming follows standard SBFL notation:
/// `ef` / `ep` = failing / passing tests that **execute** the line;
/// `nf` / `np` = failing / passing tests that do **not** execute the line.
#[non_exhaustive]
pub struct CoverageRecord {
    /// File path.
    pub file: String,
    /// Line number.
    pub line: u32,
    /// Number of failing tests that cover this line.
    pub ef: u32,
    /// Number of passing tests that cover this line.
    pub ep: u32,
    /// Number of failing tests that do NOT cover this line.
    pub nf: u32,
    /// Number of passing tests that do NOT cover this line.
    pub np: u32,
}

impl CoverageRecord {
    /// Create a new coverage record for a single source line.
    pub const fn new(file: String, line: u32, ef: u32, ep: u32, nf: u32, np: u32) -> Self {
        Self {
            file,
            line,
            ef,
            ep,
            nf,
            np,
        }
    }
}

/// Compute the Ochiai SBFL coefficient.
///
/// Higher scores indicate higher fault likelihood.
///
/// Formula: `ef / sqrt((ef + nf) * (ef + ep))`
///
/// # Examples
///
/// ```
/// use synwire_agent::sbfl::ochiai;
/// assert_eq!(ochiai(0, 0, 5), 0.0);
/// let score = ochiai(10, 0, 0);
/// assert!(score > 0.9, "expected high score, got {score}");
/// ```
#[allow(clippy::cast_precision_loss, clippy::similar_names)]
pub fn ochiai(ef: u32, nf: u32, ep: u32) -> f32 {
    if ef == 0 {
        return 0.0;
    }
    let ef_f = ef as f32;
    let nf_f = nf as f32;
    let ep_f = ep as f32;
    ef_f / ((ef_f + nf_f) * (ef_f + ep_f)).sqrt()
}

/// Ranks source files by their maximum Ochiai score.
pub struct SbflRanker {
    records: Vec<CoverageRecord>,
}

impl SbflRanker {
    /// Create a ranker from coverage records.
    pub const fn new(records: Vec<CoverageRecord>) -> Self {
        Self { records }
    }

    /// Rank files by highest Ochiai score across all their lines.
    ///
    /// Returns `(file_path, max_ochiai_score)` sorted by score descending.
    pub fn rank_files(&self) -> Vec<(String, f32)> {
        let mut file_scores: HashMap<String, f32> = HashMap::new();
        for record in &self.records {
            let score = ochiai(record.ef, record.nf, record.ep);
            let _ = file_scores
                .entry(record.file.clone())
                .and_modify(|s| {
                    if score > *s {
                        *s = score;
                    }
                })
                .or_insert(score);
        }
        let mut ranked: Vec<_> = file_scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }
}

/// Fuse SBFL file rankings with semantic search results.
///
/// Takes SBFL file scores and semantic similarity scores, returns combined ranking.
/// `sbfl_weight` controls the relative contribution of SBFL scores; semantic
/// weight is `1.0 - sbfl_weight`.
pub fn fuse_sbfl_semantic(
    sbfl: &[(String, f32)],
    semantic: &[(String, f32)],
    sbfl_weight: f32,
) -> Vec<(String, f32)> {
    let mut combined: HashMap<String, f32> = HashMap::new();

    for (file, score) in sbfl {
        let _ = combined
            .entry(file.clone())
            .and_modify(|s| *s += sbfl_weight * score)
            .or_insert(sbfl_weight * score);
    }

    let semantic_weight = 1.0 - sbfl_weight;
    for (file, score) in semantic {
        let _ = combined
            .entry(file.clone())
            .and_modify(|s| *s += semantic_weight * score)
            .or_insert(semantic_weight * score);
    }

    let mut result: Vec<_> = combined.into_iter().collect();
    result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    result
}

#[cfg(test)]
#[allow(clippy::similar_names, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn ochiai_zero_when_no_failures() {
        assert_eq!(ochiai(0, 0, 5), 0.0);
    }

    #[test]
    fn ochiai_high_score_for_fault_concentrated() {
        // ef=10 (all failing tests hit it), nf=0, np=0
        let score = ochiai(10, 0, 0);
        assert!(score > 0.9, "expected > 0.9, got {score}");
    }

    #[test]
    fn sbfl_ranks_correctly() {
        let records = vec![
            CoverageRecord {
                file: "buggy.rs".to_owned(),
                line: 42,
                ef: 8,
                ep: 2,
                nf: 0,
                np: 0,
            },
            CoverageRecord {
                file: "clean.rs".to_owned(),
                line: 10,
                ef: 0,
                ep: 5,
                nf: 0,
                np: 5,
            },
        ];
        let ranker = SbflRanker::new(records);
        let ranked = ranker.rank_files();
        assert_eq!(ranked[0].0, "buggy.rs");
    }
}
