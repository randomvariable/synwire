//! Maximal Marginal Relevance (MMR) algorithm.
//!
//! MMR balances relevance to the query with diversity among selected results,
//! controlled by the `lambda` parameter.

/// Cosine similarity between two vectors.
///
/// Returns a value in `[-1.0, 1.0]`. Returns `0.0` if either vector has zero
/// magnitude.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    let denom = mag_a * mag_b;
    if denom < f32::EPSILON {
        return 0.0;
    }
    dot / denom
}

/// Selects indices using Maximal Marginal Relevance.
///
/// Balances relevance to `query_embedding` with diversity among the selected
/// embeddings:
///
/// - `lambda = 1.0`: pure relevance (equivalent to top-k similarity)
/// - `lambda = 0.0`: maximum diversity
///
/// Returns up to `k` indices into the `embeddings` slice.
pub fn maximal_marginal_relevance(
    query_embedding: &[f32],
    embeddings: &[Vec<f32>],
    k: usize,
    lambda: f32,
) -> Vec<usize> {
    if embeddings.is_empty() || k == 0 {
        return Vec::new();
    }

    let k = k.min(embeddings.len());

    // Pre-compute query similarities
    let query_sims: Vec<f32> = embeddings
        .iter()
        .map(|e| cosine_similarity(query_embedding, e))
        .collect();

    let mut selected: Vec<usize> = Vec::with_capacity(k);
    let mut remaining: Vec<usize> = (0..embeddings.len()).collect();

    // Select the most relevant document first
    let first = remaining.iter().copied().max_by(|&a, &b| {
        query_sims[a]
            .partial_cmp(&query_sims[b])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if let Some(first_idx) = first {
        selected.push(first_idx);
        remaining.retain(|&i| i != first_idx);
    }

    // Greedily select remaining documents
    while selected.len() < k && !remaining.is_empty() {
        let mut best_idx = None;
        let mut best_score = f32::NEG_INFINITY;

        for &candidate in &remaining {
            let relevance = query_sims[candidate];

            // Maximum similarity to any already-selected document
            let max_sim_to_selected = selected
                .iter()
                .map(|&s| cosine_similarity(&embeddings[candidate], &embeddings[s]))
                .fold(f32::NEG_INFINITY, f32::max);

            let mmr_score = lambda.mul_add(relevance, -(1.0 - lambda) * max_sim_to_selected);

            if mmr_score > best_score {
                best_score = mmr_score;
                best_idx = Some(candidate);
            }
        }

        if let Some(idx) = best_idx {
            selected.push(idx);
            remaining.retain(|&i| i != idx);
        } else {
            break;
        }
    }

    selected
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn cosine_similarity_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_similarity_orthogonal_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-5);
    }

    #[test]
    fn cosine_similarity_zero_vector() {
        let a = vec![1.0, 2.0];
        let b = vec![0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-5);
    }

    #[test]
    fn mmr_empty_embeddings() {
        let query = vec![1.0, 0.0];
        let result = maximal_marginal_relevance(&query, &[], 5, 0.5);
        assert!(result.is_empty());
    }

    #[test]
    fn mmr_k_zero() {
        let query = vec![1.0, 0.0];
        let embeddings = vec![vec![1.0, 0.0]];
        let result = maximal_marginal_relevance(&query, &embeddings, 0, 0.5);
        assert!(result.is_empty());
    }

    #[test]
    fn mmr_selects_k_results() {
        let query = vec![1.0, 0.0];
        let embeddings = vec![
            vec![1.0, 0.0],
            vec![0.9, 0.1],
            vec![0.0, 1.0],
            vec![0.5, 0.5],
        ];
        let result = maximal_marginal_relevance(&query, &embeddings, 3, 0.5);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn mmr_pure_relevance_selects_most_similar() {
        let query = vec![1.0, 0.0];
        let embeddings = vec![
            vec![0.0, 1.0], // orthogonal
            vec![1.0, 0.0], // identical
            vec![0.5, 0.5], // medium
        ];
        let result = maximal_marginal_relevance(&query, &embeddings, 1, 1.0);
        assert_eq!(result, vec![1]); // Most similar
    }

    #[test]
    fn mmr_diverse_selection() {
        // Two very similar embeddings and one different
        let query = vec![1.0, 0.0];
        let embeddings = vec![
            vec![1.0, 0.0],   // most similar to query
            vec![0.99, 0.01], // very similar to [0]
            vec![0.0, 1.0],   // very different
        ];
        // With low lambda (high diversity), MMR should pick the diverse one second
        let result = maximal_marginal_relevance(&query, &embeddings, 2, 0.0);
        assert_eq!(result[0], 0); // First pick: most relevant
        assert_eq!(result[1], 2); // Second pick: most diverse
    }

    #[test]
    fn mmr_k_exceeds_embeddings() {
        let query = vec![1.0, 0.0];
        let embeddings = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let result = maximal_marginal_relevance(&query, &embeddings, 10, 0.5);
        assert_eq!(result.len(), 2);
    }
}
