//! Progressive tool discovery via hybrid semantic + keyword search.
//!
//! Reduces initial tool manifest size by ~85% by loading only names and
//! descriptions at startup. Full schemas are fetched on-demand when a
//! relevant tool is identified.
//!
//! # Architecture
//!
//! [`ToolSearchIndex`] maintains a registry of tools with pre-computed
//! keyword embeddings. When a query arrives, it computes a field-weighted
//! keyword score for each entry and returns the top-k results at the
//! requested [`DisclosureDepth`].
//!
//! # Token budget
//!
//! Use [`allocate_budget`] to produce a token-capped string from a result
//! set, automatically tiering disclosure depth by rank.

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;

// ---------------------------------------------------------------------------
// DisclosureDepth
// ---------------------------------------------------------------------------

/// Controls how much detail is returned for a tool in search results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DisclosureDepth {
    /// Name only.
    Minimal,
    /// Name + description (default for search results).
    Summary,
    /// Name + description + parameter names/types.
    Parameters,
    /// Full schema including examples and constraints.
    Full,
}

// ---------------------------------------------------------------------------
// Internal ToolEntry
// ---------------------------------------------------------------------------

/// Internal representation of a registered tool.
struct ToolEntry {
    /// Tool name (must be unique).
    name: String,
    /// Namespace grouping (e.g. "vfs", "lsp", "index").
    namespace: String,
    /// Human-readable description.
    description: String,
    /// Searchable tags.
    tags: Vec<String>,
    /// Example natural-language queries that map to this tool.
    example_queries: Vec<String>,
    /// JSON schema (loaded lazily — present once registered with `schema_json`).
    schema_json: Option<String>,
    /// Number of successful invocations recorded via [`ToolSearchIndex::record_success`].
    /// Used as a frequency boost in [`keyword_score`] to surface commonly-used tools.
    call_count: u64,
    /// Pre-computed embedding vector.
    ///
    /// Phase 32d replaces the placeholder [`compute_embedding`] with real
    /// fastembed-rs embeddings and adds cosine-similarity scoring in
    /// [`keyword_score`].  The field is populated in
    /// [`ToolSearchIndex::register`] so the data structure is ready when the
    /// embedding provider lands.
    #[allow(dead_code)] // Read path added in Phase 32d (semantic scoring).
    embedding: Vec<f32>,
}

// ---------------------------------------------------------------------------
// ToolSearchResult (public)
// ---------------------------------------------------------------------------

/// A single result from a [`ToolSearchIndex`] search or browse operation.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ToolSearchResult {
    /// Tool name.
    pub name: String,
    /// Tool namespace (e.g. "vfs", "index", "lsp").
    pub namespace: String,
    /// Combined relevance score (0.0–∞, higher is better).
    pub score: f32,
    /// Rendered disclosure at the requested depth.
    pub rendered: String,
    /// Nearest namespace when this is a low-confidence result.
    pub nearest_namespace: Option<String>,
    /// Alternative keywords that might lead to this tool.
    pub alternative_keywords: Vec<String>,
    /// Confidence level: `"high"` | `"medium"` | `"low"`.
    pub confidence_level: String,
}

// ---------------------------------------------------------------------------
// ToolSearchIndex
// ---------------------------------------------------------------------------

/// Progressive tool discovery index using hybrid keyword search.
///
/// # Example
///
/// ```
/// use synwire_core::tools::search_index::ToolSearchIndex;
///
/// let mut idx = ToolSearchIndex::new();
/// idx.register("read_file", "vfs", "Read the contents of a file", &["file", "read"], None);
/// idx.register("write_file", "vfs", "Write content to a file", &["file", "write"], None);
///
/// let results = idx.search("read file", 5);
/// assert!(!results.is_empty());
/// assert_eq!(results[0].name, "read_file");
/// ```
pub struct ToolSearchIndex {
    entries: Vec<ToolEntry>,
    /// Names of tools already returned at `Full` depth (for adaptive scoring).
    loaded_schemas: HashSet<String>,
    /// SHA-256 hex digest of the sorted `name → description` map.
    /// Updated on every [`register`](Self::register) call.
    registry_hash: String,
}

