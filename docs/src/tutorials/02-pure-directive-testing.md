# Testing Agent Logic Without Side Effects

**Time**: ~25 minutes
**Prerequisites**: Completed `01-first-agent.md`, familiarity with Rust `#[test]`

Agent nodes often want to spawn child agents, emit events, or stop themselves. These are
side effects that are expensive or inconvenient to trigger in unit tests. Synwire separates
the _description_ of effects from their _execution_ through the directive/effect pattern.
This tutorial teaches you to write pure node functions, assert on the directives they
return, and verify nothing actually executed.

---

## What you are building

A counter agent node that:

1. Increments a counter in state.
2. Emits a `SpawnAgent` directive when the counter reaches a threshold.
3. Emits a `Stop` directive when it reaches a hard limit.

You will test all three behaviours without running any LLM or spawning any real child agent.

---

## Step 1: Understand DirectiveResult

`DirectiveResult<S>` is the return type of a pure agent node function:

```rust
pub struct DirectiveResult<S: State> {
    pub state: S,
    pub directives: Vec<Directive>,
}
```

- `state` is the new state value after the node ran. It is applied immediately.
- `directives` is a list of effects to be executed by the runtime later.

The split matters: your node function can be a plain synchronous `fn` returning a
`DirectiveResult`. It does not need to be `async`, does not take a runtime reference, and
cannot accidentally trigger real side effects.

---

## Step 2: Define a State type

Add this to `src/lib.rs` (or a test module):

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;
use synwire_core::State;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterState {
    pub count: u32,
}

impl State for CounterState {
    fn to_value(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Ok(serde_json::to_value(self)?)
    }

    fn from_value(value: Value) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(serde_json::from_value(value)?)
    }
}
```

`State` requires `Send + Sync + Clone + Serialize + DeserializeOwned`. The body of both
methods is identical for almost every concrete state type — delegate to `serde_json`.

---

## Step 3: Write the pure node function

```rust
use synwire_core::agents::directive::{Directive, DirectiveResult};
use serde_json::json;

const SPAWN_THRESHOLD: u32 = 3;
const STOP_LIMIT: u32 = 5;

pub fn counter_node(state: CounterState) -> DirectiveResult<CounterState> {
    let new_count = state.count + 1;
    let new_state = CounterState { count: new_count };

    if new_count >= STOP_LIMIT {
        // Request agent stop. The runtime will act on this; the node does not.
        return DirectiveResult::with_directive(
            new_state,
            Directive::Stop {
                reason: Some(format!("limit {STOP_LIMIT} reached")),
            },
        );
    }

    if new_count == SPAWN_THRESHOLD {
        // Request spawning a helper agent. Config is arbitrary JSON.
        return DirectiveResult::with_directive(
            new_state,
            Directive::SpawnAgent {
                name: "helper-agent".to_string(),
                config: json!({ "model": "gpt-4o-mini", "task": "summarise" }),
            },
        );
    }

    // No side effects — state only.
    DirectiveResult::state_only(new_state)
}
```

Key constructors on `DirectiveResult`:

| Constructor | Use when |
|---|---|
| `DirectiveResult::state_only(state)` | No side effects needed |
| `DirectiveResult::with_directive(state, d)` | Exactly one effect |
| `DirectiveResult::with_directives(state, vec![...])` | Multiple effects |
| `state.into()` (via `From<S>`) | Shorthand for `state_only` |

---

## Step 4: The Directive enum

`Directive` is `#[non_exhaustive]`. The variants you will use most often:

| Variant | Purpose |
|---|---|
| `Emit { event: AgentEvent }` | Push an event to the event stream |
| `SpawnAgent { name, config }` | Ask the runtime to start a child agent |
| `StopChild { name }` | Ask the runtime to stop a named child agent |
| `Stop { reason }` | Ask the runtime to stop this agent |
| `SpawnTask { description, input }` | Run a background task |
| `StopTask { task_id }` | Cancel a background task |
| `RunInstruction { instruction, input }` | Delegate to the model and route result back |
| `Schedule { action, delay }` | Fire an action after a delay |
| `Cron { expression, action }` | Fire an action on a cron schedule |

---

