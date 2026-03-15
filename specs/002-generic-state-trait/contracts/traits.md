# Contract: State Trait and Generic Graph API

**Branch**: `002-generic-state-trait` | **Date**: 2026-03-15

## State Trait

```rust
/// Trait for typed graph state.
///
/// Implementors define the channel configuration for their fields and
/// provide deserialisation from channel values. Use `#[derive(State)]`
/// for automatic implementation.
pub trait State: Send + Sync + Clone + Serialize + DeserializeOwned + 'static {
    /// Returns channel configuration for each field in this state.
    fn channels() -> Vec<(String, Box<dyn BaseChannel>)>;

    /// Reconstructs this state from channel values.
    ///
    /// # Errors
    ///
    /// Returns `GraphError::DeserializationError` if a channel value
    /// cannot be deserialised into the expected field type.
    fn from_channels(
        channels: &HashMap<String, Box<dyn BaseChannel>>,
    ) -> Result<Self, GraphError>;

    /// Serialises this state to a JSON Value for checkpoint storage.
    ///
    /// Default implementation uses `serde_json::to_value`.
    fn to_value(&self) -> Result<serde_json::Value, GraphError> {
        serde_json::to_value(self).map_err(|e| GraphError::Checkpoint {
            message: format!("failed to serialise state: {e}"),
        })
    }

    /// Deserialises a state from a JSON Value (checkpoint restore).
    ///
    /// Default implementation uses `serde_json::from_value`.
    fn from_value(value: serde_json::Value) -> Result<Self, GraphError> {
        serde_json::from_value(value).map_err(|e| GraphError::Checkpoint {
            message: format!("failed to deserialise state: {e}"),
        })
    }
}
```

## NodeFn and ConditionFn Type Aliases

```rust
/// Boxed async node function operating on typed state.
pub type NodeFn<S> = Box<
    dyn Fn(S) -> synwire_core::BoxFuture<'static, Result<S, GraphError>>
        + Send
        + Sync,
>;

/// Boxed condition function inspecting typed state.
pub type ConditionFn<S> = Box<dyn Fn(&S) -> String + Send + Sync>;
```

## StateGraph\<S\> API

```rust
impl<S: State> StateGraph<S> {
    pub fn new() -> Self;
    pub fn add_node(&mut self, name: impl Into<String>, func: NodeFn<S>) -> Result<&mut Self, GraphError>;
    pub fn add_edge(&mut self, from: impl Into<String>, to: impl Into<String>) -> &mut Self;
    pub fn add_conditional_edges(
        &mut self,
        from: impl Into<String>,
        condition: ConditionFn<S>,
        mapping: HashMap<String, String>,
    ) -> &mut Self;
    pub fn set_entry_point(&mut self, name: impl Into<String>) -> &mut Self;
    pub fn set_finish_point(&mut self, name: impl Into<String>) -> &mut Self;
    pub fn compile(self) -> Result<CompiledGraph<S>, GraphError>;
}
```

## CompiledGraph\<S\> API

```rust
impl<S: State> CompiledGraph<S> {
    /// Executes the graph to completion with typed state.
    pub async fn invoke(&self, input: S) -> Result<S, GraphError>;

    /// Returns a Mermaid diagram of the graph topology (state-type-independent).
    pub fn to_mermaid(&self) -> String;

    /// Sets the recursion limit.
    pub const fn with_recursion_limit(self, limit: usize) -> Self;

    /// Returns the entry point node name.
    pub fn entry_point(&self) -> &str;

    /// Returns all node names.
    pub fn node_names(&self) -> Vec<&str>;
}
```

## ValueState (backward compatibility)

```rust
/// Wrapper enabling existing Value-based code to work with generic graphs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueState(pub serde_json::Value);

impl State for ValueState { ... }

impl From<serde_json::Value> for ValueState { ... }
impl From<ValueState> for serde_json::Value { ... }
```

## MessagesState

```rust
/// Built-in state type for chat-based agents.
#[derive(Debug, Clone, Serialize, Deserialize, State)]
pub struct MessagesState {
    /// Conversation history. Uses Topic channel (append semantics).
    #[reducer(topic)]
    pub messages: Vec<Message>,
}
```

## ToolNode Generic Interface

```rust
impl ToolNode {
    /// Converts this ToolNode into a NodeFn<S> for use in a generic StateGraph.
    ///
    /// The closure extracts `messages` from `S` via serde, executes tool calls,
    /// and injects tool messages back into `S` via serde.
    pub fn into_node_fn<S: State>(self) -> NodeFn<S>
    where
        Self: 'static;
}
```

## create_react_agent

```rust
/// Creates a ReAct agent as a compiled StateGraph<MessagesState>.
pub fn create_react_agent(
    model: Box<dyn BaseChatModel>,
    tools: Vec<Box<dyn Tool>>,
) -> Result<CompiledGraph<MessagesState>, GraphError>;
```

## #[derive(State)] Macro Output

For a struct:
```rust
#[derive(State, Clone, Serialize, Deserialize)]
struct MyState {
    counter: i32,
    #[reducer(topic)]
    messages: Vec<Message>,
}
```

The macro generates:
```rust
impl State for MyState {
    fn channels() -> Vec<(String, Box<dyn BaseChannel>)> {
        vec![
            ("counter".into(), Box::new(LastValue::new("counter"))),
            ("messages".into(), Box::new(Topic::new("messages"))),
        ]
    }

    fn from_channels(
        channels: &HashMap<String, Box<dyn BaseChannel>>,
    ) -> Result<Self, GraphError> {
        let counter = channels.get("counter")
            .and_then(|c| c.get())
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()
            .map_err(|e| GraphError::DeserializationError {
                field: "counter".into(),
                message: e.to_string(),
            })?
            .unwrap_or_default();
        let messages = channels.get("messages")
            .and_then(|c| c.get())
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()
            .map_err(|e| GraphError::DeserializationError {
                field: "messages".into(),
                message: e.to_string(),
            })?
            .unwrap_or_default();
        Ok(Self { counter, messages })
    }
}
```
