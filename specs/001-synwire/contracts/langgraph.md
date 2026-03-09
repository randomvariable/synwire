# Public Trait Contracts: synwire-orchestrator & synwire-checkpoint

**Date**: 2026-03-09
**Branch**: `001-synwire`

All trait signatures use manual BoxFuture desugaring for dyn-compatibility.
Types from `synwire-core` (Message, RunnableConfig, Runnable, etc.) are
used directly — `synwire-orchestrator` depends on `synwire-core`.

## BaseChannel

```rust
/// A state management channel within a graph superstep.
/// Generic over the value type (V), update type (U), and checkpoint type (C).
pub trait BaseChannel: Send + Sync {
    type Value;
    type Update;
    type Checkpoint: Serialize + DeserializeOwned;

    /// Channel name in the state schema.
    fn key(&self) -> &str;

    /// Restore channel state from a checkpoint.
    fn from_checkpoint(&mut self, checkpoint: Self::Checkpoint) -> Result<(), SynwireGraphError>;

    /// Apply updates from the current superstep.
    /// Returns true if the channel value changed.
    fn update(&mut self, values: &[Self::Update]) -> Result<bool, InvalidUpdateError>;

    /// Get the current channel value.
    /// Returns EmptyChannelError if the channel has not been written to.
    fn get(&self) -> Result<&Self::Value, EmptyChannelError>;

    /// Whether the channel has a value available for reading.
    fn is_available(&self) -> bool;

    /// Serialise the current state for checkpointing.
    fn checkpoint(&self) -> Self::Checkpoint;

    /// Called after all nodes have read the channel in a superstep.
    /// Returns true if the channel was consumed (cleared).
    fn consume(&mut self) -> bool { false }

    /// Called when all nodes in a superstep have finished.
    /// Returns true if the channel value changed.
    fn finish(&mut self) -> bool { false }
}
```

### Concrete Channel Types

```rust
/// Stores the most recent value. Rejects multiple updates per superstep.
pub struct LastValue<V> {
    key: String,
    value: Option<V>,
}
// Implements BaseChannel<Value=V, Update=V, Checkpoint=Option<V>>
// update(): InvalidUpdateError if values.len() != 1
// get(): EmptyChannelError if value.is_none()

/// Pub-sub accumulation channel.
pub struct Topic<V> {
    key: String,
    values: Vec<V>,
    accumulate: bool,
}
// Implements BaseChannel<Value=Vec<V>, Update=V, Checkpoint=Vec<V>>
// update(): if !accumulate, clears before appending; flattens nested vecs
// get(): EmptyChannelError if values.is_empty()

/// Accumulates values using a binary operator (reducer function).
pub struct BinaryOperatorAggregate<V> {
    key: String,
    value: Option<V>,
    operator: fn(&V, &V) -> V,
}
// Implements BaseChannel<Value=V, Update=V, Checkpoint=Option<V>>
// update(): applies operator cumulatively; supports Overwrite to bypass reducer
// get(): EmptyChannelError if value.is_none()

/// Accepts any value. Multiple values treated as equal.
pub struct AnyValue<V> {
    key: String,
    value: Option<V>,
}
// Implements BaseChannel<Value=V, Update=V, Checkpoint=Option<V>>

/// Temporary value cleared after each superstep.
pub struct EphemeralValue<V> {
    key: String,
    value: Option<V>,
    guard: bool,
}
// Implements BaseChannel<Value=V, Update=V, Checkpoint=Option<V>>
// update(): if guard, InvalidUpdateError when values.len() != 1

/// Barrier that fires when all named triggers received.
pub struct NamedBarrierValue<V: Hash + Eq> {
    key: String,
    names: HashSet<V>,
    seen: HashSet<V>,
}
// Implements BaseChannel<Value=(), Update=V, Checkpoint=HashSet<V>>
// is_available(): seen == names
// consume(): clears seen if complete
```

## State Trait

