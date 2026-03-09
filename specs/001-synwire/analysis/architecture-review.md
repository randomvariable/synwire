# Synwire Architecture Review

**Date**: 2026-03-09
**Scope**: All design documents in `specs/001-synwire/`
**Method**: Council review — 5 specialist reviewers (Rust, Architecture, Security, API Design, Observability)

---

## Executive Summary

Synwire is an ambitious unified Rust port of LangChain Python, LangGraph Python, and multiple agent protocols (A2A, AG-UI, MCP, Agent Spec) into a single Cargo workspace. The specification contains **450+ functional requirements** across **25+ crates** with an estimated 45–60k lines of Rust.

The design demonstrates strong domain knowledge and thorough edge-case coverage. However, the review council identified **critical risks in scope management, type safety, security, and API complexity** that should be addressed before implementation begins.

**Top-level verdict**: The core architecture (traits, orchestrator, agent builder) is sound. The risk lies in doing everything at once, carrying Python idioms that fight Rust's type system, and under-specifying security boundaries for LLM-controlled execution.

---

## 1. Critical Findings (P0)

### 1.1 Scope Is Unsustainable as a Single Release

**Source**: Architecture, API Design

The spec defines 450+ FRs, 4 external protocols, 3 evaluation frameworks, a DSPy port, a CLI, and sandbox systems. This is an ecosystem, not a library. Attempting to ship all of this simultaneously guarantees either years of development or a shallow implementation across every surface.

**Recommendation**: Hard-phase into three milestones with independent utility:

| Milestone | Crates | Value |
|-----------|--------|-------|
| **M1: Core + Orchestrator** | synwire-core, synwire-orchestrator, synwire-checkpoint-sqlite, synwire-llm-openai, synwire-llm-ollama, synwire, synwire-derive | LangChain + LangGraph core. Competitive with `rig`. |
| **M2: Agents + MCP** | synwire-agents, synwire-sandbox-local, synwire-mcp-adapters, synwire-cli | Agentic workflows with MCP tool interop. |
| **M3: Protocols + DSPy + Evals** | synwire-a2a, synwire-ag-ui, synwire-agent-spec, synwire-dspy, synwire-evals | Full ecosystem. |

Do not build M2/M3 features until M1 has real users and passing integration tests.

### 1.2 `serde_json::Value` as Universal Graph State Erases Type Safety

**Source**: Rust Expert

The `CompiledGraph` implements `Runnable<Value, Value>`. Graph state is serialised to/from `serde_json::Value` at every node boundary. This means a node writing `{"messages": 42}` instead of `{"messages": [...]}` fails only at runtime.

**Costs**: No compile-time safety, serialisation overhead at every boundary, deep-clone cost for large conversation state.

**Recommendation**: Keep `StateGraph<S>` and `CompiledGraph<S>` generic over `S: State` through compilation. Erase to `Value` only at serialisation boundaries (checkpoint save/load, HTTP API). Consider `Arc<Value>` with structural sharing for checkpoint copies.

### 1.3 `RunnableConfig` Is Not Cloneable

**Source**: Rust Expert

`RunnableConfig` holds `Vec<Box<dyn CallbackHandler>>` which is not `Clone`. Every graph node needs a config. This is a blocking design issue that surfaces immediately during implementation.

**Recommendation**: Switch to `Arc<dyn CallbackHandler>` throughout. Make `RunnableConfig` cheaply cloneable. Pass by value or `Arc<RunnableConfig>` instead of `&RunnableConfig` to eliminate lifetime coupling with async futures.

### 1.4 Headless Mode Has No Safe Default Approval Policy

**Source**: Security

In headless mode (FR-102), "tool calls requiring approval are auto-approved or fail based on policy." The spec does not define a default policy. If auto-approve is default, headless agents get unrestricted execution including shell commands via `LocalShellBackend`.

**Recommendation**: Mandate fail-closed default. Require operators to explicitly configure which tools are auto-approved in headless mode. No implicit blanket approval.

### 1.5 `always_approve` Ledger Escalates from Single Approval to Blanket Access

**Source**: Security

The approval ledger (FR-392) is keyed by tool name only. Approving `execute` once with `always: true` auto-approves ALL subsequent shell commands. Attack: benign `ls` approved with "always" → followed by `rm -rf /`.

**Recommendation**: Scope the ledger by tool name + argument pattern hash. Add `max_auto_approvals` counter. Never allow "always approve" on `execute` without argument constraints.

### 1.6 Tenant ID Used as Key Prefix Without Sanitisation

**Source**: Security

Tenant isolation uses `tenant_id` as a string prefix/partition key (FR-328). No format validation is specified. SQL injection in tenant_id could break isolation. Additionally, missing `tenant_id` silently returns all tenants' data.

