# Feature Specification: LangChain Rust Port

**Feature Branch**: `001-langchain-rust-port`
**Created**: 2026-03-09
**Status**: Draft
**Input**: User description: "Port LangChain Python to Rust"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Define and Invoke a Chat Model (Priority: P1)

A Rust developer adds the langchain-core crate and a provider crate (e.g.
langchain-openai) to their project. They construct a chat model instance,
send a prompt message, and receive a structured response. The developer
can switch to a different provider by swapping the provider crate without
changing their application logic.

**Why this priority**: The ability to invoke a language model through a
common abstraction is the foundational value proposition of LangChain.
Without this, no other feature is useful.

**Independent Test**: Can be tested by creating a mock language model
that implements the core trait, invoking it with a prompt, and verifying
the response structure matches the expected output type.

**Acceptance Scenarios**:

1. **Given** a Rust project with `langchain-core` as a dependency,
   **When** the developer implements the `ChatModel` trait for a mock
   provider, **Then** they can call `invoke()` with a list of messages
   and receive a typed `ChatResult` response.
2. **Given** a working chat model invocation,
   **When** the developer replaces the mock provider with a different
   trait implementation, **Then** the calling code compiles and runs
   without modification.
3. **Given** a chat model invocation that encounters a network or
   provider error, **When** the error occurs, **Then** the library
   returns a typed `Result::Err` with an actionable error message
   rather than panicking.

---

### User Story 2 - Compose a Prompt Template and Chain (Priority: P2)

A Rust developer creates a prompt template with variable placeholders,
formats it with runtime values, and chains it with a language model to
produce a complete invocation pipeline (the Rust equivalent of LCEL).

**Why this priority**: Prompt templates and composable chains are the
second most-used feature in LangChain after raw model invocation. They
enable reusable, parameterised prompts.

**Independent Test**: Can be tested by creating a prompt template,
formatting it with test values, and verifying the output string. Chain
composition can be tested by connecting a template to a mock model and
verifying end-to-end data flow.

**Acceptance Scenarios**:

1. **Given** a prompt template with placeholders `{topic}` and
   `{style}`, **When** the developer provides values for both
   variables, **Then** the formatted output contains the substituted
   values in the correct positions.
2. **Given** a prompt template chained to a mock chat model,
   **When** the developer invokes the chain with input variables,
   **Then** the chain formats the prompt, passes it to the model,
   and returns the model's response.
3. **Given** a prompt template with a missing required variable,
   **When** the developer attempts to format it, **Then** the library
   returns a descriptive error identifying the missing variable.

---

### User Story 3 - Stream Responses from a Language Model (Priority: P3)

A Rust developer invokes a language model in streaming mode and
processes response tokens as they arrive, rather than waiting for the
complete response. This enables responsive user interfaces and
real-time processing pipelines.

**Why this priority**: Streaming is critical for production LLM
applications where latency to first token matters. It builds on the
core model invocation from US1.

**Independent Test**: Can be tested with a mock model that yields
tokens incrementally via an async stream, verifying that each token
is received in order and the stream terminates correctly.

**Acceptance Scenarios**:

1. **Given** a chat model that supports streaming,
   **When** the developer calls `stream()` with a prompt,
   **Then** they receive an async stream of response chunks that can
   be consumed with `while let Some(chunk) = stream.next().await`.
2. **Given** a streaming invocation,
   **When** all chunks have been received, **Then** concatenating
   the chunks produces the same content as a non-streaming `invoke()`
   call with the same input.
3. **Given** a streaming invocation that encounters an error mid-stream,
   **When** the error occurs, **Then** the stream yields an error item
   and terminates cleanly without resource leaks.

---

### User Story 4 - Embed Text and Query a Vector Store (Priority: P4)

A Rust developer generates embeddings for a set of documents, stores
them in a vector store, and retrieves the most similar documents for a
given query. This enables retrieval-augmented generation (RAG) workflows.

**Why this priority**: RAG is one of the most common LangChain
production patterns. It depends on the embedding and vector store
abstractions being in place.

