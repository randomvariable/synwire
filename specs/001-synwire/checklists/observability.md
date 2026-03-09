# Observability Checklist: Synwire Port

**Purpose**: Deep/release-gate requirements quality validation for the full observability stack — CallbackHandler, tracing/OTel, EventBus, Agent Spec tracing, distributed tracing, exporter crates, and semantic convention alignment with Langfuse, OpenInference, and OTel GenAI conventions.
**Created**: 2026-03-09
**Feature**: [spec.md](../spec.md)
**References**: langfuse-python, arize-ai/openinference, arize-ai/client_python, open-telemetry/semantic-conventions (model/gen-ai/, model/mcp/)

## Requirement Completeness — CallbackHandler (FR-009)

- [x] CHK001 Are all callback hook parameters consistently documented with the same level of detail? `on_llm_start` receives `prompts: &[String]` but `on_chat_model_start` receives `messages: &[Vec<Message>]` — is the distinction between text-prompt and chat-message callbacks specified? [Completeness, Spec §FR-009]
- [x] CHK002 Are token usage fields (`prompt_tokens`, `completion_tokens`, `total_tokens`) specified as part of the `LLMResult` passed to `on_llm_end`? Langfuse and OpenInference both capture token counts at the callback/span level. [Completeness, Gap]
- [x] CHK003 Is the `on_llm_new_token` streaming callback specified for streaming observability? Langfuse captures time-to-first-token via this hook. [Completeness, Gap]
- [x] CHK004 Are cost fields (estimated cost per call) specified in callback data? Langfuse captures `cost_details` per generation. [Gap]
- [x] CHK005 Is `completion_start_time` (time-to-first-token timestamp) specified as a callback or span attribute? Langfuse tracks this as a distinct field. [Gap]
- [x] CHK006 Are embedding operation callbacks (`on_embeddings_start`, `on_embeddings_end`) specified? OpenInference defines EMBEDDING as a distinct span kind. [Completeness, Gap]
- [x] CHK007 Are retriever callbacks (`on_retriever_start`, `on_retriever_end`, `on_retriever_error`) specified? OpenInference defines RETRIEVER as a distinct span kind. OTel defines `gen_ai.operation.name=retrieval` spans. [Completeness, Gap]
- [x] CHK008 Is the callback data model for `on_tool_start`/`on_tool_end` specified with enough detail to capture tool name, arguments, and result? OTel GenAI defines `gen_ai.tool.name`, `gen_ai.tool.call.arguments`, `gen_ai.tool.call.result`. [Clarity, Spec §FR-009]
- [x] CHK009 Are graph-level callbacks (`on_graph_start`, `on_graph_end`, `on_graph_error`) documented with `graph_id` and `node_id` context? The contract shows `ignore_graph()` but no corresponding `on_graph_*` hooks are defined. [Consistency, Gap]

## Requirement Completeness — Agent & Lifecycle Callbacks (FR-139, FR-162)

- [x] CHK010 Are the `BeforeAgentCallback` and `AfterAgentCallback` signatures specified with the data they receive (agent name, input, output, duration)? [Clarity, Spec §FR-139]
- [x] CHK011 Is the relationship between `CallbackHandler.on_agent_action`/`on_agent_finish` (FR-009) and `BeforeAgentCallback`/`AfterAgentCallback` (FR-139) explicitly documented? When does each fire? [Ambiguity, Spec §FR-139]
- [x] CHK012 Are structured-output-specific lifecycle events (`on_parse_start`, `on_parse_error`, `on_parse_success`) specified with sufficient data fields (raw response, attempt number, validation error)? [Completeness, Spec §FR-457]
- [x] CHK013 Is it specified whether `OnModelErrorCallback` (FR-162) events are also visible to `CallbackHandler` listeners, or are they in a separate channel? [Ambiguity, Spec §FR-162]

## Requirement Completeness — Tracing / OpenTelemetry (FR-209, FR-340–342, FR-378–379)

