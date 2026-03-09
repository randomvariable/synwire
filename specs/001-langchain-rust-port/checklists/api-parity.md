# API Parity Checklist: LangChain Rust Port

**Purpose**: Validate that spec, contracts, and data model adequately document parity with Python langchain-core public API — method-by-method signature audit plus behavioural semantics
**Created**: 2026-03-09
**Feature**: [spec.md](../spec.md) | [contracts/traits.md](../contracts/traits.md) | [data-model.md](../data-model.md)
**Depth**: Rigorous | **Scope**: API surface + behavioural parity
**Audience**: Reviewer (PR) | **Timing**: Pre-implementation gate

## BaseChatModel — Trait Method Parity

- [x] CHK001 Are `generate` and `generate_prompt` methods documented as intentionally excluded or mapped to an equivalent? Python BaseChatModel exposes both; Rust contracts only define `invoke`/`batch`/`stream` [Gap, Contracts §BaseChatModel]
- [x] CHK002 Is the `bind_tools` method mapped to a Rust equivalent or explicitly scoped out? Python BaseChatModel.bind_tools returns a configured Runnable [Gap, Contracts §BaseChatModel]
- [x] CHK003 Is the `with_structured_output` method mapped or explicitly scoped out? Python BaseChatModel.with_structured_output returns a Runnable that produces structured types [Gap, Contracts §BaseChatModel]
- [x] CHK004 Is the `stop` parameter requirement specified? Python invoke/stream accept `stop: list[str]`; Rust contracts omit it [Gap, Contracts §BaseChatModel]
- [x] CHK005 Are the return type differences documented? Python invoke returns `AIMessage` directly; Rust returns `ChatResult` wrapping the message — is the mapping rationale specified? [Clarity, Contracts §BaseChatModel]
- [x] CHK006 Is the `LanguageModelInput` type union (str | list[BaseMessage] | PromptValue) mapped to Rust? Python accepts multiple input types; Rust contracts only accept `&[Message]` [Gap, Contracts §BaseChatModel]

## BaseLLM — Trait Method Parity

- [x] CHK007 Are `generate` and `generate_prompt` methods documented as intentionally excluded? Python BaseLLM.generate is the core abstract method [Gap, Contracts §BaseLLM]
- [x] CHK008 Are serialisation methods (`dict`, `save`) documented as out-of-scope or mapped? [Gap, Contracts §BaseLLM]

## Embeddings — Trait Method Parity

- [x] CHK009 Are the async variants `aembed_documents` and `aembed_query` addressed? Research.md states async/sync duality collapses to single async method — is this mapping explicitly documented per trait? [Clarity, Research §3]
- [x] CHK010 Is the `config` parameter requirement specified? Rust Embeddings trait omits `RunnableConfig` unlike other traits — is this intentional? [Consistency, Contracts §Embeddings]

## VectorStore — Trait Method Parity

- [x] CHK011 Is `add_texts` (accepting raw strings + metadatas) mapped or excluded? Python VectorStore has both `add_documents` and `add_texts` [Gap, Contracts §VectorStore]
- [x] CHK012 Is `similarity_search_with_relevance_scores` mapped or excluded? Distinct from `similarity_search_with_score` in Python [Gap, Contracts §VectorStore]
- [x] CHK013 Are Maximal Marginal Relevance methods (`max_marginal_relevance_search`, `max_marginal_relevance_search_by_vector`) documented as out-of-scope? [Gap, Contracts §VectorStore]
- [x] CHK014 Is `as_retriever` documented? Python VectorStore.as_retriever is a key convenience method for RAG workflows [Gap, Contracts §VectorStore]
- [x] CHK015 Are factory class methods (`from_documents`, `from_texts`) mapped or excluded? [Gap, Contracts §VectorStore]
- [x] CHK016 Is `get_by_ids` mapped or excluded? Python VectorStore.get_by_ids retrieves documents by ID without similarity search [Gap, Contracts §VectorStore]
- [x] CHK017 Is the `embeddings` property requirement specified? Python VectorStore exposes the underlying Embeddings instance [Gap, Contracts §VectorStore]
- [x] CHK018 Is the default `k=4` parameter documented consistently? Python defaults to k=4; Rust requires explicit k — is the deviation intentional? [Consistency, Contracts §VectorStore]

## Runnable — Trait Method Parity

