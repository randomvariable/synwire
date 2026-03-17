# Composing Plugins with Type-Safe State

**Time**: ~30 minutes
**Prerequisites**: Completed `01-first-agent.md`, familiarity with Rust generics

Synwire agents support a plugin system that lets independent modules each hold their own
private state alongside the agent, without being able to interfere with one another.
The isolation is enforced at compile time through Rust's type system, not at runtime through
naming conventions.

This tutorial shows you how to define plugin state keys, store state in a `PluginStateMap`,
access it through typed handles, implement the `Plugin` lifecycle trait, and verify that two
plugins cannot read each other's data.

---

## What you are building

Two plugins:

- `CachePlugin` — holds a simple in-memory cache with a hit counter.
- `MetricsPlugin` — holds a message count.

You will verify that mutating `CachePlugin` state does not affect `MetricsPlugin` state,
and that `PluginStateMap` enforces this isolation.

---

## Step 1: Add dependencies

```toml
[dependencies]
synwire-core = { path = "../../crates/synwire-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

---

## Step 2: Understand PluginStateKey

`PluginStateKey` is a marker trait that pairs a zero-sized key type with its associated
state type and a unique string identifier:

```rust
pub trait PluginStateKey: Send + Sync + 'static {
    type State: Send + Sync + 'static;
    const KEY: &'static str;
}
```

- `type State` is the concrete data type stored for this plugin.
- `KEY` is a stable string used in serialised output (e.g. debug dumps or checkpoints).
  It must be unique across all plugins in an agent. Duplicate registrations are detected
  at runtime and return an error.

The key type itself is never instantiated — it is purely a compile-time token.

---

## Step 3: Define the CachePlugin key and state

```rust
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use synwire_core::agents::plugin::PluginStateKey;

/// State stored by CachePlugin.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CacheState {
    pub hits: u32,
    pub entries: HashMap<String, String>,
}

/// Zero-sized key type. Never instantiated.
pub struct CachePlugin;

impl PluginStateKey for CachePlugin {
    type State = CacheState;
    const KEY: &'static str = "cache";
}
```

---

## Step 4: Define the MetricsPlugin key and state

```rust
/// State stored by MetricsPlugin.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MetricsState {
    pub messages_processed: u64,
}

/// Zero-sized key type.
pub struct MetricsPlugin;

impl PluginStateKey for MetricsPlugin {
    type State = MetricsState;
    const KEY: &'static str = "metrics";
}
```

---

## Step 5: Register state in PluginStateMap

`PluginStateMap` is the container that holds all plugin state slices for one agent
instance. Register each plugin's initial state with `register`:

```rust
use synwire_core::agents::plugin::{PluginHandle, PluginStateMap};

fn create_plugin_map() -> (PluginStateMap, PluginHandle<CachePlugin>, PluginHandle<MetricsPlugin>) {
    let mut map = PluginStateMap::new();

    // register() returns a PluginHandle — a zero-sized proof token.
    // If you try to register the same type twice, register() returns Err(KEY).
    let cache_handle = map
        .register::<CachePlugin>(CacheState::default())
        .expect("cache not yet registered");

    let metrics_handle = map
        .register::<MetricsPlugin>(MetricsState::default())
        .expect("metrics not yet registered");

    (map, cache_handle, metrics_handle)
}
```

`PluginHandle<P>` is a `Copy + Clone` zero-sized struct. Holding one proves that the
plugin is registered in the associated map. It has no runtime data — it is a compile-time
witness only.

---

## Step 6: Read and write through typed access

`PluginStateMap::get::<P>()` and `get_mut::<P>()` are generic over the key type. They
return `Option<&P::State>` and `Option<&mut P::State>` respectively. The type checker
prevents you from using the wrong key:

```rust
#[test]
fn read_and_write_cache_state() {
    let (mut map, _cache, _metrics) = create_plugin_map();

    // Read initial state.
    let state = map.get::<CachePlugin>().expect("cache registered");
    assert_eq!(state.hits, 0);
    assert!(state.entries.is_empty());

    // Mutate via get_mut.
    {
        let state = map.get_mut::<CachePlugin>().expect("cache registered");
        state.hits += 1;
        state.entries.insert("greeting".to_string(), "hello".to_string());
    }

    // Verify mutation.
    let state = map.get::<CachePlugin>().expect("cache registered");
    assert_eq!(state.hits, 1);
    assert_eq!(state.entries.get("greeting").map(String::as_str), Some("hello"));
}
```

You cannot pass `MetricsPlugin` to `get::<CachePlugin>()` — the types do not match and
the code will not compile.

---

## Step 7: Verify cross-plugin isolation

The following test confirms that mutating one plugin's state does not affect another:

```rust
#[test]
fn plugin_isolation_is_enforced() {
    let (mut map, _cache, _metrics) = create_plugin_map();

    // Mutate cache state aggressively.
    {
        let cache = map.get_mut::<CachePlugin>().expect("registered");
        cache.hits = 9999;
        cache.entries.insert("key".to_string(), "value".to_string());
    }

    // MetricsPlugin state is untouched.
    let metrics = map.get::<MetricsPlugin>().expect("registered");
    assert_eq!(metrics.messages_processed, 0);

    // Mutate metrics state.
    map.get_mut::<MetricsPlugin>().expect("registered").messages_processed = 42;

    // Cache state is untouched.
    let cache = map.get::<CachePlugin>().expect("registered");
    assert_eq!(cache.hits, 9999);
}
```

The isolation holds because the map is keyed by `TypeId::of::<P>()` — the Rust type
identity, not a string. Even if two plugins happened to share the same `KEY` string,
the `TypeId` lookup would still route correctly. (Duplicate `TypeId` registrations are
caught and return `Err`.)

---

## Step 8: Implement the Plugin lifecycle trait

To hook into agent events, implement `Plugin` on a concrete struct:

```rust
use synwire_core::agents::directive::Directive;
use synwire_core::agents::plugin::{Plugin, PluginInput, PluginStateMap};
use synwire_core::BoxFuture;

