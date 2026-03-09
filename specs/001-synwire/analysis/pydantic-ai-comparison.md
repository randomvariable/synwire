# Pydantic-AI vs Synwire: Architectural Comparison & Recommendations

**Date**: 2026-03-09
**Context**: Pydantic-AI is gaining adoption as a simpler alternative to Synwire. This document compares its architecture with our synwire spec and makes recommendations.

---

## Executive Summary

Pydantic-AI and LangChain represent fundamentally different philosophies:

| Dimension | Pydantic-AI | LangChain / LangGraph |
|-----------|------------|----------------------|
| **Philosophy** | Agent-first, minimal abstractions | Composable primitives, maximum flexibility |
| **Core unit** | `Agent[Deps, Output]` | `Runnable[Input, Output]` |
| **Complexity** | ~15 key types | ~50+ traits, 26 crates |
| **Graph system** | Lightweight `pydantic_graph` (internal) | Full Pregel engine with channels, checkpointing, 7 stream modes |
| **Type safety** | Python generics + Pydantic validation | Trait-based, compile-time enforced |
| **Output handling** | First-class structured output (Tool/Native/Prompted modes) | Output parsers + `with_structured_output` |
| **DI system** | Built-in `RunContext[DepsT]` | None (manual wiring) |
| **Tool model** | Functions with optional context injection | Trait-based `Tool` with schema builder |
| **Provider abstraction** | Thin `Model` protocol, string-based selection | Thick `BaseChatModel` trait, per-provider crates |
| **MCP** | Sampling + tool integration | Full 4-transport client with multi-server routing |

**Bottom line**: Pydantic-AI is winning mindshare because it's dramatically simpler for the 80% case (single agent + tools + structured output). LangChain/LangGraph wins for complex multi-agent orchestration, checkpointed workflows, and production graph execution.

---

## Detailed Architectural Differences

### 1. Core Abstraction Model

**Pydantic-AI**: The `Agent` is the only top-level abstraction users interact with. An agent encapsulates:
- A model (or list of fallback models)
- System prompts (static or dynamic functions)
- Tools (functions, optionally context-aware)
- Output specification (type, mode, validators)
- Dependency type (generic parameter)

```python
agent = Agent('openai:gpt-4', deps_type=MyDeps, output_type=MyOutput)
```

**Synwire (our spec)**: The `Runnable<I, O>` trait is the universal composable unit. Chat models, chains, tools, retrievers — everything implements `Runnable`. Agents are built by composing runnables via LangGraph's `StateGraph`.

**Impact**: Pydantic-AI users write ~5 lines to get a working agent. Our spec requires understanding Runnables, StateGraph, channels, and the Pregel execution model before building equivalent functionality.

### 2. Dependency Injection

**Pydantic-AI**: First-class `RunContext[AgentDepsT]` passed to tools and system prompt functions. The deps type is generic on the agent, providing compile-time (in Rust terms) safety.

```python
@agent.tool
async def get_user(ctx: RunContext[DatabasePool], user_id: str) -> str:
    return await ctx.deps.fetch_user(user_id)
```

**Synwire**: No built-in DI. Dependencies are threaded via `RunnableConfig.configurable` (stringly-typed) or closure capture. LangGraph adds `InjectedState` and `InjectedStore` marker traits but these are graph-specific.

**Impact**: Pydantic-AI's DI is significantly more ergonomic and type-safe. Our spec's approach requires more boilerplate and loses type information at boundaries.

### 3. Structured Output

**Pydantic-AI**: Three distinct modes, automatically negotiated per model:
- **ToolOutput**: Model returns structured data via a tool call (works everywhere)
- **NativeOutput**: Uses model's native JSON mode (OpenAI, Gemini)
- **PromptedOutput**: Sends JSON schema in prompt for models without native support

Model profiles automatically select the best mode. Output is validated via Pydantic `TypeAdapter` with partial validation during streaming.

**Synwire**: Output parsers (`JsonOutputParser`, `StructuredOutputParser`, etc.) or `with_structured_output()` on chat models. 9+ concrete parser types. The user must understand which parser to use and configure it.