```rust
/// Trait for graph state types. Derive macro generates the channel mapping.
///
/// Each field maps to a channel:
/// - Default: LastValue
/// - #[reducer(fn)]: BinaryOperatorAggregate with the given function
/// - #[channel(Topic)]: Topic channel
pub trait State: Send + Sync + Serialize + DeserializeOwned + 'static {
    /// Create channels for this state schema.
    fn channels() -> HashMap<String, Box<dyn BaseChannel>>;

    /// Apply channel values to construct the state.
    fn from_channels(channels: &HashMap<String, Box<dyn BaseChannel>>) -> Result<Self, SynwireGraphError>;
}
```

## StateGraph Builder

```rust
/// Builder for constructing a state graph.
impl<S: State> StateGraph<S> {
    /// Create a new state graph with the given state schema.
    pub fn new() -> Self;

    /// Add a node to the graph.
    /// The action receives the current state and returns a partial state update.
    pub fn add_node(
        &mut self,
        name: &str,
        action: impl Fn(S, RunnableConfig) -> BoxFuture<'_, Result<Value, SynwireGraphError>>
            + Send + Sync + 'static,
    ) -> &mut Self;

    /// Add a direct edge from source to target.
    pub fn add_edge(&mut self, source: &str, target: &str) -> &mut Self;

    /// Add conditional edges from source based on the routing function.
    pub fn add_conditional_edges(
        &mut self,
        source: &str,
        path: impl Fn(&S) -> RoutingResult + Send + Sync + 'static,
        path_map: Option<HashMap<String, String>>,
    ) -> &mut Self;

    /// Set the entry point (equivalent to add_edge(START, node)).
    pub fn set_entry_point(&mut self, node: &str) -> &mut Self;

    /// Set a finish point (equivalent to add_edge(node, END)).
    pub fn set_finish_point(&mut self, node: &str) -> &mut Self;

    /// Compile the graph into an executable CompiledGraph.
    /// Validates edges, checks for unreachable nodes, and constructs
    /// the Pregel execution engine.
    pub fn compile(
        self,
        config: GraphCompileConfig,
    ) -> Result<CompiledGraph<S>, GraphCompileError>;
}
```

### GraphCompileConfig

```rust
/// Configuration for graph compilation.
pub struct GraphCompileConfig {
    pub checkpointer: Option<Box<dyn BaseCheckpointSaver>>,
    pub store: Option<Box<dyn BaseStore>>,
    pub cache: Option<Box<dyn BaseCache>>,
    pub interrupt_before: Vec<String>,
    pub interrupt_after: Vec<String>,
    pub debug: bool,
    pub retry_policy: Option<Vec<RetryPolicy>>,
    pub cache_policy: Option<CachePolicy>,
}

impl Default for GraphCompileConfig { /* all None/empty/false */ }

impl GraphCompileConfig {
    pub fn builder() -> GraphCompileConfigBuilder;
}
```

## CompiledGraph

```rust
/// A compiled, executable graph. Implements Runnable<Value, Value>.
impl<S: State> CompiledGraph<S> {
    /// Invoke the graph synchronously (runs to completion or interrupt).
    pub fn invoke<'a>(
        &'a self,
        input: Value,
        config: Option<&'a RunnableConfig>,
    ) -> BoxFuture<'a, Result<Value, SynwireGraphError>>;

    /// Stream graph execution with the specified stream mode.
    pub fn stream<'a>(
        &'a self,
        input: Value,
        config: Option<&'a RunnableConfig>,
        stream_mode: Option<StreamMode>,
    ) -> BoxStream<'a, Result<(String, Value), SynwireGraphError>>;

    /// Get the current state snapshot for a thread.
    pub fn get_state<'a>(
        &'a self,
        config: &'a RunnableConfig,
    ) -> BoxFuture<'a, Result<StateSnapshot, SynwireGraphError>>;

    /// Get state history for a thread.
    pub fn get_state_history<'a>(
        &'a self,
        config: &'a RunnableConfig,
        limit: Option<usize>,
        before: Option<&'a RunnableConfig>,
    ) -> BoxStream<'a, Result<StateSnapshot, SynwireGraphError>>;

    /// Manually update the state for a thread.
    pub fn update_state<'a>(
        &'a self,
        config: &'a RunnableConfig,
        values: Value,
        as_node: Option<&'a str>,
    ) -> BoxFuture<'a, Result<RunnableConfig, SynwireGraphError>>;
}
```

