# Your First Agent

**Time**: ~20 minutes
**Prerequisites**: Rust 1.85+, Cargo, a working internet connection for crate downloads

By the end of this tutorial you will have a Rust program that constructs a Synwire agent,
drives it with the `Runner`, and collects streaming events from the response. You will
understand what each component does and how errors surface.

---

## What you are building

A minimal binary that:

1. Constructs an `Agent` using the builder API.
2. Wraps it in a `Runner`.
3. Sends a single input message and reads the event stream to completion.
4. Prints any text delta and the termination reason.

---

## Step 1: Create a new Cargo project

```bash
cargo new synwire-hello
cd synwire-hello
```

---

## Step 2: Add Synwire to Cargo.toml

Open `Cargo.toml` and add the dependencies. If you are working inside the Synwire
workspace, use the workspace path. For a standalone project, add version numbers from
crates.io once the crate is published.

```toml
[dependencies]
# Core agent types (Agent builder, Runner, AgentError, AgentEvent)
synwire-core = { path = "../../crates/synwire-core" }

# Tokio async runtime
tokio = { version = "1", features = ["full"] }

# JSON value construction
serde_json = "1"
```

> If you are working inside the Synwire repository workspace, use
> `synwire-core = { workspace = true }` and `tokio = { workspace = true }`.

---

## Step 3: Write the agent

Replace the contents of `src/main.rs`:

```rust
use synwire_core::agents::agent_node::Agent;
use synwire_core::agents::runner::{Runner, RunnerConfig};
use synwire_core::agents::streaming::AgentEvent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build the agent.
    //
    // Agent::new(name, model) creates the builder directly.
    // Every method on Agent consumes and returns `self`, so they chain.
    let agent: Agent = Agent::new("my-agent", "stub-model")
        .description("A simple demonstration agent")
        .max_turns(10);

    // The Runner drives the agent turn loop. It holds the agent behind an Arc
    // so it can be shared and stopped from another task if needed.
    let runner = Runner::new(agent);

    // runner.run() returns a channel receiver. The runner spawns a background
    // task that sends events; we read them in this loop.
    let config = RunnerConfig::default();
    let mut rx = runner
        .run(serde_json::json!("What is 2+2?"), config)
        .await?;

    // Drain the event stream until the channel closes.
    while let Some(event) = rx.recv().await {
        match event {
            AgentEvent::TextDelta { content } => {
                print!("{content}");
            }
            AgentEvent::TurnComplete { reason } => {
                println!("\n[done: {reason:?}]");
            }
            AgentEvent::Error { message } => {
                eprintln!("Agent error: {message}");
            }
            _ => {
                // Other events (UsageUpdate, ToolCallStart, etc.) are ignored here.
            }
        }
    }

    Ok(())
}
```

Run it:

```bash
cargo run
```

You will see `[done: Complete]` printed — the stub model finishes immediately. In a
production setup you would provide a real LLM backend crate (such as `synwire-llm-openai`)
that replaces the stub model invocation.

---

## Step 4: Understand the Agent builder

`Agent<O>` is a plain builder struct. The type parameter `O` is the optional structured
output type. Omitting it (or writing `Agent` without a type argument) defaults `O` to `()`,
which means the agent returns unstructured text.

The builder fields you will use most often:

| Method | Purpose |
|---|---|
| `Agent::new(name, model)` | Set the agent name and primary model identifier |
| `.description(text)` | Human-readable description used in logging |
| `.max_turns(n)` | Abort after `n` conversation turns |
| `.max_budget(usd)` | Abort when cumulative cost exceeds the USD limit |
| `.fallback_model(name)` | Switch to this model on retryable errors |
| `.tool(t)` | Register a tool the model can call |
| `.plugin(p)` | Attach a plugin (covered in tutorial 04) |

The builder is consumed by `Runner::new(agent)`. After that point, configuration is fixed.

---

## Step 5: Understand the Runner

`Runner<O>` drives the agent execution loop in a background Tokio task. It is intentionally
stateless between calls to `run()`.

