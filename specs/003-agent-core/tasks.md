# Tasks: Agent Core Runtime

**Input**: Design documents from `/specs/003-agent-core/`
**Prerequisites**: plan.md, spec.md, data-model.md, contracts/, research.md, quickstart.md

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Include exact file paths in descriptions

## Path Conventions

Cargo workspace with crates:
- `crates/synwire-core/src/` — core traits
- `crates/synwire-agent/src/` — implementations
- `crates/synwire-test-utils/src/` — test helpers
- `crates/synwire/src/` — convenience re-exports

---

## Phase 1: Setup

**Purpose**: New crate creation and workspace configuration

- [X] T001 Create `crates/synwire-agent/Cargo.toml` with workspace dependencies (synwire-core, tokio, serde, thiserror, futures-util, reqwest, tracing, typetag, bitflags, dyn-clone)
- [X] T002 Create `crates/synwire-agent/src/lib.rs` with module declarations and `#![forbid(unsafe_code)]`
- [X] T003 Add `synwire-agent` to workspace members in `/Cargo.toml`
- [X] T004 [P] Add `typetag`, `dyn-clone`, and `bitflags` to `[workspace.dependencies]` in `/Cargo.toml`
- [X] T005 [P] Create module stubs in `crates/synwire-core/src/agents/` for new files: `directive.rs`, `directive_executor.rs`, `directive_filter.rs`, `execution_strategy.rs`, `plugin.rs`, `signal.rs`, `middleware.rs`, `hooks.rs`, `agent_node.rs`, `runner.rs`, `session.rs`, `streaming.rs`, `output_mode.rs`, `usage.rs`, `error.rs`, `permission.rs`, `model_info.rs`, `sandbox.rs`
- [X] T006 [P] Create module stubs in `crates/synwire-core/src/backends/` for: `mod.rs`, `protocol.rs`, `sandbox.rs`, `types.rs`, `error.rs`, `state_backend.rs`, `approval.rs`, `grep_options.rs`
- [X] T007 [P] Create module stubs in `crates/synwire-core/src/mcp/` for: `mod.rs`, `config.rs`, `traits.rs`, `elicitation.rs`
- [X] T008 Update `crates/synwire-core/src/lib.rs` to declare new `backends` and `mcp` modules and extend `agents` module exports
- [X] T009 Verify `cargo make fmt` and `cargo make clippy` pass with empty stubs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core types and error taxonomy that ALL user stories depend on

**CRITICAL**: No user story work can begin until this phase is complete

- [X] T010 Implement `AgentError` top-level error enum with model/tool/strategy/middleware/directive/backend/session/panic/budget variants in `crates/synwire-core/src/agents/error.rs` (FR-627–629)
- [X] T011 [P] Implement `ModelError` subtypes (Authentication, Billing, RateLimit, ServerError, InvalidRequest, MaxOutputTokens) with retryability flag in `crates/synwire-core/src/agents/error.rs` (FR-627–628)
- [X] T012 [P] Implement `BackendError` enum with all error codes (file_not_found, permission_denied, path_traversal, scope_violation, etc.) in `crates/synwire-core/src/backends/error.rs` (FR-073)
- [X] T013 [P] Implement `BackendCapabilities` bitflags (LS, READ, WRITE, EDIT, GREP, GLOB, UPLOAD, DOWNLOAD, PWD, CD, RM, CP, MV, EXEC) in `crates/synwire-core/src/backends/types.rs` (FR-072)
- [X] T014 [P] Implement backend response types (`DirEntry`, `FileContent`, `WriteResult`, `EditResult`, `GrepMatch`, `GlobEntry`, `TransferResult`, `FileInfo`, `ExecuteResponse`, `ProcessInfo`, `JobInfo`, `ArchiveInfo`) in `crates/synwire-core/src/backends/types.rs` (FR-072, FR-072a)
- [X] T015 [P] Implement `GrepOptions` and `GrepOutputMode` in `crates/synwire-core/src/backends/grep_options.rs` (FR-070a)
- [X] T016 [P] Implement `Usage` struct (input_tokens, output_tokens, cache_read, cache_creation, cost_usd, context_utilization_pct) in `crates/synwire-core/src/agents/usage.rs` (FR-618)
- [X] T017 [P] Implement `ModelInfo`, `ModelCapabilities`, `EffortLevel`, `ThinkingConfig` in `crates/synwire-core/src/agents/model_info.rs` (FR-591–597)
- [X] T018 [P] Implement `PermissionMode` enum (Default, AcceptEdits, PlanOnly, BypassAll, DenyUnauthorized) and `PermissionRule` in `crates/synwire-core/src/agents/permission.rs` (FR-613–614)
- [X] T019 [P] Implement `SandboxConfig` (network, filesystem, allowed/denied commands) in `crates/synwire-core/src/agents/sandbox.rs` (FR-620)
- [X] T020 [P] Implement `SystemPromptConfig` (Append/Replace variants) in `crates/synwire-core/src/agents/output_mode.rs` (FR-610)
- [X] T021 [P] Implement `ToolAnnotations` (read_only, destructive, open_world) and `ToolResultStatus` (Success/Failure/Rejected/Denied) in `crates/synwire-core/src/tools/types.rs` — extend existing file (FR-608–609)
- [X] T022 [P] Implement `ToolOutput` extended fields (binary_results, status, telemetry) in `crates/synwire-core/src/tools/types.rs` — extend existing (FR-609)
- [X] T023 Run `cargo make ci` — all foundational types must compile and pass clippy

**Checkpoint**: Foundation ready — user story implementation can now begin

---

## Phase 3: User Story 1 — Pure Testable Agent Logic (P1) MVP

**Goal**: Developers write agent logic as pure functions returning state + directives, testable without side effects

**Independent Test**: Write agent node returning directives, assert directives match expectations, confirm zero side effects

- [X] T024 [US1] Implement `Directive` enum with variants (Emit, SpawnAgent, StopChild, Schedule, RunInstruction, Cron, Stop, SpawnTask, StopTask, Custom) in `crates/synwire-core/src/agents/directive.rs` (FR-557)
- [X] T025 [P] [US1] Implement `DirectivePayload` trait with `typetag::serde` for Custom variant serialization in `crates/synwire-core/src/agents/directive.rs` (FR-557, FR-562)
- [X] T026 [P] [US1] Implement directive payload structs (EmitDirective, SpawnAgentDirective, StopChildDirective, ScheduleDirective, RunInstructionDirective, CronDirective, StopDirective, SpawnTaskDirective, StopTaskDirective) in `crates/synwire-core/src/agents/directive.rs` (FR-557)
- [X] T027 [US1] Implement `DirectiveResult<S>` generic over `State` with `state_only()`, `with_directive()`, `with_directives()` constructors and `From<S>` impl in `crates/synwire-core/src/agents/directive.rs` (FR-558)
- [X] T028 [US1] Implement `DirectiveExecutor` trait and `NoOpExecutor` (records without executing) in `crates/synwire-core/src/agents/directive_executor.rs` (FR-559)
- [X] T029 [US1] Implement `RecordingExecutor` that captures directives in `Mutex<Vec<Directive>>` for test assertions in `crates/synwire-test-utils/src/lib.rs` (FR-559, SC-097)
- [X] T030 [US1] Implement `DirectiveFilter` trait with `FilterDecision` (Pass/Suppress/Reject) and `FilterChain` in `crates/synwire-core/src/agents/directive_filter.rs` (FR-561)
- [X] T031 [US1] Implement serde round-trip for all Directive variants including Custom via typetag in `crates/synwire-core/src/agents/directive.rs` (FR-562, SC-099)
- [X] T032 [US1] Write unit tests: directive creation, `DirectiveResult` construction, `NoOpExecutor` records without executing, `FilterChain` suppress/reject/pass, serde round-trip for all variants in `crates/synwire-core/src/agents/directive.rs` `#[cfg(test)]` (SC-097, SC-098, SC-099)

**Checkpoint**: US1 complete — pure directive testing works with zero side effects

---

## Phase 4: User Story 2 — Pluggable Execution Strategies (P1)

**Goal**: Same agent logic runs under Direct (immediate) and FSM (state-constrained) strategies by configuration

**Independent Test**: Run same agent logic under both strategies, verify identical results for valid sequences, verify FSM rejects invalid transitions

- [X] T033 [US2] Implement `ExecutionStrategy` trait with `execute`, `tick`, `snapshot`, `signal_routes` in `crates/synwire-core/src/agents/execution_strategy.rs` (FR-563)
- [X] T034 [P] [US2] Implement `FsmStateId`, `ActionId` newtypes, `StrategyError` enum (InvalidTransition, GuardRejected, NoInitialState, Execution), `StrategySnapshot` trait in `crates/synwire-core/src/agents/execution_strategy.rs` (FR-563, FR-566)
- [X] T035 [P] [US2] Implement `GuardCondition` trait and `ClosureGuard` adapter in `crates/synwire-core/src/agents/execution_strategy.rs` (FR-566)
- [X] T036 [US2] Implement `DirectStrategy` in `crates/synwire-agent/src/strategies/direct.rs` (FR-564)
- [X] T037 [US2] Implement `FsmStrategy` with `Mutex<FsmStateId>` current state, transition table `HashMap<(FsmStateId, ActionId), Vec<FsmTransition>>`, guard evaluation in `crates/synwire-agent/src/strategies/fsm.rs` (FR-565, FR-566)
- [X] T038 [US2] Implement `FsmStrategyBuilder` with `.state()`, `.transition()`, `.transition_with_guard()`, `.route()`, `.build()` in `crates/synwire-agent/src/strategies/fsm.rs` (FR-566)
- [X] T039 [US2] Write unit tests: DirectStrategy passthrough, FsmStrategy valid transitions, FsmStrategy rejects invalid transitions with `InvalidTransition` error (current_state + attempted_action), guard rejection, builder validation, snapshot serialization in `crates/synwire-agent/src/strategies/` `#[cfg(test)]` (SC-100, SC-101)

**Checkpoint**: US2 complete — both strategies work, same agent logic produces identical results under both