## BaseCheckpointSaver

```rust
/// Trait for persisting graph state across invocations.
/// Generic over the version type V used for channel versioning.
pub trait BaseCheckpointSaver: Send + Sync {
    type Version: Send + Sync + Clone + Default;

    /// Fetch a checkpoint tuple by config (thread_id + optional checkpoint_id).
    fn get_tuple<'a>(
        &'a self,
        config: &'a RunnableConfig,
    ) -> BoxFuture<'a, Result<Option<CheckpointTuple>, CheckpointError>>;

    /// List checkpoints matching the filter criteria.
    fn list<'a>(
        &'a self,
        config: Option<&'a RunnableConfig>,
        filter: Option<&'a HashMap<String, Value>>,
        before: Option<&'a RunnableConfig>,
        limit: Option<usize>,
    ) -> BoxStream<'a, Result<CheckpointTuple, CheckpointError>>;

    /// Persist a checkpoint and return the updated config.
    fn put<'a>(
        &'a self,
        config: &'a RunnableConfig,
        checkpoint: &'a Checkpoint,
        metadata: &'a CheckpointMetadata,
        new_versions: &'a ChannelVersions,
    ) -> BoxFuture<'a, Result<RunnableConfig, CheckpointError>>;

    /// Persist intermediate writes for fault tolerance.
    fn put_writes<'a>(
        &'a self,
        config: &'a RunnableConfig,
        writes: &'a [(String, Value)],
        task_id: &'a str,
        task_path: &'a str,
    ) -> BoxFuture<'a, Result<(), CheckpointError>>;

    /// Delete all checkpoints for a thread.
    fn delete_thread<'a>(
        &'a self,
        thread_id: &'a str,
    ) -> BoxFuture<'a, Result<(), CheckpointError>>;

    /// Delete checkpoints for specific runs.
    fn delete_for_runs<'a>(
        &'a self,
        run_ids: &'a [String],
    ) -> BoxFuture<'a, Result<(), CheckpointError>>;

    /// Copy all checkpoints from one thread to another.
    fn copy_thread<'a>(
        &'a self,
        source_thread_id: &'a str,
        target_thread_id: &'a str,
    ) -> BoxFuture<'a, Result<(), CheckpointError>>;

    /// Prune old checkpoints.
    fn prune<'a>(
        &'a self,
        thread_ids: &'a [String],
        strategy: PruneStrategy,
    ) -> BoxFuture<'a, Result<(), CheckpointError>>;

    /// Generate the next version ID for a channel.
    fn get_next_version(
        &self,
        current: Option<&Self::Version>,
        channel: &str,
    ) -> Self::Version;
}

/// Pruning strategy for checkpoint cleanup.
pub enum PruneStrategy {
    KeepLatest,
}
```

### Concrete Checkpoint Implementations

```rust
/// In-memory checkpoint saver for testing.
pub struct InMemoryCheckpointSaver { /* ... */ }
// Implements BaseCheckpointSaver<Version = i64>
// Stores checkpoints in a HashMap<String, Vec<CheckpointTuple>>

/// SQLite checkpoint saver (synwire-checkpoint-sqlite crate).
pub struct SqliteSaver { /* ... */ }
// Implements BaseCheckpointSaver<Version = i64>
// Tables: checkpoints, checkpoint_blobs, writes
// Connection pooling via r2d2 or deadpool

/// PostgreSQL checkpoint saver (synwire-checkpoint-postgres crate).
pub struct PostgresSaver { /* ... */ }
// Implements BaseCheckpointSaver<Version = i64>
// Async via tokio-postgres
// Tables: checkpoints, checkpoint_blobs, writes
// Connection pooling via deadpool-postgres
```