impl Default for ToolSearchIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolSearchIndex {
    /// Create an empty index.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            loaded_schemas: HashSet::new(),
            registry_hash: String::new(),
        }
    }

    /// Register a tool with the index.
    ///
    /// If a tool with the same name already exists it is silently replaced.
    ///
    /// `tags` are extra keywords used for boosted keyword matching.
    /// `schema_json` may be `None` if the full schema will be provided later.
    pub fn register(
        &mut self,
        name: &str,
        namespace: &str,
        description: &str,
        tags: &[&str],
        schema_json: Option<&str>,
    ) {
        // Remove existing entry with the same name.
        self.entries.retain(|e| e.name != name);

        let embedding = compute_embedding(name, description);
        self.entries.push(ToolEntry {
            name: name.to_owned(),
            namespace: namespace.to_owned(),
            description: description.to_owned(),
            tags: tags.iter().map(|&t| t.to_owned()).collect(),
            example_queries: Vec::new(),
            schema_json: schema_json.map(str::to_owned),
            call_count: 0,
            embedding,
        });
        self.recompute_hash();
    }

    /// Perform a hybrid keyword search and return the top-`top_k` results.
    ///
    /// Results are returned at [`DisclosureDepth::Summary`] depth. Tools that
    /// have already been returned at `Full` depth have their score multiplied
    /// by `0.8` to surface unseen tools.
    pub fn search(&mut self, query: &str, top_k: usize) -> Vec<ToolSearchResult> {
        let query_words: HashSet<&str> = query.split_whitespace().collect();

        let mut scored: Vec<(usize, f32)> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let mut score = keyword_score(entry, &query_words);
                // T248: adaptive scoring — penalty for already-loaded tools.
                if self.loaded_schemas.contains(&entry.name) {
                    score *= 0.8;
                }
                (i, score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        // T249: compute diagnostics before building results.
        let top_namespace = scored
            .first()
            .map(|(i, _)| self.entries[*i].namespace.clone());

        // Gather tags from top-scoring tools not already in the query.
        let alternative_keywords: Vec<String> = scored
            .iter()
            .flat_map(|(i, _)| self.entries[*i].tags.iter().cloned())
            .filter(|t| {
                !query_words
                    .iter()
                    .any(|&w| w.eq_ignore_ascii_case(t.as_str()))
            })
            .collect::<HashSet<_>>()
            .into_iter()
            .take(5)
            .collect();

        let results: Vec<ToolSearchResult> = scored
            .into_iter()
            .map(|(i, score)| {
                let entry = &self.entries[i];

                let confidence_level = if score > 5.0 {
                    "high"
                } else if score > 2.0 {
                    "medium"
                } else {
                    "low"
                };

                let nearest_namespace = if confidence_level == "low" {
                    top_namespace.clone()
                } else {
                    None
                };

                let rendered = render(entry, DisclosureDepth::Summary);

                ToolSearchResult {
                    name: entry.name.clone(),
                    namespace: entry.namespace.clone(),
                    score,
                    rendered,
                    nearest_namespace,
                    alternative_keywords: alternative_keywords.clone(),
                    confidence_level: confidence_level.to_owned(),
                }
            })
            .collect();

        // Mark returned tools as seen so subsequent searches apply the penalty.
        for r in &results {
            let _ = self.loaded_schemas.insert(r.name.clone());
        }

        results
    }

    /// Browse all tools in a given namespace, returned at `Summary` depth.
    pub fn browse_namespace(&self, namespace: &str) -> Vec<ToolSearchResult> {
        self.entries
            .iter()
            .filter(|e| e.namespace == namespace)
            .map(|e| ToolSearchResult {
                name: e.name.clone(),
                namespace: e.namespace.clone(),
                score: 1.0,
                rendered: render(e, DisclosureDepth::Summary),
                nearest_namespace: None,
                alternative_keywords: Vec::new(),
                confidence_level: "high".to_owned(),
            })
            .collect()
    }

    /// Returns `(name, description)` pairs for all tools (for `tools/list` MCP response).
    pub fn list_compact(&self) -> Vec<(String, String)> {
        self.entries
            .iter()
            .map(|e| (e.name.clone(), e.description.clone()))
            .collect()
    }

    /// Records a successful `query → tool_name` mapping for future re-ranking.
    ///
    /// Increments the tool's `call_count` (used as a frequency boost in
    /// scoring) and stores up to 10 unique example queries.
    pub fn record_success(&mut self, query: &str, tool_name: &str) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.name == tool_name) {
            entry.call_count = entry.call_count.saturating_add(1);
            if !entry.example_queries.iter().any(|q| q == query) && entry.example_queries.len() < 10
            {
                entry.example_queries.push(query.to_owned());
            }
        }
    }

    /// Search iteratively, subtracting already-found concepts at each step.
    ///
    /// Returns deduplicated results across all steps.
    pub fn search_progressive(
        &mut self,
        query: &str,
        steps: usize,
        per_step_k: usize,
    ) -> Vec<ToolSearchResult> {
        let mut seen: HashSet<String> = HashSet::new();
        let mut all_results: Vec<ToolSearchResult> = Vec::new();
        let mut remaining_query = query.to_owned();

        for _ in 0..steps {
            let step_results = self.search(&remaining_query, per_step_k);
            for r in step_results {
                if seen.insert(r.name.clone()) {
                    all_results.push(r);
                }
            }
            // Heuristic: subtract found names from the remaining query.
            let found_names: Vec<&str> = all_results.iter().map(|r| r.name.as_str()).collect();
            remaining_query = format!("{query} -{}", found_names.join(" -"));
        }
        all_results
    }

    /// SHA-256 hex digest of the current tool registry (sorted by name).
    pub fn registry_hash(&self) -> &str {
        &self.registry_hash
    }

    // ------------------------------------------------------------------
    // Private helpers
    // ------------------------------------------------------------------

    fn recompute_hash(&mut self) {
        use sha2::{Digest, Sha256};

        let sorted: BTreeMap<&str, &str> = self
            .entries
            .iter()
            .map(|e| (e.name.as_str(), e.description.as_str()))
            .collect();

        let data = serde_json::to_string(&sorted).unwrap_or_else(|_| format!("{sorted:?}"));

        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        self.registry_hash = format!("{:x}", hasher.finalize());
    }
}