---

## Phase 5: User Story 3 — Composable Plugin System with State Isolation (P1)

**Goal**: Plugins compose into an agent with type-safe isolated state per plugin

**Independent Test**: Compose two plugins with different state types, verify state isolation and compile-time key collision detection

- [X] T040 [US3] Implement `PluginStateKey` trait with associated `State` type and `const KEY` in `crates/synwire-core/src/agents/plugin.rs` (FR-568)
- [X] T041 [US3] Implement `PluginStateMap` (TypeId-keyed `HashMap<TypeId, Box<dyn Any + Send + Sync>>`) with `get::<P>()`, `get_mut::<P>()`, `insert::<P>()` in `crates/synwire-core/src/agents/plugin.rs` (FR-569)
- [X] T042 [US3] Implement `PluginHandle<P>` zero-sized proof token returned at registration in `crates/synwire-core/src/agents/plugin.rs` (FR-569)
- [X] T043 [US3] Implement `Plugin` trait with lifecycle hooks (on_user_message, on_event, before_run, after_run, signal_routes) with default no-op implementations in `crates/synwire-core/src/agents/plugin.rs` (FR-143, FR-144)
- [X] T044 [US3] Implement `PluginStateMap` serialization (stored fn pointers for per-key serialize/deserialize captured at registration) in `crates/synwire-core/src/agents/plugin.rs` (for checkpoint support)
- [X] T045 [US3] Write unit tests: type-safe access, cross-plugin isolation, key collision detection at registration, serialization round-trip, concurrent write isolation in `crates/synwire-core/src/agents/plugin.rs` `#[cfg(test)]` (SC-102)

**Checkpoint**: US3 complete — plugin state fully isolated, compile/runtime collision detection works

---

## Phase 6: User Story 11 — Enhanced Search with Context and Filtering (P1)

**Goal**: Agents perform ripgrep-style searches with context lines, filtering, and match counting

**Independent Test**: Search test files with various grep options, verify context lines, line numbers, filtering

- [X] T046 [US11] Implement `BackendProtocol` trait with all file operation signatures (ls, read, write, edit, grep, glob, upload, download, pwd, cd, rm, cp, mv_file, capabilities) in `crates/synwire-core/src/backends/protocol.rs` (FR-070)
- [X] T047 [US11] Implement `MemoryProvider` (ephemeral in-memory with `RwLock<BTreeMap<String, Vec<u8>>>` and `Mutex<String>` for cwd) in `crates/synwire-core/src/backends/state_backend.rs` (FR-075)
- [X] T048 [US11] Implement grep on `MemoryProvider` with full `GrepOptions` support (context, case-insensitive, file type, max matches, invert, count, line numbers) in `crates/synwire-core/src/backends/state_backend.rs` (FR-070a, SC-015)
- [X] T049 [US11] Write unit tests: grep with -C 3, case-insensitive, file type filter, invert match, count mode, max matches, binary file skip, line numbers in `crates/synwire-core/src/backends/state_backend.rs` `#[cfg(test)]` (SC-015, SC-020)

**Checkpoint**: US11 complete — enhanced grep works against in-memory backend

---

## Phase 7: User Story 14 — Working Directory State and Navigation (P1)

**Goal**: Agents navigate directories with persistent cwd across operations

**Independent Test**: cd to directories, verify pwd returns correct path, verify relative path resolution

- [X] T050 [US14] Implement persistent cwd on `MemoryProvider` — cd changes `Mutex<String>`, relative paths resolve from cwd, pwd returns current path in `crates/synwire-core/src/backends/state_backend.rs` (FR-070b)
- [X] T051 [US14] Implement path resolution logic: absolute paths from root, relative from cwd, reject `..` traversal, reject non-existent cd targets in `crates/synwire-core/src/backends/state_backend.rs` (FR-070b)
- [X] T052 [US14] Write unit tests: cd + pwd round-trip, relative path resolution, cd to non-existent fails without changing state, cd to `..' rejected, concurrent cwd access in `crates/synwire-core/src/backends/state_backend.rs` `#[cfg(test)]` (SC-016)

**Checkpoint**: US14 complete — directory navigation works with persistent state

---

## Phase 8: User Story 4 — File and Shell Operations via Backend Protocol (P2)

**Goal**: Agents perform file and shell operations through uniform backend interface with bash-style conventions

**Independent Test**: Agent writes files to ephemeral backend, swaps to persistent backend, verifies cross-conversation retention

- [X] T053 [P] [US4] Implement `FilesystemBackend` (virtual mode + real mode, path traversal protection via `security::path`, `Mutex<PathBuf>` for cwd) in `crates/synwire-agent/src/backends/filesystem.rs` (FR-077)
- [X] T054 [P] [US4] Implement `StoreBackend` (persistent cross-conversation via `BaseStore` trait with namespace isolation) in `crates/synwire-agent/src/backends/store.rs` (FR-076)
- [X] T055 [US4] Implement `LocalShellBackend` extending `FilesystemBackend` with `execute`, env var control, output truncation, timeout in `crates/synwire-agent/src/backends/local_shell.rs` (FR-078)
- [X] T056 [US4] Implement `CompositeBackend` with sorted `Vec<Mount>` routing by descending prefix length, segment-boundary matching, aggregated listings in `crates/synwire-agent/src/backends/composite.rs` (FR-079)
- [X] T057 [US4] Implement bash-style command translation layer (ls, cd, grep, rm, cp, mv, pwd) mapping to `BackendProtocol` operations in `crates/synwire-agent/src/backends/mod.rs` (FR-074a)
- [X] T058 [US4] Implement `SandboxBackendProtocol` trait (execute, execute_pipeline, id) and `BaseSandbox` abstract type in `crates/synwire-core/src/backends/sandbox.rs` (FR-071, FR-080)
- [X] T059 [US4] Write unit tests: ephemeral backend lifecycle, persistent backend cross-conversation, composite routing, path traversal rejection, bash command translation in `crates/synwire-agent/src/backends/` `#[cfg(test)]` (SC-013, SC-013a)

**Checkpoint**: US4 complete — file operations work across all backend types

---

## Phase 9: User Story 5 — Middleware Stack for Cross-Cutting Concerns (P2)

**Goal**: Middleware components add tools, modify prompts, and transform state in declared order

**Independent Test**: Build agent with/without specific middleware, verify middleware transforms and tool injection

- [X] T060 [US5] Implement `Middleware` trait with `name`, `process`, `tools`, `system_prompt_additions` in `crates/synwire-core/src/agents/middleware.rs` (FR-083)
- [X] T061 [US5] Implement `MiddlewareResult` (Continue/Terminate), `MiddlewareInput`, `MiddlewareNext` chain types in `crates/synwire-core/src/agents/middleware.rs` (FR-366)
- [X] T062 [US5] Implement middleware stack execution engine (compose in order, system prompt contributions in order) in `crates/synwire-core/src/agents/middleware.rs` (FR-083, FR-611)
- [X] T063 [P] [US5] Implement `FilesystemMiddleware` exposing backend file ops as agent tools in `crates/synwire-agent/src/middleware/filesystem.rs` (FR-084)
- [X] T064 [P] [US5] Implement `SummarisationMiddleware` with configurable thresholds (message count, token count, context utilization %) in `crates/synwire-agent/src/middleware/summarisation.rs` (FR-089)
- [X] T065 [P] [US5] Implement `PatchToolCallsMiddleware` detecting dangling tool calls in `crates/synwire-agent/src/middleware/patch_tool_calls.rs` (FR-090)
- [X] T066 [P] [US5] Implement `PromptCachingMiddleware` in `crates/synwire-agent/src/middleware/prompt_caching.rs` (FR-091)
- [X] T067 [P] [US5] Implement `EnvironmentMiddleware` exposing env var operations in `crates/synwire-agent/src/middleware/environment.rs` (FR-093)
- [X] T068 [US5] Write unit tests: stack order, early termination, tool injection, prompt composition order in `crates/synwire-core/src/agents/middleware.rs` `#[cfg(test)]` (SC-014)

**Checkpoint**: US5 complete — middleware stack works with correct ordering and tool injection

---

## Phase 10: User Story 6 — Three-Tier Signal Routing (P2)

**Goal**: Signals route through strategy > agent > plugin priority with first-match-wins

**Independent Test**: Define conflicting routes at all three levels, verify strategy-level wins

- [X] T069 [US6] Implement `Signal`, `SignalKind`, `Action` types in `crates/synwire-core/src/agents/signal.rs` (FR-571)
- [X] T070 [US6] Implement `SignalRoute` with kind, predicate, action, priority in `crates/synwire-core/src/agents/signal.rs` (FR-571)
- [X] T071 [US6] Implement `SignalRouter` trait and `ComposedRouter` composing three tiers with debug-level tracing of routing decisions in `crates/synwire-core/src/agents/signal.rs` (FR-572)
- [X] T072 [US6] Write unit tests: strategy route wins over agent/plugin, agent wins over plugin, first-match semantics, predicate filtering, debug log output in `crates/synwire-core/src/agents/signal.rs` `#[cfg(test)]` (SC-103)

**Checkpoint**: US6 complete — three-tier routing resolves correctly

---

## Phase 11: User Story 10 — Approval Gates for Risky Operations (P2)

**Goal**: Backends require user approval before destructive operations

**Independent Test**: Agent with approval gates attempts deletion, verify approval requested with context, verify execution only after approval

- [X] T073 [US10] Implement `ApprovalCallback` trait, `ApprovalRequest`, `ApprovalDecision` (Allow, Deny, AllowAlways, Abort) with modified-input support, `RiskLevel` in `crates/synwire-core/src/backends/approval.rs` (FR-082a–082d, FR-615–616)
- [X] T074 [US10] Implement `ThresholdGate` composable gate (auto-approve up to risk level, delegate above) in `crates/synwire-agent/src/backends/mod.rs` (FR-082b)
- [X] T075 [US10] Integrate approval gates into `FilesystemBackend` write/rm/execute operations in `crates/synwire-agent/src/backends/filesystem.rs` (FR-082a)
- [X] T076 [US10] Implement approval timeout as denial (not queue) in `crates/synwire-core/src/backends/approval.rs` (FR-082d)
- [X] T077 [US10] Write unit tests: gate intercept, approve-then-execute, deny-then-fail, AllowAlways caching, Abort stops agent, timeout=deny, read-only no-approval in `crates/synwire-agent/src/backends/` `#[cfg(test)]` (SC-014a–014c)

