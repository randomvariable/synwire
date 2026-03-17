# How to: Configure the Middleware Stack

**Goal:** Build a `MiddlewareStack`, add the provided middleware components, and implement your own custom middleware.

---

## Overview

`MiddlewareStack` runs its components in insertion order. Each component receives a `MiddlewareInput` (messages + context metadata) and returns a `MiddlewareResult`:

- `MiddlewareResult::Continue(MiddlewareInput)` — pass (possibly modified) input to the next component.
- `MiddlewareResult::Terminate(String)` — stop the chain immediately and return the given message.

System prompt additions and tools from all components are collected in declaration order.

```rust
use synwire_core::agents::middleware::MiddlewareStack;

let mut stack = MiddlewareStack::new();
// Push components in the order they should run.
stack.push(component_a);
stack.push(component_b);

let result = stack.run(input).await?;
let prompts = stack.system_prompt_additions();
let tools   = stack.tools();
```

---

## FilesystemMiddleware

Advertises filesystem tools (`ls`, `read_file`, `write_file`, `edit_file`, `rm`, `pwd`, `cd`) and adds a system prompt note about their availability. The runner wires the tools to the configured `LocalProvider` at start-up.

```rust
use synwire_agent::middleware::filesystem::FilesystemMiddleware;

stack.push(FilesystemMiddleware);
```

---

## GitMiddleware

Advertises git tools and injects a system prompt describing available git operations.

```rust
use synwire_agent::middleware::git::GitMiddleware;

stack.push(GitMiddleware);
```

---

## HttpMiddleware

Advertises HTTP request tools and injects a system prompt describing them.

```rust
use synwire_agent::middleware::http::HttpMiddleware;

stack.push(HttpMiddleware);
```

---

## ProcessMiddleware

Advertises process management tools (`list_processes`, `kill_process`, `spawn_background`, `execute`, `list_jobs`).

```rust
use synwire_agent::middleware::process::ProcessMiddleware;

stack.push(ProcessMiddleware);
```

---

## ArchiveMiddleware

Advertises archive tools (`create_archive`, `extract_archive`, `list_archive`).

```rust
use synwire_agent::middleware::archive::ArchiveMiddleware;

stack.push(ArchiveMiddleware);
```

---

## PipelineMiddleware

Advertises the pipeline execution tool.

```rust
use synwire_agent::middleware::pipeline::PipelineMiddleware;

stack.push(PipelineMiddleware);
```

---

## SummarisationMiddleware

Monitors message and token counts. When a threshold is exceeded, it sets `summarisation_pending: true` in the context metadata so the runner can trigger a summarisation step.

```rust
use synwire_agent::middleware::summarisation::{SummarisationMiddleware, SummarisationThresholds};

let thresholds = SummarisationThresholds {
    max_messages: Some(40),
    max_tokens: Some(60_000),
    max_context_utilisation: Some(0.75),
};
stack.push(SummarisationMiddleware::new(thresholds));

// Use defaults (50 messages / 80,000 tokens / 80% utilisation).
stack.push(SummarisationMiddleware::default());
```

---

## PromptCachingMiddleware

Marks system prompts for provider-side caching. Push it before any middleware that adds large static system prompt additions.

```rust
use synwire_agent::middleware::prompt_caching::PromptCachingMiddleware;

stack.push(PromptCachingMiddleware);
```

---

## PatchToolCallsMiddleware

Repairs malformed tool call JSON emitted by the model (e.g. missing required fields, incorrect types) before the agent attempts to execute them.

```rust
use synwire_agent::middleware::patch_tool_calls::PatchToolCallsMiddleware;

stack.push(PatchToolCallsMiddleware);
```

---

## EnvironmentMiddleware

Injects environment variable tools (`get_env`, `set_env`, `list_env`) and adds a system prompt explaining how to use them.

```rust
use synwire_agent::middleware::environment::EnvironmentMiddleware;

stack.push(EnvironmentMiddleware);
```

---

## Ordering

Middlewares execute in the order they were pushed. Recommended ordering for a typical agent:

1. `PromptCachingMiddleware` — mark static prompts before any additions accumulate
2. `PatchToolCallsMiddleware` — fix model output before tools run
3. `FilesystemMiddleware` / `HttpMiddleware` / `GitMiddleware` / … — capability injection
4. `SummarisationMiddleware` — history compaction last so it sees the full message list

---

## Implementing a custom middleware

Implement the `Middleware` trait. Only override the methods you need; `process` defaults to `Continue` and `tools` / `system_prompt_additions` default to empty.

```rust
use synwire_core::agents::middleware::{Middleware, MiddlewareInput, MiddlewareResult};
use synwire_core::agents::error::AgentError;
use synwire_core::BoxFuture;

struct AuditMiddleware {
    log_prefix: String,
}

impl Middleware for AuditMiddleware {
    fn name(&self) -> &str {
        "audit"
    }

    fn process(
        &self,
        input: MiddlewareInput,
    ) -> BoxFuture<'_, Result<MiddlewareResult, AgentError>> {
        let prefix = self.log_prefix.clone();
        Box::pin(async move {
            tracing::info!("{prefix}: {} messages in context", input.messages.len());
            // Return unchanged input to continue the chain.
            Ok(MiddlewareResult::Continue(input))
        })
    }

    fn system_prompt_additions(&self) -> Vec<String> {
        vec!["All operations are audited.".to_string()]
    }
}
```

To terminate the chain early (e.g. a rate-limit guard):

```rust
fn process(
    &self,
    _input: MiddlewareInput,
) -> BoxFuture<'_, Result<MiddlewareResult, AgentError>> {
    Box::pin(async move {
        if self.rate_limit_exceeded() {
            return Ok(MiddlewareResult::Terminate(
                "Rate limit exceeded — request blocked".to_string(),
            ));
        }
        Ok(MiddlewareResult::Continue(_input))
    })
}
```

---

**See also**

- [How to: Use the Backend Implementations](vfs.md)
- [How to: Configure Approval Gates](approval-gates.md)
- [Explanation: Architecture](../explanation/architecture.md)
