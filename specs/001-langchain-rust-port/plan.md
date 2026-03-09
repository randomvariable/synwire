# Implementation Plan: LangChain Rust Port

**Branch**: `001-langchain-rust-port` | **Date**: 2026-03-09 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-langchain-rust-port/spec.md`

## Summary

Port the core LangChain Python abstractions to idiomatic Rust as a Cargo
workspace. The `langchain-core` crate defines traits for chat models,
embeddings, vector stores, prompts, runnables, tools, callbacks, output
parsers, and retrievers. 16 provider crates cover all Python LangChain partners. All I/O operations are async-first
with tokio, all fallible operations return `Result`, and the entire core
crate compiles with zero `unsafe`.

## Technical Context

**Language/Version**: Rust (stable, edition 2024)
**Primary Dependencies**: tokio, serde, serde_json, reqwest (rustls), thiserror, futures, backoff, json-patch; optional: tracing, tracing-opentelemetry, opentelemetry
**Storage**: N/A (vector store trait is abstract; in-memory impl for testing only)
**Testing**: cargo test, mockall, tokio::test, cargo-llvm-cov for coverage; FakeChatModel + FakeEmbeddings for chain testing without API calls
**Target Platform**: Cross-platform (Linux, macOS, Windows); `no_std` is not a goal
**Project Type**: Library (Cargo workspace with multiple crates)
**Performance Goals**: Streaming latency overhead < 1ms per chunk above provider latency; batch operations parallelise across available connections
**Constraints**: Zero `unsafe` in langchain-core; all public types Send + Sync; no panics in library code
**Scale/Scope**: ~15 core traits, ~65 core types, 16 provider integrations, ~25k-35k lines for full port

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
|-----------|--------|----------|
| I. Trait-Based Abstractions | PASS | All FR-001 through FR-011 define traits in langchain-core; providers in separate crates |
| II. API Conceptual Parity | PASS | Module list maps 1:1 to Python langchain_core; Runnable supports invoke/batch/stream |
| III. Safety and Correctness | PASS | FR-012 mandates Result<T,E> everywhere; zero unsafe constraint; thiserror for errors |
| IV. Async-First | PASS | FR-013 mandates async; streaming via futures::Stream; tokio as runtime |
| V. Comprehensive Testing | PASS | SC-002 requires 90% coverage; integration tests feature-gated; mockall for mocking |

All gates pass. No violations to justify.

## Project Structure

### Documentation (this feature)

```text
specs/001-langchain-rust-port/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (public trait API contracts)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
Cargo.toml                        # Workspace root
.github/
└── workflows/
    ├── ci.yml                    # PR/push: fmt, clippy, test, doc
    └── coverage.yml              # Main branch: cargo-llvm-cov coverage