**Impact**: Pydantic-AI's approach is more discoverable (just set `output_type=MyStruct`). Our spec's approach is more flexible but requires more expertise.

### 4. Graph / Workflow Execution

**Pydantic-AI**: Internally uses `pydantic_graph` with 4 node types (`UserPromptNode`, `ModelRequestNode`, `CallToolsNode`, `End`). The graph is an implementation detail — users interact via `agent.run()` or `agent.run_stream()`. The graph library is available separately for custom workflows but is secondary.

**Synwire**: LangGraph is a primary, user-facing framework. StateGraph with typed state, conditional edges, multiple channel types (LastValue, Topic, BinaryOperatorAggregate, etc.), Pregel superstep execution, 7 streaming modes, checkpointing with SQLite/Postgres, interrupts, time travel, and state snapshots.

**Impact**: Our spec's graph system is far more capable for complex workflows. However, it's unnecessary complexity for simple agent use cases. Pydantic-AI users who need graphs can use `pydantic_graph` directly, but it's much simpler than LangGraph.

### 5. Streaming

**Pydantic-AI**: `StreamedRunResult` with three methods:
- `stream_output()` — validated structured output chunks with debouncing
- `stream_text()` — text with optional delta mode
- `stream_responses()` — raw model response events

**Synwire**: Three levels at core (`stream`, `stream_events`, `stream_log`) plus 7 LangGraph modes (values, updates, debug, messages, custom, tasks, checkpoints).

**Impact**: Our spec has much richer streaming for production observability. Pydantic-AI's streaming is simpler but covers the common cases well.

### 6. Error Handling & Retries

**Pydantic-AI**: `ModelRetry` exception triggers automatic retry with the error message sent back to the model. Per-tool `max_retries`. HTTP retries via tenacity with `Retry-After` header support. `CallDeferred` and `ApprovalRequired` for human-in-the-loop.

**Synwire**: Error handling via `thiserror` Result types. Tool errors returned as `ToolMessage` with error status. No built-in model-aware retry mechanism at the tool level. HITL via LangGraph interrupts.

**Impact**: Pydantic-AI's `ModelRetry` pattern (send error back to model for self-correction) is elegant and directly useful. Our spec should consider adopting this pattern.

### 7. Provider Integration

**Pydantic-AI**: Single `Model` protocol with string-based selection (`"openai:gpt-4"`). `FallbackModel` for cascading. Model profiles auto-configure capabilities.

**Synwire**: 16 separate provider crates, each implementing `BaseChatModel`. OpenAI-compatible providers share `BaseChatOpenAI` base. More explicit but more code to maintain.

**Impact**: Our multi-crate approach gives better compile-time guarantees and tree-shaking in Rust. Pydantic-AI's approach is more convenient but less type-safe.

### 8. Human-in-the-Loop

**Pydantic-AI**: `requires_approval` flag on tools. `DeferredToolRequests` output type. `ApprovalRequired` exception. External tool execution pattern.

**Synwire**: LangGraph interrupts with checkpoint/resume. More powerful (can pause at any graph node, not just tool calls) but more complex.

**Impact**: Both approaches are valid for different complexity levels. Our spec's interrupt system is more general-purpose.

---

## What Pydantic-AI Gets Right

1. **Agent as the primary abstraction** — users think in agents, not runnables
2. **Built-in dependency injection** — type-safe, ergonomic, eliminates boilerplate
3. **Automatic structured output negotiation** — just set the output type
4. **ModelRetry pattern** — self-correcting tool calls with minimal code
5. **Minimal surface area** — ~15 types to learn vs ~50+
6. **Output modes as first-class concept** — ToolOutput/NativeOutput/PromptedOutput
7. **Tool preparation** — dynamic tool definition modification at runtime

## What Synwire Gets Right

1. **Production graph orchestration** — checkpointing, time travel, interrupts
2. **Multi-agent patterns** — subgraphs, fan-out via Send, Command routing
3. **Rich streaming** — 7 modes covering every observability need
4. **Provider ecosystem** — 16 providers with deep, type-safe integration
5. **Agents layer** — full AI coding assistant framework
6. **MCP integration** — 4 transports, multi-server, comprehensive
7. **Rust type safety** — compile-time enforcement of all contracts

