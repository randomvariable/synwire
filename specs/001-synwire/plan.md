# Implementation Plan: Synwire — M1 Core + Orchestrator

**Branch**: `001-synwire` | **Date**: 2026-03-09 | **Spec**: [spec.md](spec.md)

> M1 scope only. M2 (Agents + MCP) and M3 (Protocols + DSPy + Evals) deferred
> to [roadmap](../../docs/roadmap.md). Architecture review fixes applied throughout.

## Summary

Port LangChain Python and LangGraph Python core to idiomatic Rust as a unified
Cargo workspace. M1 delivers:

- **synwire-core**: Traits for chat models, embeddings, vector stores, prompts,
  runnables (split: `RunnableCore` + `ObservableRunnable`), tools, callbacks
  (`Arc<dyn CallbackHandler>`), output parsers, retrievers. Layered error types
  (`ModelError`, `GraphError`, `ToolError`, etc.) with `#[non_exhaustive]`.
- **synwire-orchestrator**: Graph-based orchestration — `StateGraph<S>` generic
  over state type, Pregel execution, channels, checkpointing, interrupts,
  streaming with lossless/lossy semantics.
- **synwire-checkpoint-sqlite**: SQLite checkpoint persistence.
- **synwire-llm-openai**: OpenAI provider (ChatOpenAI, OpenAIEmbeddings).
- **synwire-llm-ollama**: Ollama provider (ChatOllama, OllamaEmbeddings).
- **synwire**: Convenience re-exports + reference implementations.
- **synwire-derive**: Proc macros (`#[tool]`, `#[derive(State)]`).

All I/O operations are async-first with tokio. Core crates compile with zero `unsafe`.

## Technical Context

**Language/Version**: Rust (stable, edition 2024)
**Primary Dependencies**: tokio, serde, serde_json, reqwest (rustls), thiserror,
futures, backoff, json-patch, uuid, chrono, secrecy (for `SecretValue` zeroisation);
optional: tracing, tracing-opentelemetry, opentelemetry; checkpoint: rusqlite;
derive: syn, quote, proc-macro2, schemars
**Testing**: nextest (primary runner), proptest (property-based), mockall, tokio::test,
cargo-llvm-cov; FakeChatModel + FakeEmbeddings for testing without API calls
**Constraints**: Zero `unsafe` in synwire-core and synwire-orchestrator (`#![forbid(unsafe_code)]`);
all public types `Send + Sync`; no panics in library code

## Constitution Check (v2.0.0)

| Principle | Status | Evidence |
|-----------|--------|----------|
| I. Trait-Based Abstractions | PASS | All core traits in synwire-core; providers separate |
| II. Safety and Correctness (NON-NEGOTIABLE) | PASS | `Result<T,E>` everywhere; zero unsafe; secrecy for secrets |
| III. Async-First with Sync Wrappers | PASS | tokio runtime; `BoxFuture` for dyn-compat |
| IV. BDD Test-First (NON-NEGOTIABLE) | PASS | nextest, proptest, conformance suites; red-green-refactor |
| V. Always Be Linting (NON-NEGOTIABLE) | PASS | `cargo clippy -- -D warnings`; rustfmt enforced; CI gates |
| VI. Diataxis Documentation | PASS | mdbook site; tutorials, how-to, reference, explanation |

## Project Structure (M1)

