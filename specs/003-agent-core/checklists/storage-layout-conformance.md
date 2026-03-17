# Storage Layout Conformance Checklist: Agent Core Runtime

**Purpose**: Validate requirements quality, completeness, and consistency for configurable persistent storage layout (US36, FR-816–845)
**Created**: 2026-03-16
**Feature**: [spec.md](../spec.md) — US36, FR-816–845, FR-831a–831g, SC-116–129

## Requirement Completeness

- [x] CHK001 - Are platform-specific path conventions explicitly specified for all three platforms (Linux XDG, macOS ~/Library, Windows %LOCALAPPDATA%)? [Completeness, Spec §FR-816]
  > ✅ FR-816: "platform conventions (XDG on Linux, ~/Library on macOS, %LOCALAPPDATA% on Windows) via the `directories` crate"
- [x] CHK002 - Is every subsystem's path accessor enumerated — are there any subsystems that would need a path but don't have a `StorageLayout` accessor? [Completeness, Spec §FR-821]
  > ✅ FR-821 lists: session_db, index_cache, graph_dir, communities_dir, experience_db, lsp_cache, models_cache, logs_dir, skills_dir, project_skills_dirname, repos_cache, repo_cache. FR-833 adds global tier accessors. Daemon socket/PID in FR-825a.
- [x] CHK003 - Is the `project.json` schema fully defined (all fields, types, required vs optional)? [Completeness, Spec §FR-830]
  > ✅ FR-830: WorktreeId, display_name, canonical_path, remote_url (FR-831f: origin remote, nullable), created_at. FR-819: RepoId included. All typed.
- [x] CHK004 - Are requirements for `global/registry.json` schema defined (fields, types, update semantics)? [Completeness, Spec §FR-834]
  > ✅ FR-834: WorktreeId, display_name, canonical_path, remote_url, last_accessed, tags. Updated on every project access.
- [x] CHK005 - Is the durable-vs-cache classification documented for every data type? [Completeness, Spec §FR-817]
  > ✅ FR-817: durable = sessions, community summaries, experience pool, code graph. Cache = vector indices, BM25, content hashes, LSP/DAP caches, models, repos.
- [x] CHK006 - Are requirements for what `discover_project(path)` returns when multiple data directories match the same `RepoId`? [Gap, Spec §FR-831]
  > ✅ FR-831a: returns all WorktreeIds sharing the RepoId, caller selects by path match.
- [x] CHK007 - Are cleanup/garbage-collection requirements defined for orphaned project data? [Gap]
  > ✅ FR-831b: `storage_gc` operation, configurable 90-day stale threshold, confirmation required.
- [x] CHK008 - Are disk space monitoring or quota requirements defined? [Gap]
  > ✅ Covered by edge case (disk space runs out during indexing → detect write failures, mark incomplete). No proactive quota — acceptable for a developer tool.

## Requirement Clarity

- [x] CHK009 - Is "lazy directory creation" precisely defined? [Clarity, Spec §FR-822]
  > ✅ FR-822: "create parent directories on first access (lazy creation) — not at StorageLayout construction time"
- [x] CHK010 - Is the native concurrency model specified for each storage backend? [Clarity, Spec §FR-825]
  > ✅ FR-825: SQLite WAL for structured data, LanceDB concurrent reads for vectors, tantivy IndexReader/Writer for BM25. FR-826: atomic rename for binary blobs.
- [x] CHK011 - Is "copy-then-swap" migration strategy specified with enough detail? [Clarity, Spec §FR-828]
  > ✅ FR-828: "check version.json, run migrations if older, copy-then-swap. Failed migrations leave previous data intact." FR-831c defines version.json format.
- [x] CHK012 - Is "configurable via `StorageLayout.project_skills_dirname()`" specified — at what level? [Ambiguity, Spec §FR-803i]
  > ✅ FR-803i: derived from product name at StorageLayout construction time. `.<product>/skills/` where product is set by --product-name. Runtime configurable.
- [x] CHK013 - Is the `version.json` format defined? [Gap, Spec §FR-828]
  > ✅ FR-831c: `{"subsystem": "<name>", "version": <u32>, "migrated_at": "<rfc3339>"}`. One per subsystem directory.
- [x] CHK014 - Is "0600 file permissions on sensitive data" scoped? [Clarity]
  > ✅ FR-831d: SQLite databases + daemon socket = 0600, logs = 0640, directories = 0700.

## Requirement Consistency

- [x] CHK015 - Does `StorageLayout` produce the same paths as existing `synwire-index/src/cache.rs`? [Consistency, Spec §FR-824]
  > ✅ FR-824: replaces hardcoded path. This is a breaking change. Edge case added: one-time migration from old layout via FR-828.
- [x] CHK016 - Is `WorktreeId` used consistently everywhere? [Consistency, Spec §FR-819]
  > ✅ FR-819 defines two-level identity. FR-819a: daemon per-RepoId, indices per-WorktreeId. FR-825c: daemon manages per-WorktreeId. Consistent throughout.
- [x] CHK017 - Are the `StorageLayout` path accessors consistent with the plan's directory layout? [Consistency]
  > ✅ Plan Phase 25 files list matches FR-821 accessors. Daemon socket/PID paths in FR-825a.
- [x] CHK018 - Do the global tier paths use the same concurrency mechanism? [Consistency, Spec §FR-839]
  > ✅ FR-839: "same BaseStore trait backed by SQLite WAL. No external lock files." Consistent with FR-825.
- [x] CHK019 - Is the config hierarchy consistent with MCP server --product-name? [Consistency]
  > ✅ FR-829: SYNWIRE_DATA_DIR > with_root() > project-local config > platform default. FR-860: --product-name is StorageLayout product name. FR-888c: config file mirrors CLI. Consistent.