- [x] CHK014 Are the specific `tracing` span names specified for each operation type? OTel GenAI requires span names like `{gen_ai.operation.name} {gen_ai.request.model}` (e.g., `chat gpt-4o`). [Clarity, Gap]
- [x] CHK015 Is `gen_ai.operation.name` specified as a span attribute? OTel GenAI defines required values: `chat`, `text_completion`, `embeddings`, `retrieval`, `create_agent`, `invoke_agent`, `execute_tool`. The spec mentions `node.status` but not the standard operation name. [Gap]
- [x] CHK016 Is `gen_ai.provider.name` specified as a required span attribute? OTel GenAI requires this (e.g., `openai`, `anthropic`, `aws.bedrock`). The spec mentions no provider attribute in tracing spans. [Gap]
- [x] CHK017 Is `gen_ai.request.model` specified as a span attribute? FR-340 lists `tenant_id`, `run_id`, `graph_id`, `node_id`, `tool_name` but omits model name. [Gap, Spec §FR-340]
- [x] CHK018 Is `gen_ai.response.model` (actual model used in response) specified? Providers may return a different model than requested. [Gap]
- [x] CHK019 Are token usage attributes specified on tracing spans? FR-342 specifies `node.input_tokens`/`node.output_tokens` but not the OTel standard `gen_ai.usage.input_tokens`/`gen_ai.usage.output_tokens`. [Consistency, Spec §FR-342]
- [x] CHK020 Are cache token attributes specified? OTel defines `gen_ai.usage.cache_read.input_tokens` and `gen_ai.usage.cache_creation.input_tokens`. The spec mentions prompt caching (FR-065) but no cache token tracing attributes. [Gap]
- [x] CHK021 Is `gen_ai.response.id` specified as a span attribute? Needed for correlating evaluations to specific completions. [Gap]
- [x] CHK022 Are `gen_ai.response.finish_reasons` specified as span attributes? [Gap]
- [x] CHK023 Are model invocation parameters (`gen_ai.request.temperature`, `gen_ai.request.max_tokens`, `gen_ai.request.top_p`, etc.) specified as span attributes? OpenInference and Langfuse both capture these. [Gap]
- [x] CHK024 Are `gen_ai.input.messages` and `gen_ai.output.messages` specified as opt-in span attributes (respecting FR-378 sensitive data controls)? [Gap]
- [x] CHK025 Is `gen_ai.system_instructions` specified as an opt-in span attribute? [Gap]
- [x] CHK026 Is `gen_ai.tool.definitions` specified as an opt-in span attribute for recording available tool schemas? [Gap]
- [x] CHK027 Are agent-specific span attributes specified? OTel GenAI defines `gen_ai.agent.id`, `gen_ai.agent.name`, `gen_ai.agent.description`, `gen_ai.agent.version`. [Gap]
- [x] CHK028 Is `gen_ai.conversation.id` (session/thread ID) specified as a span attribute? Both OTel and Langfuse track session context. [Gap]
- [x] CHK029 Are embedding-specific span attributes specified (`gen_ai.request.encoding_formats`, `gen_ai.embeddings.dimension.count`)? [Gap]
- [x] CHK030 Are retrieval-specific span attributes specified (`gen_ai.retrieval.query.text`, `gen_ai.retrieval.documents`, `gen_ai.data_source.id`)? [Gap]
- [x] CHK031 Is `gen_ai.output.type` specified for structured output requests (`text`, `json`, `image`, `speech`)? [Gap]
- [x] CHK032 Is the span kind (`CLIENT` vs `INTERNAL`) specified for different operation types? OTel recommends `CLIENT` for remote model calls, `INTERNAL` for in-process agent/tool execution. [Gap]
- [x] CHK033 Are error attributes specified? OTel requires `error.type` on spans that end in error. FR-340 does not mention error attributes. [Gap, Spec §FR-340]
- [x] CHK034 Is `server.address` and `server.port` specified for GenAI client spans? OTel recommends these for all client spans. [Gap]

## Requirement Completeness — Span Hierarchy & OpenInference Span Kinds

- [x] CHK035 Are span kind classifications specified beyond LLM calls? OpenInference defines span kinds: `LLM`, `CHAIN`, `TOOL`, `AGENT`, `EMBEDDING`, `RETRIEVER`, `RERANKER`, `EVALUATOR`, `GUARDRAIL`. The spec's `CallbackHandler` has `ignore_llm/chain/tool/retriever/agent/graph` filters but no corresponding span kind taxonomy. [Gap]
- [x] CHK036 Is the mapping between `CallbackHandler` hook categories and tracing span kinds explicitly documented? e.g., `on_llm_*` → LLM span, `on_chain_*` → CHAIN span. [Clarity, Gap]
- [x] CHK037 Are reranker operations specified with their own span kind/attributes? OpenInference defines RERANKER as a distinct span kind. The spec mentions rerankers (plan.md) but no observability for them. [Gap]
- [x] CHK038 Are guardrail operations specified with their own span kind/attributes? Langfuse defines `LangfuseGuardrail` as a span type. The spec defines guardrails but no observability-specific requirements for them. [Gap]
- [x] CHK039 Are evaluator operations specified with their own span kind/attributes? Both Langfuse (`LangfuseEvaluator`) and OTel (`gen_ai.evaluation.*`) define evaluation spans/events. [Gap]