```text
Cargo.toml                        # Workspace root
.config/
└── nextest.toml                  # default + ci profiles
.github/
└── workflows/
    ├── ci.yml                    # PR: fmt, clippy, unit+property tests
    └── ci-full.yml               # merge: full suite + coverage + docs
crates/
├── synwire-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # Re-exports, prelude
│       ├── error/
│       │   ├── mod.rs            # SynwireError (top-level, #[non_exhaustive])
│       │   ├── model.rs          # ModelError
│       │   ├── graph.rs          # re-export from orchestrator
│       │   ├── tool.rs           # ToolError
│       │   ├── parse.rs          # ParseError
│       │   ├── embedding.rs      # EmbeddingError
│       │   ├── vectorstore.rs    # VectorStoreError
│       │   └── kind.rs           # SynwireErrorKind discriminant
│       ├── messages/
│       │   ├── mod.rs
│       │   ├── types.rs          # Message enum, ContentBlock, UsageMetadata
│       │   ├── traits.rs         # MessageLike trait
│       │   ├── filter.rs         # MessageFilter (builder pattern)
│       │   └── utils.rs          # trim_messages, merge_message_runs
│       ├── prompts/
│       │   ├── mod.rs
│       │   ├── template.rs       # PromptTemplate
│       │   └── chat.rs           # ChatPromptTemplate
│       ├── language_models/
│       │   ├── mod.rs
│       │   ├── traits.rs         # BaseLLM, BaseChatModel
│       │   ├── types.rs          # ChatResult, Generation, LLMResult, CostEstimate
│       │   └── fake.rs           # FakeChatModel (test utility)
│       ├── embeddings/
│       │   ├── mod.rs
│       │   ├── traits.rs         # Embeddings trait
│       │   └── fake.rs           # FakeEmbeddings (test utility)
│       ├── vectorstores/
│       │   ├── mod.rs
│       │   ├── traits.rs         # VectorStore trait
│       │   ├── in_memory.rs      # InMemoryVectorStore
│       │   ├── mmr.rs            # MMR algorithm utility
│       │   └── filter.rs         # MetadataFilter
│       ├── documents/
│       │   └── types.rs          # Document
│       ├── runnables/
│       │   ├── mod.rs
│       │   ├── core.rs           # RunnableCore trait (invoke/batch/stream)
│       │   ├── observable.rs     # ObservableRunnable trait (stream_events/transform)
│       │   ├── chain.rs          # RunnableSequence, RunnableParallel
│       │   ├── passthrough.rs    # RunnablePassthrough
│       │   ├── lambda.rs         # RunnableLambda
│       │   ├── branch.rs         # RunnableBranch
│       │   ├── retry.rs          # RunnableRetry, RetryConfig
│       │   ├── fallbacks.rs      # RunnableWithFallbacks
│       │   ├── events.rs         # StreamEvent, EventData, dispatch_custom_event
│       │   └── as_tool.rs        # RunnableTool
│       ├── output_parsers/
│       │   ├── mod.rs
│       │   ├── traits.rs         # OutputParser<T> trait
│       │   ├── string.rs         # StrOutputParser
│       │   ├── json.rs           # JsonOutputParser
│       │   ├── structured.rs     # StructuredOutputParser<T>
│       │   └── tools.rs          # ToolsOutputParser
│       ├── tools/
│       │   ├── mod.rs
│       │   ├── traits.rs         # Tool trait
│       │   ├── types.rs          # ToolCall, ToolSchema, ToolOutput, ToolResult
│       │   └── structured.rs     # StructuredTool, StructuredToolBuilder
│       ├── callbacks/
│       │   └── traits.rs         # CallbackHandler trait (Arc-compatible)
│       ├── retrievers/
│       │   ├── mod.rs
│       │   ├── traits.rs         # Retriever trait
│       │   └── runnable.rs       # RetrieverRunnable adapter
│       ├── credentials/
│       │   ├── mod.rs
│       │   ├── traits.rs         # CredentialProvider trait
│       │   ├── secret.rs         # SecretValue (backed by secrecy crate)
│       │   ├── env.rs            # EnvCredentialProvider
│       │   └── static_creds.rs   # StaticCredentialProvider
│       ├── security/
│       │   ├── mod.rs
│       │   ├── ssrf.rs           # SsrfProtectedClient (DNS pinning)
│       │   └── http_factory.rs   # HttpClientFactory trait
│       ├── loaders/
│       │   └── traits.rs         # DocumentLoader trait
│       ├── rerankers/
│       │   └── traits.rs         # Reranker trait
│       ├── agents/
│       │   └── types.rs          # AgentAction, AgentFinish, AgentStep, AgentDecision (minimal)
│       └── prelude.rs            # Convenience re-exports
├── synwire-orchestrator/
│   ├── Cargo.toml                # deps: synwire-core, tokio, serde, uuid, futures
│   └── src/
│       ├── lib.rs
│       ├── error.rs              # SynwireGraphError (#[non_exhaustive])
│       ├── constants.rs          # START, END
│       ├── graph/
│       │   ├── state.rs          # StateGraph<S>, State trait, MessagesState
│       │   └── compiled.rs       # CompiledGraph<S> (generic, RunnableCore impl)
│       ├── channels/
│       │   ├── traits.rs         # BaseChannel trait
│       │   ├── last_value.rs     # LastValue
│       │   ├── topic.rs          # Topic
│       │   ├── binary_operator.rs # BinaryOperatorAggregate
│       │   ├── any_value.rs      # AnyValue
│       │   ├── ephemeral.rs      # EphemeralValue
│       │   └── barrier.rs        # NamedBarrierValue
│       ├── pregel/
│       │   ├── engine.rs         # Pregel execution engine
│       │   └── types.rs          # PregelTask
│       ├── types/
│       │   ├── send.rs           # Send
│       │   ├── command.rs        # Command
│       │   ├── interrupt.rs      # Interrupt, interrupt()
│       │   ├── overwrite.rs      # Overwrite
│       │   ├── snapshot.rs       # StateSnapshot<S>
│       │   ├── stream_mode.rs    # StreamMode (lossless/lossy annotations)
│       │   ├── node_state.rs     # NodeState, NodeErrorStrategy
│       │   └── typed_value.rs    # TypedValue
│       ├── config/
│       │   ├── retry_policy.rs   # RetryPolicy (per-node)
│       │   ├── cache_policy.rs   # CachePolicy
│       │   └── runtime.rs        # Runtime context
│       ├── managed/
│       │   └── values.rs         # IsLastStep, RemainingSteps
│       ├── registry/
│       │   └── node_registry.rs  # NodeRegistry
│       ├── metrics/
│       │   ├── execution.rs      # GraphExecutionMetrics
│       │   ├── node.rs           # NodeMetrics
│       │   └── quota.rs          # QuotaEnforcer trait
│       ├── func/
│       │   ├── task.rs           # TaskFunction
│       │   └── entrypoint.rs     # Entrypoint, EntrypointFinal
│       ├── messages/
│       │   └── reducers.rs       # add_messages, RemoveMessage
│       └── prebuilt/
│           ├── react_agent.rs    # create_react_agent
│           ├── tool_node.rs      # ToolNode
│           └── nodes.rs          # IfElse, Loop, HttpRequest
├── synwire-checkpoint/
│   ├── Cargo.toml
│   └── src/
│       ├── base.rs               # BaseCheckpointSaver trait
│       ├── types.rs              # Checkpoint, CheckpointMetadata, CheckpointTuple
│       ├── serde/
│       │   ├── protocol.rs       # SerializerProtocol trait
│       │   └── json_plus.rs      # JsonPlusSerializer
│       ├── store/
│       │   ├── base.rs           # BaseStore trait
│       │   ├── types.rs          # Item, SearchItem, ops
│       │   └── in_memory.rs      # InMemoryStore
│       ├── cache/
│       │   └── base.rs           # BaseCache trait
│       └── memory.rs             # InMemoryCheckpointSaver
├── synwire-checkpoint-sqlite/
│   ├── Cargo.toml                # deps: synwire-checkpoint, rusqlite, r2d2
│   └── src/
│       ├── saver.rs              # SqliteSaver (mode 0600 for files)
│       └── schema.rs             # DDL
├── synwire-llm-openai/
│   ├── Cargo.toml
│   └── src/
│       ├── base.rs               # BaseChatOpenAI
│       ├── chat.rs               # ChatOpenAI
│       ├── embeddings.rs         # OpenAIEmbeddings
│       └── error.rs              # OpenAI-specific errors → ModelError
├── synwire-llm-ollama/
│   ├── Cargo.toml
│   └── src/
│       ├── chat.rs               # ChatOllama
│       ├── embeddings.rs         # OllamaEmbeddings
│       └── error.rs
├── synwire/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # Re-exports from core + reference impls
│       ├── chat_history/
│       │   ├── traits.rs         # ChatMessageHistory trait
│       │   ├── in_memory.rs      # InMemoryChatMessageHistory
│       │   └── runnable.rs       # RunnableWithMessageHistory
│       ├── output_parsers/       # Reference parsers (XML, CSV, regex, etc.)
│       ├── prompts/
│       │   ├── few_shot.rs       # FewShotPromptTemplate
│       │   └── example_selector.rs
│       └── text_splitters/
│           ├── character.rs
│           └── recursive.rs
├── synwire-derive/
│   ├── Cargo.toml                # deps: syn, quote, proc-macro2, schemars
│   └── src/
│       ├── lib.rs
│       ├── tool.rs               # #[tool] macro (uses schemars for schema gen)
│       └── state.rs              # #[derive(State)] macro
├── synwire-test-utils/
│   ├── Cargo.toml                # dev-dependency crate
│   └── src/
│       ├── fake_model.rs         # FakeChatModel, FakeEmbeddings (re-exports)
│       ├── proptest_strategies.rs # Arbitrary impls for Message, Document, etc.
│       └── collectors.rs         # TestSpanCollector, FakeEventBus
└── synwire-checkpoint-conformance/
    ├── Cargo.toml
    └── src/
        └── lib.rs                # run_conformance_tests()
```