## SerializerProtocol

```rust
/// Protocol for serialising/deserialising checkpoint data.
pub trait SerializerProtocol: Send + Sync {
    /// Serialise an object, returning a type tag and bytes.
    fn dumps_typed(&self, obj: &Value) -> Result<(String, Vec<u8>), SerializeError>;

    /// Deserialise bytes with a type tag back to an object.
    fn loads_typed(&self, type_tag: &str, data: &[u8]) -> Result<Value, DeserializeError>;
}

/// Default JSON serializer with type discrimination.
pub struct JsonPlusSerializer;
// Type tags: "json" for native JSON values, "bytes" for binary data (base64)
```

## BaseStore

```rust
/// Trait for persistent key-value storage with namespace hierarchy.
pub trait BaseStore: Send + Sync {
    /// Whether this store supports TTL.
    fn supports_ttl(&self) -> bool { false }

    /// TTL configuration.
    fn ttl_config(&self) -> Option<&TTLConfig> { None }

    /// Execute a batch of operations.
    fn batch<'a>(
        &'a self,
        ops: &'a [StoreOp],
    ) -> BoxFuture<'a, Result<Vec<StoreResult>, StoreError>>;

    /// Get a single item by namespace and key.
    fn get<'a>(
        &'a self,
        namespace: &'a [String],
        key: &'a str,
        refresh_ttl: Option<bool>,
    ) -> BoxFuture<'a, Result<Option<Item>, StoreError>>;

    /// Search items by namespace prefix with optional semantic query.
    fn search<'a>(
        &'a self,
        namespace_prefix: &'a [String],
        query: Option<&'a str>,
        filter: Option<&'a HashMap<String, Value>>,
        limit: usize,
        offset: usize,
        refresh_ttl: Option<bool>,
    ) -> BoxFuture<'a, Result<Vec<SearchItem>, StoreError>>;

    /// Store an item. Pass value=None to delete.
    fn put<'a>(
        &'a self,
        namespace: &'a [String],
        key: &'a str,
        value: Option<&'a HashMap<String, Value>>,
        index: Option<IndexDirective>,
        ttl: Option<f64>,
    ) -> BoxFuture<'a, Result<(), StoreError>>;

    /// Delete an item by namespace and key.
    fn delete<'a>(
        &'a self,
        namespace: &'a [String],
        key: &'a str,
    ) -> BoxFuture<'a, Result<(), StoreError>>;

    /// List namespaces matching prefix/suffix patterns.
    fn list_namespaces<'a>(
        &'a self,
        prefix: Option<&'a [String]>,
        suffix: Option<&'a [String]>,
        max_depth: Option<usize>,
        limit: usize,
        offset: usize,
    ) -> BoxFuture<'a, Result<Vec<Vec<String>>, StoreError>>;
}

/// Discriminant for index directives on put operations.
pub enum IndexDirective {
    Disabled,                          // Don't index
    Fields(Vec<String>),               // Index specific fields
    Auto,                              // Use store's default indexing
}
```

### Concrete Store Implementations

```rust
/// In-memory store for testing.
pub struct InMemoryStore { /* ... */ }
// Implements BaseStore
// Stores items in a HashMap<(Vec<String>, String), Item>
```

## BaseCache

