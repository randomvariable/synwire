# Tasks: LangChain Rust Port

**Input**: Design documents from `/specs/001-langchain-rust-port/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/

**Tests**: Tests are included — the spec requires 90% coverage (SC-002) and the constitution mandates comprehensive testing (Principle V).

**Organisation**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Workspace root**: `Cargo.toml`
- **Core crate**: `crates/langchain-core/src/`
- **OpenAI crate**: `crates/langchain-openai/src/`
- **Re-export crate**: `crates/langchain/src/`
- **Examples**: `examples/`
- **Integration tests**: `tests/integration/`

---

## Phase 1: Setup

**Purpose**: Cargo workspace initialisation and project scaffolding

- [ ] T001 Create workspace root Cargo.toml with members: langchain-core, langchain-openai, langchain in Cargo.toml
- [ ] T002 [P] Create langchain-core crate with Cargo.toml (deps: futures-core, futures-util, pin-project-lite, thiserror, serde, serde_json, uuid) in crates/langchain-core/Cargo.toml
- [ ] T003 [P] Create langchain-openai crate with Cargo.toml (deps: langchain-core, reqwest, eventsource-stream, tokio) in crates/langchain-openai/Cargo.toml
- [ ] T004 [P] Create langchain re-export crate with Cargo.toml (deps: langchain-core, langchain-openai) in crates/langchain/Cargo.toml
- [ ] T005 [P] Add .gitignore for Rust (target/, Cargo.lock for libraries) in .gitignore
- [ ] T006 [P] Add clippy.toml and rustfmt.toml with project conventions in clippy.toml and rustfmt.toml
- [ ] T007 [P] Create CI workflow (fmt check, clippy, test, doc build) in .github/workflows/ci.yml
- [ ] T008 [P] Create coverage workflow (cargo-llvm-cov, upload report) in .github/workflows/coverage.yml
- [ ] T009 Verify workspace builds with `cargo check` on all member crates

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core types, error handling, and type aliases that ALL user stories depend on

**CRITICAL**: No user story work can begin until this phase is complete

- [ ] T010 Define BoxFuture and BoxStream type aliases in crates/langchain-core/src/lib.rs
- [ ] T011 [P] Implement LangChainError enum with thiserror in crates/langchain-core/src/error.rs
- [ ] T012 [P] Implement Message enum (Human, AI, System, Tool) with MessageContent, ContentBlock in crates/langchain-core/src/messages/types.rs
- [ ] T013 [P] Implement ToolCall, UsageMetadata structs with serde derives in crates/langchain-core/src/messages/types.rs
- [ ] T014 [P] Implement Document struct with serde derives in crates/langchain-core/src/documents/types.rs
- [ ] T015 [P] Implement ChatResult, LLMResult, Generation structs in crates/langchain-core/src/language_models/types.rs
- [ ] T016 [P] Implement RunnableConfig struct in crates/langchain-core/src/runnables/traits.rs
- [ ] T017 [P] Implement ToolSchema struct in crates/langchain-core/src/tools/types.rs
- [ ] T018 Create module hierarchy (mod.rs files) for all core submodules in crates/langchain-core/src/
- [ ] T019 Create prelude.rs re-exporting all public types and traits in crates/langchain-core/src/prelude.rs
- [ ] T020 Wire up lib.rs with pub mod declarations and prelude re-export in crates/langchain-core/src/lib.rs
- [ ] T021 Add unit tests for Message enum construction, serialisation, content() accessor in crates/langchain-core/src/messages/types.rs
- [ ] T022 [P] Add unit tests for Document construction and serde round-trip in crates/langchain-core/src/documents/types.rs
- [ ] T023 [P] Add unit tests for LangChainError Display and From conversions in crates/langchain-core/src/error.rs
- [ ] T024 Verify `cargo test -p langchain-core` passes and `cargo clippy` is clean

**Checkpoint**: Foundation ready — all core types compile, serialise, and pass unit tests

---

## Phase 3: User Story 1 — Define and Invoke a Chat Model (Priority: P1) MVP

**Goal**: A developer can implement a chat model trait, invoke it, and receive a typed ChatResult

**Independent Test**: Mock chat model implementing BaseChatModel; invoke with messages; verify ChatResult structure

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T025 [P] [US1] Unit test: mock BaseChatModel invoke returns ChatResult in crates/langchain-core/src/language_models/traits.rs
- [ ] T026 [P] [US1] Unit test: mock BaseChatModel invoke with error returns Result::Err in crates/langchain-core/src/language_models/traits.rs
- [ ] T027 [P] [US1] Unit test: swapping mock providers compiles and produces correct output in crates/langchain-core/src/language_models/traits.rs
- [ ] T028 [P] [US1] Unit test: BaseChatModel batch invokes multiple inputs concurrently in crates/langchain-core/src/language_models/traits.rs
- [ ] T029 [P] [US1] Unit test: invoking with empty message list returns validation error in crates/langchain-core/src/language_models/traits.rs

### Implementation for User Story 1

- [ ] T030 [US1] Define BaseChatModel trait with invoke, batch, stream, model_type in crates/langchain-core/src/language_models/traits.rs
- [ ] T031 [US1] Define BaseLLM trait with invoke, batch, stream, model_type in crates/langchain-core/src/language_models/traits.rs
- [ ] T032 [US1] Implement ChatOpenAI struct with builder pattern in crates/langchain-openai/src/chat.rs
- [ ] T033 [US1] Implement BaseChatModel for ChatOpenAI (invoke via reqwest POST to /v1/chat/completions) in crates/langchain-openai/src/chat.rs
- [ ] T034 [US1] Define OpenAI-specific error types with From<LangChainError> in crates/langchain-openai/src/error.rs
- [ ] T035 [US1] Implement batch for ChatOpenAI (concurrent invoke via futures::join_all) in crates/langchain-openai/src/chat.rs
- [ ] T036 [US1] Wire up langchain-openai lib.rs with pub exports in crates/langchain-openai/src/lib.rs
- [ ] T037 [US1] Create simple_chat.rs example (invoke ChatOpenAI, print response) in examples/simple_chat.rs
- [ ] T038 [US1] Add feature-gated integration test for ChatOpenAI invoke in tests/integration/openai_chat.rs

**Checkpoint**: US1 complete — mock and real chat model invocation works end-to-end

---

## Phase 4: User Story 2 — Compose Prompt Templates and Chains (Priority: P2)

**Goal**: A developer can create prompt templates, format them, and chain them with a model

**Independent Test**: Create template with variables, format, verify output; chain template to mock model, invoke chain

### Tests for User Story 2

- [ ] T039 [P] [US2] Unit test: PromptTemplate.format substitutes variables correctly in crates/langchain-core/src/prompts/template.rs
- [ ] T040 [P] [US2] Unit test: PromptTemplate.format with missing variable returns PromptError in crates/langchain-core/src/prompts/template.rs
- [ ] T041 [P] [US2] Unit test: ChatPromptTemplate.format_messages produces correct Message list in crates/langchain-core/src/prompts/chat.rs
- [ ] T042 [P] [US2] Unit test: RunnableSequence chains template to mock model and returns result in crates/langchain-core/src/runnables/chain.rs
- [ ] T043 [P] [US2] Unit test: Runnable.pipe composes two runnables sequentially in crates/langchain-core/src/runnables/chain.rs

### Implementation for User Story 2

- [ ] T044 [US2] Implement PromptTemplate with new() and format() in crates/langchain-core/src/prompts/template.rs
- [ ] T045 [US2] Implement ChatPromptTemplate with from_messages() and format_messages() in crates/langchain-core/src/prompts/chat.rs
- [ ] T046 [US2] Define Runnable<I, O> trait with invoke, batch, stream in crates/langchain-core/src/runnables/traits.rs
- [ ] T047 [US2] Implement RunnableSequence (chains two Runnables) in crates/langchain-core/src/runnables/chain.rs
- [ ] T048 [US2] Implement RunnablePassthrough (forwards input unchanged) in crates/langchain-core/src/runnables/passthrough.rs
- [ ] T049 [US2] Add pipe() method to Runnable trait for composition in crates/langchain-core/src/runnables/traits.rs
- [ ] T050 [US2] Implement OutputParser<T> trait in crates/langchain-core/src/output_parsers/traits.rs
- [ ] T051 [US2] Create prompt_chain.rs example (template → model → print) in examples/prompt_chain.rs

**Checkpoint**: US2 complete — templates format correctly and chain composition works with mock models

---

## Phase 5: User Story 3 — Stream Responses (Priority: P3)

**Goal**: A developer can stream model responses as async chunks

**Independent Test**: Mock model yields incremental chunks; verify order, completeness, and error handling

### Tests for User Story 3

- [ ] T052 [P] [US3] Unit test: mock model stream yields chunks in order in crates/langchain-core/src/language_models/traits.rs
- [ ] T053 [P] [US3] Unit test: concatenated stream chunks equal invoke result in crates/langchain-core/src/language_models/traits.rs
- [ ] T054 [P] [US3] Unit test: stream with mid-stream error yields error item and terminates in crates/langchain-core/src/language_models/traits.rs
- [ ] T055 [P] [US3] Unit test: dropping stream mid-way does not leak resources in crates/langchain-core/src/language_models/traits.rs

### Implementation for User Story 3

- [ ] T056 [US3] Define ChatChunk struct (delta content, tool_calls delta, usage) in crates/langchain-core/src/language_models/types.rs
- [ ] T057 [US3] Implement BaseChatModel.stream for ChatOpenAI using SSE parsing in crates/langchain-openai/src/chat.rs
- [ ] T058 [US3] Add SSE stream parsing using eventsource-stream for OpenAI response format in crates/langchain-openai/src/chat.rs
- [ ] T059 [US3] Implement Runnable.stream default (wraps invoke output as single-item stream) in crates/langchain-core/src/runnables/traits.rs
- [ ] T060 [US3] Create streaming.rs example (stream ChatOpenAI, print tokens as they arrive) in examples/streaming.rs
- [ ] T061 [US3] Add feature-gated integration test for ChatOpenAI streaming in tests/integration/openai_chat.rs

**Checkpoint**: US3 complete — streaming works with mock and real providers

---

## Phase 6: User Story 4 — Embed Text and Query a Vector Store (Priority: P4)

**Goal**: A developer can embed documents, store them, and perform similarity search

**Independent Test**: Mock embeddings return deterministic vectors; InMemoryVectorStore returns correct ranked results

### Tests for User Story 4

- [ ] T062 [P] [US4] Unit test: mock Embeddings.embed_documents returns one vector per text in crates/langchain-core/src/embeddings/traits.rs
- [ ] T063 [P] [US4] Unit test: mock Embeddings.embed_query returns single vector in crates/langchain-core/src/embeddings/traits.rs
- [ ] T064 [P] [US4] Unit test: InMemoryVectorStore.add_documents stores and returns IDs in crates/langchain-core/src/vectorstores/in_memory.rs
- [ ] T065 [P] [US4] Unit test: InMemoryVectorStore.similarity_search returns ranked results in crates/langchain-core/src/vectorstores/in_memory.rs
- [ ] T066 [P] [US4] Unit test: InMemoryVectorStore.similarity_search on empty store returns empty vec in crates/langchain-core/src/vectorstores/in_memory.rs
- [ ] T067 [P] [US4] Unit test: InMemoryVectorStore rejects mismatched embedding dimensions in crates/langchain-core/src/vectorstores/in_memory.rs

### Implementation for User Story 4

- [ ] T068 [US4] Define Embeddings trait with embed_documents, embed_query in crates/langchain-core/src/embeddings/traits.rs
- [ ] T069 [US4] Define VectorStore trait with add_documents, similarity_search, similarity_search_with_score, delete in crates/langchain-core/src/vectorstores/traits.rs
- [ ] T070 [US4] Implement InMemoryVectorStore (cosine similarity, brute-force search) in crates/langchain-core/src/vectorstores/in_memory.rs
- [ ] T071 [US4] Define Retriever trait with get_relevant_documents in crates/langchain-core/src/retrievers/traits.rs
- [ ] T072 [US4] Implement VectorStoreRetriever (wraps VectorStore as a Retriever) in crates/langchain-core/src/retrievers/traits.rs
- [ ] T073 [US4] Implement OpenAIEmbeddings struct with builder pattern in crates/langchain-openai/src/embeddings.rs
- [ ] T074 [US4] Implement Embeddings for OpenAIEmbeddings (POST to /v1/embeddings) in crates/langchain-openai/src/embeddings.rs
- [ ] T075 [US4] Create rag.rs example (embed docs → store → search → model answer) in examples/rag.rs
- [ ] T076 [US4] Add feature-gated integration test for OpenAIEmbeddings in tests/integration/openai_embeddings.rs

**Checkpoint**: US4 complete — embeddings, vector store, and retrieval work independently

---

## Phase 7: User Story 5 — Define and Use Tools (Priority: P5)

**Goal**: A developer can define tools with typed schemas and invoke them

**Independent Test**: Mock tool with schema; invoke with valid/invalid input; verify schema serialisation

### Tests for User Story 5

- [ ] T077 [P] [US5] Unit test: mock Tool.invoke with valid input returns result in crates/langchain-core/src/tools/traits.rs
- [ ] T078 [P] [US5] Unit test: mock Tool.schema returns serialisable ToolSchema in crates/langchain-core/src/tools/traits.rs
- [ ] T079 [P] [US5] Unit test: mock Tool.invoke with invalid input returns ToolError in crates/langchain-core/src/tools/traits.rs

### Implementation for User Story 5

- [ ] T080 [US5] Define Tool trait with name, description, schema, invoke in crates/langchain-core/src/tools/traits.rs
- [ ] T081 [US5] Define CallbackHandler trait with all hook methods (default no-op impls) in crates/langchain-core/src/callbacks/traits.rs
- [ ] T082 [US5] Add ToolCall and ToolMessage handling to ChatOpenAI response parsing in crates/langchain-openai/src/chat.rs

**Checkpoint**: US5 complete — tools can be defined, invoked, and their schemas serialised

---

## Phase 8: Polish and Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T083 [P] Wire up langchain crate re-exports (core prelude + openai types) in crates/langchain/src/lib.rs
- [ ] T084 [P] Add rustdoc comments to all public traits and types; verify `cargo doc --no-deps` builds clean
- [ ] T085 [P] Add LICENSE file (MIT) in LICENSE
- [ ] T086 Run `cargo clippy -- -D warnings` across entire workspace and fix any warnings
- [ ] T087 Run `cargo fmt --check` and fix any formatting issues
- [ ] T088 Run full `cargo test` and verify all unit tests pass
- [ ] T089 Validate quickstart.md examples compile (as doc tests or by running examples)

---

## Phase 9: Scope Expansion — Agents, Observability, Resilience, MMR

**Purpose**: Features brought into scope from the excluded list: agent framework,
event streaming, retry/fallbacks, MMR, batch_as_completed, transform.

**Dependencies**: Requires Phase 2 (Foundational) + Phase 7 (US5 — Tools, CallbackHandler)

### Agent Framework

- [ ] T090 [P] Add AgentAction, AgentFinish, AgentStep, AgentDecision, AgentInput types in crates/langchain-core/src/agents/types.rs
- [ ] T091 [P] Add agents module (mod.rs with re-exports) in crates/langchain-core/src/agents/mod.rs
- [ ] T092 Implement AgentExecutor struct with ReAct loop (invoke: LLM → parse → tool → observe → repeat) in crates/langchain-core/src/agents/executor.rs
- [ ] T093 Implement Runnable<HashMap<String,Value>, HashMap<String,Value>> for AgentExecutor in crates/langchain-core/src/agents/executor.rs
- [ ] T094 [P] Add on_agent_action, on_agent_finish, ignore_agent hooks to CallbackHandler trait in crates/langchain-core/src/callbacks/traits.rs
- [ ] T095 [P] Add on_custom_event hook to CallbackHandler trait in crates/langchain-core/src/callbacks/traits.rs
- [ ] T096 Add unit tests for AgentExecutor with mock LLM and tools in crates/langchain-core/src/agents/executor.rs (tests module)

### VectorStore MMR

- [ ] T097 [P] Add max_marginal_relevance_search, max_marginal_relevance_search_by_vector methods to VectorStore trait in crates/langchain-core/src/vectorstores/traits.rs
- [ ] T098 [P] Implement MMR algorithm (cosine similarity + diversity scoring) in crates/langchain-core/src/vectorstores/mmr.rs
- [ ] T099 Add MMR support to InMemoryVectorStore in crates/langchain-core/src/vectorstores/in_memory.rs

### Runnable Composition

- [ ] T100 [P] Add transform method to Runnable trait with default buffer-then-stream impl in crates/langchain-core/src/runnables/traits.rs
- [ ] T101 [P] Add batch_as_completed method to Runnable trait returning BoxStream<(usize, Result<O>)> in crates/langchain-core/src/runnables/traits.rs
- [ ] T102 Add RetryConfig, LangChainErrorKind types and RunnableRetry struct in crates/langchain-core/src/runnables/retry.rs
- [ ] T103 Add RunnableWithFallbacks struct and with_fallbacks composition fn in crates/langchain-core/src/runnables/fallbacks.rs
- [ ] T104 Add backoff dependency to langchain-core Cargo.toml in crates/langchain-core/Cargo.toml

### Event Streaming & Observability

- [ ] T105 [P] Add StreamEvent, EventData, RunLogPatch, JsonPatchOp types in crates/langchain-core/src/runnables/events.rs
- [ ] T106 Add stream_events method to Runnable trait in crates/langchain-core/src/runnables/traits.rs
- [ ] T107 Add stream_log method to Runnable trait in crates/langchain-core/src/runnables/traits.rs
- [ ] T108 [P] Add optional tracing feature flag to langchain-core Cargo.toml in crates/langchain-core/Cargo.toml
- [ ] T109 Add tracing + OTel instrumentation behind feature flag in crates/langchain-core/src/lib.rs
- [ ] T110 [P] Add reqwest-retry + reqwest-middleware to langchain-openai in crates/langchain-openai/Cargo.toml

### Runnable as_tool

- [ ] T113 Implement RunnableTool struct and as_tool composition fn in crates/langchain-core/src/runnables/as_tool.rs

### Callback on_retry

- [ ] T114 [P] Add on_retry hook and RetryState type to CallbackHandler trait in crates/langchain-core/src/callbacks/traits.rs
- [ ] T115 Wire on_retry callback into RunnableRetry invoke loop in crates/langchain-core/src/runnables/retry.rs

### Integration

- [ ] T111 Update langchain-core/src/lib.rs with new module re-exports (agents, mmr, events, retry, as_tool) in crates/langchain-core/src/lib.rs
- [ ] T112 [P] Add agent example (simple_agent.rs — ReAct loop with mock tools) in examples/simple_agent.rs

**Checkpoint**: Phase 9 complete — agents, MMR, retry/fallbacks, event streaming, as_tool all functional

---

## Phase 10: Expanded Core — Message Utilities, Concrete Types, Parsers, Test Utilities

**Purpose**: Newly in-scope items from parity analysis: message utilities, expanded content
blocks, concrete runnable types, output parser implementations, StructuredTool, test utilities,
dispatch_custom_event, RetrieverRunnable adapter.

**Dependencies**: Requires Phase 2 (Foundational) + Phase 4 (US2 — Runnable trait, OutputParser trait)

### Message Types & Utilities

- [ ] T116 [P] Add Chat variant (generic role) to Message enum in crates/langchain-core/src/messages/types.rs
- [ ] T117 [P] Add Audio, Video, File, Reasoning, Thinking variants to ContentBlock enum in crates/langchain-core/src/messages/types.rs
- [ ] T118 [P] Implement MessageLike trait (Into<Vec<Message>> conversions) in crates/langchain-core/src/messages/traits.rs
- [ ] T119 Implement filter_messages function in crates/langchain-core/src/messages/utils.rs
- [ ] T120 [P] Implement trim_messages function with TrimStrategy enum in crates/langchain-core/src/messages/utils.rs
- [ ] T121 [P] Implement merge_message_runs function in crates/langchain-core/src/messages/utils.rs
- [ ] T122 Add unit tests for filter_messages, trim_messages, merge_message_runs in crates/langchain-core/src/messages/utils.rs
- [ ] T123 [P] Add ToolOutput type (content + optional artifact) in crates/langchain-core/src/tools/types.rs

### Concrete Runnable Types

- [ ] T124 [P] Implement RunnableLambda (closure wrapper with new/with_name) in crates/langchain-core/src/runnables/lambda.rs
- [ ] T125 [P] Implement RunnableBranch (condition/runnable pairs + default) in crates/langchain-core/src/runnables/branch.rs
- [ ] T126 Add unit tests for RunnableLambda invoke/stream in crates/langchain-core/src/runnables/lambda.rs
- [ ] T127 [P] Add unit tests for RunnableBranch routing in crates/langchain-core/src/runnables/branch.rs

### Concrete Output Parsers

- [ ] T128 [P] Implement StrOutputParser in crates/langchain-core/src/output_parsers/string.rs
- [ ] T129 [P] Implement JsonOutputParser in crates/langchain-core/src/output_parsers/json.rs
- [ ] T130 [P] Implement StructuredOutputParser<T: DeserializeOwned> in crates/langchain-core/src/output_parsers/structured.rs
- [ ] T131 [P] Implement ToolsOutputParser in crates/langchain-core/src/output_parsers/tools.rs
- [ ] T132 Add unit tests for all four output parsers in crates/langchain-core/src/output_parsers/ (test modules)

### StructuredTool

- [ ] T133 Implement StructuredTool and StructuredToolBuilder in crates/langchain-core/src/tools/structured.rs
- [ ] T134 Add unit tests for StructuredTool builder and invoke in crates/langchain-core/src/tools/structured.rs

### Test Utilities

- [ ] T135 [P] Implement FakeChatModel (configurable responses, call tracking) in crates/langchain-core/src/language_models/fake.rs
- [ ] T136 [P] Implement FakeEmbeddings (deterministic vectors from hash) in crates/langchain-core/src/embeddings/fake.rs
- [ ] T137 Add unit tests for FakeChatModel and FakeEmbeddings in their respective modules

### Retriever Adapter

- [ ] T138 Implement RetrieverRunnable adapter (blanket Runnable impl for Retriever) in crates/langchain-core/src/retrievers/runnable.rs
- [ ] T139 Add unit test for RetrieverRunnable with mock retriever in crates/langchain-core/src/retrievers/runnable.rs

### Custom Events

- [ ] T140 Implement dispatch_custom_event standalone function in crates/langchain-core/src/runnables/events.rs
- [ ] T141 Add unit test for dispatch_custom_event triggering on_custom_event callback in crates/langchain-core/src/runnables/events.rs

### Integration

- [ ] T142 Update crates/langchain-core/src/lib.rs with new module re-exports (messages/utils, runnables/lambda, runnables/branch, output_parsers/*, tools/structured, language_models/fake, embeddings/fake, retrievers/runnable) in crates/langchain-core/src/lib.rs
- [ ] T143 Update prelude.rs with new public types (MessageLike, StrOutputParser, JsonOutputParser, StructuredOutputParser, ToolsOutputParser, StructuredTool, FakeChatModel, FakeEmbeddings, RunnableLambda, RunnableBranch, RetrieverRunnable) in crates/langchain-core/src/prelude.rs

**Checkpoint**: Phase 10 complete — all core parity items implemented

---

## Phase 11: Reference Implementations (`langchain` crate)

**Purpose**: Provide ready-to-use reference implementations for common
application-level patterns in the `langchain` convenience crate, mirroring
Python's `langchain` package layering. Previously excluded as "application-level
concerns" — now concrete implementations.

**Dependencies**: Requires Phase 2 (Foundational), Phase 4 (US2 — OutputParser trait),
Phase 6 (US4 — Embeddings, VectorStore), Phase 7 (US5 — Tool, CallbackHandler)

### Setup

- [ ] T144 Update crates/langchain/Cargo.toml with new dependencies (moka, quick-xml, regex) and module structure in crates/langchain/Cargo.toml

### Embedding Cache

- [ ] T145 [P] Define EmbeddingCache trait and InMemoryEmbeddingCache (moka) in crates/langchain/src/cache/embeddings.rs
- [ ] T146 [P] Implement CacheBackedEmbeddings wrapping Embeddings + EmbeddingCache in crates/langchain/src/cache/embeddings.rs
- [ ] T147 Add unit tests for CacheBackedEmbeddings (cache hit, cache miss, invalidation) in crates/langchain/src/cache/embeddings.rs

### Chat History

- [ ] T148 [P] Define ChatMessageHistory trait (get/add/clear messages) in crates/langchain/src/chat_history/traits.rs
- [ ] T149 [P] Implement InMemoryChatMessageHistory in crates/langchain/src/chat_history/in_memory.rs
- [ ] T150 Implement RunnableWithMessageHistory wrapping Runnable + ChatMessageHistory in crates/langchain/src/chat_history/runnable.rs
- [ ] T151 Add unit tests for RunnableWithMessageHistory (history injection, multi-session) in crates/langchain/src/chat_history/runnable.rs

### Few-Shot Prompts

- [ ] T152 [P] Define ExampleSelector trait in crates/langchain/src/prompts/example_selector.rs
- [ ] T153 [P] Implement SemanticSimilarityExampleSelector using VectorStore in crates/langchain/src/prompts/example_selector.rs
- [ ] T154 Implement FewShotPromptTemplate and FewShotChatMessagePromptTemplate in crates/langchain/src/prompts/few_shot.rs
- [ ] T155 Add unit tests for few-shot templates with mock examples in crates/langchain/src/prompts/few_shot.rs

### Text Splitters

- [ ] T156 [P] Implement CharacterTextSplitter in crates/langchain/src/text_splitters/character.rs
- [ ] T157 [P] Implement RecursiveCharacterTextSplitter in crates/langchain/src/text_splitters/recursive.rs
- [ ] T158 Add unit tests for text splitters (chunk size, overlap, edge cases) in crates/langchain/src/text_splitters/

### Additional Output Parsers

- [ ] T159 [P] Implement CommaSeparatedListOutputParser in crates/langchain/src/output_parsers/list.rs
- [ ] T160 [P] Implement EnumOutputParser in crates/langchain/src/output_parsers/enum_parser.rs
- [ ] T161 [P] Implement XMLOutputParser in crates/langchain/src/output_parsers/xml.rs
- [ ] T162 [P] Implement RegexParser in crates/langchain/src/output_parsers/regex.rs
- [ ] T163 [P] Implement RetryOutputParser (wraps parser + LLM) in crates/langchain/src/output_parsers/retry.rs
- [ ] T164 [P] Implement CombiningOutputParser in crates/langchain/src/output_parsers/combining.rs
- [ ] T165 Add unit tests for all additional output parsers in crates/langchain/src/output_parsers/

### OpenAI Moderation

- [ ] T166 Implement OpenAIModerationMiddleware with as_runnable() in crates/langchain-openai/src/moderation.rs
- [ ] T167 Add unit test for moderation middleware in crates/langchain-openai/src/moderation.rs

### Integration

- [ ] T168 Wire up crates/langchain/src/lib.rs with all reference impl modules and re-exports in crates/langchain/src/lib.rs
- [ ] T169 Add example: few_shot_rag.rs (text splitter → embed → store → few-shot retrieval → model) in examples/few_shot_rag.rs

**Checkpoint**: Phase 11 complete — all reference implementations provided, full Python parity

---

## Phase 12: Provider Crates — All Partners

**Goal**: Implement all 16 provider crates covering the full Python LangChain
partners ecosystem. OpenAI-compatible providers share `BaseChatOpenAI`.

### OpenAI Base Refactor (prerequisite for OpenAI-compatible providers)

- [ ] T170 [P] Extract `BaseChatOpenAI` shared base type from `ChatOpenAI` in `crates/langchain-openai/src/base.rs`
- [ ] T171 Refactor `ChatOpenAI` to wrap `BaseChatOpenAI` in `crates/langchain-openai/src/chat.rs`
- [ ] T172 Add `base` module re-export (pub(crate)) in `crates/langchain-openai/src/lib.rs`

### Anthropic (native API)

- [ ] T173 [P] Create `crates/langchain-anthropic/Cargo.toml` with langchain-core, reqwest, eventsource-stream dependencies
- [ ] T174 [P] Define `AnthropicError` enum with HTTP status mapping in `crates/langchain-anthropic/src/error.rs`
- [ ] T175 Implement `ChatAnthropic` struct with builder in `crates/langchain-anthropic/src/chat.rs`
- [ ] T176 Implement `BaseChatModel` for `ChatAnthropic` (POST `/v1/messages`, system prompt extraction) in `crates/langchain-anthropic/src/chat.rs`
- [ ] T177 Implement SSE streaming for `ChatAnthropic` (content_block_delta events) in `crates/langchain-anthropic/src/chat.rs`
- [ ] T178 Implement Anthropic tool-use format (content_block type: tool_use) in `crates/langchain-anthropic/src/chat.rs`
- [ ] T179 [P] Add integration test `tests/integration/anthropic_chat.rs` (feature-gated)

### Ollama (native API, local server)

- [ ] T180 [P] Create `crates/langchain-ollama/Cargo.toml` with langchain-core, reqwest dependencies
- [ ] T181 [P] Define `OllamaError` enum in `crates/langchain-ollama/src/error.rs`
- [ ] T182 Implement `ChatOllama` struct with builder in `crates/langchain-ollama/src/chat.rs`
- [ ] T183 Implement `BaseChatModel` for `ChatOllama` (POST `/api/chat`, NDJSON streaming) in `crates/langchain-ollama/src/chat.rs`
- [ ] T184 Implement `OllamaLLM` with `BaseLLM` (POST `/api/generate`) in `crates/langchain-ollama/src/llm.rs`
- [ ] T185 Implement `OllamaEmbeddings` with `Embeddings` (POST `/api/embed`) in `crates/langchain-ollama/src/embeddings.rs`
- [ ] T186 [P] Add integration test `tests/integration/ollama_chat.rs` (feature-gated, requires local server)

### HuggingFace (native API)

- [ ] T187 [P] Create `crates/langchain-huggingface/Cargo.toml` with langchain-core, reqwest dependencies
- [ ] T188 [P] Define `HuggingFaceError` enum in `crates/langchain-huggingface/src/error.rs`
- [ ] T189 Implement `ChatHuggingFace` struct with builder in `crates/langchain-huggingface/src/chat.rs`
- [ ] T190 Implement `BaseChatModel` for `ChatHuggingFace` (HF Inference API) in `crates/langchain-huggingface/src/chat.rs`
- [ ] T191 Implement `HuggingFaceEmbeddings` with `Embeddings` trait in `crates/langchain-huggingface/src/embeddings.rs`
- [ ] T192 Implement `HuggingFacePipeline` with `BaseLLM` (API-only) in `crates/langchain-huggingface/src/pipeline.rs`

### Vector Stores

- [ ] T193 [P] Create `crates/langchain-chroma/Cargo.toml` with langchain-core, reqwest dependencies
- [ ] T194 Implement `ChromaClient` REST client in `crates/langchain-chroma/src/client.rs`
- [ ] T195 Implement `Chroma` struct with `VectorStore` trait in `crates/langchain-chroma/src/vectorstore.rs`
- [ ] T196 [P] Add integration test `tests/integration/chroma_vectorstore.rs` (feature-gated, requires local server)
- [ ] T197 [P] Create `crates/langchain-qdrant/Cargo.toml` with langchain-core, reqwest dependencies
- [ ] T198 Implement `QdrantClient` REST client in `crates/langchain-qdrant/src/client.rs`
- [ ] T199 Implement `QdrantVectorStore` with `VectorStore` trait in `crates/langchain-qdrant/src/vectorstore.rs`
- [ ] T200 [P] Add integration test `tests/integration/qdrant_vectorstore.rs` (feature-gated, requires local server)

### MistralAI (OpenAI-partial)

- [ ] T201 [P] Create `crates/langchain-mistralai/Cargo.toml` with langchain-core, langchain-openai dependencies
- [ ] T202 Implement `ChatMistralAI` wrapping `BaseChatOpenAI` in `crates/langchain-mistralai/src/chat.rs`
- [ ] T203 Implement `MistralAIEmbeddings` with `Embeddings` trait in `crates/langchain-mistralai/src/embeddings.rs`
- [ ] T204 [P] Add integration test `tests/integration/mistralai_chat.rs` (feature-gated)

### Fireworks (OpenAI-compatible)

- [ ] T205 [P] Create `crates/langchain-fireworks/Cargo.toml` with langchain-core, langchain-openai dependencies
- [ ] T206 Implement `ChatFireworks` wrapping `BaseChatOpenAI` in `crates/langchain-fireworks/src/chat.rs`
- [ ] T207 Implement `FireworksEmbeddings` with `Embeddings` trait in `crates/langchain-fireworks/src/embeddings.rs`

### Groq (OpenAI-compatible)

- [ ] T208 [P] Create `crates/langchain-groq/Cargo.toml` with langchain-core, langchain-openai dependencies
- [ ] T209 Implement `ChatGroq` wrapping `BaseChatOpenAI` (+ reasoning_format, service_tier) in `crates/langchain-groq/src/chat.rs`
- [ ] T210 [P] Add integration test `tests/integration/groq_chat.rs` (feature-gated)

### Nomic (embeddings-only)

- [ ] T211 [P] Create `crates/langchain-nomic/Cargo.toml` with langchain-core, reqwest dependencies
- [ ] T212 Implement `NomicEmbeddings` with `Embeddings` trait in `crates/langchain-nomic/src/embeddings.rs`

### Exa (retriever + tools)

- [ ] T213 [P] Create `crates/langchain-exa/Cargo.toml` with langchain-core, reqwest dependencies
- [ ] T214 Implement `ExaSearchRetriever` with `Retriever` trait in `crates/langchain-exa/src/retriever.rs`
- [ ] T215 Implement `ExaSearchResults` and `ExaFindSimilarResults` with `Tool` trait in `crates/langchain-exa/src/tools.rs`

### DeepSeek (OpenAI-compatible)

- [ ] T216 [P] Create `crates/langchain-deepseek/Cargo.toml` with langchain-core, langchain-openai dependencies
- [ ] T217 Implement `ChatDeepSeek` wrapping `BaseChatOpenAI` in `crates/langchain-deepseek/src/chat.rs`

### xAI (OpenAI-compatible)

- [ ] T218 [P] Create `crates/langchain-xai/Cargo.toml` with langchain-core, langchain-openai dependencies
- [ ] T219 Implement `ChatXAI` wrapping `BaseChatOpenAI` in `crates/langchain-xai/src/chat.rs`

### OpenRouter (OpenAI-compatible)

- [ ] T220 [P] Create `crates/langchain-openrouter/Cargo.toml` with langchain-core, langchain-openai dependencies
- [ ] T221 Implement `ChatOpenRouter` wrapping `BaseChatOpenAI` in `crates/langchain-openrouter/src/chat.rs`

### Perplexity (OpenAI-partial + search)

- [ ] T222 [P] Create `crates/langchain-perplexity/Cargo.toml` with langchain-core, langchain-openai dependencies
- [ ] T223 Implement `ChatPerplexity` wrapping `BaseChatOpenAI` (+ search_mode, search params) in `crates/langchain-perplexity/src/chat.rs`
- [ ] T224 Implement `PerplexitySearchRetriever` with `Retriever` trait in `crates/langchain-perplexity/src/retriever.rs`

### Workspace Integration

- [ ] T225 Add all 15 new provider crates to workspace `Cargo.toml` members list
- [ ] T226 Update `crates/langchain/src/lib.rs` to optionally re-export provider types
- [ ] T227 Add provider examples: `examples/anthropic_chat.rs`, `examples/ollama_chat.rs`

## Dependencies and Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion — BLOCKS all user stories
- **User Stories (Phase 3–7)**: All depend on Foundational phase completion
  - US1 (Phase 3): Can start immediately after Phase 2
  - US2 (Phase 4): Depends on US1 (needs BaseChatModel trait for chain testing)
  - US3 (Phase 5): Depends on US1 (extends BaseChatModel with streaming)
  - US4 (Phase 6): Independent of US1–US3 (separate traits), can run in parallel after Phase 2
  - US5 (Phase 7): Independent of US1–US4, can run in parallel after Phase 2
- **Scope Expansion (Phase 9)**: Depends on Phase 2 + Phase 7 (US5); agent tasks depend on Tool trait and CallbackHandler
  - Agent Framework (T090–T096): Needs Tool trait (T080) and CallbackHandler (T081)
  - VectorStore MMR (T097–T099): Needs VectorStore trait (Phase 6) and Embeddings (Phase 6)
  - Runnable Composition (T100–T104): Needs Runnable trait (Phase 2)
  - Event Streaming (T105–T110): Needs Runnable trait (Phase 2) and CallbackHandler (T081)
  - Integration (T111–T112): Depends on all Phase 9 sub-groups
- **Expanded Core (Phase 10)**: Depends on Phase 2 + Phase 4 (US2)
  - Message Utilities (T116–T123): Needs Message enum (Phase 2)
  - Concrete Runnables (T124–T127): Needs Runnable trait (Phase 4)
  - Output Parsers (T128–T132): Needs OutputParser trait (Phase 4)
  - StructuredTool (T133–T134): Needs Tool trait (Phase 7)
  - Test Utilities (T135–T137): Needs BaseChatModel (Phase 3) and Embeddings (Phase 6)
  - Retriever Adapter (T138–T139): Needs Retriever trait (Phase 6)
  - Custom Events (T140–T141): Needs CallbackHandler (Phase 7) and stream_events (Phase 9)
  - Integration (T142–T143): Depends on all Phase 10 sub-groups
- **Reference Implementations (Phase 11)**: Depends on Phase 2, Phase 4, Phase 6, Phase 7
  - Embedding Cache (T145–T147): Needs Embeddings trait (Phase 6)
  - Chat History (T148–T151): Needs Runnable trait (Phase 4)
  - Few-Shot Prompts (T152–T155): Needs VectorStore (Phase 6) and PromptTemplate (Phase 4)
  - Text Splitters (T156–T158): Needs Document type (Phase 2)
  - Additional Parsers (T159–T165): Needs OutputParser trait (Phase 4) and BaseChatModel (Phase 3)
  - OpenAI Moderation (T166–T167): Needs RunnableLambda (Phase 10) and reqwest (Phase 3)
  - Integration (T168–T169): Depends on all Phase 11 sub-groups
- **Provider Crates (Phase 12)**: Depends on Phase 3 (US1 — BaseChatModel) + Phase 6 (US4 — VectorStore/Embeddings)
  - OpenAI Base Refactor (T170–T172): Needs ChatOpenAI from Phase 3
  - Anthropic (T173–T179): Needs BaseChatModel (Phase 3); independent of OpenAI refactor
  - Ollama (T180–T186): Needs BaseChatModel + BaseLLM + Embeddings (Phase 3+6)
  - HuggingFace (T187–T192): Same as Ollama
  - Vector Stores (T193–T200): Needs VectorStore + Embeddings (Phase 6)
  - OpenAI-compatible wrappers (T201–T224): Need BaseChatOpenAI (T170–T172)
  - Workspace Integration (T225–T227): Depends on all Phase 12 sub-groups
- **Polish (Phase 8)**: Depends on all user stories, Phase 9, Phase 10, Phase 11, and Phase 12 being complete

### User Story Dependencies

- **US1 (P1)**: No dependencies on other stories. MVP.
- **US2 (P2)**: Needs BaseChatModel from US1 for chain testing (can start tests early, needs trait defined)
- **US3 (P3)**: Needs BaseChatModel from US1 (extends with stream() implementation)
- **US4 (P4)**: Independent — different traits (Embeddings, VectorStore). Can run in parallel with US1–US3
- **US5 (P5)**: Independent — Tool trait and CallbackHandler. Can run in parallel with US1–US4

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Types/structs before traits
- Traits before implementations
- Core crate before provider crate
- Story complete before moving to next priority (unless parallelising)

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- US4 and US5 can start in parallel after Phase 2 (independent traits)
- All tests within a story marked [P] can run in parallel
- All Polish tasks marked [P] can run in parallel
- Phase 12 providers: Anthropic, Ollama, HuggingFace, Chroma, Qdrant all independent of each other
- Phase 12 OpenAI-compatible wrappers (Groq, Fireworks, DeepSeek, xAI, OpenRouter, MistralAI, Perplexity) all independent after BaseChatOpenAI refactor (T170–T172)

---

## Parallel Example: User Story 1

```bash
# Launch all tests for US1 together:
Task: "Unit test: mock BaseChatModel invoke returns ChatResult"
Task: "Unit test: mock BaseChatModel invoke with error returns Result::Err"
Task: "Unit test: swapping mock providers compiles and produces correct output"
Task: "Unit test: BaseChatModel batch invokes multiple inputs concurrently"
Task: "Unit test: invoking with empty message list returns validation error"