## Architecture Review Fixes Applied

| Finding | Fix | Location |
|---------|-----|----------|
| §1.2 Value as state | `StateGraph<S>`, `CompiledGraph<S>` generic over `S: State` | synwire-orchestrator |
| §1.3 RunnableConfig not Clone | `Arc<dyn CallbackHandler>`, `RunnableConfig: Clone` | synwire-core |
| §2.1 Unbounded error enum | Layered: `ModelError`, `GraphError`, `ToolError`, `#[non_exhaustive]` | all crates |
| §2.2 Runnable too wide | Split: `RunnableCore` + `ObservableRunnable`; drop `stream_log` | synwire-core |
| §2.5 SecretValue not zeroised | `secrecy` crate backing | synwire-core |
| §2.6 DNS rebinding | DNS pinning in `SsrfProtectedClient` | synwire-core |
| §2.9 Compilation time | `synwire-derive` optional for core; `test-utils` feature-gated | workspace |
| §2.10 Stream backpressure | Lossless/lossy per `StreamMode`; `DroppedEvents` marker | synwire-orchestrator |
| §3.2 Core too large | No Agent<D,O>, extraction, guardrails in core (M2) | synwire-core |
| §3.4 BoxFuture everywhere | RPITIT candidates identified for static-dispatch traits | synwire-core |
| §3.7 Sensitive data default | `trace_include_sensitive_data: false` default | synwire-core |
| §4.1 filter_messages ergonomics | `MessageFilter` builder pattern | synwire-core |
| §4.7 Schema generation | `schemars` for `#[tool]` macro | synwire-derive |
| §4.8 Zero unsafe scope | `#![forbid(unsafe_code)]` on core + orchestrator only | synwire-core, orchestrator |
| §5.5 Non-exhaustive | `#[non_exhaustive]` on all enums and config structs | all crates |