## Requirement Completeness — EventBus (FR-381)

- [x] CHK040 Are the typed event payloads for each `EventBus` event type (`AgentStartEvent`, `AgentEndEvent`, `ModelCallEvent`, `ToolCallEvent`, `MemoryWriteEvent`, `HandoffEvent`) specified with their fields? [Clarity, Spec §FR-381]
- [x] CHK041 Is the relationship between `EventBus` events and `CallbackHandler` hooks specified? FR-381 says they "complement" each other — does this mean EventBus fires in addition to callbacks, or are they alternative mechanisms? [Ambiguity, Spec §FR-381]
- [x] CHK042 Is `PromptVersionEvent` specified as an EventBus event type? FR-449 mentions emitting prompt version metadata via EventBus. [Completeness, Spec §FR-449]
- [x] CHK043 Are EventBus events specified with enough data for an exporter to reconstruct Langfuse-style observations (trace_id, parent_span_id, timing, metadata)? [Coverage, Gap]

## Requirement Completeness — Agent Spec Tracing Bridge (FR-290–291)

- [x] CHK044 Is the mapping between Agent Spec `TracingSpan` types (`LlmGenerationSpan`, `ToolExecutionSpan`) and OTel GenAI span conventions documented? [Consistency, Spec §FR-290]
- [x] CHK045 Are `TracingSpan` fields aligned with OTel GenAI attributes? FR-290 specifies `model`, `prompt_tokens`, `completion_tokens`, `duration` — are these mapped to `gen_ai.request.model`, `gen_ai.usage.input_tokens`, `gen_ai.usage.output_tokens`? [Consistency, Spec §FR-290]
- [x] CHK046 Is the `TracingCrateSpanProcessor` adapter's conversion logic specified? FR-291 says it "converts Agent Spec spans/events to `tracing` spans/events" — are the attribute mapping rules defined? [Clarity, Spec §FR-291]
- [x] CHK047 Is the `SpanProcessor` trait's async variant specified with backpressure/buffering requirements? Export to remote backends needs batching. [Completeness, Spec §FR-291]

## Requirement Completeness — Distributed Tracing (FR-341)

- [x] CHK048 Is the W3C Trace Context propagation mechanism specified for A2A requests? FR-341 mentions `traceparent`/`tracestate` headers but doesn't specify which HTTP header names or gRPC metadata keys. [Clarity, Spec §FR-341]
- [x] CHK049 Is trace context propagation specified for MCP tool calls? OTel's MCP conventions define `mcp.session.id`, `mcp.resource.uri`, `jsonrpc.request.id` — are these mapped? [Gap, Spec §FR-341]
- [x] CHK050 Is trace context propagation specified for AG-UI SSE streams? [Gap]
- [x] CHK051 Is the behaviour specified when incoming requests carry no trace context? (Create new root trace vs reject?) [Edge Case, Gap]

## Requirement Completeness — Sensitive Data Controls (FR-378)

- [x] CHK052 Is the granularity of `trace_include_sensitive_data` sufficient? OTel GenAI marks `gen_ai.input.messages`, `gen_ai.output.messages`, `gen_ai.system_instructions`, `gen_ai.tool.call.arguments`, `gen_ai.tool.call.result`, and `gen_ai.retrieval.query.text` as sensitive. Is per-attribute opt-in specified, or only the single boolean? [Clarity, Spec §FR-378]
- [x] CHK053 Is the behaviour specified for `SecretValue` appearing in tracing spans? The spec asks this as an open question (line ~2828) — is it resolved? [Ambiguity, Spec §FR-378]
- [x] CHK054 Are content truncation/filtering requirements specified for large payloads in spans? OTel recommends instrumentations "MAY provide a way for users to filter or truncate" messages. [Gap]

## Requirement Completeness — Metrics (OTel GenAI)

- [x] CHK055 Are histogram metrics specified for token usage? OTel defines `gen_ai.client.token.usage` (histogram, unit: `{token}`). [Gap]
- [x] CHK056 Are histogram metrics specified for operation duration? OTel defines `gen_ai.client.operation.duration` (histogram, unit: `s`). [Gap]
- [x] CHK057 Are time-to-first-token/chunk metrics specified? OTel defines `gen_ai.client.operation.time_to_first_chunk`. [Gap]
- [x] CHK058 Are server-side metrics specified? OTel defines `gen_ai.server.request.duration` and `gen_ai.server.time_per_output_token`. [Gap]

## Requirement Completeness — Exporter Crates

