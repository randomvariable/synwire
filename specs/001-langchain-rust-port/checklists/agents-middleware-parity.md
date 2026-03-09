# Agents/Middleware Parity Checklist: LangChain Rust Port

**Purpose**: Validate that spec, contracts, and data model adequately address parity with Python `langchain/agents/middleware` — the LangGraph-based agent middleware system
**Created**: 2026-03-09
**Feature**: [spec.md](../spec.md) | [contracts/traits.md](../contracts/traits.md) | [data-model.md](../data-model.md)
**Depth**: Rigorous | **Scope**: Agent middleware architecture + types
**Audience**: Reviewer (PR) | **Timing**: Pre-implementation gate
**Source**: `/langchain/libs/langchain_v1/langchain/agents/middleware`

## Scope & Architecture Decision

- [x] CHK071 Is the relationship between the Python `agents/middleware` system (LangGraph-based, decorator-driven) and the Rust `AgentExecutor` (classic ReAct loop) documented with a clear rationale for the divergence? [Clarity, Spec §Assumptions vs Python agents/middleware]
- [x] CHK072 Is the `agents/middleware` package explicitly documented as in-scope or out-of-scope in the spec? Current Rust design models classic ReAct agents, not the LangGraph middleware pipeline [Gap, Spec §Assumptions]
- [x] CHK073 If out-of-scope, is the exclusion rationale documented — e.g. that `agents/middleware` depends on LangGraph state management which is a separate system? [Completeness, Research §7]

## AgentMiddleware Base Class

- [x] CHK074 Is the `AgentMiddleware` base class pattern (hooks: `before_agent`, `after_agent`, `before_model`, `after_model`, `wrap_model_call`, `wrap_tool_call`) documented as excluded or mapped to an equivalent? Python uses this as an interceptor/decorator pipeline [Gap, Contracts §AgentExecutor]
- [x] CHK075 Is the middleware chaining pattern (ordered list of middleware applied sequentially) addressed? Python `AgentMiddleware` supports composable middleware stacks [Gap, data-model.md §AgentExecutor]

## Middleware Decorator Pattern

- [x] CHK076 Are the Python decorator-based middleware hooks (`@before_model`, `@after_model`, `@before_agent`, `@after_agent`, `@wrap_model_call`, `@wrap_tool_call`) documented as excluded or mapped? These enable lightweight middleware definition [Gap, Contracts]
- [x] CHK077 Is the Rust equivalent pattern for middleware-style interception specified? Possible Rust patterns include trait composition, tower-like Layers, or callback hooks — is a decision documented? [Gap, Research]

## Concrete Middleware Classes

- [x] CHK078 Is `ModelRetryMiddleware` (retry failed model calls with backoff) addressed? The Rust `RunnableRetry` covers retry at the Runnable level — is the distinction documented between model-level vs runnable-level retry? [Clarity, Contracts §Runnable vs Python ModelRetryMiddleware]
- [x] CHK079 Is `ToolRetryMiddleware` (retry failed tool calls) addressed? Related to but distinct from Runnable retry — operates at tool execution granularity [Gap, Contracts]
- [x] CHK080 Is `ModelFallbackMiddleware` (fallback to alternative models) addressed? The Rust `RunnableWithFallbacks` covers fallback at Runnable level — is the relationship documented? [Clarity, Contracts §Runnable vs Python ModelFallbackMiddleware]
- [x] CHK081 Is `ModelCallLimitMiddleware` (max model invocations per agent run) addressed? Rust `AgentExecutor.max_iterations` partially covers this — is the mapping documented? [Clarity, data-model.md §AgentExecutor]
- [x] CHK082 Is `ToolCallLimitMiddleware` (max tool invocations per agent run) addressed or excluded? No Rust equivalent documented [Gap, data-model.md §AgentExecutor]
- [x] CHK083 Is `HumanInTheLoopMiddleware` (approval gates requiring human confirmation before tool execution) documented as out-of-scope or mapped? [Gap, Contracts]
- [x] CHK084 Is `LLMToolEmulator` (emulates tool-calling for models without native function-calling) documented as out-of-scope or mapped? [Gap, Contracts]
- [x] CHK085 Is `LLMToolSelectorMiddleware` (filter/select available tools per step) documented as out-of-scope or mapped? [Gap, Contracts]

## Agent State & Types

- [x] CHK086 Is `AgentState` (TypedDict-based mutable state for LangGraph agents) documented as excluded? Rust `AgentInput` uses `HashMap<String, Value>` — is the divergence from LangGraph's typed state documented? [Clarity, data-model.md §AgentInput vs Python AgentState]
- [x] CHK087 Are `ModelRequest` and `ModelResponse` types (middleware-level request/response wrappers) documented as excluded or mapped? [Gap, data-model.md]
- [x] CHK088 Is `ExtendedModelResponse` (model response + metadata from middleware pipeline) documented as excluded? [Gap, data-model.md]

## Execution Policies

- [x] CHK089 Are execution policy types (`Host`, `CodexSandbox`, `Docker`) documented as out-of-scope? These control where tools execute in sandboxed environments [Gap, Spec §Assumptions]

## Utility Middleware (Application-Level)

- [x] CHK090 Are application-specific middleware classes (`PIIMiddleware`, `ShellToolMiddleware`, `SummarizationMiddleware`, `TodoListMiddleware`, `ContextEditingMiddleware`, `FilesystemFileSearchMiddleware`) documented as out-of-scope? These are higher-level application utilities, not core abstractions [Gap, Spec §Assumptions]

## Notes

- Check items off as completed: `[x]`
- Items marked [Gap] indicate missing requirements that need spec/contract updates
- Items marked [Clarity] indicate existing requirements needing more precision
- Items marked [Completeness] indicate partially specified requirements
- The Python `agents/middleware` system is part of the LangGraph-based agent architecture, which is architecturally distinct from the classic ReAct `AgentExecutor` pattern modelled in the Rust design docs
- Reference: Python API audited from `/langchain/libs/langchain_v1/langchain/agents/middleware`
