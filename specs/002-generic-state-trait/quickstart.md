# Quickstart: Generic State Trait

**Branch**: `002-generic-state-trait` | **Date**: 2026-03-15

## Define a Custom State

```rust
use synwire_orchestrator::graph::{State, StateGraph};
use synwire_orchestrator::constants::END;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, State)]
struct CounterState {
    counter: i32,
    done: bool,
}

let mut graph = StateGraph::<CounterState>::new();

graph.add_node("increment", Box::new(|mut state: CounterState| {
    Box::pin(async move {
        state.counter += 1;
        state.done = state.counter >= 3;
        Ok(state)
    })
}))?;

graph.set_entry_point("increment");
graph.add_conditional_edges(
    "increment",
    Box::new(|state: &CounterState| {
        if state.done { END.to_owned() } else { "increment".to_owned() }
    }),
    HashMap::new(),
);

let compiled = graph.compile()?;
let result = compiled.invoke(CounterState { counter: 0, done: false }).await?;
assert_eq!(result.counter, 3);
assert!(result.done);
```

## Use MessagesState for Chat

```rust
use synwire_orchestrator::messages::MessagesState;
use synwire_orchestrator::prebuilt::create_react_agent;

let model: Box<dyn BaseChatModel> = Box::new(ChatOpenAI::new("gpt-4o"));
let tools: Vec<Box<dyn Tool>> = vec![Box::new(my_search_tool())];

let agent = create_react_agent(model, tools)?;
let result = agent.invoke(MessagesState {
    messages: vec![Message::human("What is the weather?")],
}).await?;

// result.messages contains the full conversation
for msg in &result.messages {
    println!("{msg:?}");
}
```

## Migrate from Value-Based Graphs

```rust
use synwire_orchestrator::graph::ValueState;

// Before: StateGraph with serde_json::Value
// After:  StateGraph<ValueState> — minimal changes

let mut graph = StateGraph::<ValueState>::new();
graph.add_node("echo", Box::new(|state: ValueState| {
    Box::pin(async move { Ok(state) })
}))?;
graph.set_entry_point("echo");
graph.set_finish_point("echo");

let compiled = graph.compile()?;
let result = compiled.invoke(ValueState(serde_json::json!({"msg": "hi"}))).await?;
assert_eq!(result.0["msg"], "hi");
```