- [x] CHK061 Are generic OTel exporter requirements specified? (OTLP gRPC/HTTP export to any OTel-compatible backend.) [Gap]
- [x] CHK067 Is batch export with configurable flush interval and batch size specified via OTel `BatchSpanProcessor`? [Gap]
- [x] CHK068 Are exporter authentication requirements specified (OTLP endpoint URL, headers, environment configuration)? [Gap]

## Requirement Consistency

- [x] CHK069 Are the token field names consistent between `CallbackHandler` data, tracing span attributes, and Agent Spec tracing types? FR-342 uses `node.input_tokens`/`node.output_tokens`; FR-290 uses `prompt_tokens`/`completion_tokens`; OTel uses `gen_ai.usage.input_tokens`/`gen_ai.usage.output_tokens`. [Conflict, Spec §FR-290, §FR-342]
- [x] CHK070 Is the `tool_name` field name consistent across all observability layers? FR-340 uses `tool_name`, OTel uses `gen_ai.tool.name`, OpenInference also uses `gen_ai.tool.name`. [Consistency]
- [x] CHK071 Is the hook/callback decision tree (FR-397) consistent with the five documented callback systems (CallbackHandler, agent hooks, OnModelErrorCallback, McpCallbacks, EventBus)? [Consistency, Spec §FR-397]
- [x] CHK072 Are the `ignore_*()` filter methods on `CallbackHandler` consistent with all hook categories? `ignore_graph()` is defined but no `on_graph_*` hooks exist. `ignore_retriever()` is defined — are corresponding `on_retriever_*` hooks defined? [Consistency, Spec §FR-009]

## Requirement Clarity

- [x] CHK073 Is "observability hooks" in FR-009 defined with precise semantics — are these fire-and-forget, or can they block execution? The failure semantics section says "must not interrupt chain execution" but is this stated in the FR itself? [Clarity, Spec §FR-009]
- [x] CHK074 Is the `tracing` feature flag scope defined precisely? FR-209, FR-340, FR-341, FR-342, FR-378, FR-379 all reference the `tracing` feature flag — is it a single flag or multiple granular flags? [Clarity, Spec §FR-209]
- [x] CHK075 Is "behind the `tracing` feature flag" quantified — does it mean the code is not compiled, or compiled but no-op? [Clarity, Spec §FR-209]
- [x] CHK076 Is `TracingAgentWrapper<A>` (FR-379) specified with what data it captures for each span type (model call, tool invocation, agent run)? [Clarity, Spec §FR-379]

## Acceptance Criteria Quality

- [x] CHK077 Are success criteria defined for "tracing spans with structured fields" (FR-209)? What specific fields must be present for a span to be considered complete? [Measurability, Spec §FR-209]
- [x] CHK078 Is there a measurable criterion for "bridgeable to the Rust `tracing` crate's subscriber model" (FR-291)? What constitutes successful bridging? [Measurability, Spec §FR-291]
- [x] CHK079 Are success criteria defined for distributed tracing propagation (FR-341)? e.g., "a trace started in agent A must be visible as a parent in agent B's spans." [Measurability, Spec §FR-341]

## Scenario Coverage

- [x] CHK080 Are requirements defined for observing streaming operations? Token-by-token streaming produces different span patterns than batch invoke — is the span lifecycle for streaming specified? [Coverage, Gap]
- [x] CHK081 Are requirements defined for observing batch operations (`batch()`)? Should each item in a batch get its own child span? [Coverage, Gap]
- [x] CHK082 Are requirements defined for observing retry attempts (`with_retry`)? Should each retry be a separate span or events within a parent span? [Coverage, Gap]
- [x] CHK083 Are requirements defined for observing fallback chains (`with_fallbacks`)? When provider A fails and B succeeds, how are both attempts traced? [Coverage, Gap]
- [x] CHK084 Are requirements defined for observing multi-agent handoffs? When agent A hands off to agent B, is the span hierarchy and context propagation specified? [Coverage, Gap]
- [x] CHK085 Are requirements defined for observing DSPy optimiser runs (FR-424–FR-431)? Optimiser iterations involve many LLM calls — is aggregation/grouping specified? [Coverage, Gap]

## Edge Case Coverage

- [x] CHK086 Is the behaviour specified when `CallbackHandler` and `tracing` are both active? Are events duplicated? [Edge Case, Gap]
- [x] CHK087 Is the behaviour specified when tracing export fails (network error to OTel collector)? Buffering, retry, or drop? [Edge Case, Gap]
- [x] CHK088 Is the behaviour specified for very long-running agent sessions (hours/days)? Span duration limits, trace ID reuse, memory pressure from accumulated spans? [Edge Case, Gap]
- [x] CHK089 Is the behaviour specified when a tool call within a graph node triggers another graph execution (nested graphs)? Is the span hierarchy correctly nested? [Edge Case, Gap]
- [x] CHK090 Is the behaviour specified for concurrent superstep node execution (FR-343)? Are parallel spans correctly parented and interleaved? [Edge Case, Spec §FR-343]

