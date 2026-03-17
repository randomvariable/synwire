//! Plugin system with isolated state.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;

use serde_json::Value;

use std::sync::Arc;

use crate::BoxFuture;
use crate::agents::directive::Directive;
use crate::agents::streaming::AgentEvent;
use crate::tools::Tool;

/// Typed key for plugin state stored in a [`PluginStateMap`].
///
/// Implement this trait to define a plugin's state type.
pub trait PluginStateKey: Send + Sync + 'static {
    /// The state type stored for this key.
    type State: Send + Sync + 'static;

    /// Unique string key for serialization.
    const KEY: &'static str;
}

/// Zero-sized proof token returned when a plugin state is registered.
///
/// Holding a `PluginHandle<P>` proves that the plugin `P` has been registered
/// in the associated `PluginStateMap`.
pub struct PluginHandle<P: PluginStateKey> {
    _marker: PhantomData<P>,
}

impl<P: PluginStateKey> std::fmt::Debug for PluginHandle<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginHandle")
            .field("key", &P::KEY)
            .finish()
    }
}

impl<P: PluginStateKey> Clone for PluginHandle<P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P: PluginStateKey> Copy for PluginHandle<P> {}

/// Type-erased serializer stored alongside plugin state.
struct PluginStateMeta {
    value: Box<dyn Any + Send + Sync>,
    serialize: fn(&dyn Any) -> Option<Value>,
    key: &'static str,
}

fn serialize_fn<T: serde::Serialize + 'static>(v: &dyn Any) -> Option<Value> {
    v.downcast_ref::<T>()
        .and_then(|t| serde_json::to_value(t).ok())
}

/// Type-keyed map for plugin state with serialization support.
///
/// Provides type-safe access keyed by [`PluginStateKey`] implementations.
/// Plugins cannot access each other's state — the type key enforces isolation.
#[derive(Default)]
pub struct PluginStateMap {
    entries: HashMap<TypeId, PluginStateMeta>,
}

impl std::fmt::Debug for PluginStateMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginStateMap")
            .field("len", &self.entries.len())
            .finish()
    }
}

impl PluginStateMap {
    /// Create an empty map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register plugin state, returning a proof handle.
    ///
    /// # Errors
    ///
    /// Returns the key string if a plugin with the same `TypeId` is already registered.
    pub fn register<P>(&mut self, state: P::State) -> Result<PluginHandle<P>, &'static str>
    where
        P: PluginStateKey,
        P::State: serde::Serialize + 'static,
    {
        let id = TypeId::of::<P>();
        if self.entries.contains_key(&id) {
            return Err(P::KEY);
        }

        let _ = self.entries.insert(
            id,
            PluginStateMeta {
                value: Box::new(state),
                serialize: serialize_fn::<P::State>,
                key: P::KEY,
            },
        );

        Ok(PluginHandle {
            _marker: PhantomData,
        })
    }

    /// Get an immutable reference to plugin state.
    #[must_use]
    pub fn get<P: PluginStateKey>(&self) -> Option<&P::State> {
        self.entries
            .get(&TypeId::of::<P>())
            .and_then(|m| m.value.downcast_ref::<P::State>())
    }

    /// Get a mutable reference to plugin state.
    pub fn get_mut<P: PluginStateKey>(&mut self) -> Option<&mut P::State> {
        self.entries
            .get_mut(&TypeId::of::<P>())
            .and_then(|m| m.value.downcast_mut::<P::State>())
    }

    /// Insert or replace plugin state.
    pub fn insert<P: PluginStateKey>(&mut self, state: P::State)
    where
        P::State: serde::Serialize + 'static,
    {
        let _ = self.entries.insert(
            TypeId::of::<P>(),
            PluginStateMeta {
                value: Box::new(state),
                serialize: serialize_fn::<P::State>,
                key: P::KEY,
            },
        );
    }

    /// Serialize all plugin state to a JSON object keyed by plugin key strings.
    #[must_use]
    pub fn serialize_all(&self) -> Value {
        let mut map = serde_json::Map::new();
        for meta in self.entries.values() {
            if let Some(v) = (meta.serialize)(meta.value.as_ref()) {
                let _ = map.insert(meta.key.to_string(), v);
            }
        }
        Value::Object(map)
    }
}

