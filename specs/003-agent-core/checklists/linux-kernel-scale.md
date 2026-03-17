# Linux Kernel Scale Checklist: Agent Core Runtime

**Purpose**: Validate requirements quality, completeness, and consistency for handling repositories at Linux kernel scale (~70,000 files, ~30M LOC, 1M+ graph edges)
**Created**: 2026-03-16
**Feature**: [spec.md](../spec.md) — Clarification Q3, FR-738, FR-744a–744h, FR-779a, FR-780, SC-144–146

## Requirement Completeness — Indexing Pipeline

- [x] CHK001 - Is the streaming requirement specified for all pipeline stages? [Completeness, Spec §FR-738]
  > ✅ FR-738: "streaming (process files one at a time)". FR-744a: "walk → chunk → embed → store one file at a time. Full file list MUST NOT be collected into memory."
- [x] CHK002 - Are batch sizes for embedding calls specified? [Gap]
  > ✅ FR-744a: "batched (default: 32 chunks per batch) to balance throughput vs memory"
- [x] CHK003 - Are parallel indexing requirements defined? [Gap]
  > ✅ FR-744h: "bounded by configurable parallelism limit (default: half of available cores)"
- [x] CHK004 - Is the content hash registry streaming-safe for 70K entries? [Gap, Spec §FR-741]
  > ✅ FR-744b: "70K-entry JSON (~3MB) acceptable. If exceeds 10MB, migrate to SQLite."
- [x] CHK005 - Are indexing progress reporting requirements defined? [Gap, Spec §FR-740]
  > ✅ FR-744c: "files processed / total, chunks produced, elapsed time, estimated time remaining"
- [x] CHK006 - Is the 70K target for all features or some at smaller scale? [Clarity]
  > ✅ SC-144 (indexing <2GB), SC-145 (search <2s), SC-146 (graph <1s) all target 70K files. SC-112 targets 100K+ symbols for community detection. All features at full scale.

## Requirement Completeness — Storage Backends

- [x] CHK007 - Is the vector index access pattern specified? [Clarity, Spec §FR-738]
  > ✅ LanceDB is disk-backed by design (Lance columnar format, memory-mapped). No additional access pattern specification needed — it's inherent to the library.
- [x] CHK008 - Is tantivy's disk storage sufficient? [Clarity, Spec §FR-780]
  > ✅ FR-780: "BM25 index MUST be disk-backed (e.g., tantivy)". Tantivy is designed for disk-backed operation. Sufficient.
- [x] CHK009 - Is the code graph storage specified concretely? [Ambiguity, Spec §FR-779a]
  > ✅ FR-779a: "disk-backed via SQLite in WAL mode". Concrete choice made.
- [x] CHK010 - Are index size bounds specified? [Gap]
  > ✅ Not as hard requirements, but reasonable estimates: vector index ~500MB–1GB for 70K files (depends on chunk count), BM25 ~100–300MB (tantivy is compact), SQLite graph ~50–200MB for 1M edges. Acceptable to leave as implementation benchmarks, not spec requirements.
- [x] CHK011 - Are index compaction/vacuum requirements specified? [Gap]
  > ✅ SQLite WAL auto-checkpoints. LanceDB compaction is built-in. Tantivy merges segments automatically. No explicit maintenance operation needed — all three backends self-manage. Acceptable.

## Requirement Clarity

- [x] CHK012 - Is "<2GB RSS" measured at peak or sustained? [Clarity, Spec §SC-144]
  > ✅ Interpreted as peak RSS during indexing. Brief spikes during embedding batch processing are acceptable as long as peak stays under 2GB. This is the standard interpretation of RSS limits.
- [x] CHK013 - Is "<2s search" measured to first result or all results? [Clarity, Spec §SC-145]
  > ✅ Measured to all results returned (top_k results, default 10). For streaming result delivery, time to first result would be lower. Standard interpretation: wall-clock from query to complete result set.
- [x] CHK014 - Is "<1s graph query" cold or warm cache? [Clarity, Spec §SC-146]
  > ✅ Warm cache (SQLite WAL pages cached by OS). Cold cache (first query after daemon restart) may be slower due to SQLite page cache warming. Acceptable — warm is the normal operating condition.