**Independent Test**: Can be tested with a mock embedding model that
returns deterministic vectors and an in-memory vector store, verifying
that similarity search returns documents in the expected order.

**Acceptance Scenarios**:

1. **Given** a set of text documents and an embedding model,
   **When** the developer calls `embed_documents()`, **Then** they
   receive a vector of floating-point arrays, one per document.
2. **Given** documents stored in a vector store,
   **When** the developer performs a similarity search with a query,
   **Then** the results are returned in descending order of relevance
   with associated similarity scores.
3. **Given** an empty vector store,
   **When** the developer performs a similarity search, **Then** the
   library returns an empty result set rather than an error.

---

### User Story 5 - Define and Use Tools (Priority: P5)

A Rust developer defines custom tools (functions with typed input/output
schemas) that can be invoked by a language model or agent. Tools enable
LLMs to interact with external systems.

**Why this priority**: Tool use is essential for agentic workflows.
It builds on the model invocation layer and adds the function-calling
abstraction.

**Independent Test**: Can be tested by defining a mock tool, invoking
it with typed input, and verifying the output. Tool schema generation
can be verified by inspecting the serialised tool description.

**Acceptance Scenarios**:

1. **Given** a struct implementing the `Tool` trait with typed input
   and output, **When** the developer invokes the tool with valid
   input, **Then** the tool executes and returns a typed result.
2. **Given** a tool definition, **When** the developer requests its
   schema, **Then** the library produces a serialisable description
   including name, description, and input parameter schema.
3. **Given** a tool invoked with invalid input, **When** the
   invocation fails, **Then** the library returns a typed error
   with context about the validation failure.

---

### Edge Cases

- What happens when a prompt template references a variable that is not
  provided? The library MUST return a descriptive error, not a panic or
  empty substitution.
- What happens when a language model provider returns malformed JSON?
  The library MUST return a parse error with the raw response body for
  debugging.
- What happens when an async stream is dropped before completion?
  Resources MUST be cleaned up without blocking or leaking.
- What happens when embedding dimensions from different providers do
  not match? The vector store MUST reject mismatched dimensions with a
  clear error at insertion time.
- What happens when the user passes an empty message list to a chat
  model? The library MUST return a validation error rather than
  sending an empty request to the provider.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Library MUST define a `ChatModel` trait with `invoke()`,
  `batch()`, and `stream()` methods for interacting with language models.
- **FR-002**: Library MUST define a `PromptTemplate` type supporting
  named variable placeholders and runtime formatting.
- **FR-003**: Library MUST define a `Runnable` trait enabling
  composable chains of prompt templates, models, and output parsers.
- **FR-004**: Library MUST define an `Embeddings` trait with
  `embed_documents()` and `embed_query()` methods.
- **FR-005**: Library MUST define a `VectorStore` trait with
  `add_documents()`, `similarity_search()`, and
  `similarity_search_with_score()` methods.
- **FR-006**: Library MUST define a `Tool` trait with `invoke()` and
  `schema()` methods for function-calling integrations.
- **FR-007**: Library MUST define message types (`HumanMessage`,
  `AIMessage`, `SystemMessage`, `ToolMessage`) as an enum with
  shared metadata.
- **FR-008**: Library MUST define a `Document` type with content and
  arbitrary metadata fields.
- **FR-009**: Library MUST define a `CallbackHandler` trait for
  observability hooks (on_llm_start, on_llm_end, on_llm_error, etc.).
- **FR-010**: Library MUST define an `OutputParser` trait for
  converting raw model output into structured types.
- **FR-011**: Library MUST define a `Retriever` trait with a
  `get_relevant_documents()` method.
- **FR-012**: All fallible operations MUST return `Result<T, E>` with
  typed error enums; panics in library code are forbidden.
- **FR-013**: All I/O-bound operations MUST be async-compatible.
- **FR-014**: All public types MUST be serialisable and deserialisable
  where semantically meaningful.
- **FR-015**: The library MUST be organised as a Cargo workspace with
  `langchain-core` as the foundational crate and provider integrations
  as separate member crates.