```rust
/// Trait for node result memoisation.
pub trait BaseCache: Send + Sync {
    type Value: Send + Sync;

    /// Get cached values by keys (namespace, key pairs).
    fn get<'a>(
        &'a self,
        keys: &'a [(Vec<String>, String)],
    ) -> BoxFuture<'a, Result<HashMap<(Vec<String>, String), Self::Value>, CacheError>>;

    /// Set cached values with optional TTL (seconds).
    fn set<'a>(
        &'a self,
        pairs: &'a [((Vec<String>, String), (Self::Value, Option<u64>))],
    ) -> BoxFuture<'a, Result<(), CacheError>>;

    /// Clear cached values, optionally filtered by namespace.
    fn clear<'a>(
        &'a self,
        namespaces: Option<&'a [Vec<String>]>,
    ) -> BoxFuture<'a, Result<(), CacheError>>;
}
```

## Runtime Context Accessors

```rust
/// Get the current RunnableConfig from the graph execution context.
/// Must be called from within a graph node.
pub fn get_config() -> Result<RunnableConfig, SynwireGraphError>;

/// Get the store from the graph execution context.
/// Must be called from within a graph node compiled with a store.
pub fn get_store() -> Result<&'static dyn BaseStore, SynwireGraphError>;

/// Get the stream writer from the graph execution context.
/// Must be called from within a graph node.
pub fn get_stream_writer() -> Result<StreamWriter, SynwireGraphError>;
```

Implementation note: These use `tokio::task_local!` or a similar
thread-local/task-local mechanism to provide context without explicit
parameter passing.

## Graph Control Flow Functions

```rust
/// Pause graph execution and request user input.
/// On first call: returns Err(GraphInterrupt) causing the graph to checkpoint and pause.
/// On resume: returns Ok(resume_value) provided by the user.
pub fn interrupt(value: Value) -> Result<Value, GraphInterrupt>;

/// Reducer function for message lists. Handles:
/// - Appending new messages
/// - Deduplicating by ID (updates existing messages)
/// - Processing RemoveMessage entries (removes target messages)
pub fn add_messages(existing: &[Message], new: &[Message]) -> Vec<Message>;
```

## Checkpoint Conformance Tests

```rust
/// Run the full conformance test suite against a checkpoint saver implementation.
/// Validates all BaseCheckpointSaver methods work correctly.
///
/// Usage in integration tests:
/// ```
/// use synwire_checkpoint_conformance::run_conformance_tests;
///
/// #[tokio::test]
/// async fn test_my_saver() {
///     let saver = MySaver::new();
///     run_conformance_tests(&saver).await;
/// }
/// ```
pub async fn run_conformance_tests(saver: &dyn BaseCheckpointSaver);
```

## NodeRegistry

```rust
/// Registry for extensible node type registration.
pub struct NodeRegistry {
    // Internal: HashMap<String, HashMap<String, NodeConstructor>>
    // Keyed by (type_name, version)
}

impl NodeRegistry {
    pub fn new() -> Self;

    /// Register a node constructor for a type.
    pub fn register(
        &mut self,
        type_name: &str,
        constructor: impl Fn(Value) -> Result<Box<dyn NodeFn>, SynwireGraphError> + Send + Sync + 'static,
    );

    /// Register a versioned node constructor.
    pub fn register_versioned(
        &mut self,
        type_name: &str,
        version: &str,
        constructor: impl Fn(Value) -> Result<Box<dyn NodeFn>, SynwireGraphError> + Send + Sync + 'static,
    );

    /// Resolve a node constructor by type and optional version.
    pub fn resolve(
        &self,
        type_name: &str,
        version: Option<&str>,
    ) -> Result<&dyn Fn(Value) -> Result<Box<dyn NodeFn>, SynwireGraphError>, SynwireGraphError>;
}
```

## CheckpointMigration

```rust
/// Trait for migrating checkpoint data between format versions.
pub trait CheckpointMigration: Send + Sync {
    /// Source version this migration handles.
    fn from_version(&self) -> &str;

    /// Target version after migration.
    fn to_version(&self) -> &str;

    /// Migrate checkpoint data from source to target version.
    fn migrate(&self, data: Value) -> Result<Value, SynwireGraphError>;
}
```