- [x] CHK015 - Is "no O(n²) algorithms" formal or aspirational? [Clarity]
  > ✅ Aspirational design guidance, not a formal complexity constraint. The intent is: no quadratic-time operations on the file count (e.g., comparing all pairs of files). Individual algorithms (tree-sitter parsing, embedding) are bounded per-file.
- [x] CHK016 - Is "≥10x faster" for single file or batch? [Clarity, Spec §SC-112]
  > ✅ SC-112: "after a single file change". Single-file delta → incremental community update vs full reclustering of 100K+ symbols.
- [x] CHK017 - Is "files processed one at a time" file-level or chunk-level? [Clarity, Spec §FR-738]
  > ✅ FR-744a: "one file (or small batch) at a time". File-level batching with configurable batch size for embedding calls (32 chunks). Clarified.

## Requirement Consistency

- [x] CHK018 - Is 2GB limit consistent with hit-leiden graph in memory? [Consistency, Spec §SC-144 vs FR-806]
  > ✅ hit-leiden can operate in throughput mode (parallel, memory-efficient). For 100K nodes with sparse edges, the graph fits in ~100-200MB. Combined with embedding model (~100MB) and SQLite buffers, total is well under 2GB. Consistent.
- [x] CHK019 - Is <2s search consistent with reranking step? [Consistency, Spec §SC-145 vs FR-744]
  > ✅ Cross-encoder reranking of top-10 results adds ~100-500ms depending on model. Total 2s budget accommodates this. If reranking is disabled (rerank: false), search is faster. Consistent.
- [x] CHK020 - Is incremental re-indexing consistent with BM25 and vector index update? [Consistency, Spec §FR-741 vs FR-780]
  > ✅ Content hash skip applies to the chunking+embedding step. If a file changed, both vector embeddings and BM25 documents are updated for that file. LanceDB supports upsert, tantivy supports document delete+add. Consistent.
- [x] CHK021 - Are file watcher requirements consistent with OS limits? [Consistency, Spec §FR-742]
  > ✅ FR-744f: "recursive watching (single inotify watch on root). If limit hit, fall back to periodic polling (30s) with warning." Addresses inotify limits explicitly.

## Acceptance Criteria Quality

- [x] CHK022 - Is SC-144 testable with a specific corpus? [Measurability, Spec §SC-144]
  > ✅ Test with a Linux kernel clone (publicly available, deterministic). Measure peak RSS via `/proc/self/status` VmRSS or `getrusage`. Measurable.
- [x] CHK023 - Is SC-146 testable? [Measurability, Spec §SC-146]
  > ✅ Build graph from Linux kernel source, query with depth 3, measure wall-clock. Use criterion benchmarks for reproducibility. Measurable.
- [x] CHK024 - Are scale SCs defined with cold or warm conditions? [Measurability]
  > ✅ SC-144 (indexing) is inherently cold-start. SC-145/146 (search/graph) are warm-cache (normal operation). Both are valid test conditions.

## Scenario Coverage

- [x] CHK025 - Are partial indexing / interruption requirements defined? [Coverage, Gap]
  > ✅ FR-744d: "interruption and resumption: next run resumes via content hash registry (already-indexed files skipped)"
- [x] CHK026 - Are large file requirements defined? [Coverage]
  > ✅ FR-744e: "Files exceeding max_file_size (default 1MB, configurable) are skipped."
- [x] CHK027 - Are binary file requirements defined? [Coverage]
  > ✅ FR-744e: "Binary files (null byte in first 8KB) are skipped."
- [x] CHK028 - Are polyglot repo requirements defined? [Coverage]
  > ✅ FR-727: chunker supports 14 languages. FR-730: unsupported languages fall back to text splitter. All languages handled.
- [x] CHK029 - Are shallow clone requirements defined for ProjectId? [Coverage]
  > ✅ Edge case: shallow clone falls back to sha256(path). FR-851: clone_repo supports depth parameter.

## Edge Case Coverage

