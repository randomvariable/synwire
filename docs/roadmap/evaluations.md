# Evaluation Framework

## Overview
Crates: synwire-evals (implementations), synwire-core (core types). Covers the Harbor sandbox for evaluation environments, the scorer/dataset/experiment system derived from Braintrust SDK Rust + Autoevals, and agent trajectory recording. M2/M3 feature.

## Harbor Sandbox (FR-104-108)
Async-only sandbox backend for evaluation environments:
- `HarborSandbox` implementing SandboxVfs for Docker/kagent/E2B environments (FR-105)
- `HarborRuntime` trait: exec, upload, download, id
- `EvaluationRunner` orchestrating agent evaluation runs with CLI mode (auto-approve) and SDK mode (FR-106)
- `EvaluationConfig`: instruction, cwd, backend, model, auto_approve
- ATIF (Agent Trajectory Interchange Format) output v1.2 with token usage and cost tracking (FR-107)
- Optional tracing integration behind tracing feature flag (FR-108)
- Per-metric evaluation: goal_achievement, tool_usage_efficiency, reasoning_quality, latency_p99 (FR-396)

## Scorer System (FR-465-478)

### Core Types (FR-465-468)
- `Scorer<I, O>` trait with score() returning Vec<Score>. Typed, distinct from external metric types (FR-465)
- `Score` type: name, score (0.0–1.0, None to skip), metadata (FR-466)
- `CompositeScorer` with configurable weights for weighted average (FR-467)
- `PartialScorer` wrapper for pre-filling scorer arguments (FR-468)

### Heuristic Scorers (FR-469-473)
- `ExactMatch`, `LevenshteinScorer`, `NumericDiffScorer` (FR-469)
- `ValidJsonScorer` with optional JSON Schema validation (FR-470)
- `JsonDiffScorer` for recursive structural comparison (FR-471)
- `ListContainsScorer` with pairwise matching via linear sum assignment (FR-472)
- `EmbeddingSimilarityScorer` via cosine similarity with thread-safe cache (FR-473)

### LLM-as-Judge (FR-474-477)
- `ModerationScorer` using provider moderation API (FR-474)
- `LlmClassifier` with template prompts, choice_scores, optional CoT, tool-based extraction (FR-475)
- Declarative evaluator from YAML/TOML via LlmClassifier::from_spec_file() (FR-476)
- Built-in spec files: BattleScorer, FactualityScorer, ClosedQAScorer, SecurityScorer (FR-477)

### RAG Scorers (FR-478)
Faithfulness, ContextRecall, ContextPrecision, ContextRelevancy, AnswerCorrectness, AnswerRelevancy, ContextEntityRecall

## Dataset & Task (FR-479-483)
- `Dataset<I, O>` async trait with VecDataset and StreamDataset implementations (FR-479)
- `EvalCase<I, O>` with input, expected, metadata, tags, id (FR-480)
- Dataset versioning and per-case IDs for reproducibility (FR-481)
- `Task<I, O>` trait separating "thing being evaluated" from scoring. FnTask, AsyncFnTask wrappers (FR-482)
- `TaskHooks` for mutable metadata/tags enrichment during task execution (FR-483)

## Results & Summary (FR-484-486)
- `EvalResult<I, O>` with input, output, expected, scores, metadata, tags, error, duration (FR-484)
- `EvalSummary<I, O>` with results, score_stats, case counts, dataset_id, duration (FR-485)
- `ScoreStats` with mean, min, max, count, std_dev (FR-486)

## Runner & Experiment (FR-487-510)

### Execution (FR-487-488, FR-498-499, FR-502-508)
- `Evaluator::run()` with parallel task execution and scoring, max_concurrency (default 5) (FR-487)
- `SyncEvaluator` wrapper for non-async code (FR-488)
- CLI evaluation runner via EvalRunner::from_env() with env var configuration (FR-498)
- terminate_on_failure option for early termination (FR-499)
- Streaming task output support via StreamTask (FR-502)
- Multimodal input/output support (generic I/O type parameters) (FR-503)
- Multi-turn evaluation via ConversationEvalCase and ConversationScorer (FR-504)
- score: None treated as skipped for aggregates (FR-505)
- Task failure handling: capture error, skip scoring, increment failed_cases, continue (FR-506)
- LLM scorer failure: ExceptionHandlingStrategy (Raise, SetConstant, SetNone, SetZero) (FR-507)
- Reproducibility: deterministic ordering, configurable seed, dataset versioning (FR-508)

### Experiments (FR-489-493)
- `Experiment` type with name, description, base_experiment, metadata, repo_info (FR-489)
- `RepoInfo` with git metadata auto-detection via from_git() (FR-490)
- Experiment comparison: ExperimentSummary with per-score diff, improvements, regressions (FR-491)
- `Feedback` type for post-hoc score correction (FR-492)
- Experiment export and summarisation (FR-493)

### Tracing & Integration (FR-494-501)
- Evaluator tracing spans: root span per evaluation, child spans per case (FR-494)
- EvaluationMetric deepening: prompt template format, response extraction, CoT support (FR-495)
- Decision tree for EvaluationMetric vs DSPy metric vs Scorer usage with adapter functions (FR-500)
- Common EvaluationReport trait unifying evaluation result formats (FR-501)

## Crate Boundaries (FR-509-510)
- synwire-evals crate separate from core. Core types in synwire-core, implementations in synwire-evals (FR-509)
- LLM-based scorers depend on ChatModel/Embeddings traits. No circular dependencies (FR-510)

## Success Criteria
- **SC-017**: Evaluation run produces valid ATIF trajectory with token usage and cost
- **SC-075**: Evaluate produces EvalResult with per-example scores and aggregates
- **SC-088**: ExactMatch scorer returns correct Score
- **SC-089**: CompositeScorer computes correct weighted average
- **SC-090**: Evaluator processes cases at max_concurrency and produces correct ScoreStats
- **SC-091**: ExperimentSummary computes diffs, improvements, regressions vs baseline
- **SC-092**: FaithfulnessScorer extracts and verifies claims
- **SC-093**: LlmClassifier loads from YAML spec file
- **SC-094**: Dataset trait supports async streaming
- **SC-095**: SyncEvaluator produces identical results to async Evaluator
- **SC-097**: ConversationScorer evaluates multi-turn dialogue
- **SC-098**: EvalRunner::from_env() reads env vars and produces JSONL output