- [x] CHK019 Is `pipe` method specified in the Runnable contract? Only mentioned in research.md as planned; absent from contracts/traits.md [Gap, Contracts §Runnable]
- [x] CHK020 Are `with_config`, `with_retry`, `with_fallbacks` mapped or documented as out-of-scope? These are key Runnable composition methods in Python [Gap, Contracts §Runnable]
- [x] CHK021 Is `bind` (currying kwargs) mapped or excluded? [Gap, Contracts §Runnable]
- [x] CHK022 Are `batch_as_completed` and `abatch_as_completed` addressed? These yield results as they complete rather than in order [Gap, Contracts §Runnable]
- [x] CHK023 Are `transform`/`atransform` (stream-to-stream) methods addressed? [Gap, Contracts §Runnable]
- [x] CHK024 Are schema introspection methods (`get_input_schema`, `get_output_schema`, `InputType`, `OutputType`) addressed? [Gap, Contracts §Runnable]
- [x] CHK025 Are `astream_log` and `astream_events` documented as out-of-scope? These are advanced observability features [Gap, Contracts §Runnable]
- [x] CHK026 Is the `return_exceptions: bool` parameter for batch documented? Python batch can return exceptions inline rather than failing [Gap, Contracts §Runnable]
- [x] CHK027 Are `pick` and `assign` (dictionary output manipulation) methods addressed? [Gap, Contracts §Runnable]

## Prompts — Type Method Parity

- [x] CHK028 Is `PromptTemplate.from_template` factory method specified? Python uses this as the primary constructor; Rust contracts only show `new()` [Gap, Contracts §PromptTemplate]
- [x] CHK029 Is `ChatPromptTemplate.from_messages` specified in contracts? Used in quickstart.md examples but not in contracts/traits.md [Consistency, Contracts §PromptTemplate vs quickstart.md]
- [x] CHK030 Is `partial` (pre-filling some variables) mapped or excluded? Python PromptTemplate.partial is commonly used [Gap, Contracts §PromptTemplate]
- [x] CHK031 Is `template_format` (f-string vs mustache vs jinja2) support documented? Data-model.md mentions `TemplateFormat` enum but contracts don't specify it [Consistency, data-model.md vs Contracts §PromptTemplate]
- [x] CHK032 Are `MessageTemplate` variants complete? Python ChatPromptTemplate supports `Placeholder` for injecting `Vec<Message>` — is this documented with clear semantics? [Completeness, data-model.md §ChatPromptTemplate]
- [x] CHK033 Is `format_prompt` (returns PromptValue) mapped or excluded? Python has both `format` (string) and `format_prompt` (PromptValue) [Gap, Contracts §PromptTemplate]

## Tool — Trait Method Parity

- [x] CHK034 Is the `_run`/`_arun` abstract method pattern mapped to Rust? Python BaseTool separates public `invoke` from implementor's `_run` [Clarity, Contracts §Tool]
- [x] CHK035 Is `args_schema` (Pydantic BaseModel for argument validation) mapped to a Rust equivalent? [Gap, Contracts §Tool]
- [x] CHK036 Is the `is_single_input` property mapped? Determines tool invocation mode [Gap, Contracts §Tool]
- [x] CHK037 Are `run`/`arun` (legacy execution methods with callbacks) documented as excluded? [Gap, Contracts §Tool]

## OutputParser — Trait Method Parity

- [x] CHK038 Is `parse_result` (accepting `list[Generation]`) mapped? Contracts only specify `parse(text)` [Gap, Contracts §OutputParser]
- [x] CHK039 Is `parse_with_prompt` mapped or excluded? Python OutputParser.parse_with_prompt provides prompt context during parsing [Gap, Contracts §OutputParser]

## Retriever — Trait Method Parity

- [x] CHK040 Is the `invoke`/`ainvoke` Runnable interface documented for Retriever? Python BaseRetriever inherits from Runnable with `invoke(input: str)` [Gap, Contracts §Retriever]
- [x] CHK041 Is the `_get_relevant_documents` abstract pattern (with underscore prefix) mapped? Python separates public interface from implementor override [Clarity, Contracts §Retriever]

## CallbackHandler — Event Hook Parity

- [x] CHK042 Is `on_chat_model_start` specified? Python has this in addition to `on_llm_start` — Rust contracts omit it [Gap, Contracts §CallbackHandler]
- [x] CHK043 Is `on_text` mapped or excluded? General-purpose text event hook in Python [Gap, Contracts §CallbackHandler]
- [x] CHK044 Is `on_retry` mapped or excluded? Tenacity retry event hook [Gap, Contracts §CallbackHandler]
- [x] CHK045 Are `on_agent_action`/`on_agent_finish` documented as out-of-scope? Agents are excluded from initial port [Gap, Contracts §CallbackHandler]
- [x] CHK046 Is `on_custom_event` documented as out-of-scope or mapped? [Gap, Contracts §CallbackHandler]
- [x] CHK047 Are `ignore_*` properties (ignore_llm, ignore_chain, etc.) mapped? Python uses these to selectively filter callback events [Gap, Contracts §CallbackHandler]
- [x] CHK048 Are `tags` and `metadata` parameters specified on callback hooks? Python callbacks receive these for tracing; Rust contracts only pass `run_id` and `parent_run_id` [Gap, Contracts §CallbackHandler]