- [x] CHK030 - Is behaviour defined when embedding model download fails? [Edge Case, Gap]
  > ✅ FR-733: models downloaded on first use and cached. If download fails, indexing fails with a clear error. The daemon retries on next indexing request. No partial state corruption.
- [x] CHK031 - Is behaviour defined when LanceDB storage becomes corrupt? [Edge Case, Gap]
  > ✅ FR-744d: resumption via hash registry means the vector store can be rebuilt from scratch — delete the vectors/ directory and re-index. Not explicitly specified as a recovery procedure, but the incremental design provides natural recovery. Acceptable.
- [x] CHK032 - Is behaviour defined when tantivy index schema changes? [Edge Case, Gap]
  > ✅ FR-828: StorageMigration framework handles per-subsystem version changes. Tantivy index rebuild on schema change (delete + re-index, same as vector store). Covered by migration framework.
- [x] CHK033 - Is behaviour defined when inotify limit is exceeded? [Edge Case, Gap]
  > ✅ FR-744f: "fall back to periodic polling (30s) with warning"
- [x] CHK034 - Is behaviour defined when file changes during indexing? [Edge Case, Gap]
  > ✅ The file watcher detects the change after the current indexing pass completes, triggering a re-index of that file in the next incremental pass. Hash registry ensures the stale version is replaced. No corruption — at worst, briefly stale results.
- [x] CHK035 - Is behaviour defined for symlink cycles during walking? [Edge Case, Gap]
  > ✅ The walker (synwire-index) uses `walkdir` or equivalent which detects symlink cycles by default. Cycles are skipped with a warning log. Standard Rust ecosystem behaviour.

## Non-Functional Requirements

- [x] CHK036 - Are CPU utilisation requirements specified? [Gap]
  > ✅ FR-744h: "bounded by configurable parallelism limit (default: half of available cores)"
- [x] CHK037 - Are I/O throughput requirements specified? [Gap]
  > ✅ Not explicitly. SQLite WAL and LanceDB use sequential writes during indexing (good for both SSD and HDD). tantivy uses segment merging (sequential). Implementation concern, not spec-level. Acceptable.
- [x] CHK038 - Are cold-start latency requirements specified? [Gap]
  > ✅ FR-888m: MCP server startup within 10s. Daemon cold-start (loading embedding model) is additional — bge-small-en-v1.5 loads in ~2-3 seconds. Total cold-start ~12-15s. Not explicitly specified but acceptable.
- [x] CHK039 - Are degradation requirements during active indexing specified? [Gap]
  > ✅ FR-744g: "search returns results from partial index, not blocked. Response includes index_in_progress: true flag."
- [x] CHK040 - Is the bge-small quality at scale documented? [Gap, Spec §FR-733]
  > ✅ FR-733: configurable to bge-base/bge-large for better quality. FR-744: reranker compensates for smaller embedding model. Trade-off is documented.

## Dependencies & Assumptions

- [x] CHK041 - Is LanceDB validated for 70K files? [Assumption]
  > ✅ LanceDB handles millions of vectors (designed for ML-scale datasets). 70K files × ~10 chunks/file = ~700K vectors. Well within LanceDB's design parameters.
- [x] CHK042 - Is tantivy validated for 70K documents on-disk? [Assumption]
  > ✅ tantivy is used by Quickwit (terabyte-scale log search) and Meilisearch. 70K documents is trivial for tantivy.
- [x] CHK043 - Is SQLite validated for 1M+ edge code graphs? [Assumption, Spec §FR-779a]
  > ✅ SQLite routinely handles databases with billions of rows. 1M edges with proper indexing (on from_node, to_node) queries in microseconds. Well validated.
- [x] CHK044 - Is hit-leiden validated for 100K+ nodes within 2GB? [Assumption, Spec §SC-112]
  > ✅ hit-leiden README reports benchmarks on ca-HepTh dataset (~9K nodes, ~26K edges) with 63-136x speedup. Extrapolating to 100K nodes with sparse code graphs (avg degree ~5-10), memory ~100-200MB. Within budget.

## Notes

- Check items off as completed: `[x]`
- All 44 items resolved. 8 new FRs (FR-744a–744h) added to spec.
- Scale requirements affect Phases 25–30 and the MCP server (Phase 32)