```rust
// Create a runner from the agent.
let runner = Runner::new(agent);

// Override the model for one run without rebuilding the agent.
runner.set_model("gpt-4o").await;

// Start a run and receive the event channel.
let mut rx = runner.run(input, RunnerConfig::default()).await?;
```

`RunnerConfig` lets you pass per-run options:

```rust
use synwire_core::agents::runner::RunnerConfig;

let config = RunnerConfig {
    // Resume an existing conversation by session ID.
    session_id: Some("session-abc".to_string()),
    // Override the model for this single run.
    model_override: Some("claude-3-5-sonnet".to_string()),
    // Retry transient model errors up to this many times.
    max_retries: 3,
};
```

---

## Step 6: Understand AgentError

`Runner::run` returns `Result<mpsc::Receiver<AgentEvent>, AgentError>`. The error fires
only if setup fails before the event stream starts (for example, an invalid configuration).

`AgentError` is `#[non_exhaustive]`, meaning new variants may be added in future releases.
Always include a catch-all arm:

```rust
use synwire_core::agents::error::AgentError;

async fn run_agent(runner: &Runner) -> Result<(), AgentError> {
    let config = RunnerConfig::default();
    let mut rx = runner
        .run(serde_json::json!("Hello"), config)
        .await?;   // <-- AgentError propagated here with `?`

    while let Some(event) = rx.recv().await {
        if let AgentEvent::Error { message } = event {
            // Errors during the run arrive as events, not as Err(AgentError).
            eprintln!("runtime error: {message}");
        }
    }
    Ok(())
}
```

The key `AgentError` variants you are likely to encounter:

| Variant | Meaning |
|---|---|
| `AgentError::Model(ModelError::RateLimit(_))` | Rate-limited; the runner retries automatically |
| `AgentError::Model(ModelError::Authentication(_))` | Bad API key; not retryable |
| `AgentError::BudgetExceeded(cost)` | Cumulative spend exceeded `max_budget` |
| `AgentError::Tool(msg)` | A registered tool returned an error |

Because `AgentError` is `#[non_exhaustive]`, write:

```rust
match err {
    AgentError::Model(model_err) => { /* ... */ }
    AgentError::BudgetExceeded(cost) => { /* ... */ }
    _ => eprintln!("unexpected agent error: {err}"),
}
```

---

## Step 7: Read the full event stream

`AgentEvent` carries all observable agent behaviour. The events you should always handle:

| Event | When emitted |
|---|---|
| `TextDelta { content }` | Model produces a text chunk (streaming) |
| `UsageUpdate { usage }` | After each turn; contains `input_tokens`, `output_tokens` |
| `TurnComplete { reason }` | Final event; reason is one of `Complete`, `MaxTurnsExceeded`, `BudgetExceeded`, `Stopped`, `Aborted`, `Error` |
| `Error { message }` | Non-fatal error during the run |

The channel closes after the `TurnComplete` (or `Error`) event, so `rx.recv().await`
returning `None` is the correct loop termination signal.

---

## Stopping an agent from outside

You can stop a running agent by holding an `Arc<Runner>` and calling `stop_graceful` or
`stop_force` from another task:

```rust
use std::sync::Arc;

let runner = Arc::new(Runner::new(agent));
let runner_handle = Arc::clone(&runner);

// Spawn the run.
let mut rx = runner.run(serde_json::json!("Long task"), RunnerConfig::default()).await?;

// In another task, stop after 5 seconds.
tokio::spawn(async move {
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    runner_handle.stop_graceful().await;
});

// Drain events normally; you will receive TurnComplete { reason: Stopped }.
while let Some(event) = rx.recv().await {
    // ...
}
```

---

## Next steps

- **Add tools**: See `../how-to/tools.md` for registering typed tool handlers.
- **Structured output**: See `../how-to/structured_output.md` for binding `Agent<MyType>`.
- **Understanding the event model**: See `../explanation/event_model.md` for a deep dive
  into how events, turns, and retries interact.
- **Next tutorial**: Continue with `02-pure-directive-testing.md` to learn how to test
  agent logic without executing any side effects.

> **Background**: [Agent Components](https://www.promptingguide.ai/agents/components) — the memory, tools, and planning components of an agent; all three appear in this tutorial.