**Recommendation**: Validate tenant_id format (alphanumeric + hyphens, max length). Use parameterised queries. Make `tenant_id` required (not `Option`) for multi-tenant deployments with a separate admin API path.

---

## 2. High-Priority Findings (P1)

### 2.1 Single `SynwireError` Enum Will Grow Unbounded

**Source**: Rust Expert

As crates are added (A2A, AG-UI, Agent Spec, DSPy, MCP), the error enum grows to 30+ variants. Changes to any variant force recompilation of every dependent crate. A flat enum also loses causation chains.

**Recommendation**: Layered error types per domain (`ModelError`, `GraphError`, `ToolError`, etc.) with a top-level `SynwireError` wrapping via `#[from]`. Each crate defines its own error type. Use `#[non_exhaustive]` from day one.

### 2.2 `Runnable<I, O>` Trait Is Too Wide

**Source**: Rust Expert, API Design

The trait has 8 methods: `invoke`, `batch`, `stream`, `transform`, `batch_as_completed`, `stream_events`, `stream_log`, and composition methods. Every implementor pays the vtable cost. `stream_log` is a Python/LangSmith legacy with no Rust equivalent need.

**Recommendation**: Split into `RunnableCore` (invoke, batch, stream) + `ObservableRunnable` (stream_events) as extension traits. Drop `stream_log` entirely. Default-implement all non-core methods.

### 2.3 Six Naming Collisions on "Agent"

**Source**: API Design

A developer searching "how to make an agent" finds: `Agent<D,O>`, `AgentExecutor`, `AgentNode`, `create_react_agent()`, `create_agent()`, and `Runner`. The spec says `AgentExecutor` is "superseded" yet mandates it as MUST.

**Recommendation**: Deprecate `AgentExecutor` from MUST to MAY. Rename `create_agent()` to `create_coding_agent()` (it specifically creates agents with file/shell tools). Lead all documentation with `Agent::builder()`.

### 2.4 Checkpoint Data Stored Unencrypted

**Source**: Security

Explicitly acknowledged as out of scope, but checkpoints contain full conversation state including user messages, tool results, and potentially sensitive data. `SecretValue` checkpoint serialisation behaviour is undefined — either secrets are stored in plaintext or lost on restore.

**Recommendation**: Specify that `SecretValue` in checkpoints serialises as a sentinel reference. Secrets are re-fetched from `CredentialProvider` on restore. Add SQLite file permissions (`0600`). Plan `EncryptedSerializer` for M2.

### 2.5 `SecretValue` Not Zeroised on Drop

**Source**: Security

`SecretValue` wraps a `String` that leaves data in the heap allocator's free list after drop. Memory dumps or core dumps expose credentials.

**Recommendation**: Adopt the `secrecy` crate (or `zeroize`) for `SecretValue` to ensure memory zeroisation on drop.

### 2.6 DNS Rebinding Possible Without IP Pinning

**Source**: Security

The SSRF client resolves DNS before connecting, but if `reqwest` performs a second lookup at connection time (common with connection pools), DNS rebinding returns a public IP first and a private IP second.

**Recommendation**: Resolve once, pin the resolved IP, connect to that exact IP. Also block IPv4-mapped IPv6 addresses (`::ffff:10.0.0.1`) and tunnelling schemes.

### 2.7 MCP Stdio Spawns Unvalidated Subprocess Commands

**Source**: Security

MCP stdio transport spawns processes with `command/args/env/cwd` from configuration. Agent Spec YAML can specify arbitrary commands. No validation or sandboxing.

**Recommendation**: Validate MCP stdio commands against an allowlist. When loaded from Agent Spec YAML, require explicit operator approval for subprocess commands.

### 2.8 YAML Billion-Laughs Not Mitigated

**Source**: Security

The spec mentions streaming parsing but does not specify YAML alias/anchor limits. `serde_yaml` does not protect against exponential expansion by default.

**Recommendation**: Configure YAML parser with recursion/expansion limits. Disable anchors for untrusted input. Consider `yaml-rust2` with explicit limits.

### 2.9 Compilation Time Risk

**Source**: Rust Expert

25+ crates, proc macros in the critical path (`synwire-derive` with `syn`/`quote`), 125+ types with serde derives. Estimated cold build: 8–15 minutes (release).

**Recommendation**: Make `synwire-derive` optional for `synwire-core`. Gate test utilities behind `test-utils` feature. Share workspace-level `reqwest` dependency. Use `#[cfg(test)]` aggressively.

### 2.10 Streaming Backpressure Underspecified

**Source**: Observability

Seven stream modes but no distinction between lossy (debug, messages) and lossless (updates, checkpoints, values). The `InMemoryEventBus` skips events for lagging subscribers — dangerous for state consistency.

