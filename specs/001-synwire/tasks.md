# Tasks: Synwire M1 — Core + Orchestrator + Providers + Derive

**Input**: Design documents from `/specs/001-synwire/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/

**Tests**: Included — the constitution mandates BDD test-first (Principle IV, NON-NEGOTIABLE), and the spec requires 90% coverage on synwire-core (SC-002) and 80% on synwire-orchestrator (SC-010).

**Organisation**: Tasks are grouped by user story to enable independent implementation and testing of each story. Cross-cutting concerns (observability, testing infrastructure, documentation) follow in dedicated phases.

**Scope**: Strictly M1. M2 (Agents + MCP) and M3 (Protocols + DSPy + Evals) are excluded. See [roadmap](../../docs/roadmap.md) for M2/M3 scope.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Workspace root**: `Cargo.toml`
- **Core crate**: `crates/synwire-core/src/`
- **Orchestrator crate**: `crates/synwire-orchestrator/src/`
- **Checkpoint traits crate**: `crates/synwire-checkpoint/src/`
- **SQLite checkpoint crate**: `crates/synwire-checkpoint-sqlite/src/`
- **OpenAI LLM crate**: `crates/synwire-llm-openai/src/`
- **Ollama LLM crate**: `crates/synwire-llm-ollama/src/`
- **Derive crate**: `crates/synwire-derive/src/`
- **Re-export crate**: `crates/synwire/src/`
- **Test utilities**: `crates/synwire-test-utils/src/`
- **Checkpoint conformance**: `crates/synwire-checkpoint-conformance/src/`
- **Examples**: `examples/`
- **Integration tests**: `tests/integration/`

---

## Phase 1: Setup

**Purpose**: Cargo workspace initialisation and project scaffolding

- [X] T001 Create workspace root Cargo.toml with all M1 members (synwire-core, synwire-orchestrator, synwire-checkpoint, synwire-checkpoint-sqlite, synwire-llm-openai, synwire-llm-ollama, synwire, synwire-derive, synwire-test-utils, synwire-checkpoint-conformance) and shared workspace dependencies in Cargo.toml
- [X] T002 [P] Create synwire-core crate Cargo.toml (deps: futures-core, futures-util, pin-project-lite, thiserror, serde, serde_json, uuid, chrono, secrecy; optional: tracing, tracing-opentelemetry, opentelemetry, backoff, reqwest with rustls) in crates/synwire-core/Cargo.toml
- [X] T003 [P] Create synwire-orchestrator crate Cargo.toml (deps: synwire-core, tokio, serde, serde_json, uuid, futures, json-patch) in crates/synwire-orchestrator/Cargo.toml
- [X] T004 [P] Create synwire-checkpoint crate Cargo.toml (deps: synwire-core, serde, serde_json, chrono, tokio) in crates/synwire-checkpoint/Cargo.toml
- [X] T005 [P] Create synwire-checkpoint-sqlite crate Cargo.toml (deps: synwire-checkpoint, rusqlite, r2d2) in crates/synwire-checkpoint-sqlite/Cargo.toml
- [X] T006 [P] Create synwire-llm-openai crate Cargo.toml (deps: synwire-core, reqwest with rustls, eventsource-stream, tokio, serde, serde_json) in crates/synwire-llm-openai/Cargo.toml
- [X] T007 [P] Create synwire-llm-ollama crate Cargo.toml (deps: synwire-core, reqwest with rustls, tokio, serde, serde_json) in crates/synwire-llm-ollama/Cargo.toml
- [X] T008 [P] Create synwire re-export crate Cargo.toml (deps: synwire-core, optional: synwire-llm-openai, synwire-llm-ollama; moka for cache) in crates/synwire/Cargo.toml
- [X] T009 [P] Create synwire-derive crate Cargo.toml (proc-macro = true; deps: syn, quote, proc-macro2, schemars) in crates/synwire-derive/Cargo.toml
- [X] T010 [P] Create synwire-test-utils crate Cargo.toml (dev-dependency crate; deps: synwire-core, proptest, tokio-test, mockall) in crates/synwire-test-utils/Cargo.toml
- [X] T011 [P] Create synwire-checkpoint-conformance crate Cargo.toml (deps: synwire-checkpoint, synwire-core, tokio) in crates/synwire-checkpoint-conformance/Cargo.toml
- [X] T012 [P] Add .gitignore for Rust (target/, *.lock for libraries) in .gitignore
- [X] T013 [P] Add rustfmt.toml with edition = "2024", max_width = 100, use_field_init_shorthand = true, imports_granularity = "Crate", group_imports = "StdExternalCrate" in rustfmt.toml
- [X] T014 [P] Configure workspace lints in Cargo.toml: [workspace.lints.clippy] deny unwrap_used, expect_used, panic, todo, unimplemented, dbg_macro, print_stdout, print_stderr; warn pedantic, nursery; deny correctness; allow module_name_repetitions, must_use_candidate. [workspace.lints.rust] deny unsafe_code, missing_docs, unused_results, unused_imports, dead_code in Cargo.toml
- [X] T015 [P] Set lints.workspace = true in every crate Cargo.toml and add #![warn(clippy::all, clippy::pedantic)] #![deny(unsafe_code)] to all lib.rs files
- [X] T016 [P] Create stub lib.rs with //! module docs for all 10 crates (empty modules, compiles clean) in crates/*/src/lib.rs
- [X] T017 [P] Create CI workflow: fmt check, clippy --workspace --all-targets -- -D warnings, cargo test, cargo doc --no-deps in .github/workflows/ci.yml
- [X] T018 [P] Create coverage workflow: cargo-llvm-cov with nextest, upload report in .github/workflows/coverage.yml
- [X] T019 Verify workspace builds with `cargo check --workspace` and `cargo clippy --workspace -- -D warnings` passes clean

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core types, error handling, credentials, and security infrastructure that ALL user stories depend on

**CRITICAL**: No user story work can begin until this phase is complete

- [X] T020 Define BoxFuture and BoxStream type aliases in crates/synwire-core/src/lib.rs
- [X] T021 [P] Implement SynwireError top-level enum (#[non_exhaustive], thiserror) with Model, Prompt, Parse, Embedding, VectorStore, Tool, Serialization, Graph, Io, Other variants in crates/synwire-core/src/error/mod.rs
- [X] T022 [P] Implement ModelError enum (#[non_exhaustive]) with RateLimit, AuthenticationFailed, InvalidRequest, ContentFiltered, Timeout, Connection, Other variants in crates/synwire-core/src/error/model.rs
- [X] T023 [P] Implement ToolError, ParseError, EmbeddingError, VectorStoreError, SerializationError enums (#[non_exhaustive]) in crates/synwire-core/src/error/tool.rs, parse.rs, embedding.rs, vectorstore.rs
- [X] T024 [P] Implement SynwireErrorKind discriminant enum for retry/fallback matching in crates/synwire-core/src/error/kind.rs
- [X] T025 [P] Implement Message enum (#[non_exhaustive]: Human, AI, System, Tool, Chat) with all fields per data-model.md in crates/synwire-core/src/messages/types.rs
- [X] T026 [P] Implement MessageContent (Text, Blocks), ContentBlock enum (#[non_exhaustive]: Text, Image, Audio, Video, File, Reasoning, Thinking) in crates/synwire-core/src/messages/types.rs
- [X] T027 [P] Implement ToolCall, InvalidToolCall, ToolStatus, UsageMetadata, InputTokenDetails, OutputTokenDetails structs with serde derives in crates/synwire-core/src/messages/types.rs
- [X] T028 [P] Implement Document struct (id, page_content, metadata) with serde derives in crates/synwire-core/src/documents/types.rs
- [X] T029 [P] Implement ChatResult, CostEstimate, LLMResult, Generation structs in crates/synwire-core/src/language_models/types.rs
- [X] T030 [P] Implement ChatChunk and ToolCallChunk structs with merge() method in crates/synwire-core/src/language_models/types.rs
- [X] T031 [P] Implement PromptValue enum (String, Messages) with to_string and to_messages in crates/synwire-core/src/prompts/types.rs
- [X] T032 [P] Implement RunnableConfig struct (callbacks, tags, metadata, max_concurrency, run_name, run_id, configurable): Clone + Send + Sync in crates/synwire-core/src/runnables/config.rs
- [X] T033 [P] Implement ToolSchema struct (name, description, parameters) in crates/synwire-core/src/tools/types.rs
- [X] T034 [P] Implement ToolOutput (content, artifact), ToolResult enum (#[non_exhaustive]: Success, Error, Retry), ToolContentType in crates/synwire-core/src/tools/types.rs
- [X] T035 [P] Implement SecretValue backed by secrecy crate (Debug as "***", Display as "***", Serialize as null, expose(), Clone + Send + Sync) in crates/synwire-core/src/credentials/secret.rs
- [X] T036 [P] Implement CredentialProvider trait, EnvCredentialProvider, StaticCredentialProvider in crates/synwire-core/src/credentials/traits.rs, env.rs, static_creds.rs
- [X] T037 [P] Implement SsrfProtectedClient wrapping reqwest::Client with DNS pinning, private IP rejection, IPv4-mapped IPv6 blocking, configurable allow-list in crates/synwire-core/src/security/ssrf.rs
- [X] T038 [P] Implement HttpClientFactory trait and DefaultHttpClientFactory (returns SsrfProtectedClient) in crates/synwire-core/src/security/http_factory.rs
- [X] T039 [P] Implement HttpClientConfig struct (timeout, ssrf_protection, allow_list, proxy, user_agent) in crates/synwire-core/src/security/http_factory.rs
- [X] T040 [P] Implement RetryConfig struct (retry_on, max_attempts, wait_exponential_jitter, initial_interval, max_interval), RetryState struct in crates/synwire-core/src/runnables/retry.rs
- [X] T041 [P] Implement StreamEvent enum (#[non_exhaustive]: Standard, Custom) and EventData struct in crates/synwire-core/src/runnables/events.rs
- [X] T042 [P] Implement ContentCategory enum (#[non_exhaustive]: Primary, Secondary) in crates/synwire-core/src/runnables/events.rs
- [X] T043 Create module hierarchy (mod.rs files) for all core submodules: error, messages, documents, language_models, prompts, runnables, tools, callbacks, embeddings, vectorstores, retrievers, output_parsers, credentials, security, loaders, rerankers, agents in crates/synwire-core/src/
- [X] T044 Create prelude.rs re-exporting all public types and traits in crates/synwire-core/src/prelude.rs
- [X] T045 Wire up lib.rs with pub mod declarations and prelude re-export in crates/synwire-core/src/lib.rs
- [X] T046 Add unit tests for Message enum construction, serde round-trip, content() accessor, convenience constructors (Message::human, Message::system, Message::ai) in crates/synwire-core/src/messages/types.rs
- [X] T047 [P] Add unit tests for Document construction and serde round-trip in crates/synwire-core/src/documents/types.rs
- [X] T048 [P] Add unit tests for SynwireError Display, From conversions, SynwireErrorKind matching in crates/synwire-core/src/error/mod.rs
- [X] T049 [P] Add unit tests for SecretValue (Debug redaction, Display redaction, Serialize as null, expose, zeroisation) in crates/synwire-core/src/credentials/secret.rs
- [X] T050 [P] Add unit tests for SsrfProtectedClient (rejects private IPs, allows public, blocks IPv4-mapped IPv6) in crates/synwire-core/src/security/ssrf.rs
- [X] T051 [P] Add unit tests for ChatChunk merge (content concatenation, tool call merging by index) in crates/synwire-core/src/language_models/types.rs
- [X] T052 Verify `cargo test -p synwire-core` passes and `cargo clippy -p synwire-core` is clean

### Coverage Gap Additions (FR-202, FR-203, FR-204, FR-205)

- [X] T410 [P] Define ModelProfileRegistry trait (register, get, supports) and InMemoryModelProfileRegistry in crates/synwire-core/src/language_models/registry.rs
- [X] T411 [P] Add unit test: ModelProfileRegistry registers and retrieves profiles by model_id in crates/synwire-core/src/language_models/registry.rs
- [X] T412 [P] Implement multimodal ContentBlock handling: Image, Audio, File blocks in structured extraction contexts in crates/synwire-core/src/messages/types.rs
- [X] T413 [P] Implement reasoning/thinking content preservation (thinking: Option<Vec<String>> alongside structured output) in crates/synwire-core/src/language_models/types.rs

**Checkpoint**: Foundation ready — all core types compile, serialise, and pass unit tests

---

## Phase 3: User Story 1 — Invoke a Chat Model (Priority: P1) MVP

**Goal**: A developer can implement or use a chat model, invoke it with messages, and receive a typed ChatResult

**Independent Test**: Mock chat model implementing BaseChatModel; invoke with messages; verify ChatResult structure; swap providers compiles

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation (Constitution Principle IV)**

- [X] T053 [P] [US1] Unit test: FakeChatModel invoke returns ChatResult with expected AI message in crates/synwire-core/src/language_models/fake.rs
- [X] T054 [P] [US1] Unit test: FakeChatModel invoke with error returns Result::Err(ModelError) in crates/synwire-core/src/language_models/fake.rs
- [X] T055 [P] [US1] Unit test: swapping FakeChatModel for another mock compiles and produces correct output in crates/synwire-core/src/language_models/traits.rs
- [X] T056 [P] [US1] Unit test: BaseChatModel batch invokes multiple inputs concurrently in crates/synwire-core/src/language_models/traits.rs
- [X] T057 [P] [US1] Unit test: invoking with empty message list returns ModelError::InvalidRequest in crates/synwire-core/src/language_models/traits.rs
- [X] T058 [P] [US1] Unit test: BaseChatModel bind_tools returns model with tools configured in crates/synwire-core/src/language_models/traits.rs

### Implementation for User Story 1

- [X] T059 [US1] Define BaseChatModel trait (invoke, batch, stream, model_type, bind_tools, with_structured_output) in crates/synwire-core/src/language_models/traits.rs
- [X] T060 [US1] Define BaseLLM trait (invoke, batch, stream, model_type) in crates/synwire-core/src/language_models/traits.rs
- [X] T061 [US1] Implement FakeChatModel (configurable responses, call tracking, error injection) in crates/synwire-core/src/language_models/fake.rs
- [X] T062 [US1] Implement BaseChatOpenAI shared base type with builder, HTTP client, request/response serialisation in crates/synwire-llm-openai/src/base.rs
- [X] T063 [US1] Implement ChatOpenAI struct wrapping BaseChatOpenAI with builder pattern in crates/synwire-llm-openai/src/chat.rs
- [X] T064 [US1] Implement BaseChatModel for ChatOpenAI (invoke via POST /v1/chat/completions, batch via futures::join_all) in crates/synwire-llm-openai/src/chat.rs
- [X] T065 [US1] Define OpenAI-specific error types (OpenAIError) with From<OpenAIError> for ModelError in crates/synwire-llm-openai/src/error.rs
- [X] T066 [US1] Wire up synwire-llm-openai lib.rs with pub exports (ChatOpenAI, BaseChatOpenAI, OpenAIError) in crates/synwire-llm-openai/src/lib.rs
- [X] T067 [US1] Create simple_chat.rs example (invoke ChatOpenAI, print response) in examples/simple_chat.rs
- [X] T068 [US1] Add feature-gated integration test for ChatOpenAI invoke in tests/integration/openai_chat.rs

### Coverage Gap Additions (FR-174, FR-203)

- [X] T414 [US1] Implement credential refresh on 401/403 in ChatOpenAI (call refresh_credential, retry once) in crates/synwire-llm-openai/src/chat.rs
- [X] T415 [US1] Wire ModelProfileRegistry into ChatOpenAI construction (register profile at builder.build()) in crates/synwire-llm-openai/src/chat.rs

**Checkpoint**: US1 complete — mock and real chat model invocation works end-to-end

---

## Phase 4: User Story 2 — Compose Prompt Templates, Chains, and Parse Output (Priority: P2)

**Goal**: A developer can create prompt templates, format them, chain them with a model, compose runnables, and parse structured output

**Independent Test**: Create template with variables, format, verify output; chain template to mock model; use output parsers; compose with retry/fallbacks

### Tests for User Story 2

- [X] T069 [P] [US2] Unit test: PromptTemplate.format substitutes variables correctly in crates/synwire-core/src/prompts/template.rs
- [X] T070 [P] [US2] Unit test: PromptTemplate.format with missing variable returns descriptive error naming the variable in crates/synwire-core/src/prompts/template.rs
- [X] T071 [P] [US2] Unit test: ChatPromptTemplate.format_messages produces correct Message list in crates/synwire-core/src/prompts/chat.rs
- [X] T072 [P] [US2] Unit test: RunnableSequence chains template to mock model and returns result in crates/synwire-core/src/runnables/chain.rs
- [X] T073 [P] [US2] Unit test: pipe() composes two runnables sequentially in crates/synwire-core/src/runnables/chain.rs
- [X] T074 [P] [US2] Unit test: RunnableParallel executes named steps concurrently in crates/synwire-core/src/runnables/chain.rs
- [X] T075 [P] [US2] Unit test: RunnablePassthrough forwards input unchanged in crates/synwire-core/src/runnables/passthrough.rs
- [X] T076 [P] [US2] Unit test: RunnableLambda wraps closure and invokes correctly in crates/synwire-core/src/runnables/lambda.rs
- [X] T077 [P] [US2] Unit test: RunnableBranch routes to correct branch based on condition in crates/synwire-core/src/runnables/branch.rs
- [X] T078 [P] [US2] Unit test: RunnableRetry retries on configured error kinds with backoff in crates/synwire-core/src/runnables/retry.rs
- [X] T079 [P] [US2] Unit test: RunnableWithFallbacks falls back to secondary on primary failure in crates/synwire-core/src/runnables/fallbacks.rs
- [X] T080 [P] [US2] Unit test: StrOutputParser returns raw text unchanged in crates/synwire-core/src/output_parsers/string.rs
- [X] T081 [P] [US2] Unit test: JsonOutputParser deserialises valid JSON and errors on invalid in crates/synwire-core/src/output_parsers/json.rs
- [X] T082 [P] [US2] Unit test: StructuredOutputParser<T> deserialises to typed struct in crates/synwire-core/src/output_parsers/structured.rs
- [X] T083 [P] [US2] Unit test: ToolsOutputParser extracts Vec<ToolCall> from AI message in crates/synwire-core/src/output_parsers/tools.rs
- [X] T084 [P] [US2] Unit test: MessageFilter builder filters by type, name, and ID correctly in crates/synwire-core/src/messages/filter.rs
- [X] T085 [P] [US2] Unit test: trim_messages respects token budget and strategy (First, Last) in crates/synwire-core/src/messages/utils.rs
- [X] T086 [P] [US2] Unit test: merge_message_runs combines consecutive same-type messages in crates/synwire-core/src/messages/utils.rs

### Implementation for User Story 2

- [X] T087 [US2] Implement PromptTemplate with new(), format(), input_variables, TemplateFormat in crates/synwire-core/src/prompts/template.rs
- [X] T088 [US2] Implement ChatPromptTemplate with from_messages(), format_messages(), MessageTemplate enum in crates/synwire-core/src/prompts/chat.rs
- [X] T089 [US2] Define RunnableCore<I, O> trait (invoke, batch, stream) in crates/synwire-core/src/runnables/core.rs
- [X] T090 [US2] Define ObservableRunnable<I, O> extension trait (stream_events, transform, batch_as_completed) in crates/synwire-core/src/runnables/observable.rs
- [X] T091 [US2] Implement RunnableSequence and pipe() composition function in crates/synwire-core/src/runnables/chain.rs
- [X] T092 [US2] Implement RunnableParallel (concurrent named steps) in crates/synwire-core/src/runnables/chain.rs
- [X] T093 [US2] Implement RunnablePassthrough (forwards input unchanged) in crates/synwire-core/src/runnables/passthrough.rs
- [X] T094 [US2] Implement RunnableLambda (closure wrapper with new/with_name) in crates/synwire-core/src/runnables/lambda.rs
- [X] T095 [US2] Implement RunnableBranch (condition/runnable pairs + default) in crates/synwire-core/src/runnables/branch.rs
- [X] T096 [US2] Implement RunnableRetry with exponential backoff (uses backoff crate) in crates/synwire-core/src/runnables/retry.rs
- [X] T097 [US2] Implement RunnableWithFallbacks and with_fallbacks() composition function in crates/synwire-core/src/runnables/fallbacks.rs
- [X] T098 [US2] Implement RunnableTool and as_tool() composition function in crates/synwire-core/src/runnables/as_tool.rs
- [X] T099 [US2] Implement OutputParser<T> trait (parse, parse_result, parse_with_prompt, get_format_instructions) in crates/synwire-core/src/output_parsers/traits.rs
- [X] T100 [US2] Implement StrOutputParser in crates/synwire-core/src/output_parsers/string.rs
- [X] T101 [US2] Implement JsonOutputParser in crates/synwire-core/src/output_parsers/json.rs
- [X] T102 [US2] Implement StructuredOutputParser<T: DeserializeOwned> in crates/synwire-core/src/output_parsers/structured.rs
- [X] T103 [US2] Implement ToolsOutputParser in crates/synwire-core/src/output_parsers/tools.rs
- [X] T104 [US2] Implement MessageLike trait with blanket impls (Message, &str, String, (MessageRole, &str)) in crates/synwire-core/src/messages/traits.rs
- [X] T105 [US2] Implement MessageFilter with builder pattern (include/exclude types, names, ids) in crates/synwire-core/src/messages/filter.rs
- [X] T106 [US2] Implement trim_messages function with TrimStrategy enum (First, Last) in crates/synwire-core/src/messages/utils.rs
- [X] T107 [US2] Implement merge_message_runs function in crates/synwire-core/src/messages/utils.rs
- [X] T108 [US2] Implement dispatch_custom_event standalone function in crates/synwire-core/src/runnables/events.rs
- [X] T109 [US2] Create prompt_chain.rs example (template -> model -> parser -> print) in examples/prompt_chain.rs

### Coverage Gap Additions (FR-116, FR-206, FR-207, FR-208)

- [X] T416 [US2] Evaluate &I or Cow<I> for RunnableCore input parameters; document decision in crates/synwire-core/src/runnables/core.rs
- [X] T417 [P] [US2] Implement OutputMode<T> enum (Native, Tool, Prompt, Custom) with fallback chain in crates/synwire-core/src/output_parsers/output_mode.rs
- [X] T418 [US2] Implement OutputMode provider compatibility validation at construction time in crates/synwire-core/src/output_parsers/output_mode.rs
- [X] T419 [US2] Implement structured output retry with validation_error_formatter (include validation error in next LLM prompt) in crates/synwire-core/src/output_parsers/structured.rs
- [X] T420 [P] [US2] Unit test: OutputMode fallback chain (Native->Tool->Prompt) triggers on parse failure in crates/synwire-core/src/output_parsers/output_mode.rs
- [X] T421 [P] [US2] Unit test: OutputMode rejects incompatible provider/mode at construction in crates/synwire-core/src/output_parsers/output_mode.rs

**Checkpoint**: US2 complete — templates format, chains compose, runnables work, parsers extract output

---

## Phase 5: User Story 3 — Stream Responses (Priority: P3)

**Goal**: A developer can stream model responses as async chunks

**Independent Test**: Mock model yields incremental chunks; verify order, completeness, error handling, resource cleanup on drop

### Tests for User Story 3

- [X] T110 [P] [US3] Unit test: FakeChatModel stream yields chunks in order in crates/synwire-core/src/language_models/fake.rs
- [X] T111 [P] [US3] Unit test: concatenated stream chunks equal invoke result in crates/synwire-core/src/language_models/fake.rs
- [X] T112 [P] [US3] Unit test: stream with mid-stream error yields error item and terminates in crates/synwire-core/src/language_models/fake.rs
- [X] T113 [P] [US3] Unit test: dropping stream mid-way does not leak resources in crates/synwire-core/src/language_models/fake.rs
- [X] T114 [P] [US3] Unit test: RunnableCore.stream default wraps invoke as single-item stream in crates/synwire-core/src/runnables/core.rs

### Implementation for User Story 3

- [X] T115 [US3] Extend FakeChatModel with configurable streaming (chunk_size, delay) in crates/synwire-core/src/language_models/fake.rs
- [X] T116 [US3] Implement BaseChatModel.stream for ChatOpenAI using SSE parsing via eventsource-stream in crates/synwire-llm-openai/src/chat.rs
- [X] T117 [US3] Add SSE stream parsing for OpenAI response format (data: [DONE], delta extraction) in crates/synwire-llm-openai/src/chat.rs
- [X] T118 [US3] Implement default RunnableCore.stream (wraps invoke output as single-item stream) in crates/synwire-core/src/runnables/core.rs
- [X] T119 [US3] Create streaming.rs example (stream ChatOpenAI, print tokens as they arrive) in examples/streaming.rs
- [X] T120 [US3] Add feature-gated integration test for ChatOpenAI streaming in tests/integration/openai_chat.rs

### Coverage Gap Additions (FR-046, FR-192)

- [X] T422 [US3] Wire ContentCategory (Primary/Secondary) into stream events for demultiplexing concurrent node streams in crates/synwire-core/src/runnables/events.rs

**Checkpoint**: US3 complete — streaming works with mock and real providers

---

## Phase 6: User Story 4 — Embed Text and Query a Vector Store (Priority: P4)

**Goal**: A developer can embed documents, store them in a vector store, and perform similarity search

**Independent Test**: FakeEmbeddings return deterministic vectors; InMemoryVectorStore returns correct ranked results; MetadataFilter works

### Tests for User Story 4

- [X] T121 [P] [US4] Unit test: FakeEmbeddings.embed_documents returns one vector per text with consistent dimensions in crates/synwire-core/src/embeddings/fake.rs
- [X] T122 [P] [US4] Unit test: FakeEmbeddings.embed_query returns single vector in crates/synwire-core/src/embeddings/fake.rs
- [X] T123 [P] [US4] Unit test: InMemoryVectorStore.add_documents stores and returns IDs in crates/synwire-core/src/vectorstores/in_memory.rs
- [X] T124 [P] [US4] Unit test: InMemoryVectorStore.similarity_search returns ranked results in crates/synwire-core/src/vectorstores/in_memory.rs
- [X] T125 [P] [US4] Unit test: InMemoryVectorStore.similarity_search on empty store returns empty vec in crates/synwire-core/src/vectorstores/in_memory.rs
- [X] T126 [P] [US4] Unit test: InMemoryVectorStore rejects mismatched embedding dimensions at insertion in crates/synwire-core/src/vectorstores/in_memory.rs
- [X] T127 [P] [US4] Unit test: MetadataFilter (Eq, Ne, In, Gt, Lt, And, Or) correctly filters documents in crates/synwire-core/src/vectorstores/filter.rs
- [X] T128 [P] [US4] Unit test: VectorStoreRetriever wraps VectorStore and returns documents via Retriever trait in crates/synwire-core/src/retrievers/traits.rs
- [X] T129 [P] [US4] Unit test: MMR search returns diverse results (not just top-k similar) in crates/synwire-core/src/vectorstores/mmr.rs

### Implementation for User Story 4

- [X] T130 [US4] Define Embeddings trait (embed_documents, embed_query) in crates/synwire-core/src/embeddings/traits.rs
- [X] T131 [US4] Implement FakeEmbeddings (deterministic vectors from hash) in crates/synwire-core/src/embeddings/fake.rs
- [X] T132 [US4] Define VectorStore trait (add_documents, add_texts, similarity_search, similarity_search_with_score, similarity_search_by_vector, get_by_ids, delete, max_marginal_relevance_search, as_retriever) in crates/synwire-core/src/vectorstores/traits.rs
- [X] T133 [US4] Implement MetadataFilter enum (#[non_exhaustive]: Eq, Ne, Gt, Lt, Gte, Lte, In, And, Or) in crates/synwire-core/src/vectorstores/filter.rs
- [X] T134 [US4] Implement MMR algorithm utility (cosine similarity + diversity scoring) in crates/synwire-core/src/vectorstores/mmr.rs
- [X] T135 [US4] Implement InMemoryVectorStore (brute-force cosine similarity, MMR, MetadataFilter support) in crates/synwire-core/src/vectorstores/in_memory.rs
- [X] T136 [US4] Define Retriever trait (get_relevant_documents) in crates/synwire-core/src/retrievers/traits.rs
- [X] T137 [US4] Implement VectorStoreRetriever (wraps VectorStore with k, SearchType config) in crates/synwire-core/src/retrievers/traits.rs
- [X] T138 [US4] Implement RetrieverRunnable adapter (blanket RunnableCore<String, Vec<Document>> for Retriever) in crates/synwire-core/src/retrievers/runnable.rs
- [X] T139 [US4] Define DocumentLoader trait (load, load_lazy) in crates/synwire-core/src/loaders/traits.rs
- [X] T140 [US4] Define Reranker trait (rerank) in crates/synwire-core/src/rerankers/traits.rs
- [X] T141 [US4] Implement OpenAIEmbeddings struct with builder pattern in crates/synwire-llm-openai/src/embeddings.rs
- [X] T142 [US4] Implement Embeddings for OpenAIEmbeddings (POST to /v1/embeddings) in crates/synwire-llm-openai/src/embeddings.rs
- [X] T143 [US4] Create rag.rs example (embed docs -> store -> search -> model answer) in examples/rag.rs
- [X] T144 [US4] Add feature-gated integration test for OpenAIEmbeddings in tests/integration/openai_embeddings.rs

### Coverage Gap Additions (FR-190)

- [X] T423 [US4] Implement retrieval_mode (Dense, Sparse, Hybrid { alpha }) on Retriever trait with UnsupportedRetrievalMode error in crates/synwire-core/src/retrievers/traits.rs
- [X] T424 [P] [US4] Unit test: Retriever with unsupported retrieval_mode returns UnsupportedRetrievalMode error in crates/synwire-core/src/retrievers/traits.rs

**Checkpoint**: US4 complete — embeddings, vector store, retrieval, and MMR work independently

---

## Phase 7: User Story 5 — Define and Use Tools (Priority: P5)

**Goal**: A developer can define tools with typed schemas, invoke them from models, and observe execution via callbacks

**Independent Test**: Define mock tool with schema; invoke with valid/invalid input; verify schema serialisation; callback hooks fire

### Tests for User Story 5

- [X] T145 [P] [US5] Unit test: StructuredTool.invoke with valid JSON input returns ToolOutput in crates/synwire-core/src/tools/structured.rs
- [X] T146 [P] [US5] Unit test: StructuredTool.schema returns serialisable ToolSchema in crates/synwire-core/src/tools/structured.rs
- [X] T147 [P] [US5] Unit test: StructuredTool.invoke with invalid input returns ToolError in crates/synwire-core/src/tools/structured.rs
- [X] T148 [P] [US5] Unit test: tool name validates against ^[a-zA-Z0-9_-]{1,64}$ in crates/synwire-core/src/tools/traits.rs
- [X] T149 [P] [US5] Unit test: CallbackHandler on_tool_start and on_tool_end fire during tool invocation in crates/synwire-core/src/callbacks/traits.rs
- [X] T150 [P] [US5] Unit test: CallbackHandler on_llm_start and on_llm_end fire during model invocation in crates/synwire-core/src/callbacks/traits.rs
- [X] T151 [P] [US5] Unit test: CallbackHandler ignore_tool filter prevents tool hooks from firing in crates/synwire-core/src/callbacks/traits.rs
- [X] T152 [P] [US5] Unit test: ToolCall/ToolMessage handling in ChatOpenAI response parsing in crates/synwire-llm-openai/src/chat.rs

### Implementation for User Story 5

- [X] T153 [US5] Define Tool trait (name, description, schema, invoke) in crates/synwire-core/src/tools/traits.rs
- [X] T154 [US5] Add tool name validation (^[a-zA-Z0-9_-]{1,64}$) at construction time in crates/synwire-core/src/tools/traits.rs
- [X] T155 [US5] Implement StructuredTool and StructuredToolBuilder (builder pattern) in crates/synwire-core/src/tools/structured.rs
- [X] T156 [US5] Define CallbackHandler trait with all hook methods (default no-op impls): LLM, chain, tool, retriever, embedding, agent, graph, retry, custom event hooks; ignore_* filters in crates/synwire-core/src/callbacks/traits.rs
- [X] T157 [US5] Add path traversal protection (reject .., null bytes, absolute paths) for tool arguments in scoped mode in crates/synwire-core/src/security/ssrf.rs
- [X] T158 [US5] Add ToolCall and ToolMessage handling to ChatOpenAI response parsing in crates/synwire-llm-openai/src/chat.rs
- [X] T159 [US5] Add on_retry hook and wire into RunnableRetry invoke loop in crates/synwire-core/src/callbacks/traits.rs and crates/synwire-core/src/runnables/retry.rs
- [X] T160 [US5] Update prelude.rs with new public types (Tool, StructuredTool, CallbackHandler, Embeddings, VectorStore, Retriever, RunnableCore, ObservableRunnable, OutputParser, all parsers, message utils, FakeChatModel, FakeEmbeddings) in crates/synwire-core/src/prelude.rs

**Checkpoint**: US5 complete — tools can be defined, invoked, schemas serialised, callbacks fire

---

## Phase 8: User Story 6 — Build and Run a State Graph (Priority: P6)

**Goal**: A developer can define a typed StateGraph with nodes and edges, compile it, and invoke it with the Pregel execution engine

**Independent Test**: Define 3-node StateGraph with typed state, compile, invoke, verify state transitions and channel updates

### Tests for User Story 6

- [X] T161 [P] [US6] Unit test: LastValue channel stores most recent value and rejects multiple updates in crates/synwire-orchestrator/src/channels/last_value.rs
- [X] T162 [P] [US6] Unit test: Topic channel accumulates values in order in crates/synwire-orchestrator/src/channels/topic.rs
- [X] T163 [P] [US6] Unit test: BinaryOperatorAggregate applies reducer cumulatively in crates/synwire-orchestrator/src/channels/binary_operator.rs
- [X] T164 [P] [US6] Unit test: AnyValue accepts any single value in crates/synwire-orchestrator/src/channels/any_value.rs
- [X] T165 [P] [US6] Unit test: EphemeralValue clears after superstep read in crates/synwire-orchestrator/src/channels/ephemeral.rs
- [X] T166 [P] [US6] Unit test: NamedBarrierValue fires when all triggers received in crates/synwire-orchestrator/src/channels/barrier.rs
- [X] T167 [P] [US6] Unit test: StateGraph compiles with valid topology (no orphans, valid edges) in crates/synwire-orchestrator/src/graph/state.rs
- [X] T168 [P] [US6] Unit test: StateGraph rejects invalid topology (orphan nodes, missing edges) at compile time in crates/synwire-orchestrator/src/graph/state.rs
- [X] T169 [P] [US6] Unit test: CompiledGraph.invoke runs 3-node linear graph and produces correct final state in crates/synwire-orchestrator/src/graph/compiled.rs
- [X] T170 [P] [US6] Unit test: conditional edges route to correct nodes based on state in crates/synwire-orchestrator/src/graph/compiled.rs
- [X] T171 [P] [US6] Unit test: Send fan-out creates one task per Send, all executed exactly once in crates/synwire-orchestrator/src/types/send.rs
- [X] T172 [P] [US6] Unit test: graph cycle with no exit hits recursion limit and returns GraphRecursionError in crates/synwire-orchestrator/src/graph/compiled.rs
- [X] T173 [P] [US6] Unit test: multiple nodes write to same LastValue channel in one superstep returns InvalidUpdateError in crates/synwire-orchestrator/src/graph/compiled.rs
- [X] T174 [P] [US6] Unit test: interrupt() pauses graph and returns StateSnapshot with interrupts in crates/synwire-orchestrator/src/types/interrupt.rs
- [X] T175 [P] [US6] Unit test: Command::resume continues execution from interrupt point in crates/synwire-orchestrator/src/types/command.rs
- [X] T176 [P] [US6] Unit test: CompiledGraph implements RunnableCore<S, S> in crates/synwire-orchestrator/src/graph/compiled.rs
- [X] T177 [P] [US6] Unit test: to_mermaid() produces valid Mermaid syntax with nodes, edges, conditionals in crates/synwire-orchestrator/src/graph/compiled.rs
- [X] T178 [P] [US6] Unit test: NodeErrorStrategy::FailBranch skips downstream only on failed branch in crates/synwire-orchestrator/src/types/node_state.rs
- [X] T179 [P] [US6] Unit test: Overwrite bypasses channel reducer in crates/synwire-orchestrator/src/types/overwrite.rs

### Implementation for User Story 6

- [X] T180 [US6] Implement SynwireGraphError enum (#[non_exhaustive]) with RecursionLimit, InvalidUpdate, Interrupt, EmptyInput, TaskNotFound, EmptyChannel, CompileError, Checkpoint, Store in crates/synwire-orchestrator/src/error.rs
- [X] T181 [US6] Define START and END constants in crates/synwire-orchestrator/src/constants.rs
- [X] T182 [US6] Define BaseChannel trait (key, from_checkpoint, update, get, is_available, checkpoint, consume, finish) in crates/synwire-orchestrator/src/channels/traits.rs
- [X] T183 [P] [US6] Implement LastValue<V> channel in crates/synwire-orchestrator/src/channels/last_value.rs
- [X] T184 [P] [US6] Implement Topic<V> channel in crates/synwire-orchestrator/src/channels/topic.rs
- [X] T185 [P] [US6] Implement BinaryOperatorAggregate<V> channel with Overwrite support in crates/synwire-orchestrator/src/channels/binary_operator.rs
- [X] T186 [P] [US6] Implement AnyValue<V> channel in crates/synwire-orchestrator/src/channels/any_value.rs
- [X] T187 [P] [US6] Implement EphemeralValue<V> channel in crates/synwire-orchestrator/src/channels/ephemeral.rs
- [X] T188 [P] [US6] Implement NamedBarrierValue<V> channel in crates/synwire-orchestrator/src/channels/barrier.rs
- [X] T189 [US6] Define State trait (channels, from_channels) in crates/synwire-orchestrator/src/graph/state.rs
- [X] T190 [US6] Implement StateGraph<S: State> builder (new, add_node, add_edge, add_conditional_edges, set_entry_point, set_finish_point, compile) in crates/synwire-orchestrator/src/graph/state.rs
- [X] T191 [US6] Implement GraphCompileConfig with builder pattern in crates/synwire-orchestrator/src/graph/state.rs
- [X] T192 [US6] Implement CompiledGraph`<S>` with invoke, stream, get_state, get_state_history, update_state, to_mermaid in crates/synwire-orchestrator/src/graph/compiled.rs
- [X] T193 [US6] Implement RunnableCore<S, S> for CompiledGraph`<S>` in crates/synwire-orchestrator/src/graph/compiled.rs
- [X] T194 [US6] Implement Pregel execution engine (superstep loop, node scheduling, channel updates, recursion limit) in crates/synwire-orchestrator/src/pregel/engine.rs
- [X] T195 [US6] Implement PregelTask type in crates/synwire-orchestrator/src/pregel/types.rs
- [X] T196 [US6] Implement Send (node, arg) for dynamic fan-out in crates/synwire-orchestrator/src/types/send.rs
- [X] T197 [US6] Implement Command (graph, update, resume, goto) for control flow in crates/synwire-orchestrator/src/types/command.rs
- [X] T198 [US6] Implement Overwrite<V> (bypass channel reducer) in crates/synwire-orchestrator/src/types/overwrite.rs
- [X] T199 [US6] Implement interrupt() function and Interrupt type in crates/synwire-orchestrator/src/types/interrupt.rs
- [X] T200 [US6] Implement StreamMode enum (#[non_exhaustive]: Values, Updates, Debug, Messages, Custom, Tasks, Checkpoints) with lossless/lossy annotations in crates/synwire-orchestrator/src/types/stream_mode.rs
- [X] T201 [US6] Implement StateSnapshot`<S>` (values, next, config, metadata, created_at, parent_config, tasks, interrupts) in crates/synwire-orchestrator/src/types/snapshot.rs
- [X] T202 [US6] Implement TypedValue enum (#[non_exhaustive]: String, Integer, Float, Boolean, Secret, List, Map, Json, None) in crates/synwire-orchestrator/src/types/typed_value.rs
- [X] T203 [US6] Implement NodeState enum (#[non_exhaustive]: Pending, Running, Succeeded, Failed, Skipped, Paused) and NodeErrorStrategy enum (FailWorkflow, FailBranch, Continue) in crates/synwire-orchestrator/src/types/node_state.rs
- [X] T204 [US6] Implement RetryPolicy (per-node: initial_interval, backoff_factor, max_interval, max_attempts, jitter, retry predicate, idempotent) in crates/synwire-orchestrator/src/config/retry_policy.rs
- [X] T205 [US6] Implement CachePolicy (key_func, ttl) in crates/synwire-orchestrator/src/config/cache_policy.rs
- [X] T206 [US6] Implement MessagesState convenience type with messages field using add_messages reducer in crates/synwire-orchestrator/src/graph/state.rs
- [X] T207 [US6] Implement add_messages reducer and RemoveMessage type in crates/synwire-orchestrator/src/messages/reducers.rs
- [X] T208 [US6] Implement NodeRegistry (register, register_versioned, get) in crates/synwire-orchestrator/src/registry/node_registry.rs
- [X] T209 [US6] Implement GraphExecutionMetrics, NodeMetrics, QuotaEnforcer trait, NoOpQuotaEnforcer in crates/synwire-orchestrator/src/metrics/execution.rs, node.rs, quota.rs
- [X] T210 [US6] Implement managed values: IsLastStep (bool), RemainingSteps (usize) in crates/synwire-orchestrator/src/managed/values.rs
- [X] T211 [US6] Implement TaskFunction and entrypoint/EntrypointFinal functional API in crates/synwire-orchestrator/src/func/task.rs and func/entrypoint.rs
- [X] T212 [US6] Implement runtime context: get_config(), get_store(), get_stream_writer() in crates/synwire-orchestrator/src/config/runtime.rs
- [X] T213 [US6] Implement CompiledGraph as_tool(name, description) -> Box<dyn Tool> for graph-in-graph in crates/synwire-orchestrator/src/graph/compiled.rs
- [X] T214 [US6] Wire up synwire-orchestrator lib.rs with all module declarations and re-exports in crates/synwire-orchestrator/src/lib.rs
- [X] T215 [US6] Create graph_basic.rs example (3-node StateGraph, compile, invoke) in examples/graph_basic.rs

### Coverage Gap Additions (FR-169, FR-170, FR-171, FR-200, FR-201)

- [X] T425 [P] [US6] Unit test: iteration/loop nodes create nested variable scopes; inner scope shadows outer; discarded after iteration in crates/synwire-orchestrator/src/types/typed_value.rs
- [X] T426 [P] [US6] Unit test: structured path references (node_id.key) resolve node output correctly in crates/synwire-orchestrator/src/pregel/engine.rs
- [X] T427 [P] [US6] Unit test: system variables (sys.run_id, sys.thread_id, sys.created_at, sys.step_count) are injected and accessible in crates/synwire-orchestrator/src/config/runtime.rs
- [X] T428 [US6] Implement nested variable scopes for iteration/loop nodes (inner scope shadows outer, discard after iteration) in crates/synwire-orchestrator/src/types/typed_value.rs
- [X] T429 [US6] Implement structured path references (node_id.key) for cross-node output access with descriptive errors on unresolved in crates/synwire-orchestrator/src/pregel/engine.rs
- [X] T430 [US6] Implement system variables injection (sys.run_id, sys.thread_id, sys.created_at, sys.step_count) in crates/synwire-orchestrator/src/config/runtime.rs
- [X] T431 [US6] Implement FailedAttempt type (attempt_number, error, response, token_usage, duration) and wire into GraphExecutionMetrics (total_attempts, successful_attempt) in crates/synwire-orchestrator/src/metrics/execution.rs

**Checkpoint**: US6 complete — StateGraph builds, compiles, executes via Pregel, interrupts and resumes work

---

## Phase 9: User Story 7 — Checkpoint and Resume Graph Execution (Priority: P7)

**Goal**: A developer can checkpoint graph state, persist it, and resume from a saved checkpoint

**Independent Test**: Run graph with in-memory checkpointer, interrupt, resume, verify state continuity; SQLite round-trip

### Tests for User Story 7

- [X] T216 [P] [US7] Unit test: InMemoryCheckpointSaver put and get_tuple round-trip in crates/synwire-checkpoint/src/memory.rs
- [X] T217 [P] [US7] Unit test: InMemoryCheckpointSaver list returns checkpoints in order in crates/synwire-checkpoint/src/memory.rs
- [X] T218 [P] [US7] Unit test: JsonPlusSerializer dumps_typed and loads_typed round-trip in crates/synwire-checkpoint/src/serde/json_plus.rs
- [X] T219 [P] [US7] Unit test: SecretValue serialises as sentinel reference in checkpoint, not plaintext in crates/synwire-checkpoint/src/serde/json_plus.rs
- [X] T220 [P] [US7] Unit test: InMemoryStore put, get, search, delete, list_namespaces in crates/synwire-checkpoint/src/store/in_memory.rs
- [X] T221 [P] [US7] Unit test: SqliteSaver put, get_tuple, list round-trip in crates/synwire-checkpoint-sqlite/src/saver.rs
- [X] T222 [P] [US7] Unit test: SqliteSaver file permissions are 0600 in crates/synwire-checkpoint-sqlite/src/saver.rs
- [X] T223 [P] [US7] Unit test: Checkpoint includes format_version "1.0" in crates/synwire-checkpoint/src/types.rs
- [X] T224 [P] [US7] Unit test: max_checkpoint_size enforcement returns CheckpointError::StateTooLarge in crates/synwire-checkpoint/src/base.rs
- [X] T225 [P] [US7] Unit test: graph with checkpointer saves state after each superstep and resumes correctly in crates/synwire-orchestrator/src/graph/compiled.rs

### Implementation for User Story 7

- [X] T226 [US7] Define BaseCheckpointSaver trait (get_tuple, list, put, put_writes, get_next_version) in crates/synwire-checkpoint/src/base.rs
- [X] T227 [US7] Implement Checkpoint, CheckpointMetadata, CheckpointSource, CheckpointTuple, PendingWrite, ChannelVersion types in crates/synwire-checkpoint/src/types.rs
- [X] T228 [US7] Define SerializerProtocol trait (dumps_typed, loads_typed) in crates/synwire-checkpoint/src/serde/protocol.rs
- [X] T229 [US7] Implement JsonPlusSerializer (default serialiser with SecretValue sentinel handling) in crates/synwire-checkpoint/src/serde/json_plus.rs
- [X] T230 [US7] Define BaseStore trait (get, search, put, list_namespaces, batch) with Item, SearchItem, operation types in crates/synwire-checkpoint/src/store/base.rs and types.rs
- [X] T231 [US7] Implement InMemoryStore in crates/synwire-checkpoint/src/store/in_memory.rs
- [X] T232 [US7] Define BaseCache trait (get, set, clear) with TTL support in crates/synwire-checkpoint/src/cache/base.rs
- [X] T233 [US7] Implement TTLConfig, IndexConfig types in crates/synwire-checkpoint/src/store/types.rs
- [X] T234 [US7] Implement InMemoryCheckpointSaver in crates/synwire-checkpoint/src/memory.rs
- [X] T235 [US7] Define CheckpointMigration trait (opt-in version migration) in crates/synwire-checkpoint/src/types.rs
- [X] T236 [US7] Wire up synwire-checkpoint lib.rs with all module declarations and re-exports in crates/synwire-checkpoint/src/lib.rs
- [X] T237 [US7] Implement SqliteSaver with DDL schema, mode 0600 file permissions, max_checkpoint_size in crates/synwire-checkpoint-sqlite/src/saver.rs and schema.rs
- [X] T238 [US7] Wire up synwire-checkpoint-sqlite lib.rs in crates/synwire-checkpoint-sqlite/src/lib.rs
- [X] T239 [US7] Implement checkpoint conformance test suite (round-trip, ordering, concurrent access) in crates/synwire-checkpoint-conformance/src/lib.rs
- [X] T240 [US7] Wire checkpointing into CompiledGraph (save after superstep, load on resume, partial recovery) in crates/synwire-orchestrator/src/graph/compiled.rs
- [X] T241 [US7] Create checkpoint_resume.rs example (graph -> interrupt -> save -> resume) in examples/checkpoint_resume.rs

**Checkpoint**: US7 complete — checkpoint round-trip works with in-memory and SQLite backends

---

## Phase 10: User Story 8 — Use Prebuilt Agents and Nodes (Priority: P8)

**Goal**: A developer can create a ReAct agent with create_react_agent in under 10 lines, and use prebuilt control-flow nodes

**Independent Test**: create_react_agent with FakeChatModel and mock tools; verify tool calling loop; prebuilt nodes produce correct results

### Tests for User Story 8

- [X] T242 [P] [US8] Unit test: create_react_agent produces working agent that calls tools and returns result in crates/synwire-orchestrator/src/prebuilt/react_agent.rs
- [X] T243 [P] [US8] Unit test: ToolNode executes tools in parallel with error handling in crates/synwire-orchestrator/src/prebuilt/tool_node.rs
- [X] T244 [P] [US8] Unit test: tools_condition routes to ToolNode when tool calls present, END otherwise in crates/synwire-orchestrator/src/prebuilt/react_agent.rs
- [X] T245 [P] [US8] Unit test: IfElseNode branches correctly based on condition in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [X] T246 [P] [US8] Unit test: LoopNode repeats until predicate is true and respects max_iterations in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [X] T247 [P] [US8] Unit test: HttpRequestNode makes outbound HTTP via SSRF-protected client in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [X] T248 [P] [US8] Unit test: ToolNode max_result_size truncates oversized results with flag in crates/synwire-orchestrator/src/prebuilt/tool_node.rs
- [X] T249 [P] [US8] Unit test: AgentAction, AgentFinish, AgentStep, AgentDecision types construct and serialise in crates/synwire-core/src/agents/types.rs

### Implementation for User Story 8

- [X] T250 [US8] Implement AgentAction, AgentFinish, AgentStep, AgentDecision, AgentInput types in crates/synwire-core/src/agents/types.rs
- [X] T251 [US8] Implement create_react_agent() factory (StateGraph-based ReAct from LLM + tools) in crates/synwire-orchestrator/src/prebuilt/react_agent.rs
- [X] T252 [US8] Implement ToolNode (parallel tool execution, error handling, max_result_size, state/store injection) in crates/synwire-orchestrator/src/prebuilt/tool_node.rs
- [X] T253 [US8] Implement tools_condition routing function in crates/synwire-orchestrator/src/prebuilt/react_agent.rs
- [X] T254 [US8] Implement AgentState as standard prebuilt agent state in crates/synwire-orchestrator/src/prebuilt/react_agent.rs
- [X] T255 [US8] Implement IfElseNode (conditional branching) in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [X] T256 [US8] Implement LoopNode (repeating with termination predicate and max_iterations) in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [X] T257 [US8] Implement HttpRequestNode (outbound HTTP via SSRF-protected client) in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [X] T258 [US8] Create react_agent.rs example (create_react_agent in <10 lines with mock tools) in examples/react_agent.rs

### Coverage Gap Additions (FR-151, FR-160, FR-162)

- [X] T432 [P] [US8] Unit test: ValidationNode validates tool inputs against schema before execution in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [X] T433 [P] [US8] Unit test: TemplateTransformNode applies template to state in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [X] T434 [P] [US8] Unit test: ListOperatorNode sorts, filters, slices, deduplicates in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [X] T435 [P] [US8] Unit test: QuestionClassifierNode routes based on LLM classification in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [X] T436 [US8] Implement ValidationNode for pre-execution tool input validation against JSON Schema in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [X] T437 [US8] Implement TemplateTransformNode (apply template transform to state fields) in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [X] T438 [US8] Implement ListOperatorNode (sort, filter, slice, deduplicate) in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [X] T439 [US8] Implement VariableAggregatorNode (aggregate variables from multiple upstream nodes) in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [X] T440 [US8] Implement QuestionClassifierNode (LLM-based classification routing to downstream nodes) in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [X] T441 [P] [US8] Unit test: IterationNode iterates over collection with per-item execution in crates/synwire-orchestrator/src/prebuilt/nodes.rs
- [X] T442 [US8] Implement IterationNode (iterate over collection with per-item execution) in crates/synwire-orchestrator/src/prebuilt/nodes.rs

**Checkpoint**: US8 complete — create_react_agent works, prebuilt nodes produce correct results

---

## Phase 11: Proc Macros — synwire-derive

**Purpose**: #[tool] attribute macro and #[derive(State)] derive macro

**Dependencies**: Requires Phase 7 (US5 — Tool trait) and Phase 8 (US6 — State trait)

- [X] T259 [P] Unit test: #[tool] macro generates Tool impl from annotated async function in crates/synwire-derive/src/tool.rs
- [X] T260 [P] Unit test: #[tool] macro uses schemars for JSON Schema generation in crates/synwire-derive/src/tool.rs
- [X] T261 [P] Unit test: #[tool] macro injects config: &RunnableConfig as special parameter (not in schema) in crates/synwire-derive/src/tool.rs
- [X] T262 [P] Unit test: #[derive(State)] generates State trait impl with correct channel mappings in crates/synwire-derive/src/state.rs
- [X] T263 [P] Unit test: #[derive(State)] with #[reducer(add_messages)] generates BinaryOperatorAggregate channel in crates/synwire-derive/src/state.rs
- [X] T264 Implement #[tool] attribute macro (generates StructuredTool from async fn, schemars schema, RunnableConfig injection) in crates/synwire-derive/src/tool.rs
- [X] T265 Implement #[derive(State)] derive macro (field -> channel mapping, default LastValue, #[reducer] -> BinaryOperatorAggregate) in crates/synwire-derive/src/state.rs
- [X] T266 Wire up synwire-derive lib.rs with proc_macro_attribute and proc_macro_derive exports in crates/synwire-derive/src/lib.rs
- [X] T267 Create derive_macros.rs example (#[tool] and #[derive(State)] usage) in examples/derive_macros.rs

---

## Phase 12: Ollama Provider — synwire-llm-ollama

**Purpose**: Native Ollama API provider (ChatOllama, OllamaEmbeddings)

**Dependencies**: Requires Phase 3 (US1 — BaseChatModel) and Phase 6 (US4 — Embeddings)

- [X] T268 [P] Unit test: ChatOllama builder constructs with default base_url localhost:11434 in crates/synwire-llm-ollama/src/chat.rs
- [X] T269 [P] Unit test: ChatOllama parses NDJSON streaming response correctly in crates/synwire-llm-ollama/src/chat.rs
- [X] T270 [P] Unit test: OllamaEmbeddings.embed_documents returns vectors in crates/synwire-llm-ollama/src/embeddings.rs
- [X] T271 Define OllamaError enum (#[non_exhaustive]) with From<OllamaError> for ModelError in crates/synwire-llm-ollama/src/error.rs
- [X] T272 Implement ChatOllama struct with builder (model, base_url, temperature, top_k, top_p, num_predict, timeout) in crates/synwire-llm-ollama/src/chat.rs
- [X] T273 Implement BaseChatModel for ChatOllama (POST /api/chat, NDJSON streaming) in crates/synwire-llm-ollama/src/chat.rs
- [X] T274 Implement OllamaEmbeddings with builder, Embeddings trait (POST /api/embed) in crates/synwire-llm-ollama/src/embeddings.rs
- [X] T275 Wire up synwire-llm-ollama lib.rs with pub exports in crates/synwire-llm-ollama/src/lib.rs
- [X] T276 [P] Add feature-gated integration test for ChatOllama (requires local Ollama server) in tests/integration/ollama_chat.rs
- [X] T277 Create ollama_chat.rs example (invoke ChatOllama, print response) in examples/ollama_chat.rs

### Coverage Gap Additions (FR-174)

- [X] T443 Implement credential refresh on 401/403 in ChatOllama (call refresh_credential, retry once) in crates/synwire-llm-ollama/src/chat.rs

---

## Phase 13: Convenience Crate — synwire

**Purpose**: Reference implementations for common application-level patterns

**Dependencies**: Requires Phase 4 (US2), Phase 6 (US4), Phase 7 (US5)

### Embedding Cache

- [X] T278 [P] Unit test: CacheBackedEmbeddings returns cached vectors on cache hit in crates/synwire/src/cache/embeddings.rs
- [X] T279 [P] Implement CacheBackedEmbeddings wrapping Embeddings + InMemoryEmbeddingCache (moka) in crates/synwire/src/cache/embeddings.rs

### Chat History

- [X] T280 [P] Unit test: RunnableWithMessageHistory injects history and tracks sessions in crates/synwire/src/chat_history/runnable.rs
- [X] T281 [P] Define ChatMessageHistory trait (get/add/clear) and implement InMemoryChatMessageHistory in crates/synwire/src/chat_history/traits.rs and in_memory.rs
- [X] T282 Implement RunnableWithMessageHistory wrapping Runnable + ChatMessageHistory in crates/synwire/src/chat_history/runnable.rs

### Few-Shot Prompts

- [X] T283 [P] Unit test: FewShotPromptTemplate produces correct output with mock examples in crates/synwire/src/prompts/few_shot.rs
- [X] T284 [P] Define ExampleSelector trait and implement SemanticSimilarityExampleSelector in crates/synwire/src/prompts/example_selector.rs
- [X] T285 Implement FewShotPromptTemplate and FewShotChatMessagePromptTemplate in crates/synwire/src/prompts/few_shot.rs

### Text Splitters

- [X] T286 [P] Unit test: CharacterTextSplitter respects chunk_size and overlap in crates/synwire/src/text_splitters/character.rs
- [X] T287 [P] Unit test: RecursiveCharacterTextSplitter splits on hierarchical separators in crates/synwire/src/text_splitters/recursive.rs
- [X] T288 [P] Implement CharacterTextSplitter in crates/synwire/src/text_splitters/character.rs
- [X] T289 [P] Implement RecursiveCharacterTextSplitter in crates/synwire/src/text_splitters/recursive.rs

### Additional Output Parsers

- [X] T290 [P] Implement CommaSeparatedListOutputParser in crates/synwire/src/output_parsers/list.rs
- [X] T291 [P] Implement EnumOutputParser in crates/synwire/src/output_parsers/enum_parser.rs
- [X] T292 [P] Implement XMLOutputParser in crates/synwire/src/output_parsers/xml.rs
- [X] T293 [P] Implement RegexParser in crates/synwire/src/output_parsers/regex.rs
- [X] T294 [P] Implement RetryOutputParser (wraps parser + LLM for reask) in crates/synwire/src/output_parsers/retry.rs
- [X] T295 [P] Implement CombiningOutputParser in crates/synwire/src/output_parsers/combining.rs
- [X] T296 Add unit tests for all additional output parsers in crates/synwire/src/output_parsers/

### OpenAI Moderation

- [X] T297 Implement OpenAIModerationMiddleware with as_runnable() in crates/synwire-llm-openai/src/moderation.rs
- [X] T298 Add unit test for moderation middleware (flags content, passes safe) in crates/synwire-llm-openai/src/moderation.rs

### Integration

- [X] T299 Wire up crates/synwire/src/lib.rs with all reference impl modules and re-exports in crates/synwire/src/lib.rs
- [X] T300 Create few_shot_rag.rs example (text splitter -> embed -> store -> few-shot retrieval -> model) in examples/few_shot_rag.rs

### Coverage Gap Additions (FR-199)

- [X] T444 [P] Define BatchProcessor<T> trait (feature-gated behind "batch-api") for provider batch APIs in crates/synwire-core/src/language_models/batch.rs
- [X] T445 [P] Unit test: BatchProcessor trait compiles and default impl is constructable in crates/synwire-core/src/language_models/batch.rs

**Checkpoint**: Phase 13 complete — all reference implementations provided

---

## Phase 14: Observability Stack

**Purpose**: OTel GenAI semantic conventions, EventBus, tracing bridge, metrics, sensitive data controls

**Dependencies**: Requires Phase 2 (Foundational) + Phase 7 (US5 — CallbackHandler)

### Core Observability Types

- [X] T301 [P] Implement ObservabilitySpanKind enum (Llm, Chain, Tool, Embedding, Retriever, Graph) in crates/synwire-core/src/observability/types.rs
- [X] T302 [P] Implement TraceContentFilter struct (include_input_messages, include_output_messages, include_system_instructions, include_tool_arguments, include_tool_results, include_retrieval_queries, max_content_length) in crates/synwire-core/src/observability/types.rs
- [X] T303 [P] Implement OTel GenAI attribute constants (gen_ai.operation.name, gen_ai.provider.name, gen_ai.request.model, etc.) in crates/synwire-core/src/observability/otel_attributes.rs
- [X] T304 [P] Implement OTel GenAI metrics constants (gen_ai.client.token.usage, gen_ai.client.operation.duration, gen_ai.client.operation.time_to_first_chunk) in crates/synwire-core/src/observability/otel_metrics.rs

### EventBus (feature-gated behind "event-bus")

- [X] T305 [P] Unit test: InMemoryEventBus subscribe, publish, filter, lagging subscriber in crates/synwire-core/src/observability/event_bus.rs
- [X] T306 [P] Define EventBus trait, EventFilter, EventKind, EventBusEvent in crates/synwire-core/src/observability/event_bus.rs
- [X] T307 Implement InMemoryEventBus using tokio::sync::broadcast (capacity 1024) in crates/synwire-core/src/observability/event_bus.rs

### Tracing Bridge (feature-gated behind "tracing")

- [X] T308 [P] Unit test: OTelTracingBridge maps EventBusEvent to spans with GenAI attributes in crates/synwire-core/src/observability/tracing_bridge.rs
- [X] T309 [P] Define TracingBridge trait (begin_span, end_span), SpanOutcome, SpanGuard in crates/synwire-core/src/observability/tracing_bridge.rs
- [X] T310 Implement OTelTracingBridge (EventBusEvent -> tracing spans with GenAI attributes) in crates/synwire-core/src/observability/tracing_bridge.rs

### OTel Attribute Mapper

- [X] T311 [P] Define OTelAttributeMapper trait in crates/synwire-core/src/observability/otel_mapper.rs
- [X] T312 Implement GenAIAttributeMapper (default impl mapping to OTel semantic conventions) in crates/synwire-core/src/observability/otel_mapper.rs

### Metrics Collector

- [X] T313 [P] Define MetricsCollector trait in crates/synwire-core/src/observability/metrics.rs
- [X] T314 Implement OTelMetricsCollector with histogram instruments (token.usage, operation.duration, time_to_first_chunk) in crates/synwire-core/src/observability/metrics.rs

### TracingCallbackHandler

- [X] T315 Unit test: TracingCallbackHandler redacts content when trace_include_sensitive_data=false in crates/synwire-core/src/observability/tracing_callback.rs
- [X] T316 Unit test: SecretValue in tracing span serialises as "***" in crates/synwire-core/src/observability/tracing_callback.rs
- [X] T317 Implement TracingCallbackHandler (CallbackHandler -> EventBus adapter with content_filter) in crates/synwire-core/src/observability/tracing_callback.rs

### Feature Flag Wiring

- [X] T318 [P] Implement TracingConfig, BatchConfig structs in crates/synwire-core/src/observability/config.rs
- [X] T319 Wire TracingConfig into RunnableConfig (optional field) in crates/synwire-core/src/runnables/config.rs
- [X] T320 Gate all observability modules behind #[cfg(feature = "tracing")] and #[cfg(feature = "event-bus")] in crates/synwire-core/src/observability/mod.rs
- [X] T321 Update synwire-core Cargo.toml tracing feature to include opentelemetry, opentelemetry-sdk, tracing-opentelemetry in crates/synwire-core/Cargo.toml
- [X] T322 Create otel_tracing.rs example (enable tracing, invoke model, inspect spans) in examples/otel_tracing.rs

### Coverage Gap Additions (FR-044)

- [X] T446 Resolve tracing feature flag: configure as default feature (negligible overhead when no subscriber) or document opt-in decision; update Cargo.toml accordingly in crates/synwire-core/Cargo.toml

**Checkpoint**: Phase 14 complete — full observability stack with OTel GenAI conventions

---

## Phase 15: Testing Infrastructure

**Purpose**: Nextest configuration, proptest strategies, shared test utilities, conformance suites, E2E tests

**Dependencies**: Requires Phase 1 (Setup). Property tests require their respective crate implementations.

### Nextest & CI Configuration

- [X] T323 [P] Add .config/nextest.toml with default and ci profiles (JUnit XML, timeouts: unit 10s, property 60s, integration 120s, retry config, api-tests group max concurrency 4) in .config/nextest.toml
- [X] T324 [P] Update .github/workflows/ci.yml to use cargo nextest run --profile ci in .github/workflows/ci.yml
- [X] T325 [P] Add JUnit XML upload as GitHub Actions artifact in .github/workflows/ci.yml
- [X] T326 [P] Add test partitioning via --partition hash:m/n to CI matrix for parallel execution in .github/workflows/ci.yml
- [X] T327 [P] Add cargo-geiger step to CI to enforce zero-unsafe in synwire-core and synwire-orchestrator in .github/workflows/ci.yml
- [X] T328 [P] Add cargo llvm-cov nextest --profile ci coverage step to merge-to-main workflow in .github/workflows/coverage.yml
- [X] T329 [P] Create .github/workflows/nightly.yml with extended proptest (PROPTEST_CASES=1024) and cargo audit in .github/workflows/nightly.yml
- [X] T330 [P] Add CI tier configuration: PR (Tier 1: fmt+clippy+unit), merge (Tier 1+2: +integration+coverage), nightly (Tier 3: extended) in .github/workflows/ci.yml

### Shared Test Utilities

- [X] T331 [P] Create proptest Strategy for Message (all role variants, text and structured content) in crates/synwire-test-utils/src/strategies/messages.rs
- [X] T332 [P] Create proptest Strategy for Document (arbitrary page_content, metadata HashMap) in crates/synwire-test-utils/src/strategies/documents.rs
- [X] T333 [P] Create proptest Strategy for PromptTemplate variables in crates/synwire-test-utils/src/strategies/prompts.rs
- [X] T334 [P] Create proptest Strategy for ToolInput and tool schemas in crates/synwire-test-utils/src/strategies/tools.rs
- [X] T335 [P] Create proptest Strategy for embedding vectors (arbitrary dimensionality, normalised) in crates/synwire-test-utils/src/strategies/embeddings.rs
- [X] T336 [P] Create proptest Strategy for CheckpointData in crates/synwire-test-utils/src/strategies/checkpoints.rs
- [X] T337 [P] Create proptest Strategy for channel updates (LastValue, Topic) in crates/synwire-test-utils/src/strategies/channels.rs
- [X] T338 [P] Create proptest Strategy for valid graph topologies in crates/synwire-test-utils/src/strategies/graphs.rs
- [X] T339 [P] Create test fixture builders in crates/synwire-test-utils/src/fixtures/builders.rs
- [X] T340 [P] Add proptest.toml at workspace root with cases=256, max_shrink_iters=4096 in proptest.toml
- [X] T341 Wire up synwire-test-utils lib.rs with re-exports of FakeChatModel, FakeEmbeddings, strategies, fixtures in crates/synwire-test-utils/src/lib.rs

### Property Tests — synwire-core

- [X] T342 [P] Add prop_message_serde_roundtrip: any Message serialises and deserialises to identical value in crates/synwire-core/tests/
- [X] T343 [P] Add prop_document_construction: arbitrary metadata produces valid Documents in crates/synwire-core/tests/
- [X] T344 [P] Add prop_prompt_template_substitution: all declared variables produce output containing all values in crates/synwire-core/tests/
- [X] T345 [P] Add prop_tool_schema_validation: valid ToolInput accepted, invalid rejected in crates/synwire-core/tests/
- [X] T346 [P] Add prop_synwire_error_display: all error variants implement Display without panicking in crates/synwire-core/tests/
- [X] T347 [P] Add prop_embedding_dimension_invariant: embed_documents vectors same dimensionality in crates/synwire-core/tests/
- [X] T348 [P] Add prop_vector_store_search_count: similarity_search returns exactly k results sorted in crates/synwire-core/tests/

### Property Tests — synwire-orchestrator

- [X] T349 [P] Add prop_channel_last_value_overwrites: LastValue always contains most recent in crates/synwire-orchestrator/tests/
- [X] T350 [P] Add prop_channel_topic_concatenates: Topic contains all updates in order in crates/synwire-orchestrator/tests/
- [X] T351 [P] Add prop_graph_compilation: valid topology compiles without error in crates/synwire-orchestrator/tests/
- [X] T352 [P] Add prop_pregel_determinism: same graph + same inputs produces identical output in crates/synwire-orchestrator/tests/
- [X] T353 [P] Add prop_checkpoint_roundtrip: checkpoint serialised and read back identical in crates/synwire-orchestrator/tests/
- [X] T354 [P] Add prop_conditional_edge_routing: router always selects valid node or END in crates/synwire-orchestrator/tests/
- [X] T355 [P] Add prop_send_fanout: Send lists produce one task per Send, all exactly once in crates/synwire-orchestrator/tests/

### Property Tests — Non-Functional

- [X] T356 [P] Add prop_resource_cleanup: dropped streams and cancelled futures do not leak in crates/synwire-core/tests/
- [X] T357 [P] Add prop_path_traversal_protection: malicious paths (.., null bytes, absolute) rejected in crates/synwire-core/tests/
- [X] T358 [P] Add prop_checkpoint_backwards_compat: checkpoint v1 readable by v1+ loader in crates/synwire-checkpoint/tests/

### E2E Testing with Tilt + Ollama

- [X] T359 [P] Create tilt/ directory with Tiltfile, ollama Dockerfile, pull-model.sh for qwen2.5:0.5b in tilt/
- [X] T360 [P] Create .github/workflows/e2e.yml with Tilt CI mode, model caching, JUnit upload in .github/workflows/e2e.yml
- [X] T361 Write E2E test: chat invoke + stream against Ollama in tilt/tests/e2e_chat.rs
- [X] T362 Write E2E test: RAG pipeline with real embeddings in tilt/tests/e2e_rag.rs
- [X] T363 Write E2E test: tool-using agent with Ollama in tilt/tests/e2e_agent.rs
- [X] T364 Write E2E test: graph with SQLite checkpoint and resume in tilt/tests/e2e_graph.rs

### Coverage Gap Additions (FR-041)

- [X] T447 [P] Add benchmark test: span creation overhead < 50 microseconds (criterion or divan) in crates/synwire-core/benches/span_overhead.rs

**Checkpoint**: Phase 15 complete — nextest, proptest, conformance, E2E all configured

---

## Phase 16: Documentation

**Purpose**: Documentation site, tutorials, how-to guides, explanation docs, reference docs per Diataxis framework (Constitution Principle VI)

**Dependencies**: Requires Phase 1 (Setup). Content tasks require respective implementation phases.

### Documentation Site Setup

- [X] T365 [P] Create docs/ directory with mdbook book.toml and src/SUMMARY.md in docs/
- [X] T366 [P] Add mdbook build and doc-test steps to CI workflow in .github/workflows/ci.yml
- [X] T367 [P] Add link-checking step (lychee or mdbook-linkcheck) to CI workflow in .github/workflows/ci.yml

### Tutorials (learning-oriented)

- [X] T368 Write getting-started tutorial: installation and first chat in docs/src/getting-started/first-chat.md
- [X] T369 [P] Write tutorial: prompt templates and chains in docs/src/getting-started/prompt-chains.md
- [X] T370 [P] Write tutorial: streaming responses in docs/src/getting-started/streaming.md
- [X] T371 [P] Write tutorial: RAG with vector stores in docs/src/getting-started/rag.md
- [X] T372 Write tutorial: tool-using agents with create_react_agent in docs/src/getting-started/tools-agent.md
- [X] T373 Write tutorial: graph-based agents with synwire-orchestrator in docs/src/getting-started/graph-agent.md
- [X] T374 [P] Write tutorial: #[tool] and #[derive(State)] macros in docs/src/getting-started/derive-macros.md

### How-To Guides (task-oriented)

- [X] T375 [P] Write how-to: add a custom tool in docs/src/how-to/custom-tool.md
- [X] T376 [P] Write how-to: switch LLM providers in docs/src/how-to/switch-provider.md
- [X] T377 [P] Write how-to: add checkpointing in docs/src/how-to/add-checkpointing.md
- [X] T378 [P] Write how-to: write custom channels in docs/src/how-to/custom-channel.md
- [X] T379 [P] Write how-to: use interrupts for HITL in docs/src/how-to/graph-interrupts.md
- [X] T380 [P] Write how-to: write a custom ChatModel/VectorStore provider in docs/src/how-to/custom-provider.md
- [X] T381 [P] Write how-to: enable tracing and redact sensitive data in docs/src/how-to/enable-tracing.md
- [X] T382 [P] Write how-to: credential management with SecretValue in docs/src/how-to/credentials.md
- [X] T383 [P] Write how-to: error handling with retry/fallback in docs/src/how-to/retry-fallback.md

### Explanation Documents (understanding-oriented)

- [X] T384 [P] Write explanation: trait-based architecture design in docs/src/explanation/architecture.md
- [X] T385 [P] Write explanation: Pregel execution model in docs/src/explanation/pregel.md
- [X] T386 [P] Write explanation: channel system design in docs/src/explanation/channels.md
- [X] T387 [P] Write explanation: crate organisation rationale in docs/src/explanation/crate-organisation.md
- [X] T388 Write explanation: Hook/Callback decision tree (FR-006) in docs/src/explanation/hooks-vs-callbacks.md
- [X] T389 Write explanation: LangChain-to-Synwire migration guide in docs/src/explanation/migration.md

### Reference Documents (information-oriented)

- [X] T390 [P] Write reference: terminology glossary (FR-001) in docs/src/reference/glossary.md
- [X] T391 [P] Write reference: common errors guide in docs/src/reference/error-guide.md
- [X] T392 [P] Write reference: feature flags across all crates in docs/src/reference/feature-flags.md
- [X] T393 [P] Write reference: offline/no-API-key usage in docs/src/reference/offline-usage.md

### Crate-Level Documentation

- [X] T394 Add //! module-level doc comments to all crate lib.rs files explaining purpose and relationships in crates/*/src/lib.rs
- [X] T395 [P] Add #[doc] comments with compilable examples to all public traits in synwire-core in crates/synwire-core/src/
- [X] T396 [P] Add #[doc] comments with compilable examples to all public traits in synwire-orchestrator in crates/synwire-orchestrator/src/
- [X] T397 [P] Add #[doc] comments to all error enum variants across all crates
- [X] T398 [P] Add feature flag documentation to all crate Cargo.toml and lib.rs files

### Contributing

- [X] T399 [P] Write contributor guide: setup, testing, PRs in docs/src/contributing/setup.md
- [X] T400 Write documentation style guide in docs/src/contributing/style-guide.md

### Examples

- [X] T401 Add file-level doc comments to all examples stating learning objective and expected output in examples/

### Coverage Gap Additions (FR-004, FR-067)

- [X] T44- [ ] T448  [P] Document cancellation safety per public async method in synwire-core and synwire-orchestrator in crates/synwire-core/src/ and crates/synwire-orchestrator/src/
- [X] T44- [ ] T449  [P] Write reference: OutputMode/TypedValue interop and conversion semantics in docs/src/reference/output-mode-typed-value.md

**Checkpoint**: Phase 16 complete — documentation site builds, doc-tests pass, Diataxis framework applied

---

## Phase 17: Polish & Cross-Cutting Concerns

**Purpose**: Final quality validation across entire workspace

- [X] T402 Run `cargo clippy --workspace --all-targets --all-features -- -D warnings` and fix any warnings
- [X] T403 Run `cargo fmt --check` and fix any formatting issues
- [X] T404 Run full `cargo nextest run` and verify all unit and property tests pass
- [X] T405 Run `cargo doc --workspace --no-deps` and verify zero warnings
- [X] T406 Validate quickstart.md examples compile (as doc tests or by running examples)
- [X] T407 [P] Add LICENSE file (MIT or Apache-2.0) in LICENSE
- [X] T408 Verify all examples compile with FakeChatModel (no API keys required)
- [X] T409 Verify #![forbid(unsafe_code)] on synwire-core and synwire-orchestrator crate roots

---

## Dependencies and Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion — BLOCKS all user stories
- **User Stories (Phase 3–10)**: All depend on Foundational phase completion
  - US1 (Phase 3): Can start immediately after Phase 2
  - US2 (Phase 4): Depends on US1 (needs BaseChatModel trait for chain testing)
  - US3 (Phase 5): Depends on US1 (extends BaseChatModel with streaming)
  - US4 (Phase 6): Independent of US1–US3 — can run in parallel after Phase 2
  - US5 (Phase 7): Independent of US1–US4 — can run in parallel after Phase 2
  - US6 (Phase 8): Depends on US2 (needs RunnableCore for CompiledGraph)
  - US7 (Phase 9): Depends on US6 (needs graph types for checkpointing integration)
  - US8 (Phase 10): Depends on US5 + US6 (needs tools + graph)
- **Proc Macros (Phase 11)**: Depends on US5 (Tool trait) + US6 (State trait)
- **Ollama (Phase 12)**: Depends on US1 (BaseChatModel) + US4 (Embeddings)
- **Convenience (Phase 13)**: Depends on US2 + US4 + US5
- **Observability (Phase 14)**: Depends on Phase 2 + US5 (CallbackHandler)
- **Testing Infrastructure (Phase 15)**: Depends on Phase 1; property tests need respective impls
- **Documentation (Phase 16)**: Depends on Phase 1; content needs respective impls
- **Polish (Phase 17)**: Depends on all other phases being complete

### User Story Dependencies

- **US1 (P1)**: No dependencies on other stories. MVP.
- **US2 (P2)**: Needs BaseChatModel from US1 for chain testing
- **US3 (P3)**: Needs BaseChatModel from US1 (extends with streaming)
- **US4 (P4)**: Independent — different traits (Embeddings, VectorStore). Parallel with US1–US3
- **US5 (P5)**: Independent — Tool trait and CallbackHandler. Parallel with US1–US4
- **US6 (P6)**: Needs RunnableCore from US2 for CompiledGraph
- **US7 (P7)**: Needs graph types from US6
- **US8 (P8)**: Needs tools from US5 + graph from US6

### Within Each User Story

- Tests MUST be written and FAIL before implementation (Constitution Principle IV)
- Types/structs before traits
- Traits before implementations
- Core crate before provider crate
- Story complete before moving to next priority (unless parallelising)

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- US4 and US5 can start in parallel after Phase 2 (independent traits)
- All tests within a story marked [P] can run in parallel
- Ollama (Phase 12) and Convenience (Phase 13) can run in parallel
- Observability (Phase 14) and Testing Infra (Phase 15) can run in parallel
- Documentation (Phase 16) can partially overlap with implementation phases

---

## Parallel Example: User Story 1

```bash
# Launch all tests for US1 together:
Task: "Unit test: FakeChatModel invoke returns ChatResult"
Task: "Unit test: FakeChatModel invoke with error returns Err"
Task: "Unit test: swapping mock providers compiles correctly"
Task: "Unit test: BaseChatModel batch invokes concurrently"
Task: "Unit test: invoking with empty message list returns error"