## Implementation Order

1. **synwire-core error types** — Foundation for everything else
2. **synwire-core messages** — Message, ContentBlock, MessageLike, utilities
3. **synwire-core prompts** — PromptTemplate, ChatPromptTemplate
4. **synwire-core runnables** — RunnableCore, ObservableRunnable, composition
5. **synwire-core tools** — Tool trait, StructuredTool
6. **synwire-core callbacks** — CallbackHandler trait
7. **synwire-core language_models** — BaseChatModel, BaseLLM, FakeChatModel
8. **synwire-core embeddings/vectorstores** — Traits + InMemoryVectorStore
9. **synwire-core output_parsers** — String, JSON, Structured, Tools
10. **synwire-core credentials/security** — SecretValue, SSRF, HttpClientFactory
11. **synwire-derive** — `#[tool]`, `#[derive(State)]`
12. **synwire-orchestrator channels** — All 6 channel types
13. **synwire-orchestrator graph** — StateGraph`<S>`, CompiledGraph`<S>`
14. **synwire-orchestrator pregel** — Execution engine
15. **synwire-orchestrator prebuilt** — create_react_agent, ToolNode, nodes
16. **synwire-checkpoint** — Traits + InMemoryCheckpointSaver
17. **synwire-checkpoint-sqlite** — SQLite backend
18. **synwire-llm-openai** — ChatOpenAI + OpenAIEmbeddings
19. **synwire-llm-ollama** — ChatOllama + OllamaEmbeddings
20. **synwire** — Convenience crate + reference impls
21. **synwire-test-utils** — Proptest strategies, test collectors
22. **synwire-checkpoint-conformance** — Conformance test suite
23. **CI/CD** — GitHub Actions, nextest, Tilt E2E
24. **Documentation** — mdbook site, doc-tests, examples

## Testing Strategy

- **Unit + property tests**: nextest with proptest (256 cases/property)
- **Conformance suites**: Checkpoint, provider contract testing
- **E2E**: Tilt + Ollama (small model) for integration
- **CI tiers**: T1 (unit+property on PR), T2 (integration on merge), T3 (E2E on merge)
- **Coverage**: cargo-llvm-cov, 90% target for core, 80% for orchestrator
- **Unsafe audit**: cargo-geiger in CI