**Checkpoint**: US10 complete — approval gates work with all decision types

---

## Phase 12: User Story 8 — Git Version Control Operations (P2)

**Goal**: Agents perform git operations through uniform backend interface

**Independent Test**: Agent queries git status, creates commit, verifies correct metadata

- [X] T078 [P] [US8] Implement `GitBackend` with status, diff, log, commit, push, pull, branch management, scoped to repository path in `crates/synwire-agent/src/backends/git.rs` (FR-081a, FR-081c, FR-081e)
- [X] T079 [P] [US8] Implement `GitMiddleware` exposing git operations as agent tools in `crates/synwire-agent/src/middleware/git.rs` (FR-085)
- [X] T080 [US8] Write unit tests: git status/diff/commit against test repo, scope violation rejection, structured response format in `crates/synwire-agent/src/backends/git.rs` `#[cfg(test)]` (SC-013b, SC-013d)

**Checkpoint**: US8 complete — git operations work within scoped repository

---

## Phase 13: User Story 9 — HTTP Web Operations (P2)

**Goal**: Agents perform HTTP requests through uniform backend interface

**Independent Test**: Agent performs GET/POST against test server, verifies response handling

- [X] T081 [P] [US9] Implement `HttpBackend` with GET, POST, PUT, DELETE, custom methods, headers, timeout, SSL, redirect following in `crates/synwire-agent/src/backends/http.rs` (FR-081b, FR-081d, FR-081f)
- [X] T082 [P] [US9] Implement `HttpMiddleware` exposing web requests as agent tools in `crates/synwire-agent/src/middleware/http.rs` (FR-086)
- [X] T083 [US9] Write unit tests: GET/POST, custom headers, timeout, redirect following, error reporting in `crates/synwire-agent/src/backends/http.rs` `#[cfg(test)]` (SC-013c, SC-013e)

**Checkpoint**: US9 complete — HTTP operations work with proper error handling

---

## Phase 14: User Story 12 — Process Management and Job Control (P2)

**Goal**: Agents list processes, spawn background jobs, manage job lifecycle

**Independent Test**: Spawn background job, list jobs, terminate, verify state tracking

- [X] T084 [P] [US12] Implement `ProcessBackend` with list_processes, kill_process, spawn_background, list_jobs, foreground_job, background_job in `crates/synwire-agent/src/backends/process.rs` (FR-081g, FR-081h)
- [X] T085 [P] [US12] Implement `ProcessMiddleware` exposing process ops as tools with safety guards in `crates/synwire-agent/src/middleware/process.rs` (FR-087)
- [X] T086 [US12] Write unit tests: list processes, spawn/terminate lifecycle, job status tracking in `crates/synwire-agent/src/backends/process.rs` `#[cfg(test)]` (SC-017)

**Checkpoint**: US12 complete — process management works

---

## Phase 15: User Story 13 — Archive and Compression Operations (P2)

**Goal**: Agents create, extract, and inspect compressed archives

**Independent Test**: Create tar.gz from files, extract, verify contents preserved

- [X] T087 [P] [US13] Implement `ArchiveBackend` with create_archive, extract_archive, list_contents for tar/gzip/zip/bzip2 in `crates/synwire-agent/src/backends/archive.rs` (FR-081i–081k)
- [X] T088 [P] [US13] Implement `ArchiveMiddleware` exposing archive ops as tools in `crates/synwire-agent/src/middleware/archive.rs` (FR-088)
- [X] T089 [US13] Write unit tests: create/extract round-trip, conflict resolution policies, circular symlink detection in `crates/synwire-agent/src/backends/archive.rs` `#[cfg(test)]` (SC-018a, SC-021)

**Checkpoint**: US13 complete — archive operations work with conflict resolution

---

## Phase 16: User Story 15 — Stream Handling and Command Pipelines (P2)

**Goal**: Agents compose command pipelines with stream redirection

**Independent Test**: Execute multi-stage pipeline, verify output piped correctly

- [X] T090 [P] [US15] Implement `PipelineExecutor` with multi-stage execution, stream redirection (stdin, stdout, stderr, 2>&1), per-stage timeout in `crates/synwire-agent/src/backends/pipeline.rs` (FR-071a, FR-071b)
- [X] T091 [P] [US15] Implement `PipelineMiddleware` exposing pipeline composition as tools in `crates/synwire-agent/src/middleware/pipeline.rs` (FR-092)
- [X] T092 [US15] Write unit tests: multi-stage pipe, redirect to file, combine stderr+stdout, pipeline stage failure, per-stage timeout in `crates/synwire-agent/src/backends/pipeline.rs` `#[cfg(test)]` (SC-019, SC-022)

**Checkpoint**: US15 complete — pipelines work with proper error propagation

---

## Phase 17: User Story 7 — Streaming Events with Partial and Final Results (P3)

**Goal**: Consumers distinguish partial streaming from final results with clear turn completion

**Independent Test**: Run agent streaming output, collect events, verify partial→final ordering and `is_final_response()`

- [X] T093 [US7] Implement `AgentEvent` enum with all 15 variants (TextDelta, ToolCallStart, ToolCallDelta, ToolCallEnd, ToolResult, ToolProgress, StateUpdate, DirectiveEmitted, StatusUpdate, UsageUpdate, RateLimitInfo, TaskNotification, PromptSuggestion, TurnComplete, Error) in `crates/synwire-core/src/agents/streaming.rs` (FR-157, FR-157a–157f)
- [X] T094 [US7] Implement `TerminationReason` enum (Complete, MaxTurnsExceeded, BudgetExceeded, Stopped, Aborted, Error) in `crates/synwire-core/src/agents/streaming.rs` (FR-158)
- [X] T095 [US7] Implement `is_final_response()` logic (true for TurnComplete, Error; false for all others) in `crates/synwire-core/src/agents/streaming.rs` (FR-159)
- [X] T096 [US7] Implement `UsageUpdate` emission after each model invocation in runner loop in `crates/synwire-core/src/agents/runner.rs` (FR-619)
- [X] T097 [US7] Write unit tests: partial before final ordering, is_final_response for all variants, TurnComplete carries reason, serde round-trip for all event types in `crates/synwire-core/src/agents/streaming.rs` `#[cfg(test)]`

**Checkpoint**: US7 complete — streaming events work with clear completion semantics

---

## Phase 18: Convenience API & Runner (cross-cutting, depends on US1–US7)

**Goal**: Agent builder API, Runner, session management, hooks, MCP — the integration layer

- [X] T098 Implement `AgentNode` trait (name, description, run returning `Stream<AgentEvent>`, sub_agents) in `crates/synwire-core/src/agents/agent_node.rs` (FR-138)
- [X] T099 Implement `OutputMode<T>` enum (Tool, Native, Prompt, Custom) with automatic mode negotiation per model in `crates/synwire-core/src/agents/output_mode.rs` (FR-135, FR-626)
- [X] T100 Implement `RunContext<D>` with typed dependencies, model reference, retry count, usage, metadata in `crates/synwire-core/src/agents/agent_node.rs` (FR-134)
- [X] T101 Implement `Agent<D, O>` builder with all 28 fields from data model (name, model, fallback_model, tools, allowed_tools, excluded_tools, plugins, middleware, hooks, strategy, output_mode, output_schema, max_turns, max_budget, effort, thinking, permission_mode, permission_rules, system_prompt, mcp_servers, sandbox, debug, debug_file, env, cwd) in `crates/synwire-core/src/agents/agent_node.rs` (FR-133, FR-604–605, FR-610, FR-613–614, FR-617, FR-620–622, FR-630–632)
- [X] T102 Implement `ModelSelector` with `by_name`, `by_provider`, `by_capability` (tool-calling, vision, streaming, structured output, effort levels) in `crates/synwire-core/src/agents/model_info.rs` (FR-137, FR-596)
- [X] T103 Implement `ToolResult::Retry(String)` with max 3 retries (configurable), retry message appended to context in `crates/synwire-core/src/tools/types.rs` — extend existing (FR-136)
- [X] T104 Implement `HookRegistry` with typed hook registration, `HookMatcher` (pattern, timeout), `PreToolUseHook`/`PostToolUseHook`/`PostToolUseFailureHook`/`NotificationHook`/`SubagentStartHook`/`SubagentStopHook`/`PreCompactHook`/`PostCompactHook`/`SessionStartHook`/`SessionEndHook` in `crates/synwire-core/src/agents/hooks.rs` (FR-580–590)
- [X] T105 Implement hook execution engine — invoke matching hooks in order, enforce timeout (skip on exceed + warn), pass abort signal in `crates/synwire-core/src/agents/hooks.rs` (FR-587–588, SC-069)
- [X] T106 Implement `SessionManager` trait (list, resume, delete, fork, rewind, tag, rename) in `crates/synwire-core/src/agents/session.rs` (FR-573–579)
- [X] T107 Implement `SessionManager` backed by `synwire-checkpoint` in `crates/synwire-agent/src/session/manager.rs` (FR-573–579)
- [X] T108 Implement `BeforeAgentCallback`, `AfterAgentCallback`, `OnModelErrorCallback` in `crates/synwire-core/src/agents/agent_node.rs` (FR-139, FR-162)
- [X] T109 Implement Runner core loop: session lookup → hook dispatch → middleware chain → agent node → directive filter → directive executor → event emission → usage tracking in `crates/synwire-core/src/agents/runner.rs` (FR-160–163)
- [X] T110 Implement Runner error handling: `RunErrorAction` dispatch per error source (rate-limit→Retry, auth→Abort, tool→Continue), panic catch with backtrace log + `AgentError::Panic` in `crates/synwire-core/src/agents/runner.rs` (FR-364, FR-628–629, SC-033)
- [X] T111 Implement graceful stop (drain in-flight, emit TurnComplete with `Stopped`) and force stop (cancel all, emit TurnComplete with `Aborted`) in `crates/synwire-core/src/agents/runner.rs` (FR-164–165, SC-068)
- [X] T112 Implement `max_turns` enforcement — stop with `TurnComplete { reason: MaxTurnsExceeded }` (not error) in `crates/synwire-core/src/agents/runner.rs` (FR-363, SC-058)
- [X] T113 Implement `max_budget` enforcement — stop with `BudgetExceeded` error when cumulative cost exceeds threshold in `crates/synwire-core/src/agents/runner.rs` (FR-617, SC-066)
- [X] T114 Implement dynamic model switching via `set_model()` preserving conversation state in `crates/synwire-core/src/agents/runner.rs` (FR-592, SC-064)
- [X] T115 Implement fallback model — retry with fallback on rate-limit/unavailability in `crates/synwire-core/src/agents/runner.rs` (FR-594, SC-065)
- [X] T116 Implement `ModelProvider` trait with `list_models()` returning `Vec<ModelInfo>` in `crates/synwire-core/src/agents/model_info.rs` (FR-591)

