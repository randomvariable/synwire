# Observability

## Overview
Distributed tracing, callbacks, metrics, event bus, and per-agent debug recording for production monitoring and development-time inspection. Built on OpenTelemetry with GenAI semantic conventions. All observability behind a single `tracing` feature flag.

## OTel GenAI Semantic Conventions (FR-511-521)
OTel GenAI semantic convention attributes for:
- LLM spans (model, provider, request/response attributes)
- Tool execution spans
- Agent lifecycle spans
- Embedding operation spans
- Retrieval operation spans

## Distributed Tracing (FR-532-535)
- W3C Trace Context propagation for A2A HTTP, A2A gRPC, and MCP HTTP requests (FR-532)
- MCP tool execution spans with OTel MCP semantic convention attributes: mcp.session.id, mcp.resource.uri, jsonrpc.request.id (FR-533)
- AG-UI SSE streams propagate trace context via traceparent header (FR-534)
- Missing trace context creates new root trace (default OTel behaviour) (FR-535)
- Structured context fields on spans: tenant_id, run_id, graph_id, node_id, tool_name (FR-340)
- W3C Trace Context propagation for A2A and MCP boundaries (FR-341)
- Per-node execution metrics emitted as tracing span attributes (FR-342)

## Callback System (FR-522-529)
- `CallbackHandler` embedding hooks: on_embeddings_start, on_embeddings_end, on_embeddings_error (FR-522)
- `LLMResult` includes token usage metadata and model identifier in on_llm_end (FR-523)
- Cost estimation: `CostEstimate` field on LLMResult/ChatResult (input_cost, output_cost, total_cost, currency) (FR-524)
- on_completion_start hook for time-to-first-token tracking (FR-525)
- Documentation of on_agent_action/on_agent_finish (per-step) vs BeforeAgentCallback/AfterAgentCallback (per-invocation) relationship (FR-526)
- `OnModelErrorCallback` events visible to CallbackHandler via on_llm_error (FR-527)

## Event Bus & Debug Recording (FR-528-529, FR-381, FR-588-591)

The event bus provides typed subscribe/publish for production event routing. Debug recording extends it with per-agent full-fidelity event capture for development-time inspection, without requiring OTel exporter configuration.

- `EventBus` type with typed subscribe/publish and feature-gated events (FR-381)
- Event types with specified payload fields: AgentStart, AgentEnd, ModelCall, ToolCall, MemoryWrite, Handoff, PromptVersion, Evaluation (FR-528)
- EventBus and CallbackHandler are independent complementary systems. Both fire for same operations (FR-529)
- `DebugRecorder` attachable to any agent or graph node via `agent.enable_debug()`. Records all signals received, actions taken, directives emitted (FR-557), state transitions, and tool call results. Stored in a bounded in-memory ring buffer (default 1000 events) (FR-588)
- `agent.debug_events() -> &[DebugEvent]` accessor for inspecting recorded events. `DebugEvent` includes timestamp, event_type (SignalReceived, ActionExecuted, DirectiveEmitted, StateTransition, ToolResult, Error), and payload. Filterable by event type and time range (FR-589)
- Debug recording is per-agent, not global — only agents with debug enabled incur memory overhead. Multiple agents can be independently debugged. Instance-level `SynwireInstance::enable_debug()` (FR-584) enables recording across all agents in an instance (FR-590)
- Debug events feed into EventBus when debug mode is active, enabling external consumers to subscribe to debug streams. Debug events use a distinct `DebugEvent` type that is not emitted when debug mode is off (FR-591)

## Tracing Configuration (FR-378-380, FR-530-531)
- `trace_include_sensitive_data: bool` for omitting LLM I/O from spans (FR-378)
- `TracingAgentWrapper<A>` for OpenTelemetry instrumentation (FR-379)
- Streaming event content_category: ContentCategory for primary vs secondary content distinction (FR-380)
- `TraceContentFilter` per-attribute granularity and SecretValue auto-redaction in spans (FR-530-531)

## Metrics (FR-536, FR-320-322)
- OTel GenAI metrics: gen_ai.client.token.usage, gen_ai.client.operation.duration, gen_ai.client.operation.time_to_first_chunk (FR-536)
- Aggregate execution metrics: total input/output/total tokens, model invocations, duration (FR-320)
- Per-node execution metrics: duration, input_tokens, output_tokens, retries (FR-321)
- `QuotaEnforcer` trait checked before each LLM invocation (FR-322)

## OTLP Export (FR-540-541)
- Generic OTLP export support via opentelemetry-otlp, feature-gated behind otlp (FR-540-541)

## Span Lifecycle (FR-542-551)
- Streaming spans (FR-542)
- Batch operation spans (FR-543)
- Retry spans (FR-544)
- Fallback spans (FR-545)
- Multi-agent handoff spans (FR-546)
- Callback/tracing independence (FR-547)
- Export failure handling (FR-548)
- Long-running session spans (FR-549)
- Nested graph spans (FR-550)
- Concurrent superstep spans (FR-551)

## Non-Functional Requirements (FR-552-556)
- < 50us per span creation overhead (FR-552)
- Concurrent callback safety (FR-553)
- Bounded attribute memory (FR-554)
- Single `tracing` feature flag (FR-555)
- TracingAgentWrapper span detail specification (FR-556)

## Documentation Requirements
- Decision tree for callback and hook systems (FR-351, FR-397)
- Documentation of retry mechanism relationships (FR-352)
- Terminology glossary: Middleware, Hooks, Callbacks, Plugins (FR-398)
- Consistent execution semantics across Agent, Runner, Pregel (FR-399)

## Success Criteria
- **SC-091**: LLM span includes OTel GenAI attributes
- **SC-092**: Sensitive data exclusion verified
- **SC-093**: Distributed tracing across A2A agents
- **SC-094**: OTLP exporter sends correct spans
- **SC-095**: OTel GenAI metrics emitted
- **SC-096**: Embedding callbacks produce correct spans
- **SC-062**: Sensitive data redaction omits LLM I/O from traces when configured
- **SC-052**: Aggregate token tracking correctly sums across multiple LLM invocations
- **SC-112**: DebugRecorder captures all event types for a debug-enabled agent
- **SC-113**: Debug recording disabled by default — no memory overhead for non-debug agents
- **SC-114**: Debug events are accessible via `debug_events()` and filterable by type