- **FR-016**: Library MUST define concrete runnable types
  (RunnableSequence, RunnableParallel, RunnablePassthrough,
  RunnableLambda, RunnableBranch) for chain composition.
- **FR-017**: Library MUST define concrete output parsers
  (StrOutputParser, JsonOutputParser, StructuredOutputParser,
  ToolsOutputParser) for processing model output.
- **FR-018**: Library MUST define a `StructuredTool` type with a builder
  pattern for creating tools from closures without manual trait
  implementation.
- **FR-019**: Library MUST define agent types (AgentAction, AgentFinish,
  AgentStep, AgentDecision) and an AgentExecutor for ReAct-style
  tool-calling loops.
- **FR-020**: Library MUST define message utility functions
  (filter_messages, trim_messages, merge_message_runs) for
  conversation management.
- **FR-021**: VectorStore trait MUST include maximal marginal relevance
  (MMR) search methods for diversity-aware retrieval.
- **FR-022**: Runnable trait MUST include retry (with_retry) and
  fallback (with_fallbacks) composition for resilience.
- **FR-023**: Runnable trait MUST include stream_events and stream_log
  methods for structured observability.
- **FR-024**: Library MUST provide FakeChatModel and FakeEmbeddings
  test utilities for unit testing chains without real API calls.
- **FR-025**: Library MUST define a `dispatch_custom_event` function
  for emitting custom events to the callback system.
- **FR-026**: The `langchain` convenience crate MUST provide reference
  implementations for common application-level patterns:
  CacheBackedEmbeddings, RunnableWithMessageHistory, additional output
  parsers (list, enum, XML, regex, retry, combining), few-shot prompt
  templates with example selectors, and text splitters.
- **FR-027**: The `langchain-openai` crate MUST provide an
  OpenAIModerationMiddleware reference implementation as a
  RunnableLambda wrapper.
- **FR-028**: The workspace MUST include provider crates for all 16
  Python LangChain partners: OpenAI, Anthropic, Chroma, Qdrant,
  HuggingFace, Ollama, MistralAI, Fireworks, Groq, Nomic, Exa,
  DeepSeek, xAI, OpenRouter, Perplexity. OpenAI-compatible providers
  MUST share a `BaseChatOpenAI` base type to avoid code duplication.

### Key Entities

- **Message**: A single unit of conversation — contains a role
  (human, ai, system, tool), content (text or structured), and
  optional metadata (tool call ID, name, additional kwargs).
- **PromptTemplate**: A parameterised template with named variable
  slots, an input variable list, and a format method that produces
  a prompt value.
- **Document**: A piece of retrievable content — contains page content
  (text) and a metadata map (source, page number, custom fields).
- **Embedding**: A dense vector representation of text — a fixed-length
  array of floating-point numbers produced by an embedding model.
- **ToolCall**: A structured request from a model to invoke a tool —
  contains tool name, arguments (key-value), and an optional call ID.
- **ChatResult**: The output of a model invocation — contains one or
  more message generations, token usage statistics, and model metadata.
- **AgentAction**: A structured request from an agent to invoke a tool —
  contains tool name, tool input, and reasoning log text.
- **AgentFinish**: The terminal output of an agent — contains return
  values and final reasoning log text.
- **StreamEvent**: A structured observability event emitted during
  runnable execution — contains event type, run ID, parent IDs,
  tags, metadata, and event-specific data.
- **RetryConfig**: Configuration for retry composition — specifies
  which error kinds to retry, max attempts, and backoff parameters.
- **ContentBlock**: A typed content element within a message — can be
  text, image URL, audio URL, video URL, file URL, reasoning text,
  or thinking text.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A developer can add `langchain-core` to a Rust project,
  implement a mock provider, and complete a round-trip model invocation
  in under 30 minutes using only the library documentation.
- **SC-002**: All core trait definitions compile and pass trait-level
  unit tests covering happy path, error path, and edge cases — with at
  least 90% line coverage on the core crate.