# Then implement sequentially:
Task: "Define BaseChatModel trait"
Task: "Define BaseLLM trait"
Task: "Implement FakeChatModel"
Task: "Implement BaseChatOpenAI base type"
Task: "Implement ChatOpenAI wrapping base"
Task: "Implement BaseChatModel for ChatOpenAI"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL — blocks all stories)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: FakeChatModel invoke + ChatOpenAI invoke works
5. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational -> Foundation ready
2. Add US1 -> Test independently -> Deliverable (MVP)
3. Add US2 -> Test independently -> Prompt templates and chains work
4. Add US3 -> Test independently -> Streaming works
5. Add US4 -> Test independently -> RAG pipeline works
6. Add US5 -> Test independently -> Tool use works
7. Add US6 -> Test independently -> Graph orchestration works
8. Add US7 -> Test independently -> Checkpointing works
9. Add US8 -> Test independently -> Prebuilt agents work
10. Phase 11-14 -> Cross-cutting: macros, Ollama, convenience, observability
11. Phase 15-16 -> Testing infrastructure + documentation
12. Phase 17 -> Polish

### Parallel Team Strategy

With multiple developers after Foundational is done:

- Developer A: US1 -> US2 -> US3 (sequential — dependencies)
- Developer B: US4 (independent)
- Developer C: US5 (independent)
- Developer D: Documentation (can start structural work immediately)

After US2 + US5 complete:
- Developer A: US6 -> US7 -> US8 (sequential — dependencies)
- Developer B: Phase 12 (Ollama)
- Developer C: Phase 13 (Convenience crate)

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing (Constitution Principle IV)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- M2/M3 items are excluded — see docs/roadmap.md