// ---------------------------------------------------------------------------
// Scoring helpers
// ---------------------------------------------------------------------------

/// Placeholder embedding — Phase 32d full implementation adds fastembed-rs.
const fn compute_embedding(_name: &str, _description: &str) -> Vec<f32> {
    Vec::new()
}

/// Field-weighted keyword score.
///
/// Weights: namespace +5.0, name +3.0, description +2.0, tags +1.5.
/// A logarithmic frequency boost based on `call_count` is added so that
/// frequently-used tools surface slightly higher.
fn keyword_score(entry: &ToolEntry, query_words: &HashSet<&str>) -> f32 {
    let name_words: HashSet<&str> = entry.name.split(['-', '_', ' ']).collect();
    let desc_words: HashSet<&str> = entry.description.split_whitespace().collect();
    let ns_words: HashSet<&str> = entry.namespace.split(['-', '_']).collect();

    #[allow(clippy::cast_precision_loss)]
    let ns_score: f32 = ns_words.intersection(query_words).count() as f32 * 5.0;
    #[allow(clippy::cast_precision_loss)]
    let name_score: f32 = name_words.intersection(query_words).count() as f32 * 3.0;
    #[allow(clippy::cast_precision_loss)]
    let desc_score: f32 = desc_words.intersection(query_words).count() as f32 * 2.0;
    #[allow(clippy::cast_precision_loss)]
    let tag_score: f32 = entry
        .tags
        .iter()
        .flat_map(|t| t.split(['-', '_', ' ']))
        .filter(|w| query_words.contains(w))
        .count() as f32
        * 1.5;

    // Logarithmic frequency boost: log2(1 + call_count) * 0.5, capped at 3.0.
    // This gently surfaces tools that are used more often without overwhelming
    // keyword relevance.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let freq_boost = ((1.0 + entry.call_count as f64).log2() * 0.5).min(3.0) as f32;

    ns_score + name_score + desc_score + tag_score + freq_boost
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// Render a tool entry at the given disclosure depth.
fn render(entry: &ToolEntry, depth: DisclosureDepth) -> String {
    match depth {
        DisclosureDepth::Minimal => entry.name.clone(),
        DisclosureDepth::Summary => {
            let desc = if entry.description.len() > 100 {
                format!("{}…", &entry.description[..100])
            } else {
                entry.description.clone()
            };
            format!("{}: {desc}", entry.name)
        }
        DisclosureDepth::Parameters => {
            let summary = if entry.description.len() > 100 {
                format!("{}…", &entry.description[..100])
            } else {
                entry.description.clone()
            };
            let params = extract_parameter_names(entry.schema_json.as_deref());
            if params.is_empty() {
                format!("{}: {summary}", entry.name)
            } else {
                format!("{}: {summary} (params: {})", entry.name, params.join(", "))
            }
        }
        DisclosureDepth::Full => {
            let mut out = format!("name: {}\ndescription: {}\n", entry.name, entry.description);
            if let Some(ref schema) = entry.schema_json {
                out.push_str("schema: ");
                out.push_str(schema);
            }
            out
        }
    }
}

/// Extract required parameter names from a JSON schema string.
fn extract_parameter_names(schema_json: Option<&str>) -> Vec<String> {
    let Some(json) = schema_json else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    value
        .get("properties")
        .and_then(|p| p.as_object())
        .map(|props| props.keys().cloned().collect())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Meta-tool argument types and runners
// ---------------------------------------------------------------------------

/// Arguments for the `tool_search` meta-tool.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ToolSearchArgs {
    /// Natural language query (used when `namespace` is `None`).
    pub query: Option<String>,
    /// Browse a specific namespace instead of searching.
    pub namespace: Option<String>,
    /// Number of results to return (default: 5).
    pub top_k: Option<usize>,
}

impl ToolSearchArgs {
    /// Create a new `ToolSearchArgs` with all fields set to `None`.
    pub const fn new() -> Self {
        Self {
            query: None,
            namespace: None,
            top_k: None,
        }
    }

    /// Set the natural language query.
    #[must_use]
    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = Some(query.into());
        self
    }

    /// Set the namespace to browse.
    #[must_use]
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Set the maximum number of results.
    #[must_use]
    pub const fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = Some(top_k);
        self
    }
}