## Message — Type Field Parity

- [x] CHK049 Is `response_metadata` field documented? Python BaseMessage has this for provider metadata (logprobs, token counts) [Gap, data-model.md §Message]
- [x] CHK050 Is `name` field documented? Python BaseMessage has optional `name: str | None` [Gap, data-model.md §Message]
- [x] CHK051 Is `id` field documented on Message? Python BaseMessage has `id: str | None` [Gap, data-model.md §Message]
- [x] CHK052 Is `InvalidToolCall` type documented? Python AIMessage has both `tool_calls` and `invalid_tool_calls` [Gap, data-model.md §Message]
- [x] CHK053 Are message chunk types (`AIMessageChunk`, `HumanMessageChunk`) specified for streaming? Python uses separate chunk types with `__add__` for concatenation [Gap, data-model.md §Message]
- [x] CHK054 Is the `content_blocks` property mapped? Python BaseMessage exposes typed content blocks alongside raw `content` [Gap, data-model.md §Message]
- [x] CHK055 Is `pretty_repr`/`pretty_print` mapped or excluded? Python messages have human-readable formatting methods [Gap, data-model.md §Message]
- [x] CHK056 Is `ToolStatus` specified with exact variant values? Python ToolMessage uses `Literal["success", "error"]`; data-model.md mentions `ToolStatus` but doesn't define variants [Clarity, data-model.md §Message]
- [x] CHK057 Is `artifact` field on ToolMessage documented? Python ToolMessage.artifact carries tool output not sent to model [Gap, data-model.md §Message]

## Document — Type Field Parity

- [x] CHK058 Is the `type` discriminator field documented? Python Document has `type: Literal["Document"]` for serialisation [Gap, data-model.md §Document]

## Behavioural Parity — Async Semantics

- [x] CHK059 Is the Python async/sync duality mapping (invoke/ainvoke → single async) documented per trait, not just generically? Research.md mentions it once but contracts don't annotate each trait [Completeness, Research §3 vs Contracts]
- [x] CHK060 Are blocking sync wrappers specified? Research.md mentions "sync wrappers via a blocking module" but no contract or data-model defines this [Gap, Research §3]

## Behavioural Parity — Error Semantics

- [x] CHK061 Are error mappings specified per Python exception type? Python raises `OutputParserException`, `ToolException`, etc. — are these mapped to specific `LangChainError` variants? [Completeness, Research §4]
- [x] CHK062 Is `return_exceptions: bool` behaviour on batch documented? Python batch can return exceptions inline; Rust batch returns `Result<Vec<...>>` which fails atomically [Gap, Contracts §Runnable]

## Behavioural Parity — Serialisation

- [x] CHK063 Are `to_json`/`from_json`/`dict` serialisation methods addressed for Message types? Python messages are Pydantic models with full serialisation [Gap, Research §5]
- [x] CHK064 Is the `lc_serializable`/`get_lc_namespace` pattern documented as out-of-scope? Python uses these for type-discriminated deserialisation [Gap, Research §5]

## Behavioural Parity — Streaming

- [x] CHK065 Is chunk concatenation behaviour specified? Python `AIMessageChunk.__add__` merges content, tool_calls, and usage — is the Rust equivalent documented? [Gap, data-model.md §ChatChunk]
- [x] CHK066 Is the `ChatChunk` type fully specified with all fields? data-model.md mentions it in flow diagrams but doesn't define its structure [Gap, data-model.md]

## Scope Boundaries — Intentional Exclusions

- [x] CHK067 Is there a documented list of Python API elements intentionally excluded from the Rust port? Spec §Assumptions mentions "agents, chains-of-thought, retrieval QA chains" but doesn't list excluded methods per trait [Completeness, Spec §Assumptions]
- [x] CHK068 Are Runnable advanced features (`astream_log`, `astream_events`, `with_listeners`, `get_graph`, `get_prompts`, `as_tool`) documented as out-of-scope? [Gap, Spec §Assumptions]
- [x] CHK069 Are VectorStore MMR methods documented as out-of-scope? [Gap, Spec §Assumptions]
- [x] CHK070 Is the `RunnableSerializable` (Pydantic-based Runnable) pattern addressed? Python Runnable extends Serializable for type-safe JSON round-trips [Gap, Contracts §Runnable]

## Notes

- Check items off as completed: `[x]`
- Items marked [Gap] indicate missing requirements that need spec/contract updates
- Items marked [Clarity] indicate existing requirements needing more precision
- Items marked [Consistency] indicate misalignment between documents
- Items marked [Completeness] indicate partially specified requirements
- Reference: Python API audited from langchain-core at /langchain/libs/core/langchain_core/