pub struct CachePluginImpl;

impl Plugin for CachePluginImpl {
    fn name(&self) -> &str {
        "cache"
    }

    /// Called when the agent receives a user message.
    /// We count cache accesses and emit no directives.
    fn on_user_message<'a>(
        &'a self,
        input: &'a PluginInput,
        state: &'a PluginStateMap,
    ) -> BoxFuture<'a, Vec<Directive>> {
        Box::pin(async move {
            // Read-only access is fine — PluginStateMap is shared here.
            if let Some(cache) = state.get::<CachePlugin>() {
                tracing::debug!(
                    turn = input.turn,
                    hits = cache.hits,
                    "CachePlugin: on_user_message"
                );
            }
            Vec::new()  // return empty slice — no directives
        })
    }
}
```

All `Plugin` methods have default no-op implementations. Override only the lifecycle
hooks you care about:

| Method | Called when |
|---|---|
| `on_user_message` | A user message arrives |
| `on_event` | Any `AgentEvent` is emitted |
| `before_run` | Before each turn loop iteration |
| `after_run` | After each turn loop iteration |
| `signal_routes` | At startup; contribute signal routing rules |

---

## Step 9: Register the plugin with an Agent

Attach the plugin implementation to the `Agent` builder with `.plugin()`:

```rust
use synwire_core::agents::agent_node::Agent;

let agent: Agent = Agent::new("my-agent", "stub-model")
    .plugin(CachePluginImpl)
    .plugin(MetricsPluginImpl);
```

Multiple `.plugin()` calls are chained. Each plugin is stored as `Box<dyn Plugin>` and
called in registration order during lifecycle events.

---

## Step 10: Serialise all plugin state

`PluginStateMap::serialize_all()` produces a JSON object keyed by each plugin's `KEY`
string. This is useful for logging, debugging, or persisting agent state:

```rust
#[test]
fn serialize_all_plugin_state() {
    let (mut map, _, _) = create_plugin_map();

    map.get_mut::<CachePlugin>().expect("registered").hits = 7;
    map.get_mut::<MetricsPlugin>().expect("registered").messages_processed = 3;

    let snapshot = map.serialize_all();
    assert_eq!(snapshot["cache"]["hits"], 7);
    assert_eq!(snapshot["metrics"]["messages_processed"], 3);
}
```

The keys in the output object are the `KEY` constants you defined — `"cache"` and
`"metrics"` in this example.

---

## Step 11: Detect duplicate registration

Attempting to register the same plugin key type twice returns an error:

```rust
#[test]
fn duplicate_registration_is_rejected() {
    let mut map = PluginStateMap::new();
    let _ = map
        .register::<CachePlugin>(CacheState::default())
        .expect("first registration succeeds");

    // Second registration of the same type returns the KEY string as the error.
    let err = map
        .register::<CachePlugin>(CacheState::default())
        .expect_err("duplicate registration should fail");

    assert_eq!(err, CachePlugin::KEY);  // "cache"
}
```

---

## Why this design matters

Most plugin systems use `HashMap<String, Box<dyn Any>>` with string keys. This approach
trades compile-time safety for flexibility: if you mistype a key string you get a silent
`None` at runtime, not a compiler error.

`PluginStateMap` uses `TypeId` as the map key. `TypeId` is derived from the Rust type
system, so:

- Accessing the wrong plugin state is a compile error, not a runtime panic.
- There is no string lookup — `TypeId` lookups are O(1) hash operations.
- The `KEY` string constant exists only for serialisation; it plays no role in access control.

---

## Next steps

- **Backend operations**: Continue with `05-backend-operations.md` to learn how to read
  and write files through the backend protocol.
- **Plugin how-to**: See `../how-to/plugins.md` for a complete guide to plugin
  registration, ordering, and dependency injection.
- **Architecture**: See `../explanation/plugin_system.md` for a deeper explanation of how
  `TypeId`-keyed maps enforce isolation.