- **SC-003**: At least one real provider integration (e.g. OpenAI)
  demonstrates end-to-end functionality: prompt → model → response,
  including streaming.
- **SC-004**: Switching between two provider implementations requires
  changing only the concrete type instantiation — no changes to chain
  or prompt code.
- **SC-005**: The library compiles with zero `unsafe` blocks in the
  core crate (excluding transitive dependencies).
- **SC-006**: All async operations support cancellation — dropping a
  future or stream releases resources without blocking.

## Assumptions

- The primary target audience is Rust developers building LLM-powered
  applications who are familiar with the LangChain conceptual model
  from Python or TypeScript.
- The initial port focuses on `langchain-core` abstractions plus the
  agent framework (AgentAction, AgentFinish, AgentExecutor ReAct loop).
- All 16 Python partner providers are in scope, each as a separate
  Cargo crate. OpenAI-compatible providers (Groq, Fireworks, DeepSeek,
  xAI, OpenRouter) share a common `BaseChatOpenAI` base type.
- Azure variants (AzureChatOpenAI, AzureOpenAIEmbeddings) remain
  excluded — they require Azure AD auth and are a separate crate.
- The library targets `tokio` as the primary async runtime, with
  runtime-agnostic core traits as a stretch goal.
- Serialisation uses `serde` with JSON as the default wire format,
  matching LangChain Python's JSON-based serialisation.

### In-Scope Items (Explicit)

The following are fully in scope for the initial port:

- **Core traits**: Runnable, BaseChatModel, BaseLLM, Embeddings,
  VectorStore, Tool, OutputParser, Retriever, BasePromptTemplate,
  CallbackHandler, MessageLike
- **Concrete runnable types**: RunnableSequence, RunnableParallel,
  RunnablePassthrough, RunnableLambda, RunnableBranch
- **Runnable composition**: pipe, with_retry (RunnableRetry),
  with_fallbacks (RunnableWithFallbacks), as_tool (RunnableTool),
  transform, batch_as_completed
- **Concrete output parsers**: StrOutputParser, JsonOutputParser,
  StructuredOutputParser\<T\>, ToolsOutputParser
- **Tool creation**: StructuredTool with builder pattern
  (StructuredToolBuilder)
- **Agent framework**: AgentAction, AgentFinish, AgentStep,
  AgentDecision, AgentInput, AgentExecutor (ReAct loop)
- **Message types**: Human, AI, System, Tool, Chat (generic role);
  content blocks including Text, Image, Audio, Video, File, Reasoning,
  Thinking
- **Message utilities**: filter_messages, trim_messages,
  merge_message_runs, dispatch_custom_event
- **Vector store**: VectorStore trait with MMR methods,
  InMemoryVectorStore, VectorStoreRetriever
- **Event streaming**: stream_events, stream_log (StreamEvent,
  RunLogPatch types)
- **Callbacks**: CallbackHandler with all hooks including on_agent_action,
  on_agent_finish, on_custom_event, on_retry
- **Observability**: Optional tracing + OpenTelemetry behind feature flag
- **Test utilities**: FakeChatModel, FakeEmbeddings
- **Provider crates** (16 total):
  - langchain-openai: ChatOpenAI, OpenAIEmbeddings, OpenAIModerationMiddleware
  - langchain-anthropic: ChatAnthropic
  - langchain-ollama: ChatOllama, OllamaLLM, OllamaEmbeddings
  - langchain-huggingface: ChatHuggingFace, HuggingFaceEmbeddings, HuggingFacePipeline
  - langchain-chroma: Chroma (VectorStore)
  - langchain-qdrant: QdrantVectorStore
  - langchain-mistralai: ChatMistralAI, MistralAIEmbeddings
  - langchain-fireworks: ChatFireworks, FireworksEmbeddings
  - langchain-groq: ChatGroq
  - langchain-nomic: NomicEmbeddings
  - langchain-exa: ExaSearchRetriever, ExaSearchResults (Tool)
  - langchain-deepseek: ChatDeepSeek
  - langchain-xai: ChatXAI
  - langchain-openrouter: ChatOpenRouter
  - langchain-perplexity: ChatPerplexity, PerplexitySearchRetriever