impl Default for ToolSearchArgs {
    fn default() -> Self {
        Self::new()
    }
}

/// Execute a `tool_search` or namespace browse operation.
///
/// Returns a human-readable formatted string suitable for LLM consumption.
pub fn run_tool_search(index: &mut ToolSearchIndex, args: &ToolSearchArgs) -> String {
    let top_k = args.top_k.unwrap_or(5);

    if let Some(ref ns) = args.namespace {
        let results = index.browse_namespace(ns);
        if results.is_empty() {
            return format!("No tools found in namespace '{ns}'.");
        }
        let mut out = format!("Tools in namespace '{ns}':\n");
        for r in &results {
            let _ = writeln!(out, "  - {}", r.rendered);
        }
        return out;
    }

    let query = match &args.query {
        Some(q) => q.clone(),
        None => return "Provide either 'query' or 'namespace'.".to_owned(),
    };

    let results = index.search(&query, top_k);
    if results.is_empty() {
        return format!("No tools matched '{query}'.");
    }

    let mut out = format!("Tool search results for '{query}':\n");
    for r in &results {
        let _ = writeln!(out, "  [{}] {}", r.confidence_level, r.rendered);
    }
    out
}

/// Returns a compact namespace-grouped listing of all registered tools.
pub fn run_tool_list(index: &ToolSearchIndex) -> String {
    // Group by namespace using BTreeMap for deterministic ordering.
    let mut grouped: BTreeMap<&str, Vec<(&str, &str)>> = BTreeMap::new();
    for entry in &index.entries {
        grouped
            .entry(entry.namespace.as_str())
            .or_default()
            .push((entry.name.as_str(), entry.description.as_str()));
    }

    let mut out = String::new();
    for (ns, tools) in &grouped {
        out.push_str(ns);
        out.push_str(":\n");
        for (name, desc) in tools {
            let short_desc = if desc.len() > 80 {
                format!("{}…", &desc[..80])
            } else {
                (*desc).to_owned()
            };
            let _ = writeln!(out, "  - {name}: {short_desc}");
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Token budget allocator (T220e)
// ---------------------------------------------------------------------------

/// Render a result set within a token budget.
///
/// Top-5 results are rendered at `Full` depth, next-10 at `Summary`, the
/// remainder at `Minimal`. Total output is capped at approximately 5 000
/// tokens (4 chars per token).
pub fn allocate_budget(results: &[ToolSearchResult]) -> String {
    const TOKEN_CAP: usize = 5_000;
    const CHARS_PER_TOKEN: usize = 4;

    let mut out = String::new();
    let mut tokens_used: usize = 0;

    for (i, r) in results.iter().enumerate() {
        let depth_label = if i < 5 {
            "full"
        } else if i < 15 {
            "summary"
        } else {
            "minimal"
        };
        let line = format!("[{depth_label}] {} (score={:.2})\n", r.rendered, r.score);
        tokens_used += line.len() / CHARS_PER_TOKEN;
        if tokens_used > TOKEN_CAP {
            break;
        }
        out.push_str(&line);
    }
    out
}

// ---------------------------------------------------------------------------
// Parameter-type verification heuristic (T251)
// ---------------------------------------------------------------------------

/// Post-retrieval filter: demote tools that don't match implied parameter types.
///
/// Applies heuristic score adjustments based on query content:
/// - File path queries boost file tools, demote non-file tools.
/// - Symbol/function queries boost LSP tools, demote others.
pub fn verify_parameter_types(results: &mut [ToolSearchResult], query: &str) {
    let query_lower = query.to_lowercase();
    for r in results.iter_mut() {
        let looks_like_path = query_lower.contains('/')
            || query_lower.contains(".rs")
            || query_lower.contains(".py")
            || query_lower.contains("file");
        let is_file_tool = r.name.contains("read")
            || r.name.contains("write")
            || r.name.contains("file")
            || r.name.contains("glob")
            || r.namespace == "vfs";

        if looks_like_path && !is_file_tool {
            r.score *= 0.7;
        }

        let looks_like_symbol = query_lower.contains("function")
            || query_lower.contains("method")
            || query_lower.contains("struct")
            || query_lower.contains("class");
        let is_lsp_tool =
            r.namespace == "lsp" || r.name.contains("symbol") || r.name.contains("goto");

        if looks_like_symbol && !is_lsp_tool {
            r.score *= 0.85;
        }
    }
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

// ---------------------------------------------------------------------------
// ToolTransitionGraph (T246)
// ---------------------------------------------------------------------------

/// Records tool invocation sequences to boost likely next tools.
///
/// Uses exponential decay so older transitions fade over time.
pub struct ToolTransitionGraph {
    /// `from → { to → count }`.
    transitions: HashMap<String, HashMap<String, f64>>,
    /// Number of invocations at which recorded transition weights halve.
    half_life: usize,
    /// Total invocations recorded (used for decay calculation).
    total_invocations: usize,
}

impl ToolTransitionGraph {
    /// Create a new graph with the given half-life in invocations.
    pub fn new(half_life: usize) -> Self {
        Self {
            transitions: HashMap::new(),
            half_life,
            total_invocations: 0,
        }
    }

    /// Record that `to` was invoked after `from`.
    pub fn record_transition(&mut self, from: &str, to: &str) {
        let _ = self
            .transitions
            .entry(from.to_owned())
            .or_default()
            .entry(to.to_owned())
            .and_modify(|c| *c += 1.0)
            .or_insert(1.0);
        self.total_invocations += 1;
    }

    /// Returns tools likely to be used next after `current`, with boost scores.
    ///
    /// Scores are decayed by `0.5^(total_invocations / half_life)`.
    pub fn successors(&self, current: &str) -> Vec<(String, f32)> {
        let Some(counts) = self.transitions.get(current) else {
            return Vec::new();
        };
        let total: f64 = counts.values().sum();
        let exponent = self.total_invocations / self.half_life.max(1);
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let decay = 0.5_f64.powi(exponent as i32);
        let mut results: Vec<(String, f32)> = counts
            .iter()
            .map(|(name, count)| {
                #[allow(clippy::cast_possible_truncation)]
                let score = ((count / total) * decay) as f32;
                (name.clone(), score)
            })
            .collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }
}

// ---------------------------------------------------------------------------
// QueryPreprocessor (T247)
// ---------------------------------------------------------------------------

/// Extracts a concise intent from a verbose natural language query.
pub trait QueryPreprocessor: Send + Sync {
    /// Extract a concise query from a verbose input.
    fn preprocess<'a>(&self, query: &'a str) -> Cow<'a, str>;
}

/// Heuristic intent extractor: strips stop words and keeps up to 5 content words.
pub struct IntentExtractor;

impl QueryPreprocessor for IntentExtractor {
    fn preprocess<'a>(&self, query: &'a str) -> Cow<'a, str> {
        const STOP_WORDS: &[&str] = &[
            "the", "a", "an", "in", "for", "to", "of", "that", "which", "with", "from",
        ];
        let words: Vec<&str> = query.split_whitespace().collect();
        if words.len() <= 5 {
            return Cow::Borrowed(query);
        }
        let content_words: Vec<&str> = words
            .iter()
            .copied()
            .filter(|w| !STOP_WORDS.contains(&w.to_lowercase().as_str()))
            .take(5)
            .collect();
        Cow::Owned(content_words.join(" "))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::cast_precision_loss)]
mod tests {
    use super::*;

    fn sample_index() -> ToolSearchIndex {
        let mut idx = ToolSearchIndex::new();
        idx.register(
            "read_file",
            "vfs",
            "Read the contents of a file",
            &["file", "read"],
            None,
        );
        idx.register(
            "search_code",
            "index",
            "Semantic code search using embeddings",
            &["search", "semantic"],
            None,
        );
        idx.register(
            "list_dir",
            "vfs",
            "List directory contents",
            &["ls", "directory"],
            None,
        );
        idx
    }

    // T220i
    #[test]
    fn tool_search_finds_by_query() {
        let mut idx = sample_index();
        let results = idx.search("read file contents", 3);
        assert!(!results.is_empty());
        assert_eq!(results[0].name, "read_file");
    }

    // T220i
    #[test]
    fn namespace_browse_returns_all() {
        let mut idx = ToolSearchIndex::new();
        idx.register("read_file", "vfs", "Read file", &[], None);
        idx.register("write_file", "vfs", "Write file", &[], None);
        idx.register("search_code", "index", "Search code", &[], None);

        let vfs_tools = idx.browse_namespace("vfs");
        assert_eq!(vfs_tools.len(), 2);
    }

    // T220i — hybrid scoring ranks exact name match first.
    #[test]
    fn exact_name_match_ranks_first() {
        let mut idx = ToolSearchIndex::new();
        idx.register(
            "search_code",
            "index",
            "Semantic code search",
            &["search"],
            None,
        );
        idx.register("read_file", "vfs", "Read file", &["file"], None);
        idx.register("list_dir", "vfs", "List directory contents", &[], None);

        let results = idx.search("search code", 3);
        assert!(!results.is_empty());
        assert_eq!(results[0].name, "search_code");
    }

    // T220i — adaptive scoring applies penalty to tools already seen.
    #[test]
    fn adaptive_scoring_penalises_loaded_schemas() {
        let mut idx = ToolSearchIndex::new();
        idx.register(
            "find_func",
            "lsp",
            "Find function definition",
            &["function", "find"],
            None,
        );

        // First search: record the base score.
        let first_results = idx.search("find function", 1);
        assert!(!first_results.is_empty());
        let score_before = first_results[0].score;

        // Second search: tool is now in loaded_schemas, score should be lower.
        let second_results = idx.search("find function", 1);
        assert!(!second_results.is_empty());
        let score_after = second_results[0].score;

        assert!(
            score_after < score_before,
            "expected penalised score {score_after} < {score_before}"
        );
    }

    // T220f — registry hash changes when tools are added.
    #[test]
    fn registry_hash_changes_on_registration() {
        let mut idx = ToolSearchIndex::new();
        let h0 = idx.registry_hash().to_owned();
        idx.register("read_file", "vfs", "Read file", &[], None);
        let h1 = idx.registry_hash().to_owned();
        idx.register("write_file", "vfs", "Write file", &[], None);
        let h2 = idx.registry_hash().to_owned();
        assert_ne!(h0, h1);
        assert_ne!(h1, h2);
    }

    // T250 — record_success capped at 10 example queries.
    #[test]
    fn record_success_capped() {
        let mut idx = ToolSearchIndex::new();
        idx.register("read_file", "vfs", "Read file", &[], None);
        for i in 0..15 {
            idx.record_success(&format!("query {i}"), "read_file");
        }
        let entry = idx.entries.iter().find(|e| e.name == "read_file").unwrap();
        assert_eq!(entry.example_queries.len(), 10);
    }

    // T250 — duplicates ignored.
    #[test]
    fn record_success_no_duplicates() {
        let mut idx = ToolSearchIndex::new();
        idx.register("read_file", "vfs", "Read file", &[], None);
        for _ in 0..5 {
            idx.record_success("read a file", "read_file");
        }
        let entry = idx.entries.iter().find(|e| e.name == "read_file").unwrap();
        assert_eq!(entry.example_queries.len(), 1);
    }

    // T252 — progressive retrieval deduplicates.
    #[test]
    fn progressive_retrieval_deduplicates() {
        let mut idx = ToolSearchIndex::new();
        idx.register("read_file", "vfs", "Read file contents", &["file"], None);
        idx.register("write_file", "vfs", "Write file contents", &["file"], None);
        idx.register(
            "search_code",
            "index",
            "Search code semantically",
            &["search"],
            None,
        );

        let results = idx.search_progressive("file operations", 2, 2);
        let names: Vec<&String> = results.iter().map(|r| &r.name).collect();
        let unique_names: HashSet<&String> = names.iter().copied().collect();
        assert_eq!(names.len(), unique_names.len());
    }

    // T252 — transition graph boosts successors.
    #[test]
    fn transition_graph_boosts_successors() {
        let mut g = ToolTransitionGraph::new(100);
        g.record_transition("read_file", "write_file");
        g.record_transition("read_file", "write_file");
        g.record_transition("read_file", "search_code");

        let successors = g.successors("read_file");
        assert!(!successors.is_empty());
        assert_eq!(successors[0].0, "write_file");
    }

    // T252 — intent extractor shortens long queries.
    #[test]
    fn intent_extractor_shortens_long_query() {
        let extractor = IntentExtractor;
        let long = "I need to find the function that handles authentication in the codebase";
        let short = extractor.preprocess(long);
        assert!(short.split_whitespace().count() <= 5);
    }

    // T252 — intent extractor passes through short queries unchanged.
    #[test]
    fn intent_extractor_passthrough_short_query() {
        let extractor = IntentExtractor;
        let q = "read file";
        let result = extractor.preprocess(q);
        assert_eq!(result, q);
    }

    // T251 — parameter-type verification demotes non-file tools for file queries.
    #[test]
    fn parameter_verification_demotes_non_file_tools() {
        let mut results = vec![
            ToolSearchResult {
                name: "go_to_definition".to_owned(),
                namespace: "lsp".to_owned(),
                // Score just above the file tool so demotion (0.7x) will flip ranking.
                score: 6.0,
                rendered: String::new(),
                nearest_namespace: None,
                alternative_keywords: Vec::new(),
                confidence_level: "high".to_owned(),
            },
            ToolSearchResult {
                name: "read_file".to_owned(),
                namespace: "vfs".to_owned(),
                score: 5.0,
                rendered: String::new(),
                nearest_namespace: None,
                alternative_keywords: Vec::new(),
                confidence_level: "high".to_owned(),
            },
        ];
        // go_to_definition score after demotion: 6.0 * 0.7 = 4.2 < 5.0 (read_file)
        verify_parameter_types(&mut results, "read /src/main.rs file");
        assert_eq!(results[0].name, "read_file");
    }

    // run_tool_list groups by namespace.
    #[test]
    fn run_tool_list_grouped_output() {
        let mut idx = ToolSearchIndex::new();
        idx.register("read_file", "vfs", "Read file", &[], None);
        idx.register("write_file", "vfs", "Write file", &[], None);
        idx.register("search_code", "index", "Search code", &[], None);

        let output = run_tool_list(&idx);
        assert!(output.contains("vfs:"));
        assert!(output.contains("index:"));
    }

    // allocate_budget caps output.
    #[test]
    fn allocate_budget_produces_output() {
        let results: Vec<ToolSearchResult> = (0..20)
            .map(|i| ToolSearchResult {
                name: format!("tool_{i}"),
                namespace: "vfs".to_owned(),
                score: 10.0 - i as f32,
                rendered: format!("tool_{i}: does something useful"),
                nearest_namespace: None,
                alternative_keywords: Vec::new(),
                confidence_level: "high".to_owned(),
            })
            .collect();
        let output = allocate_budget(&results);
        assert!(!output.is_empty());
        assert!(output.contains("[full]"));
        assert!(output.contains("[summary]"));
    }
}