---

## Recommendations

### R1: Add an Agent-First API Layer (HIGH PRIORITY)

Create a `synwire-agent` or similar convenience crate that provides a Pydantic-AI-style `Agent<Deps, Output>` struct:

```rust
let agent = Agent::builder()
    .model("openai:gpt-4")
    .system_prompt("You are a helpful assistant")
    .tool(my_tool)
    .output::<MyOutput>()
    .build();

let result = agent.run("Hello", &my_deps).await?;
```

This would be a thin layer over LangGraph's `create_react_agent` that hides graph complexity for the 80% case. Users who need full graph power can drop down to StateGraph.

### R2: Adopt Dependency Injection Pattern (HIGH PRIORITY)

Add a `RunContext<D>` type that tools can optionally accept:

```rust
#[tool]
async fn get_user(ctx: &RunContext<DbPool>, user_id: String) -> Result<String> {
    ctx.deps().fetch_user(&user_id).await
}
```

This is more ergonomic than threading deps through `RunnableConfig.configurable` and provides compile-time type safety that even Pydantic-AI can't match in Python.

### R3: Add Structured Output Modes (MEDIUM PRIORITY)

Instead of 9+ output parser types, provide an `OutputMode` enum:

```rust
enum OutputMode<T: DeserializeOwned> {
    Tool,           // via tool call (universal)
    Native,         // model's native JSON mode
    Prompted,       // schema in prompt
    Text(fn(String) -> Result<T>), // custom text processor
}
```

Auto-select mode based on model profiles. Keep existing output parsers for advanced cases but make the common path simpler.

### R4: Add ModelRetry Tool Pattern (MEDIUM PRIORITY)

Allow tools to return a retry signal that sends the error back to the model:

```rust
#[tool]
async fn search(query: String) -> ToolResult {
    if query.len() < 3 {
        return ToolResult::retry("Query too short, please provide more detail");
    }
    // ... actual search
}
```

This is a genuinely useful pattern that our current spec lacks.

### R5: Simplify Provider Selection (LOW PRIORITY)

Add string-based model selection as a convenience alongside explicit provider construction:

```rust
// Simple
let model = Model::from_str("openai:gpt-4")?;

// Detailed (existing)
let model = ChatOpenAI::builder()
    .model("gpt-4")
    .temperature(0.7)
    .build();
```

### R6: Keep Full LangGraph Power (NO CHANGE)

Do NOT simplify LangGraph to match pydantic_graph. The full Pregel engine, checkpointing, interrupts, and multi-agent patterns are our key differentiator for production use cases. Instead, layer the simple Agent API on top (R1).

### R7: Consider Narrowing Initial Scope (STRATEGIC)

Our spec covers 26 crates. Pydantic-AI ships a single package. For initial release velocity:

1. **Phase 1**: synwire-core + synwire-llm-openai + agent convenience layer (R1)
2. **Phase 2**: synwire-orchestrator + checkpointing + prebuilt
3. **Phase 3**: Remaining providers + agents + MCP adapters

This gets a usable product out faster while maintaining the full vision.

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Pydantic-AI captures Rust developer mindshare before synwire ships | Medium | High | Prioritize R1 (agent-first API) for fast initial release |
| Over-engineering delays delivery | High | High | R7 (phased scope) |
| LangGraph complexity deters adoption | Medium | Medium | R1 provides simple on-ramp, full power available when needed |
| Python-centric patterns don't map well to Rust | Low | Medium | Rust's type system makes DI and structured output more natural |

---

## Conclusion

Pydantic-AI's rise validates that **simplicity wins for initial adoption**. Our synwire spec is architecturally superior for production multi-agent systems, but risks losing developers who want to start simple.

The recommended strategy is **layered complexity**: ship a Pydantic-AI-competitive simple agent API (R1-R4) on top of the full LangGraph engine, with a phased release plan (R7). This captures both the "quick start" market and the "production orchestration" market.

The worst outcome would be shipping all 26 crates simultaneously with only the complex API. The best outcome is shipping a 4-crate initial release that's as simple as Pydantic-AI for basic use, with the full power available as users' needs grow.