### Reference Implementations (langchain crate)

The following are excluded from `langchain-core` traits but provided as
concrete reference implementations in the `langchain` convenience crate,
mirroring how Python's `langchain` package provides them on top of
`langchain-core` abstractions:

- **CacheBackedEmbeddings** — wraps any `Embeddings` impl with a
  configurable cache store (in-memory via `moka`); excluded from core
  trait hierarchy but provided as a ready-to-use wrapper
- **RunnableWithMessageHistory** — wraps any `Runnable` with automatic
  chat history management using a pluggable `ChatMessageHistory` store;
  includes `InMemoryChatMessageHistory` reference store
- **Additional output parsers** — CommaSeparatedListOutputParser,
  EnumOutputParser, XMLOutputParser, RegexParser, RetryOutputParser,
  CombiningOutputParser; excluded from core (4 core parsers suffice for
  most use cases) but provided for convenience
- **FewShotPromptTemplate, FewShotChatMessagePromptTemplate** — compose
  examples with templates using an ExampleSelector; excluded from core
  prompt traits but provided as concrete types
- **SemanticSimilarityExampleSelector** — selects few-shot examples
  using VectorStore similarity search
- **Text splitters** — CharacterTextSplitter,
  RecursiveCharacterTextSplitter for chunking documents for RAG
  pipelines; excluded from core document types but essential for
  production RAG workflows
- **OpenAIModerationMiddleware** — content moderation via OpenAI's
  `/v1/moderations` endpoint as a `RunnableLambda` wrapper; lives in
  `langchain-openai` crate

### Permanently Excluded Items

The following are permanently excluded and will NOT be ported — not as
core traits, not as reference implementations:

- **Legacy chains** (LLMChain, SequentialChain, RetrievalQA,
  ConversationalRetrievalChain) — deprecated in Python in favour of
  LCEL/Runnable composition
- **LangGraph middleware and state** (ChannelManager,
  StateGraph, MessageGraph, CompiledGraph, Pregel, ToolNode,
  create_react_agent) — LangGraph is an architecturally distinct
  orchestration framework, not part of langchain-core
- **RunnableSerializable** — Python-specific JSON serialisation of
  chain definitions; Rust uses compiled-in types
- **LangSmith introspection** (get_input_schema, get_output_schema,
  get_graph, get_name, get_prompts) — coupled to LangSmith tracing
  platform
- **PipelinePromptTemplate** — niche composition pattern; use
  RunnableSequence with multiple prompt templates instead
- **SimpleChatModel, LLM base class, ModelProfile, rate_limiter,
  cache** — Python implementation helpers without Rust equivalents
- **CallbackManager hierarchy, collect_runs,
  tracing_v2_enabled** — Python's manager pattern replaced by flat
  Vec\<Box\<dyn CallbackHandler\>\> in RunnableConfig
- **Blob, BaseDocumentCompressor** — document processing types
  beyond what text splitters cover
- **RunnableBinding, RunnableGenerator, RunnableAssign, RunnablePick,
  RouterRunnable, ConfigurableField** —
  Python-specific patterns or LangGraph-specific
- **FunctionMessage, RemoveMessage, AgentActionMessageLog** — deprecated
  or LangGraph-specific message types
- **GuardContent, RefusalContent, CitationContent, CacheControl
  content blocks** — provider-specific content types
- **SparseEmbeddings** — Qdrant-specific sparse vector type; out of
  scope for the `Embeddings` trait (but may be in langchain-qdrant)
- **AzureChatOpenAI, AzureOpenAIEmbeddings** — Azure AD auth and
  deployment-based routing require a separate crate
- **BaseOpenAI / OpenAI (legacy completions)** — deprecated endpoint
- **convert_to_messages, messages_to_dict, messages_from_dict** —
  Python serialisation helpers unnecessary with serde
- **Pydantic output parser** — Python-specific; Rust equivalent is
  StructuredOutputParser\<T: DeserializeOwned\> already in core
