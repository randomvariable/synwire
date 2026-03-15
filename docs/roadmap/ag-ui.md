# AG-UI Protocol

## Overview

The `synwire-ag-ui` crate implements the AG-UI (Agent-to-UI) protocol for streaming agent events to frontend clients. It targets AG-UI protocol v1.0 (FR-254) and is an M3 feature with a prerequisite of the M2 stable release. The crate is UI-framework-agnostic, carrying no React or Vue dependency (FR-256).

## Event Model (FR-213–219)

### Event Types (FR-214)

Fourteen event types are organised into categories:

**Run Lifecycle**: `RunStarted`, `RunFinished`, `RunError`

**Step Lifecycle**: `StepStarted`, `StepFinished`

**Text Streaming**: A three-phase protocol (FR-216) consisting of `TextMessageStart`, `TextMessageContent`, and `TextMessageEnd`.

**Tool Calls**: A four-phase protocol (FR-217) consisting of `ToolCallStart`, `ToolCallArgs`, `ToolCallEnd`, and `ToolCallResult`.

**State Synchronisation**:
- `StateSnapshot` provides full state replacement (FR-218).
- `StateDelta` delivers RFC 6902 JSON Patch incremental updates (FR-218).
- `MessagesSnapshot` enables late-joining client catch-up (FR-219).

All events carry a `BaseEvent` header containing event_type, thread_id, run_id, timestamp, and raw_event (FR-215).

### Messages (FR-220)

`AgUiMessage` holds id, role, content, tool_calls, and name fields. `From` trait conversions provide interoperability with internal message types.

### Activity Events (FR-243)

`ActivityEvent` supports status and progress updates that fall outside the conversation history.

## SSE Transport (FR-221–223)

`EventEncoder` serialises events as SSE frames with event type headers and monotonic IDs (FR-221). The transport uses HTTP POST connections with a 15-second keep-alive heartbeat and graceful termination (FR-222). Resumable streaming is supported via `Last-Event-ID`, with event replay or snapshot catch-up on reconnection (FR-223).

WebSocket transport is available behind a feature flag (FR-255).

Performance targets include event latency overhead under 1ms and a zero-allocation `EventEncoder` for common types (FR-259). Authentication uses Bearer tokens with configurable CORS (FR-252). `AgUiProtocolVersion` handles version negotiation between client and server (FR-253).

## Frontend Tools (FR-224–229)

Frontend tools implement client-side tool execution with an inverted execution flow (FR-225):

1. The server emits `ToolCallStart`, `ToolCallArgs`, and `ToolCallEnd`.
2. The client executes the tool locally.
3. The client returns a `ToolCallResult`.
4. The agent continues processing.

`FrontendTool` carries name, description, schema, agent_id, and available fields (FR-224). Render states progress through InProgress, Executing, and Complete (FR-226). The `follow_up: bool` flag distinguishes continued processing from terminal responses (FR-227). Frontend tool execution times out after a configurable duration, defaulting to 30 seconds (FR-249). `ToolLocation` is an enum with `Server` and `Frontend` variants (FR-245).

### Human-in-the-Loop via Frontend Tools (FR-228–229)

HITL uses a render-based approval pattern: the agent emits a frontend tool call referencing an approval widget, and the client renders it for user interaction. HITL requests time out after a configurable period, defaulting to 5 minutes (FR-229).

## State Synchronisation (FR-218, FR-246, FR-250–251)

`AgUiState` represents shared state between agent and UI. `StateSnapshot` delivers full state while `StateDelta` provides incremental JSON Patch (RFC 6902) updates. State is scoped hierarchically: agent_id, thread_id, run_id, then state (FR-239).

When a `StateDelta` application fails, recovery is available via `POST /runs/:id/resync` (FR-250). Backpressure for large state objects is enforced through a configurable `max_state_size`, defaulting to 1MB (FR-251). Terminology is explicitly disambiguated between graph state, session state, and UI state (FR-246).

## Client (FR-230–232)

`AgUiClient` exposes `run()`, `abort()`, and `send_tool_result()` methods (FR-230). A subscription model provides real-time events on existing runs via `subscribe(thread_id)` (FR-231). Connection error handling uses exponential backoff with `Last-Event-ID` catch-up, surfacing a `ConnectionLost` error when recovery fails (FR-232).

Client capability negotiation supports graceful degradation when server and client feature sets differ (FR-247). Multiple simultaneous client subscriptions are supported with broadcast distribution and automatic cleanup (FR-248).

## Server Runtime (FR-233–234, FR-260)

`AgUiRuntime` provides the following HTTP endpoints (FR-234):

- `POST /runs` — start a new run.
- `POST /runs/:id/abort` — abort a running execution (FR-258).
- `POST /runs/:id/tool-results` — return frontend tool results.

`RunAgentInput` contains thread_id, run_id, messages, tools, state, config, and forwarded_props (FR-233). Concurrency is configurable via `max_concurrent_runs` (default 100) and `max_concurrent_connections` (default 1000), with HTTP 429 responses on excess (FR-260).

## Agent Adapter (FR-235–237)

The `AgentAdapter` trait wraps agents as AG-UI endpoints (FR-235). Implementations include `CompiledGraphAdapter`, `AgentExecutorAdapter`, and a generic `Agent<D,O>` adapter. Internal events are translated to AG-UI events automatically — for example, `TextDelta` becomes `TextMessageContent` (FR-236).

Multi-agent UI routing uses the `agent_id` field on events (FR-237). Multi-agent thread isolation assigns independent `thread_id` values per agent (FR-238).

## Generative UI (FR-241–242)

Agents emit tool calls referencing UI component names (FR-241). Render state transitions follow the InProgress, Executing, Complete sequence (FR-242). Server-side tool registration is declared to the client in run response metadata (FR-240).

## Success Criteria

- **SC-038**: AG-UI server delivers all 14 event types in a single run.
- **SC-039**: Agent modifies state, client receives delta, and final state matches.
- **SC-040**: Frontend tool end-to-end flow completes successfully.
- **SC-041**: Late join with snapshot catch-up works correctly.
- **SC-042**: synwire-ag-ui tests pass with at least 80% line coverage.
- **SC-043**: Zero `unsafe` blocks in synwire-ag-ui.

## Research Findings

All 67 parity items from the ag-ui-parity analysis have been resolved, producing 48 new feature requirements (FR-213 through FR-260). Key architectural decisions:

- SSE is the primary transport; WebSocket is available behind a feature flag.
- Frontend tools serve as the HITL mechanism rather than a separate approval protocol.
- State synchronisation uses JSON Patch (RFC 6902) for efficiency.
- The relationship to A2A is documented in FR-257: AG-UI handles agent-to-UI communication while A2A handles agent-to-agent communication.