## Acceptance Criteria Quality

- [x] CHK020 - Is SC-117 ("ProjectId identical before and after moving") testable? [Measurability, Spec §SC-117]
  > ✅ RepoId from first-commit hash is path-independent. Test: compute, move dir, recompute, assert equal. Measurable.
- [x] CHK021 - Is SC-120 ("prevents concurrent corruption") measurable? [Measurability, Spec §SC-120]
  > ✅ SC-120 updated: "SQLite WAL, LanceDB, tantivy prevent corruption — parallel write + read test with zero corruption and zero blocked reads". Concrete test procedure.
- [x] CHK022 - Is SC-121 ("migration successful") defined? [Measurability, Spec §SC-121]
  > ✅ SC-121: "detects version mismatch, runs migration, failed migration leaves previous data intact". Verifiable by checking version.json updated + data accessible.
- [x] CHK023 - Is SC-119 testable? [Measurability, Spec §SC-119]
  > ✅ SC-119: "SqliteSaver and SemanticIndex receive paths from StorageLayout and function identically — zero behavioural regression". Testable by running existing test suites with StorageLayout paths.

## Scenario Coverage

- [x] CHK024 - Are requirements defined for first-run experience? [Coverage, Gap]
  > ✅ Edge case added: lazy directory creation, empty registry, daemon with no repos. First `index` call populates everything.
- [x] CHK025 - Are requirements defined for upgrading from old paths to new StorageLayout? [Coverage, Gap]
  > ✅ Edge case added: FR-828 migration detects old layout, one-time copy to new WorktreeId paths. `storage_gc` cleans old data.
- [x] CHK026 - Are requirements defined when `git` is not installed? [Coverage, Spec §FR-819]
  > ✅ FR-819: "falls back to sha256(canonical_path)". Edge case (shallow clone): documented.
- [x] CHK027 - Are requirements defined for repos with multiple remotes? [Coverage, Gap]
  > ✅ FR-831f: use `origin` remote, fallback first remote, null if no remotes.
- [x] CHK028 - Are requirements defined for monorepos? [Coverage, Gap]
  > ✅ FR-831e: WorktreeId is per-worktree-root, not per-subdirectory. Subdirectory scoping via file_filter.

## Edge Case Coverage

- [x] CHK029 - Is behaviour defined when `$XDG_DATA_HOME` points to read-only filesystem? [Edge Case, Gap]
  > ✅ Edge case added: MCP server fails at startup (same as FR-888t).
- [x] CHK030 - Is behaviour defined when project-local config is invalid JSON? [Edge Case, Gap]
  > ✅ Edge case added: skipped with warning, fall back to CLI/defaults.
- [x] CHK031 - Is behaviour defined when SQLite WAL mode cannot be enabled? [Edge Case, Gap]
  > ✅ Edge case added: fall back to journal mode DELETE, warn about degraded multi-instance performance.
- [x] CHK032 - Is behaviour defined for shallow clone lacking first commit? [Edge Case, Spec §Edge Case]
  > ✅ Edge case: falls back to sha256(path), warns suggesting `git fetch --unshallow`.
- [x] CHK033 - Is behaviour defined when two repos have the same first-commit hash? [Edge Case, Gap]
  > ✅ WorktreeId includes the worktree root path hash in addition to RepoId, so different repos at different paths always produce different WorktreeIds even with matching RepoId. Collision is a non-issue at the WorktreeId level.
- [x] CHK034 - Is behaviour defined for nested storage (with_root inside existing data)? [Edge Case, Gap]
  > ✅ Not explicitly specified but not harmful — StorageLayout creates subdirectories under the root. Nesting is technically valid. Low risk, acceptable to leave unspecified.

## Non-Functional Requirements

- [x] CHK035 - Are performance requirements specified for `ProjectId` computation? [Gap]
  > ✅ `git rev-list --max-parents=0 HEAD` is fast (<100ms even on Linux kernel). No explicit latency requirement needed — Git CLI is the bottleneck and it's already fast.
- [x] CHK036 - Are requirements for `ProjectRegistry` update frequency and atomicity? [Gap, Spec §FR-834]
  > ✅ FR-834: "updated on every project access". FR-839: SQLite WAL provides atomic writes. Acceptable.
- [x] CHK037 - Are requirements for max number of tracked projects? [Gap]
  > ✅ No hard limit. SQLite handles thousands of rows trivially. Acceptable to leave unspecified.
- [x] CHK038 - Are cross-project query latency requirements specified? [Gap]
  > ✅ SC-127–129 define cross-project xref query expectations. No explicit latency target beyond "within 2 seconds" (SC-145 for search). Acceptable.

## Dependencies & Assumptions

- [x] CHK039 - Is the `directories` crate dependency documented? [Dependency, Spec §FR-816]
  > ✅ FR-816: "via the `directories` crate". Plan Phase 25 lists it in Cargo.toml deps.
- [x] CHK040 - Is the `fs2` dependency documented for file locking? [Dependency, Spec §FR-825]
  > ✅ No longer applicable — FR-825 replaced flock with native backend concurrency. No fs2 dependency needed.
- [x] CHK041 - Is the `git rev-list` assumption validated for multiple root commits? [Assumption, Spec §FR-819]
  > ✅ FR-831g: "use first (oldest) hash, sorted lexicographically for determinism" when multiple root commits exist.

## Notes

- Check items off as completed: `[x]`
- All 41 items resolved. 7 new FRs (FR-831a–831g) + 5 edge cases added to spec.
- This checklist is critical — StorageLayout blocks all subsequent phases (26–34)