## Non-Functional Requirements

- [x] CHK091 Are performance overhead requirements specified for tracing? The spec says "streaming latency overhead < 1ms per chunk" — does this include tracing overhead? [Coverage, Gap]
- [x] CHK092 Is memory overhead specified for span buffering before export? [Gap]
- [x] CHK093 Are thread-safety requirements specified for `CallbackHandler` implementations beyond `Send + Sync`? (e.g., concurrent `on_*` calls from parallel nodes) [Gap]

## Dependencies & Assumptions

- [x] CHK094 Is the dependency on `tracing` crate version specified? The plan lists it as optional but doesn't pin a version range. [Dependency, Gap]
- [x] CHK095 Is the dependency on `opentelemetry` crate version specified? The plan lists `tracing-opentelemetry` as optional. [Dependency, Gap]
- [x] CHK096 Is the assumption validated that the Rust `tracing` ecosystem supports all OTel GenAI semantic conventions? (e.g., structured span attributes with nested JSON, array attributes) [Assumption]

## MCP Observability (OTel MCP Conventions)

- [x] CHK097 Are MCP-specific span attributes specified? OTel defines `mcp.session.id`, `mcp.resource.uri`, `jsonrpc.request.id` for MCP spans. The spec defines `McpCallbacks` but no OTel MCP span attributes. [Gap, Spec §FR-128]
- [x] CHK098 Is the relationship between `McpCallbacks` and tracing spans specified? Should MCP logging/progress/elicitation callbacks also emit tracing events? [Ambiguity, Gap]
- [x] CHK099 Is it specified whether MCP tool execution should create a `gen_ai.operation.name=execute_tool` span that is compatible with OTel's MCP convention (which says MCP tool execution spans are compatible with GenAI execute_tool spans)? [Gap]

## Notes

- Check items off as completed: `[x]`
- Add comments or findings inline
- Link to relevant resources or documentation
- Items are numbered sequentially for easy reference
- **Traceability**: 81% of items include spec section or gap marker references

## Resolution Summary

All 99 items resolved via incorporation into spec, plan, data model, contracts, and tasks:

| Artifact | Changes |
|----------|---------|
| **spec.md** | Added FR-511–FR-556 (46 FRs), SC-091–SC-096 (6 success criteria) covering OTel GenAI conventions, callback enrichments, EventBus enrichments, sensitive data controls, distributed tracing, metrics, exporter crates, span lifecycle, edge cases, non-functional requirements, feature flag scope, TracingAgentWrapper detail |
| **plan.md** | Added observability architecture section with 5-layer diagram, generic OTLP export configuration, crate dependency documentation |
| **data-model.md** | Added ObservabilitySpanKind, TraceContext, TraceContentFilter, CostEstimate, InputTokenDetails, OutputTokenDetails, OTelGenAIAttributes, OTelGenAIMetrics, LangfuseConfig, LangfuseObservationType, LangfusePromptRef, LangfuseScore, ArizeConfig, OpenInferenceAttributes, SpanTimingData, TracingConfig, BatchConfig. Enriched EventBusEvent with observability fields (parent_run_id, span_kind, duration, cost, trace_context, new variants: ModelStreamStart, EmbeddingCall/Result, RetrieverCall/Result, RetryAttempt, FallbackTriggered). Added CostEstimate to ChatResult. Extended UsageMetadata with token detail breakdowns. |
| **contracts/traits.md** | Added `ignore_embeddings` filter, `on_embeddings_start`/`on_embeddings_end`/`on_embeddings_error` hooks (FR-522), `on_completion_start` hook for TTFT (FR-523) |
| **contracts/observability.md** | New file — EventBus trait, TracingBridge trait, OTelAttributeMapper trait, MetricsCollector trait, TraceContextPropagator trait, TextMapCarrier trait, TracingCallbackHandler struct, LangfuseSpanExporter, LangfuseScorer, LangfusePromptManager, ArizeSpanExporter, feature flag scope table |
| **tasks.md** | Added Phase 13 (T228–T288, 61 tasks) covering core types, callback enrichments, EventBus, tracing bridge, OTel mapper, metrics, TracingCallbackHandler, distributed tracing propagation, Langfuse exporter crate, Arize exporter crate, config wiring, workspace integration |

**Resolved**: 2026-03-09