crates/
├── langchain-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # Re-exports, prelude
│       ├── error.rs              # LangChainError enum
│       ├── messages/
│       │   ├── mod.rs
│       │   ├── types.rs          # HumanMessage, AIMessage, SystemMessage, ToolMessage, Chat
│       │   ├── traits.rs         # MessageLike trait
│       │   └── utils.rs          # filter_messages, trim_messages, merge_message_runs
│       ├── prompts/
│       │   ├── mod.rs
│       │   ├── template.rs       # PromptTemplate
│       │   ├── chat.rs           # ChatPromptTemplate
│       │   └── traits.rs         # BasePromptTemplate trait
│       ├── language_models/
│       │   ├── mod.rs
│       │   ├── traits.rs         # BaseLLM, BaseChatModel traits
│       │   ├── types.rs          # ChatResult, Generation, LLMResult
│       │   └── fake.rs           # FakeChatModel (test utility)
│       ├── embeddings/
│       │   ├── mod.rs
│       │   ├── traits.rs         # Embeddings trait
│       │   └── fake.rs           # FakeEmbeddings (test utility)
│       ├── vectorstores/
│       │   ├── mod.rs
│       │   ├── traits.rs         # VectorStore trait
│       │   ├── in_memory.rs      # InMemoryVectorStore (for testing)
│       │   └── mmr.rs            # MMR algorithm (cosine sim + diversity scoring)
│       ├── documents/
│       │   ├── mod.rs
│       │   └── types.rs          # Document type
│       ├── runnables/
│       │   ├── mod.rs
│       │   ├── traits.rs         # Runnable trait (invoke/batch/stream/transform/batch_as_completed/stream_events/stream_log)
│       │   ├── chain.rs          # RunnableSequence, RunnableParallel
│       │   ├── passthrough.rs    # RunnablePassthrough
│       │   ├── lambda.rs         # RunnableLambda (closure wrapper)
│       │   ├── branch.rs         # RunnableBranch (conditional routing)
│       │   ├── retry.rs          # RunnableRetry, RetryConfig (with_retry)
│       │   ├── fallbacks.rs      # RunnableWithFallbacks (with_fallbacks)
│       │   ├── events.rs         # StreamEvent, EventData, RunLogPatch, JsonPatchOp, dispatch_custom_event
│       │   └── as_tool.rs        # RunnableTool (as_tool composition)
│       ├── output_parsers/
│       │   ├── mod.rs
│       │   ├── traits.rs         # BaseOutputParser trait
│       │   ├── string.rs         # StrOutputParser
│       │   ├── json.rs           # JsonOutputParser
│       │   ├── structured.rs     # StructuredOutputParser<T>
│       │   └── tools.rs          # ToolsOutputParser
│       ├── tools/
│       │   ├── mod.rs
│       │   ├── traits.rs         # Tool trait
│       │   ├── types.rs          # ToolCall, ToolResult, ToolOutput
│       │   └── structured.rs     # StructuredTool, StructuredToolBuilder
│       ├── callbacks/
│       │   ├── mod.rs
│       │   └── traits.rs         # CallbackHandler trait
│       ├── retrievers/
│       │   ├── mod.rs
│       │   ├── traits.rs         # BaseRetriever trait
│       │   └── runnable.rs       # RetrieverRunnable adapter (Runnable impl for Retriever)
│       ├── agents/
│       │   ├── mod.rs
│       │   ├── types.rs          # AgentAction, AgentFinish, AgentStep, AgentDecision, AgentInput
│       │   └── executor.rs       # AgentExecutor (ReAct loop)
│       └── prelude.rs            # Convenience re-exports
├── langchain-openai/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── base.rs               # BaseChatOpenAI (shared OpenAI-compatible base)
│       ├── chat.rs               # ChatOpenAI (implements BaseChatModel)
│       ├── embeddings.rs         # OpenAIEmbeddings (implements Embeddings)
│       ├── moderation.rs         # OpenAIModerationMiddleware (RunnableLambda wrapper)
│       └── error.rs              # OpenAI-specific errors
├── langchain-anthropic/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── chat.rs               # ChatAnthropic (native Anthropic API)
│       └── error.rs              # Anthropic-specific errors
├── langchain-ollama/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── chat.rs               # ChatOllama (Ollama native API)
│       ├── llm.rs                # OllamaLLM
│       ├── embeddings.rs         # OllamaEmbeddings
│       └── error.rs
├── langchain-huggingface/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── chat.rs               # ChatHuggingFace (HF Inference API)
│       ├── embeddings.rs         # HuggingFaceEmbeddings
│       ├── pipeline.rs           # HuggingFacePipeline (API-only initially)
│       └── error.rs
├── langchain-chroma/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── vectorstore.rs        # Chroma (implements VectorStore)
│       ├── client.rs             # ChromaClient (REST client)
│       └── error.rs
├── langchain-qdrant/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── vectorstore.rs        # QdrantVectorStore (implements VectorStore)
│       ├── client.rs             # QdrantClient (REST/gRPC)
│       └── error.rs
├── langchain-mistralai/
│   ├── Cargo.toml                # deps: langchain-core, langchain-openai (BaseChatOpenAI)
│   └── src/
│       ├── lib.rs
│       ├── chat.rs               # ChatMistralAI (extends BaseChatOpenAI)
│       ├── embeddings.rs         # MistralAIEmbeddings
│       └── error.rs
├── langchain-fireworks/
│   ├── Cargo.toml                # deps: langchain-core, langchain-openai (BaseChatOpenAI)
│   └── src/
│       ├── lib.rs
│       ├── chat.rs               # ChatFireworks (extends BaseChatOpenAI)
│       ├── embeddings.rs         # FireworksEmbeddings
│       └── error.rs
├── langchain-groq/
│   ├── Cargo.toml                # deps: langchain-core, langchain-openai (BaseChatOpenAI)
│   └── src/
│       ├── lib.rs
│       ├── chat.rs               # ChatGroq (extends BaseChatOpenAI)
│       └── error.rs
├── langchain-nomic/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── embeddings.rs         # NomicEmbeddings (implements Embeddings)
├── langchain-exa/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── retriever.rs          # ExaSearchRetriever (implements Retriever)
│       ├── tools.rs              # ExaSearchResults, ExaFindSimilar (implement Tool)
│       └── error.rs
├── langchain-deepseek/
│   ├── Cargo.toml                # deps: langchain-core, langchain-openai (BaseChatOpenAI)
│   └── src/
│       ├── lib.rs
│       └── chat.rs               # ChatDeepSeek (extends BaseChatOpenAI)
├── langchain-xai/
│   ├── Cargo.toml                # deps: langchain-core, langchain-openai (BaseChatOpenAI)
│   └── src/
│       ├── lib.rs
│       └── chat.rs               # ChatXAI (extends BaseChatOpenAI)
├── langchain-openrouter/
│   ├── Cargo.toml                # deps: langchain-core, langchain-openai (BaseChatOpenAI)
│   └── src/
│       ├── lib.rs
│       └── chat.rs               # ChatOpenRouter (extends BaseChatOpenAI)
├── langchain-perplexity/
│   ├── Cargo.toml                # deps: langchain-core, langchain-openai (BaseChatOpenAI)
│   └── src/
│       ├── lib.rs
│       ├── chat.rs               # ChatPerplexity (extends BaseChatOpenAI + search params)
│       ├── retriever.rs          # PerplexitySearchRetriever
│       └── error.rs
└── langchain/
    ├── Cargo.toml                # deps: langchain-core, moka (cache), regex (parsers)
    └── src/
        ├── lib.rs                # Re-exports from core + reference implementation modules
        ├── cache/
        │   ├── mod.rs
        │   └── embeddings.rs     # CacheBackedEmbeddings (wraps Embeddings + moka cache)
        ├── chat_history/
        │   ├── mod.rs
        │   ├── traits.rs         # ChatMessageHistory trait (get/add/clear messages)
        │   ├── in_memory.rs      # InMemoryChatMessageHistory
        │   └── runnable.rs       # RunnableWithMessageHistory (wraps Runnable + history store)
        ├── output_parsers/
        │   ├── mod.rs
        │   ├── list.rs           # CommaSeparatedListOutputParser
        │   ├── enum_parser.rs    # EnumOutputParser
        │   ├── xml.rs            # XMLOutputParser
        │   ├── regex.rs          # RegexParser
        │   ├── retry.rs          # RetryOutputParser (wraps parser + LLM for retry on failure)
        │   └── combining.rs      # CombiningOutputParser (merges multiple parsers)
        ├── prompts/
        │   ├── mod.rs
        │   ├── few_shot.rs       # FewShotPromptTemplate, FewShotChatMessagePromptTemplate
        │   └── example_selector.rs # SemanticSimilarityExampleSelector (uses VectorStore)
        └── text_splitters/
            ├── mod.rs
            ├── character.rs      # CharacterTextSplitter
            └── recursive.rs      # RecursiveCharacterTextSplitter

