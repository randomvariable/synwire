# Core Parity Checklist: LangChain Rust Port

**Purpose**: Validate that spec, contracts, and data model adequately document parity with Python `langchain_core` public API — comprehensive module-by-module audit beyond the initial api-parity.md
**Created**: 2026-03-09
**Feature**: [spec.md](../spec.md) | [contracts/traits.md](../contracts/traits.md) | [data-model.md](../data-model.md)
**Depth**: Rigorous | **Scope**: Full langchain_core module inventory
**Audience**: Reviewer (PR) | **Timing**: Pre-implementation gate
**Source**: `/langchain/libs/core/langchain_core/`

## Messages — Additional Types

- [x] CHK091 Is `FunctionMessage` (deprecated but present in Python) documented as excluded with rationale? Some older APIs still produce FunctionMessage [Gap, data-model.md §Message]
- [x] CHK092 Is `ChatMessage` (generic message with arbitrary `role: str`) documented as excluded or mapped? Python uses this for custom roles beyond human/ai/system/tool [Gap, data-model.md §Message]
- [x] CHK093 Is `RemoveMessage` (signals removal of a message from history by ID, used in LangGraph state reduction) documented as excluded or mapped? [Gap, data-model.md §Message]
- [x] CHK094 Is `AgentActionMessageLog` (AgentAction with message log reference) documented as excluded or mapped? Python agents module includes this alongside AgentAction [Gap, data-model.md §AgentAction]

## Messages — Content Block Types

- [x] CHK095 Are `Audio`, `Video`, and `File` content block types documented as excluded or mapped? Python supports rich multimodal content blocks [Gap, data-model.md §ContentBlock]
- [x] CHK096 Is the `Reasoning` content block type (chain-of-thought reasoning from models like Claude) documented as excluded or mapped? [Gap, data-model.md §ContentBlock]
- [x] CHK097 Is the `Thinking` content block type (model thinking/scratchpad content) documented as excluded or mapped? [Gap, data-model.md §ContentBlock]
- [x] CHK098 Are `GuardContent`, `RefusalContent`, and `CitationContent` block types documented as excluded? These represent safety guardrails, model refusals, and citation references [Gap, data-model.md §ContentBlock]
- [x] CHK099 Is `CacheControl` content block type (provider-specific caching hints) documented as excluded or mapped? [Gap, data-model.md §ContentBlock]
- [x] CHK100 Is the full list of supported content block types exhaustively enumerated with in-scope/excluded status for each? Current Rust design has only Text and Image [Completeness, data-model.md §ContentBlock]

## Messages — Chunk Types

- [x] CHK101 Are separate chunk types per message variant (`AIMessageChunk`, `HumanMessageChunk`, `SystemMessageChunk`, `ToolMessageChunk`) explicitly documented as collapsed into `ChatChunk`? The rationale exists but could be more specific per type [Clarity, data-model.md §ChatChunk]
- [x] CHK102 Is `FunctionMessageChunk` documented as excluded alongside `FunctionMessage`? [Consistency, data-model.md §ChatChunk]

## Messages — Utility Functions

- [x] CHK103 Is `filter_messages(messages, include_types, exclude_types, include_names, exclude_names, include_ids, exclude_ids)` documented as excluded or mapped? Commonly used for message list manipulation [Gap, Contracts]
- [x] CHK104 Is `trim_messages(messages, max_tokens, strategy, token_counter)` documented as excluded or mapped? Essential for context window management [Gap, Contracts]
- [x] CHK105 Is `merge_message_runs(messages)` documented as excluded or mapped? Merges consecutive messages of the same type [Gap, Contracts]
- [x] CHK106 Is `convert_to_messages(data)` documented as excluded or mapped? Converts dicts/tuples to Message objects [Gap, Contracts]
- [x] CHK107 Are `messages_to_dict` / `messages_from_dict` serialisation utilities documented as excluded (covered by serde) or mapped? [Clarity, Research §5]

## Prompts — Additional Types