**Checkpoint**: Core agent runtime assembled — builder, runner, hooks, sessions all functional

---

## Phase 19: MCP Integration

**Goal**: Connect to external MCP servers as dynamic tool providers

- [X] T117 Implement `McpServerConfig` (Stdio, Http, Sse, InProcess variants) in `crates/synwire-core/src/mcp/config.rs` (FR-599)
- [X] T118 Implement `McpTransport` trait (connect, reconnect, status, list_tools, call_tool, disconnect) in `crates/synwire-core/src/mcp/traits.rs` (FR-598)
- [X] T119 Implement `McpServerStatus` and `McpConnectionState` in `crates/synwire-core/src/mcp/traits.rs` (FR-603)
- [X] T120 Implement `OnElicitation` callback trait and `ElicitationRequest`/`ElicitationResult` in `crates/synwire-core/src/mcp/elicitation.rs` (FR-601)
- [X] T121 Implement stdio MCP transport (subprocess management, JSON-RPC over stdin/stdout) in `crates/synwire-agent/src/mcp/stdio.rs` (FR-599)
- [X] T122 [P] Implement HTTP/SSE MCP transports in `crates/synwire-agent/src/mcp/http.rs` (FR-599)
- [X] T123 Implement in-process MCP server creation from tool definitions in `crates/synwire-agent/src/mcp/in_process.rs` (FR-602)
- [X] T124 Implement MCP server lifecycle manager (connect on start, reconnect on failure, health monitoring, runtime toggle) in `crates/synwire-agent/src/mcp/lifecycle.rs` (FR-600)
- [X] T125 Integrate MCP tools into Runner tool resolution (MCP tools available alongside native tools) in `crates/synwire-core/src/agents/runner.rs` (FR-598)
- [X] T126 Write unit tests: connect/disconnect lifecycle, tool listing, tool invocation, reconnect on failure, elicitation round-trip in `crates/synwire-agent/src/mcp/` `#[cfg(test)]` (SC-063)

**Checkpoint**: MCP integration functional — external tool servers connect and provide tools

---

## Phase 20: Polish & Cross-Cutting Concerns

**Purpose**: Re-exports, documentation, conformance suites, quickstart validation

- [X] T127 Update `crates/synwire/src/lib.rs` to re-export agent core types under `synwire::agent::prelude::*` including Agent, Directive, DirectiveResult, AgentEvent, Runner
- [X] T128 [P] Implement `BackendProtocol` conformance test suite in `crates/synwire-test-utils/src/lib.rs` — parameterized test harness for all backend implementations (SC-013)
- [X] T129 [P] Implement MCP transport conformance test suite in `crates/synwire-test-utils/src/lib.rs`
- [X] T130 [P] Implement session manager conformance test suite in `crates/synwire-test-utils/src/lib.rs`
- [X] T131 Add proptest strategies for Directive, AgentEvent, GrepOptions, FsmTransition in `crates/synwire-test-utils/src/lib.rs`
- [X] T132 Run `cargo make ci` — verify all crates compile, clippy clean, tests pass, docs build
- [X] T133 Verify quickstart.md examples compile (at minimum, Agent builder example from quickstart.md §1 compiles in 5 lines) (SC-026)
- [X] T134 Run `cargo make coverage` — verify ≥80% line coverage across new code (SC-018)
- [X] T135 Verify zero `unsafe` blocks in synwire-core and synwire-agent (SC-019)
- [X] T136 Verify all public types are `Send + Sync` via compile-time assertion tests (SC-020)

---

## Phase 21–24: Implemented Work Reconciliation [COMPLETE]

**Purpose**: Document already-shipped crates for traceability. No new implementation tasks.

- [X] T137 [US16] VFS refactor: `backends` → `vfs`, `Vfs` trait, `VfsCapabilities`, `VfsError`, `MemoryProvider`, `LocalProvider`, `CompositeProvider`, `StoreProvider`, ReadGuard, stale-read detection, sandbox separation
- [X] T138 [US17] `synwire-chunker` crate: tree-sitter AST-aware chunking, 14 languages, top-level definitions, text splitter fallback
- [X] T139 [US18] `synwire-embeddings-local` crate: fastembed-rs embeddings (bge-small-en-v1.5 default), cross-encoder reranker
- [X] T140 [US19] `synwire-vectorstore-lancedb` crate: LanceDB `VectorStore` impl
- [X] T141 [US20] `synwire-index` crate: walk → chunk → embed → store pipeline, incremental xxh128 hashing, file watcher, background async indexing, reranking
- [X] T142 [US21] `synwire-lsp` crate: LspClient, 12 capability-conditional tools, plugin, registry
- [X] T143 [US22] `synwire-dap` crate: DapClient, debug tools, plugin, registry, stdio transport
- [X] T144 [US23] `synwire-sandbox` crate: process registry, platform isolation, output capture, plugin

---

## Phase 25: Storage Layout and Project Identity (US36)

**Purpose**: Configurable persistent storage — product-scoped paths, stable project identity, locking, migration, global tier

**CRITICAL**: Blocks Phases 26–34

- [X] T145 [US36] Create `crates/synwire-storage/Cargo.toml` with workspace deps (directories, sha2, serde, serde_json, thiserror, fs2/flock, chrono, tracing) and add to workspace members
- [X] T146 [US36] Create `crates/synwire-storage/src/lib.rs` with `#![forbid(unsafe_code)]` and module declarations
- [X] T147 [P] [US36] Implement `StorageLayout` with product-scoped path computation: `data_home()`, `cache_home()`, `session_db(session_id)`, `index_cache(project_id)`, `graph_dir(project_id)`, `communities_dir(project_id)`, `experience_db(project_id)`, `lsp_cache(project_id)`, `models_cache()`, `logs_dir()`, `skills_dir()`, `project_skills_dirname()`, `repos_cache()`, `repo_cache(owner, repo)`, `global_experience_db()`, `global_dependency_db()`, `global_registry()`, `global_config()` in `crates/synwire-storage/src/layout.rs`
- [X] T148 [P] [US36] Implement two-level identity: `RepoId` (Git first-commit hash, shared across worktrees) + `WorktreeId` (`RepoId` + sha256 of worktree root, identifying a specific working copy). Detect worktree root via `git rev-parse --show-toplevel`. Fallback to `sha256(canonical_path)` for non-Git dirs. `display_name` from repo name + branch. In `crates/synwire-storage/src/identity.rs`
- [X] T148a [US36] Create `crates/synwire-daemon/Cargo.toml` as `[[bin]]` crate with deps (synwire-core, synwire-index, synwire-storage, synwire-embeddings-local, synwire-vectorstore-lancedb, synwire-agent-skills, tokio, clap, tracing, serde_json) and add to workspace members
- [X] T148b [P] [US36] Implement daemon lifecycle: PID file at `$DATA/<product>/daemon.pid`, UDS at `$DATA/<product>/daemon.sock` (named pipe on Windows). Auto-launched by first MCP server as detached process (`Command::new("synwire-daemon").spawn()`). Grace period (5 min) after last client. In `crates/synwire-daemon/src/lifecycle.rs`
- [X] T148c [P] [US36] Implement multi-repo/worktree manager: register repos by `RepoId`, manage per-`WorktreeId` indices as async tasks, shared embedding model + tree-sitter parsers, LRU eviction for idle repos. In `crates/synwire-daemon/src/manager.rs`
- [X] T148d [P] [US36] Implement UDS IPC protocol: request/response for index, search, graph_query, graph_search, community_search, hybrid_search, clone_repo, xref_query, index_status. JSON-RPC or msgpack framing. In `crates/synwire-daemon/src/ipc.rs`
- [X] T148e [P] [US36] Implement MCP server → daemon proxy: MCP server forwards index/search/graph requests to daemon via UDS, handles VFS file ops (read, write, edit, grep) locally. In `crates/synwire-mcp-server/src/proxy.rs`
- [X] T148f [P] [US36] Implement global dependency index parser: extract dependencies from Cargo.toml, go.mod, package.json, pyproject.toml → store in global/dependencies/deps.db via BaseStore in crates/synwire-storage/src/
- [X] T148g [P] [US36] Implement cross-project xref tracking: during indexing, resolve imports against other locally-indexed projects' symbol tables → inter-project edges in global/xrefs/ in crates/synwire-daemon/src/
- [X] T149 [P] [US36] Document and enforce native concurrency conventions: SQLite WAL mode for all SQLite databases, LanceDB concurrent reads, tantivy IndexReader/Writer, atomic rename for binary blobs — no external file locks. Add `ensure_wal_mode()` helper in `crates/synwire-storage/src/concurrency.rs`
- [X] T150 [P] [US36] Implement `StorageMigration` trait with version file checking and copy-then-swap atomic migration in `crates/synwire-storage/src/migration.rs`
- [X] T151 [P] [US36] Implement `ProjectRegistry` reading/writing `global/registry.json` with known projects, last-accessed timestamps, tags in `crates/synwire-storage/src/registry.rs`
- [X] T152 [US36] Implement config hierarchy: `SYNWIRE_DATA_DIR` env > `StorageLayout::with_root()` > project-local `.<product>/config.json` > platform default in `crates/synwire-storage/src/layout.rs`
- [X] T153 [US36] Migrate `synwire-index/src/cache.rs` to use `StorageLayout.index_cache()` instead of hardcoded `"synwire"` product name — `IndexConfig.cache_base` populated by `StorageLayout`
- [X] T154 [US36] Write unit tests: two products isolated, ProjectId stable across directory moves, ProjectId identical across machines (same repo), SQLite WAL concurrent read+write (no blocking, no corruption), atomic rename for binary blobs, migration atomic, config hierarchy precedence in `crates/synwire-storage/src/` `#[cfg(test)]`
- [X] T155 [US36] Run `cargo make ci` — verify synwire-storage compiles, tests pass, clippy clean

