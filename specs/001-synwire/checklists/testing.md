# Testing Strategy Requirements Quality Checklist: Synwire

**Purpose**: Validate that testing requirements are complete, clear, and structured for BDD scenarios (complementing existing per-story acceptance scenarios), property-based testing with proptest, and nextest runner configuration across all crates and layers.
**Created**: 2026-03-09
**Feature**: [spec.md](../spec.md), [plan.md](../plan.md), [tasks.md](../tasks.md)
**Tools**: [nextest](https://nexte.st/) (test runner), [proptest](https://proptest-rs.github.io/proptest/) (property-based testing)

## BDD Scenario Architecture

- [x] CHK001 - Is a BDD framework specified for Rust (e.g. cucumber-rs, or proptest state-machine testing) and is its relationship to the existing per-story acceptance scenarios defined? [Completeness, Gap] → FR-597, FR-598 (Given/When/Then format, complements US1–US20)
- [x] CHK002 - Are cross-cutting BDD scenarios defined that span multiple user stories — e.g. "a RAG pipeline combining embedding, retrieval, and model invocation" — beyond the per-story scenarios in US1–US20? [Coverage, Gap] → FR-597 (a)–(e), SC-115
- [x] CHK003 - Are BDD scenario requirements defined for integration-level behaviours that cross crate boundaries (e.g. synwire-core + synwire-orchestrator + synwire-agents working together)? [Coverage, Gap] → FR-597 (b), (c)
- [x] CHK004 - Is the Given/When/Then format mandated for all BDD scenarios, and are guidelines specified for writing scenario steps that are implementation-agnostic? [Clarity, Gap] → FR-598
- [x] CHK005 - Are BDD scenario requirements defined for error and recovery flows — e.g. "Given a graph mid-execution, When checkpoint save fails, Then the graph reports the error and can be retried"? [Coverage, Gap — Exception Flow] → FR-597 (covered by cross-cutting scenarios), T417
- [x] CHK006 - Are BDD scenario requirements defined for the protocol crates — A2A task lifecycle, AG-UI event streaming, MCP tool discovery and invocation? [Coverage, Gap] → FR-597 (d), T414
- [x] CHK007 - Are BDD scenario requirements defined for sandbox lifecycle — sandbox creation, command execution, filesystem operations, teardown, and cross-backend portability? [Coverage, Gap] → T416
- [x] CHK008 - Are BDD scenario requirements defined for the agent middleware stack — middleware ordering, tool injection, state transformation, HITL approval flow? [Coverage, Gap] → T412, T418
- [x] CHK009 - Are BDD scenario requirements defined for evaluation workflows — scorer execution, ATIF output generation, sandbox-based evaluation lifecycle? [Coverage, Gap] → FR-597 (e), T415
- [x] CHK010 - Is a scenario tagging/categorisation scheme defined (e.g. @smoke, @integration, @slow, @requires-api-key) that maps to nextest filter expressions? [Completeness, Gap] → FR-599

## Property-Based Testing Requirements (proptest)

### Core Layer (synwire-core)

- [x] CHK011 - Are property-based testing requirements defined for Message serialisation round-trips — any valid Message serialises to JSON and deserialises back to an identical value? [Coverage, Gap] → FR-600 (a)
- [x] CHK012 - Are property-based testing requirements defined for Document construction — arbitrary metadata and page_content combinations produce valid Documents? [Coverage, Gap] → FR-600 (b)
- [x] CHK013 - Are property-based testing requirements defined for PromptTemplate — any set of declared variables, when all provided, produces output containing all substituted values without template syntax leaking? [Coverage, Gap] → FR-600 (c)
- [x] CHK014 - Are property-based testing requirements defined for ChatPromptTemplate — arbitrary message template lists with valid variables produce correctly typed Message lists? [Coverage, Gap] → FR-600 (d)
- [x] CHK015 - Are property-based testing requirements defined for tool schema validation — any ToolInput implementing JsonSchema produces a valid JSON Schema, and valid inputs against that schema are accepted? [Coverage, Gap] → FR-600 (e)
- [x] CHK016 - Are property-based testing requirements defined for SynwireError — all error variants implement Display and can round-trip through Debug without panicking? [Coverage, Gap] → FR-600 (f)
- [x] CHK017 - Are property-based testing requirements defined for embedding dimension invariants — embed_documents output vectors all have the same dimensionality, and embed_query matches that dimensionality? [Coverage, Gap] → FR-600 (g)
- [x] CHK018 - Are property-based testing requirements defined for InMemoryVectorStore — similarity_search with k ≤ stored document count always returns exactly k results, sorted by descending similarity? [Coverage, Gap] → FR-600 (h)

### Orchestrator Layer (synwire-orchestrator)

- [x] CHK019 - Are property-based testing requirements defined for channel merge semantics — LastValue overwrites, Append concatenates, BinaryOperator is associative — for arbitrary sequences of channel updates? [Coverage, Gap] → FR-601 (a)
- [x] CHK020 - Are property-based testing requirements defined for StateGraph compilation — any valid graph topology (no orphan nodes, all edges reference existing nodes) compiles without error? [Coverage, Gap] → FR-601 (b)
- [x] CHK021 - Are property-based testing requirements defined for Pregel superstep determinism — the same graph with the same inputs and same node ordering produces identical output state? [Coverage, Gap] → FR-601 (c)
- [x] CHK022 - Are property-based testing requirements defined for checkpoint round-trips — any CheckpointData serialised to SQLite/PostgreSQL and read back produces an identical value? [Coverage, Gap] → FR-601 (d)
- [x] CHK023 - Are property-based testing requirements defined for conditional edge routing — arbitrary router return values always select a valid next node or END, never panic or produce an invalid transition? [Coverage, Gap] → FR-601 (e)
- [x] CHK024 - Are property-based testing requirements defined for Send() fan-out — arbitrary Send lists produce one task per Send, and all tasks are executed exactly once? [Coverage, Gap] → FR-601 (f)

### Agents Layer (synwire-agents)

- [x] CHK025 - Are property-based testing requirements defined for middleware stack composition — any permutation of compatible middlewares produces a valid middleware chain that can process a request? [Coverage, Gap] → FR-602 (a)
- [x] CHK026 - Are property-based testing requirements defined for tool invocation — arbitrary valid JSON matching a tool's schema is accepted, and arbitrary invalid JSON is rejected with a typed error? [Coverage, Gap] → FR-602 (b)
- [x] CHK027 - Are property-based testing requirements defined for Agent<D,O> builder — any combination of valid builder parameters (model + tools + optional middleware) produces a runnable agent? [Coverage, Gap] → FR-602 (c)

### Sandbox Layer (synwire-sandbox)

- [x] CHK028 - Are property-based testing requirements defined for filesystem path traversal protection — arbitrary path inputs (including ../, symlinks, null bytes) are rejected when they escape the sandbox root? [Coverage, Gap] → FR-603 (a), SC-118
- [x] CHK029 - Are property-based testing requirements defined for StateBackend key-value round-trips — any key-value pair written to a StateBackend can be read back identically? [Coverage, Gap] → FR-603 (b)

### Protocol & Eval Layers

- [x] CHK030 - Are property-based testing requirements defined for A2A JSON-RPC message parsing — arbitrary valid JSON-RPC 2.0 messages parse correctly, and malformed messages produce typed errors? [Coverage, Gap] → FR-604 (a)
- [x] CHK031 - Are property-based testing requirements defined for ATIF trajectory output — any evaluation result produces valid ATIF JSON that conforms to the ATIF schema? [Coverage, Gap] → FR-604 (b)
- [x] CHK032 - Are property-based testing requirements defined for DSPy Signature construction — arbitrary field definitions produce valid signatures, and invalid configurations are rejected at compile time or construction? [Coverage, Gap] → FR-604 (c)

### Proptest Infrastructure

- [x] CHK033 - Are proptest Strategy definitions required for core domain types (Message, Document, ToolInput, ChatResult, CheckpointData) to enable reuse across property tests? [Completeness, Gap] → FR-605
- [x] CHK034 - Is proptest configuration specified — number of cases, max shrink iterations, regression file location, fork mode for timeout-sensitive tests? [Completeness, Gap] → FR-606 (256 cases, 4096 shrink iterations, fork mode)
- [x] CHK035 - Are proptest regression files (proptest-regressions/) required to be committed to version control for reproducible CI failures? [Completeness, Gap] → FR-606
- [ ] CHK036 - Is the relationship between proptest-derive and synwire-derive specified — can #[derive(Arbitrary)] be used alongside #[derive(Signature)] on the same types? [Clarity, Gap] — Not explicitly addressed; derive compatibility not specified

## Nextest Runner Configuration

- [x] CHK037 - Is nextest specified as the required test runner for CI, with a .config/nextest.toml configuration file? [Completeness, Gap] → FR-591, SC-109
- [x] CHK038 - Are nextest profile requirements defined — at least a default profile and a CI profile with appropriate retry, timeout, and output settings? [Completeness, Gap] → FR-591 (default + ci profiles)
- [x] CHK039 - Are test partitioning requirements defined for CI parallelism — should the CI matrix use nextest's --partition to split tests across runners? [Completeness, Gap] → FR-592
- [x] CHK040 - Are flaky test retry requirements specified — which test categories should be retried (integration tests with external services), and with what backoff strategy? [Clarity, Gap] → FR-594
- [x] CHK041 - Are test timeout requirements defined per test category — unit tests (short), property tests (medium), integration tests (longer), sandbox tests (longest)? [Completeness, Gap] → FR-595
- [x] CHK042 - Is JUnit XML output required from nextest for CI reporting and test result aggregation? [Completeness, Gap] → FR-614, SC-117
- [x] CHK043 - Are nextest test groups defined for resource-constrained tests — e.g. tests requiring Docker, Kubernetes, or external API keys should run in limited concurrency? [Coverage, Gap] → FR-593
- [x] CHK044 - Are nextest filterset expressions defined for selecting test categories — e.g. `test(~prop_)` for property tests, `test(~integration_)` for integration tests? [Clarity, Gap] → FR-596
- [x] CHK045 - Is the nextest slow-timeout threshold specified to surface unexpectedly slow tests that may indicate performance regressions? [Completeness, Gap] → FR-595

## Test Infrastructure & CI Integration

- [x] CHK046 - Are requirements defined for a shared test utilities crate or module providing FakeChatModel, FakeEmbeddings, mock sandbox backends, and proptest strategies? [Completeness, Gap] → FR-607
- [x] CHK047 - Are CI workflow requirements updated to use nextest instead of or alongside cargo test — including the plan.md CI specification (line 85: ci.yml)? [Consistency, plan.md §CI] → FR-608, plan.md §Testing Infrastructure
- [x] CHK048 - Are coverage requirements (SC-002: 90% synwire-core, SC-010: 80% synwire-orchestrator, SC-018: 80% synwire-agents) updated to account for property-based and BDD tests contributing to coverage? [Consistency, Spec SC-002/SC-010/SC-018] → FR-608 (f) coverage step uses nextest which runs all test types
- [x] CHK049 - Are requirements defined for which tests run on PR (fast feedback: unit + property) vs merge-to-main (full: unit + property + integration + BDD) vs nightly (everything + coverage)? [Completeness, Gap] → FR-613, FR-618 (4-tier matrix)
- [x] CHK050 - Is feature-gating for integration tests (plan.md line 576: "feature-gated behind integration-tests") consistent with the proposed nextest filterset approach? [Consistency, plan.md §Integration Tests] → FR-596 filtersets complement feature gates

## Existing Acceptance Scenario Quality

- [x] CHK051 - Are the existing per-story acceptance scenarios (US1–US20) written with sufficient specificity that they could be automated as BDD scenarios without ambiguity? [Clarity, Spec §US1–US20] → FR-622 (reviewability requirement)
- [ ] CHK052 - Are the existing acceptance scenarios consistent in their Given/When/Then structure across all 20 user stories? [Consistency, Spec §US1–US20] — Existing scenarios use consistent format but no explicit consistency requirement defined
- [ ] CHK053 - Do the existing acceptance scenarios cover negative/error paths for every user story, not just happy paths? [Coverage, Spec §US1–US20] — Most stories have error scenarios but coverage completeness not audited
- [ ] CHK054 - Are the existing "Independent Test" descriptions for each user story specific enough to guide property-based test strategy selection? [Clarity, Spec §US1–US20] — Independent Test sections exist but don't reference proptest strategies
- [ ] CHK055 - Are acceptance scenario requirements defined for the newer user stories (US16–US20: DSPy, structured output, AG-UI state sync, Oracle Agent Spec, sandbox evaluation) with the same rigour as US1–US5? [Consistency, Spec §US16–US20] — Not explicitly audited for parity

## Edge Cases & Non-Functional Testing

- [x] CHK056 - Are requirements defined for testing concurrent access patterns — multiple agents sharing a checkpoint store, concurrent graph executions, parallel middleware processing? [Coverage, Gap] → FR-601 (c) determinism implies concurrent testing, FR-615 (a) checkpoint concurrent access
- [x] CHK057 - Are requirements defined for property-testing Send + Sync constraints — ensuring that arbitrary types composed through the type system remain thread-safe? [Coverage, Gap] → FR-620 (resource cleanup properties cover composition), T410
- [x] CHK058 - Are requirements defined for testing resource cleanup — dropped streams, cancelled futures, interrupted graph executions do not leak file handles, connections, or memory? [Coverage, Gap] → FR-620
- [x] CHK059 - Are requirements defined for testing backwards compatibility — can a checkpoint written by version N be read by version N+1? [Coverage, Gap] → FR-621
- [ ] CHK060 - Are performance testing requirements defined — streaming latency overhead (plan.md §Performance Goals: < 1ms per chunk), batch parallelisation, property test execution time budgets? [Completeness, plan.md §Performance] — Performance testing not explicitly defined as FR; latency target exists but no test requirement
- [x] CHK061 - Are requirements defined for testing the zero-unsafe constraint (SC-005, SC-011, SC-019) — should CI include a cargo-geiger or similar audit step? [Coverage, Gap] → FR-616, SC-114
- [x] CHK062 - Are fuzz testing requirements considered alongside property testing — for parser-heavy code (JSON-RPC, MCP protocol, ATIF schema) where coverage-guided fuzzing may find issues proptest misses? [Coverage, Gap] → FR-617

## Conformance Testing

- [x] CHK063 - Are conformance test suite requirements defined for the checkpoint backends — the plan mentions `synwire-checkpoint-conformance` (plan.md line 636) but are the conformance properties enumerated? [Completeness, plan.md §Checkpoint Conformance] → FR-615 (a), SC-111
- [x] CHK064 - Are conformance test suite requirements defined for sandbox backends — should all SandboxBackendProtocol implementations pass a shared property-based conformance suite? [Coverage, Gap] → FR-615 (b)
- [x] CHK065 - Are conformance test suite requirements defined for ChatModel providers — should all provider implementations pass a shared test suite covering invoke, stream, batch, error handling? [Coverage, Gap] → FR-615 (c)

## Notes

- Check items off as completed: `[x]`
- This checklist validates testing *requirements*, not the tests themselves
- Items tagged [Gap] indicate missing testing requirements in the spec/plan
- Items tagged with Spec/plan references indicate existing requirements that need quality validation
- BDD scenarios complement (not replace) existing per-story acceptance scenarios
- proptest scope covers all layers: core, orchestrator, agents, sandbox, protocols, evals
- nextest is the target CI test runner with partitioning, retries, and JUnit output
- **Resolution summary**: 59 of 65 items resolved by FR-591–FR-622 and SC-109–SC-118
- **Remaining gaps** (6 items):
  - CHK036: proptest-derive / synwire-derive compatibility not specified
  - CHK052: Existing acceptance scenario consistency not explicitly audited
  - CHK053: Error path coverage across all user stories not audited
  - CHK054: Independent Test sections don't reference proptest strategy guidance
  - CHK055: US16–US20 acceptance scenario rigour parity not audited
  - CHK060: Performance testing requirements not defined as FRs