/// Input passed to plugin lifecycle hooks.
#[derive(Debug, Clone)]
pub struct PluginInput {
    /// Current conversation turn index.
    pub turn: u32,
    /// Optional user message text.
    pub message: Option<String>,
}

/// Plugin lifecycle trait.
///
/// All methods have default no-op implementations so plugins only need to
/// override the hooks they care about.
pub trait Plugin: Send + Sync {
    /// Plugin name (used for debugging and logging).
    fn name(&self) -> &str;

    /// Called when a user message arrives.
    fn on_user_message<'a>(
        &'a self,
        _input: &'a PluginInput,
        _state: &'a PluginStateMap,
    ) -> BoxFuture<'a, Vec<Directive>> {
        Box::pin(async { Vec::new() })
    }

    /// Called when an agent event is emitted.
    fn on_event<'a>(
        &'a self,
        _event: &'a AgentEvent,
        _state: &'a PluginStateMap,
    ) -> BoxFuture<'a, Vec<Directive>> {
        Box::pin(async { Vec::new() })
    }

    /// Called before each agent run loop iteration.
    fn before_run<'a>(&'a self, _state: &'a PluginStateMap) -> BoxFuture<'a, Vec<Directive>> {
        Box::pin(async { Vec::new() })
    }

    /// Called after each agent run loop iteration.
    fn after_run<'a>(&'a self, _state: &'a PluginStateMap) -> BoxFuture<'a, Vec<Directive>> {
        Box::pin(async { Vec::new() })
    }

    /// Signal routes contributed by this plugin.
    fn signal_routes(&self) -> Vec<crate::agents::signal::SignalRoute> {
        Vec::new()
    }

    /// Tools contributed by this plugin.
    ///
    /// Called once during agent construction. The returned tools are merged
    /// into the agent's tool registry alongside any tools provided directly
    /// via `Agent::with_tools`. Tool names must not conflict.
    fn tools(&self) -> Vec<Arc<dyn Tool>> {
        Vec::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct CounterState {
        count: u32,
    }

    struct CounterKey;

    impl PluginStateKey for CounterKey {
        type State = CounterState;
        const KEY: &'static str = "counter";
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct FlagState {
        enabled: bool,
    }

    struct FlagKey;

    impl PluginStateKey for FlagKey {
        type State = FlagState;
        const KEY: &'static str = "flag";
    }

    #[test]
    fn test_type_safe_access() {
        let mut map = PluginStateMap::new();
        let _handle = map
            .register::<CounterKey>(CounterState { count: 0 })
            .expect("register");

        let state = map.get::<CounterKey>().expect("get");
        assert_eq!(state.count, 0);

        map.get_mut::<CounterKey>().expect("get_mut").count = 42;
        assert_eq!(map.get::<CounterKey>().expect("get after mut").count, 42);
    }

    #[test]
    fn test_cross_plugin_isolation() {
        let mut map = PluginStateMap::new();
        let _ = map.register::<CounterKey>(CounterState { count: 10 });
        let _ = map.register::<FlagKey>(FlagState { enabled: true });

        assert!(map.get::<CounterKey>().is_some());
        assert!(map.get::<FlagKey>().is_some());

        map.get_mut::<CounterKey>().expect("mut").count = 99;
        assert!(map.get::<FlagKey>().expect("flag").enabled);
    }

    #[test]
    fn test_key_collision_detection() {
        let mut map = PluginStateMap::new();
        let _ = map
            .register::<CounterKey>(CounterState { count: 0 })
            .expect("first register");

        let err = map
            .register::<CounterKey>(CounterState { count: 1 })
            .expect_err("second register should fail");
        assert_eq!(err, CounterKey::KEY);
    }

    #[test]
    fn test_serialization_round_trip() {
        let mut map = PluginStateMap::new();
        let _ = map.register::<CounterKey>(CounterState { count: 7 });
        let _ = map.register::<FlagKey>(FlagState { enabled: false });

        let serialized = map.serialize_all();
        assert_eq!(serialized["counter"]["count"], 7);
        assert_eq!(serialized["flag"]["enabled"], false);
    }
}