- [x] CHK108 Is `FewShotPromptTemplate` (template with example selection) documented as excluded or mapped? Commonly used for in-context learning patterns [Gap, Contracts §PromptTemplate]
- [x] CHK109 Is `FewShotChatMessagePromptTemplate` (chat variant of few-shot) documented as excluded or mapped? [Gap, Contracts §PromptTemplate]
- [x] CHK110 Is `PipelinePromptTemplate` (compose multiple prompt templates) documented as excluded or mapped? [Gap, Contracts §PromptTemplate]
- [x] CHK111 Are `ExampleSelector` base class and concrete selectors (`SemanticSimilarityExampleSelector`, `LengthBasedExampleSelector`, `MaxMarginalRelevanceExampleSelector`) documented as excluded or mapped? [Gap, Contracts]
- [x] CHK112 Is `MessagesPlaceholder` explicitly documented as mapped to `MessageTemplate::Placeholder`? The mapping exists implicitly but isn't cross-referenced [Clarity, Contracts §ChatPromptTemplate vs data-model.md §MessageTemplate]
- [x] CHK113 Is `DictPromptTemplate` (returns dict output instead of string) documented as excluded or mapped? [Gap, Contracts]

## Language Models — Additional Features

- [x] CHK114 Is `SimpleChatModel` (minimal chat model implementation for testing) documented as excluded or provided as a test utility? Python uses this as a base for mock models [Gap, Contracts §BaseChatModel]
- [x] CHK115 Is `LLM` (simple string-in/string-out implementation base) documented? Distinct from `BaseLLM` which is the abstract trait [Gap, Contracts §BaseLLM]
- [x] CHK116 Are `FakeChatModel`, `FakeListChatModel`, `FakeListLLM`, `FakeStreamingListLLM` test utilities documented as test infrastructure or excluded? [Gap, Research]
- [x] CHK117 Is `ModelProfile` (capability profile: supports_tool_calling, supports_streaming, supports_structured_output) documented as excluded or mapped? Python uses this for runtime capability detection [Gap, Contracts §BaseChatModel]
- [x] CHK118 Is `rate_limiter` support documented? Python BaseChatModel accepts an optional rate limiter for throttling requests [Gap, Contracts §BaseChatModel]
- [x] CHK119 Is `cache` support documented? Python language models support caching via `BaseCache` for deduplicating identical requests [Gap, Contracts §BaseChatModel]

## Runnables — Additional Types

- [x] CHK120 Is `RunnableLambda` (wrap a closure/function as a Runnable) documented as excluded or mapped? This is one of the most commonly used Runnable types for ad-hoc transformations [Gap, Contracts §Runnable]
- [x] CHK121 Is `RunnableGenerator` (wrap an async generator as a Runnable) documented as excluded or mapped? Used for custom streaming transformations [Gap, Contracts §Runnable]
- [x] CHK122 Is `RunnableBranch` (conditional routing: if-else on input) documented as excluded or mapped? Enables dynamic chain selection based on input [Gap, Contracts §Runnable]
- [x] CHK123 Is `RunnableBinding` (bind kwargs to a Runnable) documented as excluded? Related to the `bind` exclusion but `RunnableBinding` is a concrete type [Consistency, Contracts §Runnable Exclusions]
- [x] CHK124 Is `RunnablePassthrough` documented with its fields and methods? It appears in plan.md project structure but not in contracts/traits.md [Gap, Contracts §Runnable]
- [x] CHK125 Is `RunnableAssign` (merge new keys into a dict output) documented as excluded alongside `assign`? [Consistency, Contracts §Runnable Exclusions]
- [x] CHK126 Is `RunnablePick` (select keys from dict output) documented as excluded alongside `pick`? [Consistency, Contracts §Runnable Exclusions]
- [x] CHK127 Is `RunnableWithMessageHistory` (automatic conversation history injection) documented as excluded or mapped? Important for chat applications [Gap, Contracts §Runnable]
- [x] CHK128 Is `RouterRunnable` (route input to one of several runnables based on a key) documented as excluded or mapped? [Gap, Contracts §Runnable]
- [x] CHK129 Are `ConfigurableField`, `ConfigurableFieldSingleOption`, `ConfigurableFieldMultiOption` (runtime-configurable runnable parameters) documented as excluded or mapped? [Gap, Contracts §Runnable]
- [x] CHK130 Is `RunnableParallel` documented in contracts? It appears in plan.md project structure (`chain.rs`) but not in contracts/traits.md [Gap, Contracts §Runnable]

