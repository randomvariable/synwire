# Graph Interrupts

Graph interrupts allow you to pause execution at a specific node and resume later, useful for human-in-the-loop workflows.

## How interrupts work

When a node returns `GraphError::Interrupt`, execution pauses. The graph state at that point can be checkpointed and later resumed by re-invoking with the saved state.

## Implementing an interrupt

```rust,ignore
use synwire_orchestrator::error::GraphError;

graph.add_node("review", Box::new(|state| {
    Box::pin(async move {
        let needs_review = state["needs_review"]
            .as_bool()
            .unwrap_or(false);

        if needs_review {
            return Err(GraphError::Interrupt {
                message: "Human review required".into(),
            });
        }

        Ok(state)
    })
}))?;
```

## Handling interrupts

```rust,ignore
match compiled.invoke(state).await {
    Ok(result) => {
        // Normal completion
    }
    Err(GraphError::Interrupt { message }) => {
        // Save state for later resumption
        println!("Paused: {message}");
    }
    Err(e) => {
        // Other error
    }
}
```

## Resume pattern

After human review modifies the state:

```rust,ignore
// Modify state based on human input
state["needs_review"] = serde_json::json!(false);
state["human_feedback"] = serde_json::json!("Approved");

// Re-invoke from the interrupted node
let result = compiled.invoke(state).await?;
```

## Combining with checkpointing

For durable interrupts, checkpoint the state before pausing:

```rust,ignore
use synwire_checkpoint::memory::InMemoryCheckpointSaver;

let saver = InMemoryCheckpointSaver::new();
// Save state on interrupt, restore on resume
```