examples/
├── simple_chat.rs                # Basic model invocation
├── prompt_chain.rs               # Prompt template → model chain
├── streaming.rs                  # Streaming response handling
├── rag.rs                        # Embed + vector store + retrieval
└── simple_agent.rs               # Agent with tools (ReAct loop)

tests/
└── integration/
    ├── openai_chat.rs            # Integration test (feature-gated)
    ├── openai_embeddings.rs      # Integration test (feature-gated)
    ├── anthropic_chat.rs         # Anthropic integration test
    ├── ollama_chat.rs            # Ollama integration test (requires local server)
    ├── groq_chat.rs              # Groq integration test
    ├── fireworks_chat.rs         # Fireworks integration test
    ├── mistralai_chat.rs         # MistralAI integration test
    ├── deepseek_chat.rs          # DeepSeek integration test
    ├── chroma_vectorstore.rs     # Chroma integration test (requires local server)
    └── qdrant_vectorstore.rs     # Qdrant integration test (requires local server)
```

**Structure Decision**: Cargo workspace with 18 member crates matching
the Python monorepo's core/partners/langchain layering. The `langchain`
crate re-exports `langchain-core` and provides reference implementations
for common application-level patterns. 16 provider crates cover all
Python partners — OpenAI-compatible providers (Groq, Fireworks, DeepSeek,
xAI, OpenRouter) depend on `langchain-openai` for the shared
`BaseChatOpenAI` base type, keeping per-provider code minimal (~100-200
lines each). Integration tests live in a top-level `tests/` directory
and are feature-gated behind `integration-tests`.

**Feature Flags** (langchain-core Cargo.toml):

```toml
[features]
default = []
tracing = ["dep:tracing", "dep:tracing-opentelemetry", "dep:opentelemetry"]
```

**Convenience Crate Dependencies** (langchain):

Additional dependencies beyond langchain-core: `moka` (async-compatible
LRU cache for CacheBackedEmbeddings), `regex` (for RegexParser). The
`langchain` crate is heavier than `langchain-core` — users who only need
traits can depend on `langchain-core` directly.

**Provider Crate Dependencies**:

All provider crates depend on `langchain-core`, `reqwest` (rustls),
`serde`, `serde_json`, `thiserror`, and `tokio`. Additional per-provider:

| Crate | Extra Dependencies | Notes |
|---|---|---|
| langchain-openai | reqwest-retry, reqwest-middleware, eventsource-stream | SSE parsing, HTTP retry |
| langchain-anthropic | reqwest-retry, reqwest-middleware, eventsource-stream | Native API, SSE streaming |
| langchain-ollama | (none) | NDJSON streaming (no SSE) |
| langchain-huggingface | (none) | API-only initially |
| langchain-chroma | (none) | REST client to Chroma |
| langchain-qdrant | (none); optional: tonic, qdrant-client | REST default; gRPC optional |
| langchain-mistralai | langchain-openai (BaseChatOpenAI) | Extends OpenAI base |
| langchain-fireworks | langchain-openai (BaseChatOpenAI) | Extends OpenAI base |
| langchain-groq | langchain-openai (BaseChatOpenAI) | Extends OpenAI base |
| langchain-deepseek | langchain-openai (BaseChatOpenAI) | Extends OpenAI base |
| langchain-xai | langchain-openai (BaseChatOpenAI) | Extends OpenAI base |
| langchain-openrouter | langchain-openai (BaseChatOpenAI) | Extends OpenAI base |
| langchain-perplexity | langchain-openai (BaseChatOpenAI) | Extends OpenAI base + search |
| langchain-nomic | (none) | Embeddings only |
| langchain-exa | (none) | Retriever + Tool |

## Complexity Tracking

No constitution violations. Table intentionally empty.