## Runnables — Utility Functions

- [x] CHK131 Is `chain` decorator (convert a generator function to a RunnableGenerator) documented as excluded or mapped? Python uses `@chain` for declarative chain definition [Gap, Contracts]
- [x] CHK132 Is `dispatch_custom_event(name, data)` function documented? This is the API for emitting custom events consumed by `stream_events` — distinct from the `on_custom_event` callback [Gap, Contracts §Runnable]

## Output Parsers — Concrete Implementations

- [x] CHK133 Is `StrOutputParser` (identity parser returning raw string) documented as a planned implementation or excluded? This is the most basic parser used in nearly every chain [Gap, Contracts §OutputParser]
- [x] CHK134 Is `JsonOutputParser` (parse JSON from model output) documented as a planned implementation or excluded? Essential for structured output workflows [Gap, Contracts §OutputParser]
- [x] CHK135 Is `PydanticOutputParser` equivalent (parse into a typed struct via JSON Schema) documented? In Rust this would parse into serde-deserializable types [Gap, Contracts §OutputParser]
- [x] CHK136 Is `XMLOutputParser` documented as excluded or mapped? [Gap, Contracts §OutputParser]
- [x] CHK137 Are list parsers (`CommaSeparatedListOutputParser`, `NumberedListOutputParser`, `MarkdownListOutputParser`) documented as excluded or mapped? [Gap, Contracts §OutputParser]
- [x] CHK138 Is `EnumOutputParser` (parse into a known set of values) documented as excluded or mapped? [Gap, Contracts §OutputParser]
- [x] CHK139 Is `RetryOutputParser` (retry parsing with model correction) documented as excluded or mapped? This wraps another parser with LLM-assisted retry on parse failure [Gap, Contracts §OutputParser]
- [x] CHK140 Is `CombiningOutputParser` (compose multiple parsers) documented as excluded or mapped? [Gap, Contracts §OutputParser]
- [x] CHK141 Is `ToolsOutputParser` (extract tool calls from model response) documented as excluded or mapped? Important for function-calling chains [Gap, Contracts §OutputParser]
- [x] CHK142 Is the scope boundary for output parser concrete implementations clearly defined — i.e. which parsers ship with langchain-core vs which are left to users? [Completeness, Spec]

## Tools — Additional Types

- [x] CHK143 Is `StructuredTool` (tool with typed input schema from a function signature) documented as excluded or mapped? In Python, this is the primary way to create tools from functions [Gap, Contracts §Tool]
- [x] CHK144 Is `Tool` (simple function-based tool with string input) documented as excluded or mapped to the Rust `Tool` trait? Note: Python `Tool` class is distinct from `BaseTool` [Clarity, Contracts §Tool]
- [x] CHK145 Is the `@tool` decorator pattern documented with a Rust equivalent? In Rust, this could map to a proc-macro or builder pattern for generating `Tool` trait implementations [Gap, Contracts §Tool]
- [x] CHK146 Is `ToolException` (exception type for tool-level errors that can be shown to the model) documented as mapped to `LangChainError::ToolError`? [Clarity, Research §8]
- [x] CHK147 Is `ToolOutput` type documented? Python tools return `ToolOutput` with content and optional artifact [Gap, data-model.md]

## Documents — Additional Types

- [x] CHK148 Is `Blob` (binary content with media type, encoding, path) documented as excluded or mapped? Used for processing non-text content (images, audio, PDFs) [Gap, data-model.md §Document]
- [x] CHK149 Is `BaseDocumentCompressor` (reduce document content for context window) documented as excluded or mapped? Used in RAG pipelines [Gap, Contracts]
- [x] CHK150 Is `BaseDocumentTransformer` (transform documents — split, translate, filter) documented as excluded or mapped? Key abstraction for document processing pipelines [Gap, Contracts]

## Callbacks — Manager Hierarchy