# Then implement sequentially:
Task: "Define BaseChatModel trait"
Task: "Define BaseLLM trait"
Task: "Implement ChatOpenAI struct"
Task: "Implement BaseChatModel for ChatOpenAI"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL — blocks all stories)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Mock invoke + ChatOpenAI invoke works
5. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add US1 → Test independently → Deliverable (MVP)
3. Add US2 → Test independently → Prompt templates and chains work
4. Add US3 → Test independently → Streaming works
5. Add US4 → Test independently → RAG pipeline works
6. Add US5 → Test independently → Tool use works
7. Phase 9 → Agents, MMR, retry/fallbacks, event streaming
8. Phase 10 → Message utilities, concrete types, parsers, test utilities
9. Phase 11 → Reference implementations (caching, history, few-shot, text splitters, extra parsers)
10. Phase 12 → All 16 provider crates (OpenAI base refactor, Anthropic, Ollama, HuggingFace, Chroma, Qdrant, MistralAI, Fireworks, Groq, Nomic, Exa, DeepSeek, xAI, OpenRouter, Perplexity)
11. Polish → Docs, lint, final validation

### Parallel Team Strategy

With multiple developers after Foundational is done:

- Developer A: US1 → US2 → US3 (sequential — dependencies)
- Developer B: US4 (independent)
- Developer C: US5 (independent)

After Phase 3 + Phase 6 are complete, provider work can parallelise heavily:

- Developer D: Anthropic (T173–T179) — native API, independent
- Developer E: Ollama (T180–T186) — native API, independent
- Developer F: HuggingFace (T187–T192) — native API, independent
- Developer G: Chroma + Qdrant (T193–T200) — vector stores, independent
- Developer H: OpenAI base refactor (T170–T172), then all OpenAI-compatible wrappers (T201–T224) — sequential

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