**Checkpoint**: StorageLayout functional — all subsequent phases can compute their persistence paths

---

## Phase 26: Per-Method Chunking + File Skeletons (US24–US25)

**Purpose**: Fine-grained method-level search + token-efficient file overviews

- [X] T156 [US24] Extend `collect_definitions` in `crates/synwire-chunker/src/ast_chunker.rs` to recurse one level into `impl_item` (Rust), `class_body`/`class_declaration` (JS/TS/Java/Python/C#/Ruby) producing per-method chunks
- [X] T157 [US24] Add parent type name as context prefix in `symbol` metadata: `Foo::bar` for method `bar` in `impl Foo` in `crates/synwire-chunker/src/ast_chunker.rs`
- [X] T158 [US24] Write tests: Rust `impl` with 5 methods → 5 chunks, Python class → per-method chunks, top-level functions unchanged, nested closures stay inside parent in `crates/synwire-chunker/src/ast_chunker.rs` `#[cfg(test)]`
- [X] T159 [P] [US25] Implement `skeleton` VFS operation in `crates/synwire-core/src/vfs/protocol.rs` — tree-sitter strips bodies, emits signatures + line numbers
- [X] T160 [P] [US25] Implement `skeleton` tool in `crates/synwire-core/src/vfs/tools.rs` exposing the operation as an LLM tool
- [X] T161 [US25] Write tests: skeleton of 10-function file < 25% tokens of full file, all signatures present, unsupported language falls back to full file in `crates/synwire-core/src/vfs/tools.rs` `#[cfg(test)]`
- [X] T162 Run `cargo make ci`

**Checkpoint**: Semantic search returns method-level results; skeleton tool available for localization

---

## Phase 27: Hierarchical Narrowing (US26)

**Purpose**: Agentless-style 3-phase localization

- [X] T163 [US26] Implement hierarchical narrowing as compound tool or middleware composing `tree` → `skeleton`/`document_symbols` → `read_range` in `crates/synwire-agent/src/middleware/` or `crates/synwire-core/src/vfs/tools.rs`
- [X] T164 [US26] Write tests: given test codebase + bug description, narrowing identifies correct file in top-3, correct function in top-3 in `#[cfg(test)]`
- [X] T165 Run `cargo make ci`

**Checkpoint**: Hierarchical narrowing localization functional

---

## Phase 28: Code Dependency Graph (US27)

**Purpose**: Cross-file call/import/inherit graph with disk-backed storage

- [X] T166 [US27] Add `code-graph` feature flag to `crates/synwire-index/Cargo.toml`
- [X] T167 [US27] Implement `CodeGraph` struct with disk-backed adjacency storage (SQLite for edges at Linux kernel scale) in `crates/synwire-index/src/graph/mod.rs`
- [X] T168 [P] [US27] Implement `GraphBuilder` extracting definition→reference edges, import relationships, and call edges from tree-sitter ASTs in `crates/synwire-index/src/graph/builder.rs`
- [X] T169 [P] [US27] Implement node types `(file, symbol)` and typed edges (calls, imports, contains, inherits) in `crates/synwire-index/src/graph/types.rs`
- [X] T170 [US27] Implement `graph_query(symbol, depth, direction)` traversal in `crates/synwire-index/src/graph/query.rs`
- [X] T171 [US27] Implement `graph_search(query, hops)` combining embedding similarity with ego-graph expansion in `crates/synwire-index/src/graph/query.rs`
- [X] T172 [US27] Implement incremental graph update: file change recomputes only that file's edges in `crates/synwire-index/src/graph/builder.rs`
- [X] T173 [US27] Add `graph_query` and `graph_search` VFS operations to `crates/synwire-core/src/vfs/protocol.rs` and tools in `crates/synwire-core/src/vfs/tools.rs`
- [X] T174 [US27] Write tests: cross-file call edges correct, `graph_query` callers at depth 2, `graph_search` finds related code, incremental update recomputes only changed file, 1M+ edges query < 1s in `crates/synwire-index/src/graph/` `#[cfg(test)]`
- [X] T175 Run `cargo make ci`

**Checkpoint**: Code graph built from ASTs, queryable via VFS, disk-backed for Linux kernel scale

---

## Phase 29: Hybrid BM25 + Vector Search (US28)

**Purpose**: Combined lexical + semantic search via tantivy

- [X] T176 [US28] Add `tantivy` dependency to `crates/synwire-index/Cargo.toml` (feature-gated under `hybrid-search`)
- [X] T177 [US28] Implement BM25 index construction alongside vector embedding in `crates/synwire-index/src/pipeline.rs` — built during same indexing pass
- [X] T178 [US28] Implement `hybrid_search(query, alpha, top_k)` combining BM25 and vector scores with configurable alpha weighting in `crates/synwire-index/src/index.rs`
- [X] T179 [US28] Implement incremental BM25 update on file change via tantivy document update in `crates/synwire-index/src/pipeline.rs`
- [X] T180 [US28] Add `hybrid_search` VFS operation and tool in `crates/synwire-core/src/vfs/`
- [X] T181 [US28] Write tests: alpha=1.0 matches pure BM25, alpha=0.0 matches pure vector, alpha=0.5 finds both exact identifiers and semantic matches in `crates/synwire-index/` `#[cfg(test)]`
- [X] T182 Run `cargo make ci`

**Checkpoint**: Hybrid search operational, disk-backed BM25 handles 70K files

---

## Phase 30: GraphRAG Community Detection (US35)

**Purpose**: HIT-Leiden community detection over code graph

- [X] T183 [US35] Add `hit-leiden` dependency to `crates/synwire-index/Cargo.toml` (feature-gated under `community-detection`)
- [X] T184 [US35] Implement community detection integration: code graph edges → HIT-Leiden `GraphInput` → `CommunityState` in `crates/synwire-index/src/`
- [X] T185 [US35] Implement `CommunityState` persistence via `into_parts()`/`from_parts()` in `StorageLayout.communities_dir()` in `crates/synwire-index/src/`
- [X] T186 [US35] Implement incremental community update: file change → delta edges → `CommunityState::update()` in `crates/synwire-index/src/`
- [X] T187 [P] [US35] Implement community summary generation and caching (LLM-generated, stored in `communities/summaries/`, stale-on-change) in `crates/synwire-index/src/`
- [X] T188 [US35] Add `communities`, `community_members`, `community_summary`, `community_search` VFS operations and tools in `crates/synwire-core/src/vfs/`
- [X] T189 [US35] Write tests: strongly-connected clusters form distinct communities (modularity >0.3), incremental update ≥10x faster than full recluster, `community_search` returns relevant results, state round-trips via `into_parts()`/`from_parts()` in `#[cfg(test)]`
- [X] T190 Run `cargo make ci`

**Checkpoint**: Community detection over code graph, incremental updates, searchable summaries

---

## Phase 31: Agent Skills (US33)

**Purpose**: agentskills.io implementation with Lua/Rhai/WASM runtimes

- [X] T191 [US33] Create `crates/synwire-agent-skills/Cargo.toml` with workspace deps (serde, serde_json, serde_yaml, thiserror, tracing, mlua, rhai, extism, synwire-core) and add to workspace members
- [X] T192 [US33] Create `crates/synwire-agent-skills/src/lib.rs` with `#![forbid(unsafe_code)]` and module declarations
- [X] T193 [US33] Implement `SKILL.md` frontmatter parser: `name` (1-64 chars, lowercase+hyphens, matches dir name), `description` (1-1024 chars), `license`, `compatibility`, `metadata` (string→string), `allowed-tools`, synwire `runtime` extension in `crates/synwire-agent-skills/src/manifest.rs`
- [X] T194 [US33] Implement skill discovery: scan `$DATA/<product>/skills/` and `.<product>/skills/`, progressive disclosure (name+description at startup) in `crates/synwire-agent-skills/src/loader.rs`
- [X] T195 [US33] Implement `SkillRegistry` with progressive disclosure: metadata loaded at startup, full body on activation, files on demand in `crates/synwire-agent-skills/src/registry.rs`
- [X] T196 [P] [US33] Implement Lua runtime binding: VFS operations as Lua functions, instruction count limit for infinite loop protection in `crates/synwire-agent-skills/src/runtime/lua.rs`
- [X] T197 [P] [US33] Implement Rhai runtime binding: VFS operations as Rhai functions, `max_operations` limit in `crates/synwire-agent-skills/src/runtime/rhai.rs`
- [X] T198 [P] [US33] Implement Extism WASM runtime binding: PDK host functions for VFS operations, only `allowed-tools` capabilities granted, source code preserved alongside `.wasm` in `crates/synwire-agent-skills/src/runtime/wasm.rs`
- [X] T199 [P] [US33] Implement tool-sequence runtime: execute a sequence of existing tool invocations in `crates/synwire-agent-skills/src/runtime/sequence.rs`
- [X] T200 [P] [US33] Implement external script runtime (subprocess, permitted but discouraged with warning) in `crates/synwire-agent-skills/src/runtime/external.rs`
- [X] T201 [US33] Implement `CreateTool` directive support: agent emits directive with Lua/Rhai script → skill manifest auto-generated → persisted to skills dir in `crates/synwire-core/src/agents/directive.rs` + `crates/synwire-agent-skills/src/`
- [X] T202 [US33] Implement skill validation: frontmatter schema, directory name match, runtime availability, entrypoint existence, permission compatibility with `SandboxConfig` in `crates/synwire-agent-skills/src/loader.rs`
- [X] T203 [US33] Write tests: manifest parsing (valid + invalid), Lua skill execution, Rhai skill execution, WASM skill sandboxed (denied capability rejected), external script warning, progressive disclosure, versioning (higher replaces lower), name collision rejection in `crates/synwire-agent-skills/` `#[cfg(test)]`
- [X] T204 Run `cargo make ci`

**Checkpoint**: Agent skills loadable, executable in all runtimes, validated against agentskills.io spec

---

## Phase 32: Standalone MCP Server Binary (US38) — P0

**Purpose**: Ship `synwire-mcp-server` binary exposing all tools via MCP stdio

- [X] T205 [US38] Create `crates/synwire-mcp-server/Cargo.toml` as `[[bin]]` crate with deps (clap, synwire-core, synwire-agent, synwire-index, synwire-storage, synwire-agent-skills, synwire-lsp, synwire-dap, synwire-embeddings-local, synwire-vectorstore-lancedb, tracing-subscriber, tracing-appender, serde_json, tokio)
- [X] T206 [US38] Implement CLI arg parsing: `--project`, `--product-name`, `--lsp`, `--dap`, `--embedding-model`, `--log-level`, `--config` in `crates/synwire-mcp-server/src/cli.rs`
- [X] T207 [US38] Implement TOML/JSON config file loading as alternative to CLI flags in `crates/synwire-mcp-server/src/config.rs`
- [X] T208 [US38] Implement MCP server setup: `StorageLayout` init → VFS (`CompositeProvider` with `LocalProvider` for project) → tool registration → stdio JSON-RPC loop in `crates/synwire-mcp-server/src/server.rs`
- [X] T209 [US38] Implement VFS tool definitions as MCP tools with JSON Schema parameters and LLM-optimised descriptions: `read`, `write`, `edit`, `grep`, `glob`, `find`, `tree`, `head`, `tail`, `stat`, `ls`, `diff`, `skeleton`, `index`, `index_status`, `semantic_search`, `hybrid_search` in `crates/synwire-mcp-server/src/tools.rs`
- [X] T210 [P] [US38] Implement code graph MCP tools: `graph_query`, `graph_search`, `community_search`, `community_members` in `crates/synwire-mcp-server/src/tools.rs`
- [X] T211 [P] [US38] Implement `clone_repo` MCP tool (FR-846–857): git clone → mount → optional index in `crates/synwire-mcp-server/src/tools.rs`
- [X] T212 [P] [US38] Implement LSP tool exposure when `--lsp` configured in `crates/synwire-mcp-server/src/server.rs`
- [X] T213 [P] [US38] Implement DAP tool exposure when `--dap` configured in `crates/synwire-mcp-server/src/server.rs`
- [X] T214 [US38] Implement agent skills auto-discovery and tool registration from skills directories in `crates/synwire-mcp-server/src/server.rs`
- [X] T215 [US38] Implement `tracing` logging: stderr (info default, `RUST_LOG` configurable) + rotated log files in `StorageLayout.logs_dir()` (7-day retention) in `crates/synwire-mcp-server/src/main.rs`
- [X] T216 [US38] Implement multi-instance safety: SQLite WAL mode for all databases, LanceDB/tantivy native concurrent access, verify cross-instance index visibility (one instance indexes, another sees results on next query) in `crates/synwire-mcp-server/src/server.rs`
- [X] T217 [US38] Implement clean shutdown on stdio pipe close: cancel background tasks, release locks in `crates/synwire-mcp-server/src/main.rs`
- [X] T218 [US38] Write integration tests: start MCP server, connect via stdio, call `index` + `semantic_search` + `grep`, verify results; two concurrent instances sharing index; clean shutdown on pipe close in `crates/synwire-mcp-server/tests/`
- [X] T219 [US38] Verify `cargo install synwire-mcp-server` produces a single static binary
- [X] T220 Run `cargo make ci`

**Checkpoint**: MCP server binary usable in Claude Code / Copilot / Cursor

---

## Phase 32a: Tool Search (US39)

**Purpose**: Framework-level progressive tool discovery with ToolSearchIndex

- [X] T220a [US39] Implement ToolSearchIndex struct in crates/synwire-core/src/tools/ with: tool registration, namespace grouping, multi-vector embedding (description + example queries), hybrid scoring (vector + keyword boosting)
- [X] T220b [P] [US39] Implement tool_search StructuredTool meta-tool: semantic search mode + namespace browsing mode in crates/synwire-core/src/tools/
- [X] T220c [P] [US39] Implement tool_list StructuredTool meta-tool: namespace-grouped listing with names+descriptions only in crates/synwire-core/src/tools/
- [X] T220d [P] [US39] Implement DisclosureDepth enum (Minimal/Summary/Parameters/Full) with render(depth) per tool in crates/synwire-core/src/tools/
- [X] T220e [P] [US39] Implement token budget allocator: top-5 full, next-10 summary, rest minimal, 5K token cap in crates/synwire-core/src/tools/
- [X] T220f [US39] Implement deterministic tool registry hashing (BTreeMap + sha256) for incremental re-embedding in crates/synwire-core/src/tools/
- [X] T220g [US39] Implement hybrid score fusion: normalised vector score + field-weighted keyword boosting (namespace +5.0, name +3.0, description +2.0, tags +1.5) in crates/synwire-core/src/tools/
- [X] T220h [US39] Integrate ToolSearchIndex into MCP server: tools/list returns compact entries, tool_search available as MCP tool, defer_loading support in crates/synwire-mcp-server/src/
- [X] T220i [US39] Write tests: tool_search returns correct top-3 for natural language queries, namespace browsing returns exact set, hybrid scoring ranks exact name match first, deduplication works in #[cfg(test)]
- [X] T220j Run cargo make ci

**Checkpoint**: Tool search functional, token reduction verified

---

## Phase 32b: MCP Sampling Integration

**Purpose**: SamplingProvider for tool-internal LLM access

- [X] T220k [US38] Implement SamplingProvider trait in crates/synwire-core/src/agents/ with sample() and is_available() methods
- [X] T220l [P] [US38] Implement McpSampling: delegates to MCP client via sampling/createMessage in crates/synwire-mcp-server/src/
- [X] T220m [P] [US38] Implement DirectModelSampling: uses configured BaseChatModel for standalone mode in crates/synwire-agent/src/
- [X] T220n [P] [US38] Implement community_summary sampling integration: structured prompt with member symbols + edges → SamplingProvider → cached result (FR-882) in crates/synwire-index/src/
- [X] T220o [P] [US38] Implement hierarchical narrowing sampling integration: file ranking prompt + function ranking prompt → SamplingProvider (FR-883, 2 calls per invocation) in crates/synwire-agent/src/middleware/
- [X] T220p2 [P] [US38] Implement experience pool summary sampling: post-edit diff + affected files → SamplingProvider → summary (FR-884, 1 call per edit) in crates/synwire-agent/src/
- [X] T220q [US38] Implement graceful degradation for each: community_summary → member list, narrowing → alphabetical, experience → raw associations (FR-885) in crates/synwire-core/src/
- [X] T220r [US38] Write tests: sampling produces summary, timeout falls back to degradation, refusal falls back, zero sampling calls during indexing, bounded call counts verified per FR-886 in #[cfg(test)]
- [X] T220p Run cargo make ci

**Checkpoint**: Sampling functional with graceful degradation

---

## Phase 32c: Auto Repository Clone and Mount (US37)

**Purpose**: Clone repos by URL, mount into VFS, auto-detect repeated GitHub fetches

- [X] T237 [US37] Implement `clone_repo` VFS tool: git clone to `StorageLayout.repos_cache()`, mount as `LocalProvider` in `CompositeProvider`, optional `index: true` trigger in `crates/synwire-core/src/vfs/tools.rs` + `crates/synwire-agent/src/vfs/`
- [X] T238 [P] [US37] Implement `StorageLayout.repos_cache()` and `repo_cache(owner, repo)` path accessors in `crates/synwire-storage/src/layout.rs`
- [X] T239 [P] [US37] Implement `RepoFetchDetector` middleware: monitor web_fetch/HTTP calls for `raw.githubusercontent.com`/`github.com/*/blob/*` patterns, emit `PromptSuggestion` after 3+ fetches from same repo in `crates/synwire-agent/src/middleware/`
- [X] T240 [P] [US37] Implement `repo_gc(max_age_days)`: remove cloned repos not accessed within configured period, skip currently-mounted repos in `crates/synwire-agent/src/vfs/` or `crates/synwire-storage/src/`
- [X] T241 [US37] Implement clone update: `git fetch` + checkout for already-cloned repos, shallow clone support (`depth` parameter), ref/tag checkout in `crates/synwire-agent/src/vfs/`
- [X] T242 [US37] Implement session-state recording of mounted repos for re-mounting on session resume in `crates/synwire-agent/src/session/`
- [X] T243 [US37] Write tests: clone + mount + grep works, RepoFetchDetector fires after 3 fetches, repo_gc skips mounted repos, shallow clone, update via fetch in `#[cfg(test)]`
- [X] T244 Run `cargo make ci`

**Checkpoint**: Clone-and-mount functional, RepoFetchDetector detects repeated fetches

---

## Phase 32d: Tool Search Enhancements from Research (US39)

**Purpose**: Paper-derived improvements for ToolSearchIndex (FR-909–915)

- [X] T245 [US39] Implement `search_progressive(query, steps, per_step_k)` with iterative residual retrieval: embed → retrieve → subtract → repeat, deduplicate results (FR-909) in `crates/synwire-core/src/tools/`
- [X] T246 [P] [US39] Implement `ToolTransitionGraph`: record_transition(from, to), boost_successors(), exponential decay with configurable half-life (100 invocations default), persist across sessions (FR-910) in `crates/synwire-core/src/tools/`
- [X] T247 [P] [US39] Implement `QueryPreprocessor` trait + `IntentExtractor` heuristic (extract verb-object pairs from long queries) (FR-911) in `crates/synwire-core/src/tools/`
- [X] T248 [P] [US39] Implement seen/unseen adaptive scoring: penalty factor (default 0.8) for tools in `loaded_schemas` (FR-912) in `crates/synwire-core/src/tools/`
- [X] T249 [P] [US39] Implement enriched `ToolSearchResult` diagnostics: `nearest_namespace`, `alternative_keywords`, `confidence_level` for low-score results (FR-913) in `crates/synwire-core/src/tools/`
- [X] T250 [P] [US39] Implement feedback loop: extract successful query→tool pairs from invocation logs, add as example queries (capped at 10 per tool, diversity-filtered), background re-embedding (FR-914) in `crates/synwire-core/src/tools/`
- [X] T251 [P] [US39] Implement parameter-type verification heuristic: post-retrieval filter matching implied parameter types (file paths → file tools, function names → LSP tools, patterns → search tools) (FR-915) in `crates/synwire-core/src/tools/`
- [X] T252 Write tests: progressive retrieval finds multi-step tools, transition graph boosts correct successors, adaptive scoring surfaces unseen tools, intent extraction strips context, parameter verification demotes mismatches in `#[cfg(test)]`
- [X] T253 Run `cargo make ci`

**Checkpoint**: All paper-derived tool search enhancements functional

---

## Phase 35: Documentation (FR-889–896)

**Purpose**: READMEs, rustdoc, mdBook, guides, migration docs for all new crates

- [X] T254 [P] Write README.md for `synwire-storage`
- [X] T255 [P] Write README.md for `synwire-agent-skills`
- [X] T256 [P] Write README.md for `synwire-mcp-server`
- [X] T257 [P] Write `docs/src/explanation/synwire-daemon.md` (daemon crate is planned; explanation doc written)
- [X] T258 [P] Add READMEs: synwire-chunker, synwire-index, synwire-embeddings-local, synwire-vectorstore-lancedb
- [X] T259 No new public items added in phase 35; pre-existing doc warnings not introduced here
- [X] T260 [P] Write mdBook explanation docs: synwire-storage.md, synwire-daemon.md written; existing chunker/index/embeddings/llm-providers docs already complete
- [X] T261 [P] MCP server section added to mcp-integration.md; hybrid search section added to semantic-search.md; migration.md written
- [X] T262 [P] SUMMARY.md updated with tutorial placeholders 11-13; stub files created
- [X] T263 feature-flags.md updated (synwire-index, synwire-agent-skills); crate-organisation.md updated (all new crates + graph); glossary updated (7 new terms)
- [X] T264 All CLI fields have `#[arg(help)]` annotations in synwire-mcp-server/src/cli.rs
- [X] T265 Write migration guide: docs/src/how-to/migration.md
- [X] T266 Doc build: no errors in new crates; no new warnings introduced in phase 35

**Checkpoint**: All documentation complete for new crates and features

---

## Phase 33: Repository Memory + Research Features (US29–US32, US34)

**Purpose**: Experience pool, SBFL, dynamic call graph, MCTS, dataflow

- [X] T221 [P] [US30] Implement global + project-local experience pool using `BaseStore` (`SqliteStore`) at paths from `StorageLayout` in `crates/synwire-agent/src/`
- [X] T222 [P] [US30] Implement auto-recording of edit→file associations on agent edit completion in `crates/synwire-agent/src/`
- [X] T223 [P] [US30] Implement two-tier query: project-local first, global fallback in `crates/synwire-agent/src/`
- [X] T224 [P] [US29] Implement SBFL Ochiai scoring from DAP coverage data in `crates/synwire-agent/src/`
- [X] T225 [P] [US29] Implement SBFL+semantic search fusion tool/middleware in `crates/synwire-agent/src/`
- [X] T226 [P] [US31] Implement dynamic call graph construction via LSP goto-definition on demand in `crates/synwire-agent/src/`
- [X] T227 [P] [US32] Implement MCTS execution strategy with configurable value function and search depth in `crates/synwire-agent/src/strategies/`
- [X] T228 [P] [US34] Implement dataflow retrieval tracing variable origins via LSP + tree-sitter heuristics in `crates/synwire-agent/src/`
- [X] T229 Write tests for each: experience pool cross-session, SBFL ranking, dynamic graph cycle detection, MCTS depth scaling, dataflow 2-hop tracing in `#[cfg(test)]`
- [X] T230 Run `cargo make ci`

**Checkpoint**: Research features functional

---

## Phase 34: Cross-Project Features (US36 global tier)

**Purpose**: Global dependency index, cross-project code references, project registry

- [X] T231 [P] [US36] Implement dependency index: parse `Cargo.toml`/`go.mod`/`package.json`/`pyproject.toml` → store project→dependency edges in `global/dependencies/deps.db` via `BaseStore` in `crates/synwire-storage/src/`
- [X] T232 [P] [US36] Implement cross-project xref graph: during indexing, resolve imports against locally-indexed projects' symbol tables → inter-project edges in `global/xrefs/` in `crates/synwire-index/src/`
- [X] T233 [US36] Implement `xref_query(symbol, direction)` returning cross-project references in `crates/synwire-index/src/`
- [X] T234 [US36] Implement stale xref invalidation: mark edges stale when provider or consumer re-indexes, lazy rebuild on query in `crates/synwire-index/src/`
- [X] T235 Write tests: dependency index answers "which projects use library X?", xref_query finds cross-project call sites, incremental xref build matches simultaneous build in `#[cfg(test)]`
- [X] T236 Run `cargo make ci`

**Checkpoint**: Cross-project dependency tracking and code references functional

---

## Phase 36: MCP Multi-Server Client + WebSocket Transport (US40, FR-916–925)

**Purpose**: `MultiServerMcpClient`, WebSocket transport, `McpClientSession`, cursor pagination, server health monitoring

- [X] T267 [P] [US40] Create `crates/synwire-mcp-adapters/` crate scaffold: `Cargo.toml` (deps: synwire-core, synwire-agent, rmcp, tokio, serde, serde_json, thiserror, tracing, tokio-tungstenite, jsonschema), `src/lib.rs` with module declarations, `src/error.rs` with `McpAdapterError` enum (ServerNotFound, Transport, ConnectionFailed, Timeout, ToolNotFound, SchemaValidation, UnsupportedContent)
- [X] T268 [P] [US40] Implement `Connection` enum (Stdio, Sse, StreamableHttp, WebSocket) with transport-specific config fields in `crates/synwire-mcp-adapters/src/lib.rs`
- [X] T269 [US40] Implement WebSocket transport via `tokio-tungstenite` implementing `McpTransport` trait in `crates/synwire-mcp-adapters/src/transport/websocket.rs`
- [X] T270 [US40] Implement `McpClientSession` with guard-based cleanup (drop teardown) wrapping `McpTransport` + cached tool list + connection state in `crates/synwire-mcp-adapters/src/session.rs`
- [X] T271 [US40] Implement cursor-based pagination helper with 1000-page safeguard cap in `crates/synwire-mcp-adapters/src/pagination.rs`
- [X] T272 [US40] Implement `MultiServerMcpClient`: accepts `HashMap<String, Connection>`, simultaneous connect via `futures::join_all`, tool aggregation with optional `tool_name_prefix`, per-server health status, `get_tools()` returning `Vec<Box<dyn Tool>>` in `crates/synwire-mcp-adapters/src/client.rs`
- [X] T273 [US40] Implement `McpCallbacks` struct with LoggingMessage + Progress + Elicitation slots in `crates/synwire-mcp-adapters/src/callbacks.rs`
- [X] T274 Write tests: simultaneous connect to 2 mock servers, tool prefixing, pagination cap, health status updates, guard cleanup on drop in `#[cfg(test)]`
- [X] T275 Run `cargo make ci`

**Checkpoint**: MultiServerMcpClient connects to multiple servers and aggregates tools

---

## Phase 37: MCP Tool Conversion + Content Mapping (US41–US42, FR-926–937)

**Purpose**: Bidirectional MCP↔Synwire tool conversion, content type mapping, resource and prompt retrieval

- [X] T276 [P] [US41] Implement `convert_mcp_tool_to_synwire_tool()`: MCP tool → Synwire `Tool` with name, description, schema, MCP annotations as metadata, `(content, artifact)` return in `crates/synwire-mcp-adapters/src/convert/tool.rs`
- [X] T277 [P] [US41] Implement `to_mcp_tool()`: Synwire tool → MCP tool definition, validate `args_schema`, reject injected arguments in `crates/synwire-mcp-adapters/src/convert/tool.rs`
- [X] T278 [P] [US41] Implement content type mapping: Text/Image/ResourceLink/EmbeddedResource direct conversion, AudioContent → UnsupportedContent, `isError` → ToolException in `crates/synwire-mcp-adapters/src/convert/content.rs`
- [X] T279 [P] [US42] Implement `get_resources()`, `convert_mcp_resource_to_blob()`, `load_mcp_resources()`, `get_mcp_resource()` — static resources only, dynamic excluded in `crates/synwire-mcp-adapters/src/convert/resource.rs`
- [X] T280 [P] [US42] Implement `get_prompt()`, `convert_mcp_prompt_message()` with role-based mapping and multi-content support in `crates/synwire-mcp-adapters/src/convert/prompt.rs`
- [X] T281 Write tests: tool round-trip (Synwire → MCP → Synwire preserves metadata), content type mapping, isError → ToolException, resource loading, prompt conversion in `#[cfg(test)]`
- [X] T282 Run `cargo make ci`

**Checkpoint**: Bidirectional tool conversion functional

---

## Phase 38: Tool Call Interceptors + JSON Schema Validation (US43, FR-938–945)

**Purpose**: Onion/middleware interceptor chain, panic safety, client-side schema validation

- [X] T283 [P] [US43] Implement `ToolCallInterceptor` trait with `McpToolCallRequest` → `McpToolCallResult`, `InterceptorNext` type, onion chain executor with correct ordering in `crates/synwire-mcp-adapters/src/interceptor.rs`
- [X] T284 [P] [US43] Implement panic-safe interceptor execution via `catch_unwind` at each boundary, converting panics to `McpAdapterError::Transport` in `crates/synwire-mcp-adapters/src/interceptor.rs`
- [X] T285 [P] [US43] Implement JSON Schema validation of tool arguments before invocation using `jsonschema` crate in `crates/synwire-mcp-adapters/src/validation.rs`
- [X] T286 Wire interceptor chain into `MultiServerMcpClient` tool invocation path in `crates/synwire-mcp-adapters/src/client.rs`
- [X] T287 Write tests: 3-interceptor onion order (A→B→C→tool→C→B→A), short-circuit interceptor, panic-safe recovery, schema validation reject + accept in `#[cfg(test)]`
- [X] T288 Run `cargo make ci`

**Checkpoint**: Interceptor chain and schema validation functional

---

## Phase 39: Tool Provider Abstraction + Classification (US44, US46, FR-946–955)

**Purpose**: `ToolProvider` trait, Static/MCP/Composite implementations, `ToolCategory`, `ToolKind`, `ToolContentType`

- [X] T289 [P] [US46] Add `ToolCategory` enum (Builtin, Custom, Mcp, Remote, WorkflowAsTool) and `ToolKind` enum (Read, Edit, Search, Execute, Other) to `crates/synwire-core/src/tools/types.rs`
- [X] T290 [P] [US46] Add `ToolContentType` enum (Text, Image, File, Json) and `content_type` field to `ToolOutput` in `crates/synwire-core/src/tools/types.rs`
- [X] T291 [P] [US44] Define `ToolProvider` trait with `discover_tools()` and `get_tool()` in `crates/synwire-core/src/tools/traits.rs`
- [X] T292 [P] [US44] Implement `StaticToolProvider` (wraps `Vec<Box<dyn Tool>>`) in `crates/synwire-core/src/tools/structured.rs`
- [X] T293 [P] [US44] Implement `CompositeToolProvider` aggregating multiple providers with configurable name collision policy in `crates/synwire-core/src/tools/structured.rs`
- [X] T294 [US44] Implement `McpToolProvider` backed by `MultiServerMcpClient` in `crates/synwire-mcp-adapters/src/provider.rs`
- [X] T295 Write tests: StaticToolProvider discover/get, CompositeToolProvider aggregation + collision handling, McpToolProvider from mock servers, ToolCategory/ToolKind on various tool types in `#[cfg(test)]`
- [X] T296 Run `cargo make ci`

**Checkpoint**: Tool provider abstraction and classification functional

---

## Phase 40: Tool Operational Controls (US45, FR-956–963)

**Purpose**: Per-tool timeout, usage limits, enablement, name validation, result truncation, argument validation

- [X] T297 [P] [US45] Add `ToolConfig` struct (timeout, timeout_behavior, is_enabled, max_usage_count, max_result_size) to `crates/synwire-core/src/tools/types.rs`
- [X] T298 [P] [US45] Implement tool name validation regex `^[a-zA-Z0-9_-]{1,64}$` enforced at construction time in `crates/synwire-core/src/tools/traits.rs`
- [X] T299 [P] [US45] Implement per-session usage counting with `ToolUsageLimitExceeded` error in `crates/synwire-core/src/tools/structured.rs`
- [X] T300 [US45] Implement timeout enforcement with `TimeoutBehavior::ReturnError` / `RaiseException` in `crates/synwire-orchestrator/src/prebuilt/tool_node.rs`
- [X] T301 [US45] Implement result truncation at `max_result_size` (default 100KB) in `crates/synwire-orchestrator/src/prebuilt/tool_node.rs`
- [X] T302 [US45] Implement argument validation via JSON Schema before tool invocation in `crates/synwire-orchestrator/src/prebuilt/tool_node.rs`
- [X] T303 Write tests: timeout (100ms tool × 500ms op → error), usage limit (3 calls ok, 4th rejected), disabled tool excluded from schema, invalid name rejected, truncation, arg validation in `#[cfg(test)]`
- [X] T304 Run `cargo make ci`

**Checkpoint**: Tool operational controls enforced

---

## Phase 41: `#[tool]` Proc-Macro Enhancements (US47, FR-964–967)

**Purpose**: Generate full `Tool` impl from async fn with name, description, JSON Schema, category, kind

- [X] T305 [US47] Extend `#[tool]` proc-macro to generate `name()` from function name, `description()` from attribute, `args_schema()` from parameter types via schemars, invocation wrapper in `crates/synwire-derive/src/tool.rs`
- [X] T306 [US47] Add `#[tool(kind = "edit")]` and `#[tool(category = "custom")]` optional attributes in `crates/synwire-derive/src/tool.rs`
- [X] T307 [US47] Add compile-time validation: return type must be `Result<ToolOutput, ToolError>`, at least one parameter in `crates/synwire-derive/src/tool.rs`
- [X] T308 Write tests: basic async fn → Tool impl, schema generation, kind/category attributes, compile-fail test for wrong return type in `crates/synwire-derive/tests/`
- [X] T309 Run `cargo make ci`

**Checkpoint**: `#[tool]` macro generates working Tool implementations

---

## Phase 42: Compiled Graph as Tool (US48, FR-968–970)

**Purpose**: `CompiledGraph::as_tool()`, graph-as-node composition

- [X] T310 [US48] Implement `CompiledGraph::as_tool()` returning `Box<dyn Tool>` that wraps graph execution — input state from JSON args, output state as `ToolOutput` with `ToolCategory::WorkflowAsTool` in `crates/synwire-orchestrator/src/`
- [X] T311 [US48] Implement graph-as-node: allow `CompiledGraph` to be used as a node within another `StateGraph` with independent state lifecycle in `crates/synwire-orchestrator/src/`
- [X] T312 [US48] Implement error propagation: inner graph errors → tool errors with full context (graph name, node name, original error) in `crates/synwire-orchestrator/src/`
- [X] T313 Write tests: as_tool() invocation from outer graph, graph-as-node execution, error propagation, independent checkpoint state in `#[cfg(test)]`
- [X] T314 Run `cargo make ci`

**Checkpoint**: Graph-in-graph composition functional via as_tool() and graph-as-node

---

## Dependencies & Execution Order

### Phase Dependencies (Phases 1–20, complete)

- **Phase 1 (Setup)**: No dependencies — start immediately
- **Phase 2 (Foundational)**: Depends on Phase 1 — BLOCKS all user stories
- **Phases 3–7 (P1 stories)**: Depend on Phase 2. Can proceed in parallel
- **Phases 8–16 (P2 stories)**: Depend on Phase 2. Can proceed in parallel
- **Phase 17 (P3 — US7)**: Depends on US1
- **Phase 18 (Runner)**: Depends on US1–US7
- **Phase 19 (MCP)**: Depends on Phase 18
- **Phase 20 (Polish)**: Depends on all prior phases

### Phase Dependencies (Phases 21–35, complete)

- **Phases 21–35**: All complete (243 tasks, all [X])

### Phase Dependencies (Phases 36–42, MCP Adapters)

- **Phase 36 (MultiServerMcpClient)**: Depends on Phase 19 (existing MCP traits). Independent of Phases 25–35
- **Phase 37 (Tool Conversion)**: Depends on Phase 36
- **Phase 38 (Interceptors)**: Depends on Phase 36
- **Phase 39 (ToolProvider + Classification)**: Depends on Phase 36. Modifies synwire-core
- **Phase 40 (Operational Controls)**: Depends on Phase 39
- **Phase 41 (#[tool] Macro)**: Depends on Phase 39
- **Phase 42 (Graph as Tool)**: Depends on Phase 39

### Parallel Opportunities (Phases 36–42, MCP Adapters)

Can start immediately (independent of Phases 25–35):
```text
Agent E: Phase 36 → 37 → 38 (MCP client + conversion + interceptors)
Agent F: Phase 39 → 40 → 41 → 42 (ToolProvider + controls + macro + graph-as-tool)
```

Phase 37 and 38 can run in parallel after Phase 36 completes.
Phase 40, 41, 42 can run in parallel after Phase 39 completes.

---

## Implementation Strategy

### Priority Order (expanded)

**Track A (Phases 25–35 — all complete)**

**Track B (MCP Adapters — can start now, independent of Track A)**:
1. **Phase 36** — MultiServerMcpClient + WebSocket (core MCP client)
2. **Phase 37** — Tool conversion + content mapping (bidirectional interop)
3. **Phase 38** — Interceptors + schema validation (middleware layer)
4. **Phase 39** — ToolProvider + classification (tool discovery)
5. **Phase 40** — Operational controls (production guardrails)
6. **Phase 41** — `#[tool]` proc-macro (developer ergonomics)
7. **Phase 42** — `CompiledGraph::as_tool()` (graph composition)

### Incremental MCP Server Releases

| Version | Includes | Tasks |
|---------|----------|-------|
| v0.1 | VFS tools + grep + semantic search (existing) | T205–T209, T215–T220 |
| v0.2 | + per-method chunking + skeletons + hierarchical narrowing | T210 after Phase 26–27 |
| v0.3 | + code graph + hybrid search | T210 after Phase 28–29 |
| v0.4 | + agent skills | T214 after Phase 31 |
| v0.5 | + community detection + cross-project | T210 after Phase 30, 34 |
| v1.0 | + research features (SBFL, MCTS, dataflow) | after Phase 33 |

---

## Notes

- [P] tasks = different files, no dependencies on incomplete tasks
- [Story] label maps task to specific user story for traceability
- Each user story checkpoint is independently testable
- Commit after each task or logical group
- `cargo make ci` at each checkpoint to catch regressions
- Total: 340 tasks across 42 phases (286 complete, 54 pending — 48 new in Phases 36–42)