## Step 5: Write unit tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use synwire_core::agents::directive::Directive;

    #[test]
    fn increments_count() {
        let state = CounterState { count: 0 };
        let result = counter_node(state);

        assert_eq!(result.state.count, 1);
        assert!(result.directives.is_empty(), "no side effects below threshold");
    }

    #[test]
    fn spawns_agent_at_threshold() {
        let state = CounterState { count: SPAWN_THRESHOLD - 1 };
        let result = counter_node(state);

        assert_eq!(result.state.count, SPAWN_THRESHOLD);
        assert_eq!(result.directives.len(), 1);
        assert!(
            matches!(
                &result.directives[0],
                Directive::SpawnAgent { name, .. } if name == "helper-agent"
            ),
            "expected SpawnAgent directive"
        );
    }

    #[test]
    fn stops_at_limit() {
        let state = CounterState { count: STOP_LIMIT - 1 };
        let result = counter_node(state);

        assert_eq!(result.state.count, STOP_LIMIT);
        assert_eq!(result.directives.len(), 1);
        assert!(
            matches!(&result.directives[0], Directive::Stop { .. }),
            "expected Stop directive"
        );
    }

    #[test]
    fn stop_reason_is_set() {
        let state = CounterState { count: STOP_LIMIT - 1 };
        let result = counter_node(state);

        if let Directive::Stop { reason } = &result.directives[0] {
            assert!(reason.is_some(), "reason should be set");
            assert!(reason.as_ref().unwrap().contains("limit"));
        } else {
            panic!("expected Stop directive");
        }
    }
}
```

Run:

```bash
cargo test
```

All four tests pass with zero network calls, zero spawned processes, and zero LLM tokens.

---

## Step 6: Use NoOpExecutor to confirm no execution

In integration tests you may wire your node function into a broader harness that passes
directives to an executor. `NoOpExecutor` records nothing and executes nothing — it always
returns `Ok(None)`:

```rust
#[cfg(test)]
mod executor_tests {
    use synwire_core::agents::directive::{Directive, DirectiveResult};
    use synwire_core::agents::directive_executor::{DirectiveExecutor, NoOpExecutor};

    #[tokio::test]
    async fn noop_executor_does_not_execute() {
        let executor = NoOpExecutor;
        let directive = Directive::SpawnAgent {
            name: "child".to_string(),
            config: serde_json::json!({}),
        };

        // execute_directive returns Ok(None) — no child was spawned.
        let result = executor
            .execute_directive(&directive)
            .await
            .expect("executor should not error");

        assert!(result.is_none(), "NoOpExecutor never returns a value");
    }
}
```

When you later integrate a real executor (for example one that makes HTTP calls to spawn
agents), you can substitute `NoOpExecutor` in tests while keeping the same node functions.

---

## Step 7: Serde round-trip

`Directive` derives `Serialize` and `Deserialize`, with `#[serde(tag = "type")]`. This
lets you persist directives to a queue, send them over the wire, or log them for auditing.

```rust
#[cfg(test)]
mod serde_tests {
    use synwire_core::agents::directive::Directive;

    #[test]
    fn stop_directive_round_trips() {
        let original = Directive::Stop {
            reason: Some("task complete".to_string()),
        };

        let json = serde_json::to_string(&original).expect("serialize");

        // The discriminant is the "type" field.
        assert!(json.contains(r#""type":"stop""#));

        let deserialized: Directive = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(deserialized, Directive::Stop { .. }));
    }

    #[test]
    fn spawn_agent_directive_round_trips() {
        let original = Directive::SpawnAgent {
            name: "worker".to_string(),
            config: serde_json::json!({ "model": "gpt-4o" }),
        };

        let json = serde_json::to_string(&original).expect("serialize");
        assert!(json.contains(r#""type":"spawn_agent""#));

        let back: Directive = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(back, Directive::SpawnAgent { name, .. } if name == "worker"));
    }

    #[test]
    fn run_instruction_directive_round_trips() {
        let original = Directive::RunInstruction {
            instruction: "summarise this text".to_string(),
            input: serde_json::json!({ "text": "hello world" }),
        };

        let json = serde_json::to_string(&original).expect("serialize");
        let back: Directive = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(back, Directive::RunInstruction { .. }));
    }
}
```

The serialised form uses `"type"` as the tag field. For example, `Directive::Stop` becomes:

```json
{"type":"stop","reason":"task complete"}
```

---

## What you have learned

- `DirectiveResult<S>` separates state mutation from side-effect description.
- Pure node functions are plain synchronous `fn`s — no `async`, no runtime references.
- The `Directive` enum describes every possible side effect.
- `NoOpExecutor` lets you wire the executor interface into tests without executing anything.
- `Directive` is fully serialisable for logging, queueing, or persistence.

---

## Next steps

- **Execution strategies**: Continue with `03-execution-strategies.md` to learn how to
  constrain which actions an agent can take based on FSM state.
- **Custom directives**: See `../explanation/directive_system.md` for implementing a
  custom `DirectivePayload` via `typetag`.
- **How-to guide**: See `../how-to/testing.md` for composing test fixtures with
  `synwire-test-utils` proptest strategies.
