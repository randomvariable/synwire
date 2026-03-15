# A2A Protocol

## Overview
The `synwire-a2a` crate implements the Agent-to-Agent (A2A) protocol for inter-agent communication. Provides client and server implementations, transport bindings (JSON-RPC, REST, gRPC), and Synwire agent integration. M3 feature (FR-164).

## Agent Card (FR-165-167)
Self-describing agent manifest:
- name, description, version, provider, capabilities, skills, interfaces, security
- `AgentCapabilities`: supports_streaming, supports_push_notifications, supports_extended_card, supported_extensions (FR-166)
- `AgentSkill` with name, description, input/output modes, examples, tags (FR-167)
- Discovery via `resolve_agent_card()` from URL (FR-199)

## Task Lifecycle (FR-168-171)
Task state machine with 8 states: Submitted, Working, InputRequired, AuthRequired, Completed, Failed, Canceled, Rejected (FR-168).
- Terminal state enforcement — no modifications after Completed/Failed/Canceled/Rejected (FR-169)
- `InputRequired` pauses execution awaiting additional client input (FR-170)
- `AuthRequired` signals credential escalation needed (FR-171)
- Task cancellation transitions to Canceled state (FR-201)
- Task history queries via list_tasks() with context ID filtering (FR-202)
- Task-to-task referencing via references field (FR-185)

## Content Model (FR-190-191)
- `Part` content types: Text, Raw, Data, Url (FR-190)
- Artifact streaming with append semantics and last_chunk flag (FR-191)
- Client-specified accepted output modes via SendMessageConfig (FR-192)

## Transport Bindings (FR-175-179)
Multiple transports with client-side preference ordering:

### JSON-RPC 2.0 (FR-176)
HTTP POST with SSE streaming. Methods: message/send, message/stream, tasks/get, tasks/list, tasks/cancel, etc.

### REST
Standard HTTP REST endpoints.

### gRPC (FR-177)
Separate feature/crate with server-streaming RPCs.

- Consistent error mapping across transports: JSON-RPC codes, RFC 7807, gRPC status (FR-178)
- Protocol version negotiation with VersionNotSupported error (FR-179)
- Backward compatibility for protocol version migration (FR-203)

## Client (FR-180-182)
- Client factories: `A2AClient::from_card()`, `from_endpoints()`, `A2AClientFactory` (FR-180)
- Call interceptors with before()/after() hooks (FR-181)
- `CredentialStore` trait for per-session auth credential management. InMemoryCredentialStore + AuthInterceptor (FR-182)

## Server (FR-183-189)
- `AgentExecutor` trait: execute(), cancel() (FR-183)
- `ExecutorContext` with triggering message, task ID, context ID, stored task, related tasks, metadata, user/tenant info (FR-184)
- Optimistic concurrency control for task mutations with version numbers (FR-186)
- Configurable concurrency limits for simultaneous task execution (FR-187)
- Cluster/distributed execution mode with work queue for horizontal scaling (FR-188)
- Configurable panic handlers for execution and transport panics (FR-189)
- Multi-tenant agent serving via tenant-aware request routing (FR-200)

## Push Notifications (FR-172-174)
- `PushConfig` with webhook URL, auth, token (FR-172)
- `PushAuthInfo` with scheme + credentials (FR-173)
- CRUD operations for per-task push notification configuration (FR-174)

## Security (FR-193-195)
- Security scheme types: ApiKey, HttpAuth, OAuth2, OpenIdConnect, MutualTls (FR-193)
- OAuth2 flow types: AuthorizationCode, ClientCredentials, Implicit, Password, DeviceCode (FR-194)
- Composable security requirements: OR-list of AND-sets (FR-195)

## HITL Permission Flow (FR-099-102)
- A2A server HITL permission request flow using InputRequired task state (FR-099)
- Plan tracking — auto-approve write_todos status-only updates after initial plan approval (FR-100)
- ToolKind enum (read, edit, search, execute, other) for permission UIs (FR-101)
- ExecutionMode (interactive vs headless) stored in task metadata (FR-102)

## Error Handling (FR-196-197, FR-204-208)
- `A2AError` catalogue: ParseError, InvalidRequest, TaskNotFound, TaskNotCancelable, Unauthenticated, VersionNotSupported, etc. (FR-196)
- Structured error details with human-readable message and machine-readable details map (FR-197)
- Concurrent modification safety with optimistic concurrency (FR-204)
- Runner invocation-level timeout distinct from per-tool timeouts (FR-205)
- Graceful partial delivery failure handling on client disconnect (FR-206)
- Panic catching from user-supplied agent code with configurable PanicHandler (FR-207)
- Graceful session shutdown with in-progress invocation draining (FR-208)

## Observability (FR-209-212)
- Agent-specific telemetry via optional tracing integration, behind feature flag (FR-209)
- Configurable max_tool_concurrency (default: unbounded) (FR-210)
- Debug inspection API for agent state, event history, telemetry (FR-211)
- Evaluation API endpoint on A2A REST server (FR-212)

## Success Criteria
- **SC-016**: A2A client connects to server, sends task, receives streaming events, handles HITL permission
- **SC-028**: A2A server with AgentCard, client connection, task send, streaming updates
- **SC-029**: Task lifecycle: create → Working → InputRequired → resume → Completed
- **SC-030**: Multi-transport: same agent accessible via JSON-RPC and REST
- **SC-036**: synwire-a2a tests pass with ≥ 80% line coverage
- **SC-037**: Zero unsafe blocks in synwire-a2a

## Research Findings (from adk-a2a-parity)
All 85+ parity items resolved. Key decisions:
- A2A protocol types closely mirror the A2A specification
- AgentExecutor trait is user-implemented (not auto-generated)
- Optimistic concurrency for task mutations prevents lost updates
- Cluster mode with work queue enables horizontal scaling
- Synwire ↔ A2A content conversion functions provided for seamless integration