**Recommendation**: Define lossless vs lossy semantics per stream mode. Enforce backpressure (suspend producers) for lossless modes. Allow event dropping only for lossy modes.

---

## 3. Medium-Priority Findings (P2)

### 3.1 Cancellation Safety Underspecified

Checkpoint writes during cancellation, interrupt persistence when futures are dropped, and stream cleanup semantics need explicit documentation per method.

### 3.2 `synwire-core` Is Too Large

Contains agents, extraction, guardrails, conversation managers, event bus, approval types, batch processing, knowledge base. The `Agent<D,O>` builder in core creates an implicit dependency on the orchestrator. Move non-foundational types to higher crates.

### 3.3 Five Callback/Hook Systems Need Ordering Guarantees

When CallbackHandler, EventBus, and plugins all observe the same event, ordering is undefined. Cancelled operations may not emit callback events. MCP callbacks are isolated from the EventBus with no default bridge.

### 3.4 Trait Design: BoxFuture vs RPITIT

Not all traits require dyn-dispatch. `MessageLike`, `OutputParser<T>`, and `Retriever` are candidates for static dispatch via `async fn` in traits (Rust 1.75+), eliminating heap allocation per call.

### 3.5 Multiple Overlapping Abstractions

| Concept | Count | Instances |
|---------|-------|-----------|
| Retry | 4 | RetryPolicy, RunnableRetry, ToolResult::Retry, RetryOutputParser |
| Output parsing | 4 | OutputParser, Adapter, OutputMode, with_structured_output |
| HITL | 4 | interrupt_before, with_confirmation, AG-UI HITL, A2A InputRequired |
| Evaluation | 3 | Harbor, Agent Spec Eval, synwire-evals scorers |
| Agent creation | 4+ | Agent::builder, create_react_agent, create_agent, AgentExecutor |