- [x] CHK151 Is `CallbackManager` / `AsyncCallbackManager` (manages a list of handlers, creates child managers for nested runs) documented as excluded or mapped? Python uses a manager hierarchy for parent/child run tracking [Gap, Contracts §CallbackHandler]
- [x] CHK152 Are run-specific managers (`CallbackManagerForChainRun`, `CallbackManagerForLLMRun`, `CallbackManagerForToolRun`, `CallbackManagerForRetrieverRun`) documented as excluded? [Gap, Contracts §CallbackHandler]
- [x] CHK153 Is the `dispatch_custom_event(name, data, config)` standalone function documented as a Rust equivalent? Distinct from `on_custom_event` callback — this is the emit side [Gap, Contracts]
- [x] CHK154 Is `collect_runs()` context manager (collect run metadata during execution) documented as excluded or mapped? [Gap, Contracts §CallbackHandler]
- [x] CHK155 Is `tracing_v2_enabled()` / `tracing_enabled()` (check if tracing is active) documented as excluded or mapped? [Gap, Research §11]

## Embeddings — Additional Features

- [x] CHK156 Is `FakeEmbeddings` (deterministic embeddings for testing) documented as test infrastructure? [Gap, Research]
- [x] CHK157 Is `CacheBackedEmbeddings` (cache embedding results to avoid recomputation) documented as excluded or mapped? [Gap, Contracts §Embeddings]

## VectorStore — Additional Features

- [x] CHK158 Is `VectorStoreRetriever` (retriever backed by a VectorStore, returned by `as_retriever`) documented in contracts? `as_retriever` returns `Box<dyn Retriever>` but the concrete type isn't specified [Clarity, Contracts §VectorStore]
- [x] CHK159 Is `InMemoryVectorStore` documented in contracts or data model? It appears in plan.md but has no specified API contract [Gap, Contracts]

## Retrievers — Additional Features

- [x] CHK160 Is the Runnable blanket implementation for Retriever (`Runnable<String, Vec<Document>>`) specified with enough detail to implement? Contracts mention it exists but don't show the adapter [Clarity, Contracts §Retriever]

## Error Types — Additional Variants

- [x] CHK161 Is `LangChainError::AgentError` variant needed for agent-specific failures (max iterations exceeded, execution timeout)? Current error enum has no agent variant [Gap, Research §4]
- [x] CHK162 Is `LangChainError::CallbackError` variant needed for callback handler failures? Current design has no callback error handling strategy [Gap, Research §4]
- [x] CHK163 Is `LangChainError::RetryExhausted` variant documented for when all retry attempts fail? Currently retry presumably re-raises the last error [Clarity, Research §10]
- [x] CHK164 Is `LangChainErrorKind::Agent` variant needed to match `LangChainError::AgentError` for retry/fallback configuration? [Consistency, data-model.md §LangChainErrorKind]

## Type Conversions & Ergonomics

- [x] CHK165 Are `Into<Vec<Message>>` implementations documented for common input types (PromptValue, &str, String)? Referenced in BaseChatModel exclusions but not specified [Gap, Contracts §BaseChatModel]
- [x] CHK166 Are `From`/`Into` conversions between related types (e.g. `ToolCall` ↔ `Value`, `Document` ↔ serialisation) documented? [Gap, data-model.md]
- [x] CHK167 Is `MessageLike` trait (convert various types to Message) documented? Referenced in plan.md project structure (`messages/traits.rs`) but absent from contracts [Gap, Contracts]

## Prelude & Re-exports

- [x] CHK168 Is the `prelude` module content specified — which types and traits are re-exported for convenience? Plan.md lists `prelude.rs` but content is unspecified [Gap, plan.md §Project Structure]
- [x] CHK169 Is the top-level `langchain` crate re-export strategy documented? Plan.md describes it as "high-level re-exports" without specifying which items [Clarity, plan.md §Project Structure]

## Notes

- Check items off as completed: `[x]`
- Items marked [Gap] indicate missing requirements that need spec/contract updates
- Items marked [Clarity] indicate existing requirements needing more precision
- Items marked [Consistency] indicate misalignment between documents
- Items marked [Completeness] indicate partially specified requirements
- This checklist complements `api-parity.md` (CHK001-CHK070) with deeper module-level coverage
- Reference: Python API audited from `/langchain/libs/core/langchain_core/`