Each exists for a reason (inherited from Python's incremental layering), but in a greenfield Rust port, unification is possible. Define one canonical abstraction per concept with multiple implementations.

### 3.6 No Structured Log Correlation

Traces and metrics are covered but structured log correlation (`trace_id`/`span_id` in log events) is not specified. Essential for log-to-trace jumping in production.

### 3.7 Default `trace_include_sensitive_data: true` Is Unsafe

Out-of-the-box, all LLM inputs/outputs and tool arguments appear in traces. For a library targeting production, default should be `false`.

### 3.8 Token Tracking: Cross-Agent Rollup Unspecified

When a parent agent delegates to subagents, token counting and quota enforcement across the hierarchy is not defined. Parent agents need subagent tokens rolled up for total cost tracking.

### 3.9 Graceful Shutdown Ordering Undefined

FR-208 says "wait for invocations, close plugins, release resources" but does not order: EventBus flush, BatchSpanProcessor flush, MCP session close, checkpoint connection close.

### 3.10 Tool Kind Self-Declaration Enables Spoofing

`ToolKind` (read/edit/execute) is declared by the tool itself. A malicious MCP-provided tool could declare as `read` while performing destructive operations. Auto-approval policies based on `ToolKind` are therefore spoofable.

---

## 4. Lower-Priority Findings (P3)

### 4.1 API Ergonomics

- `filter_messages` has 7 parameters — use a builder pattern
- `Runnable<I, O>` takes input by value, forcing clones for reuse — consider `&I` or `Cow<I>`
- No typed template variables — `HashMap<String, Value>` is stringly-typed
- Quickstart should lead with `Agent::builder()`, not raw model invocation
- `VectorStore` trait has 10 methods — split into core + MMR extension

### 4.2 Feature Flags

- `tracing` should be a default feature (negligible overhead when no subscriber attached)
- Metrics and traces share one feature flag — consider splitting into `otel-metrics` and `otel-traces`

### 4.3 Testing Gaps

- No `TestSpanCollector` or `FakeEventBus` test utilities
- No CallbackHandler conformance suite
- No criterion benchmarks for tracing overhead targets (50µs/span)
- No fuzz targets for JSON/YAML parsers, checkpoint deserialisation
- Property test for DSPy types may conflict with `#[derive(Arbitrary)]` + `#[derive(Signature)]`

### 4.4 Operational Gaps

- No health check endpoint for A2A server
- No rate-limiting error response (HTTP 429) when concurrency limits reached
- No process-level concurrency limit for non-A2A graph executions
- No resource leak detection for long-running agents

### 4.5 Distributed Tracing Gaps

- MCP stdio transport has no trace context propagation mechanism (use `_meta` field)
- WebSocket per-message trace propagation undefined
- W3C Baggage not supported (useful for tenant_id, eval_run_id)
- Checkpoint-resume trace continuation semantics undefined

### 4.6 Debug Experience

- `to_mermaid()` shows static structure only — add execution-annotated variant
- Debug stream mode content schema undefined
- No `synwire.node.id` attribute on tracing spans for trace-to-graph mapping

### 4.7 Proc Macro Concerns

- `#[tool]` schema generation may reimplement `schemars` — consider depending on it
- `#[derive(Signature)]` field description source (doc comments vs attributes) unspecified
- `#[derive(State)]` resolving function names by string is fragile — use type-based approach

### 4.8 Zero Unsafe Scope

Realistic for `synwire-core` and `synwire-orchestrator`. Potentially unrealistic for sandbox crates (signal handlers, libc calls). Scope `#![forbid(unsafe_code)]` to core/orchestrator only.

---

## 5. Strategic Recommendations

### 5.1 Lead with the Agent Builder

`Agent::builder()` should be User Story 1, Quickstart Example 1, README Example 1. Everything else is "advanced." The current spec buries it at US9b (P2).

### 5.2 Accept That Some Python Patterns Don't Port

| Python Pattern | Rust Alternative |
|----------------|-----------------|
| LCEL `\|` operator | `Chain::new().then(a).then(b).build()` or proc-macro |
| `CallbackHandler` (20+ hooks) | `tracing` crate spans + EventBus |
| `stream_log` (JSON Patch) | Drop entirely |
| `HashMap<String, Value>` kwargs | Typed structs with derive macros |
| `RunnableParallel` → dict | Typed tuples or named outputs |

### 5.3 Invest in Proc Macros Early

`#[tool]`, `#[derive(Signature)]`, and `#[derive(State)]` are critical for making the Rust API competitive with Python's decorator patterns. Prioritise in M1.

### 5.4 Security Threat Model

Before M2 (agents with shell/filesystem access), conduct a formal threat model covering:
- LLM-controlled command execution boundaries
- Multi-tenant isolation verification
- Credential lifecycle (creation → use → rotation → zeroisation)
- Protocol-level authentication and transport security

### 5.5 Use `#[non_exhaustive]` Everywhere

On all enums, error types, and config structs from day one. This preserves semver flexibility as the API evolves.

---

## Appendix: Complete Finding Index

| ID | Priority | Area | Finding |
|----|----------|------|---------|
| 1.1 | P0 | Scope | 450+ FRs unsustainable as single release |
| 1.2 | P0 | Types | `Value` as universal graph state erases type safety |
| 1.3 | P0 | Types | `RunnableConfig` not cloneable |
| 1.4 | P0 | Security | Headless mode no safe default approval |
| 1.5 | P0 | Security | `always_approve` escalation to blanket access |
| 1.6 | P0 | Security | Tenant ID unsanitised key prefix |
| 2.1 | P1 | Rust | `SynwireError` enum will grow unbounded |
| 2.2 | P1 | Rust | `Runnable` trait too wide (8 methods) |
| 2.3 | P1 | API | 6 naming collisions on "Agent" |
| 2.4 | P1 | Security | Checkpoint data unencrypted |
| 2.5 | P1 | Security | `SecretValue` not zeroised on drop |
| 2.6 | P1 | Security | DNS rebinding via SSRF client |
| 2.7 | P1 | Security | MCP stdio unvalidated subprocess |
| 2.8 | P1 | Security | YAML billion-laughs |
| 2.9 | P1 | Rust | Compilation time risk (8–15 min release) |
| 2.10 | P1 | Observability | Streaming backpressure underspecified |
| 3.1 | P2 | Rust | Cancellation safety underspecified |
| 3.2 | P2 | Architecture | synwire-core too large |
| 3.3 | P2 | Observability | Callback ordering undefined |
| 3.4 | P2 | Rust | BoxFuture vs RPITIT audit needed |
| 3.5 | P2 | Architecture | Multiple overlapping abstractions |
| 3.6 | P2 | Observability | No structured log correlation |
| 3.7 | P2 | Security | Sensitive data traced by default |
| 3.8 | P2 | Observability | Cross-agent token rollup unspecified |
| 3.9 | P2 | Operations | Shutdown ordering undefined |
| 3.10 | P2 | Security | ToolKind self-declaration spoofable |
| 4.1 | P3 | API | Various ergonomic improvements |
| 4.2 | P3 | Rust | Feature flag granularity |
| 4.3 | P3 | Testing | Missing test utilities and fixtures |
| 4.4 | P3 | Operations | Health checks, rate limiting, leak detection |
| 4.5 | P3 | Observability | Distributed tracing gaps |
| 4.6 | P3 | Debug | Execution visualisation and span mapping |
| 4.7 | P3 | Rust | Proc macro design concerns |
| 4.8 | P3 | Rust | Zero unsafe scope clarity |
