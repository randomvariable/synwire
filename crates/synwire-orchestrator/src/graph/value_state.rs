//! Backward-compatible [`ValueState`] wrapper for `serde_json::Value`.
//!
//! Enables existing Value-based graph code to work with the generic
//! [`StateGraph<S>`](super::StateGraph) / [`CompiledGraph<S>`](super::CompiledGraph)
//! by wrapping `serde_json::Value` in a type that implements [`State`].
//!
//! # Example
//!
//! ```rust,ignore
//! use synwire_orchestrator::graph::{StateGraph, ValueState};
//! use synwire_orchestrator::constants::END;
//!
//! let mut graph = StateGraph::<ValueState>::new();
//! graph.add_node("echo", Box::new(|s| Box::pin(async { Ok(s) })))?;
//! graph.set_entry_point("echo").set_finish_point("echo");
//! let compiled = graph.compile()?;
//! let result = compiled.invoke(ValueState(serde_json::json!({"msg": "hi"}))).await?;
//! assert_eq!(result.0["msg"], "hi");
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::channels::{AnyValue, BaseChannel};
use crate::error::GraphError;
use crate::graph::state::State;

/// The channel key used by [`ValueState`] for its single passthrough channel.
const VALUE_CHANNEL_KEY: &str = "__value__";

/// Wrapper enabling existing `serde_json::Value`-based code to work with
/// generic [`StateGraph<S>`](super::StateGraph) /
/// [`CompiledGraph<S>`](super::CompiledGraph).
///
/// `ValueState` is a thin newtype around `serde_json::Value`. Its [`State`]
/// implementation uses a single [`AnyValue`] channel keyed `"__value__"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueState(pub serde_json::Value);

impl State for ValueState {
    fn channels() -> Vec<(String, Box<dyn BaseChannel>)> {
        vec![(
            VALUE_CHANNEL_KEY.to_owned(),
            Box::new(AnyValue::new(VALUE_CHANNEL_KEY)) as Box<dyn BaseChannel>,
        )]
    }

    fn from_channels(channels: &HashMap<String, Box<dyn BaseChannel>>) -> Result<Self, GraphError> {
        let value = channels
            .get(VALUE_CHANNEL_KEY)
            .and_then(|c| c.get())
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        Ok(Self(value))
    }
}

impl From<serde_json::Value> for ValueState {
    fn from(value: serde_json::Value) -> Self {
        Self(value)
    }
}

impl From<ValueState> for serde_json::Value {
    fn from(state: ValueState) -> Self {
        state.0
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn value_state_round_trip_via_serde() {
        let original = ValueState(serde_json::json!({"key": "value", "num": 42}));
        let serialised = original.to_value().unwrap();
        let restored = ValueState::from_value(serialised).unwrap();
        assert_eq!(original.0, restored.0);
    }

    #[test]
    fn value_state_from_json_value() {
        let json = serde_json::json!({"hello": "world"});
        let state: ValueState = json.clone().into();
        assert_eq!(state.0, json);
    }

    #[test]
    fn value_state_into_json_value() {
        let state = ValueState(serde_json::json!({"hello": "world"}));
        let json: serde_json::Value = state.into();
        assert_eq!(json, serde_json::json!({"hello": "world"}));
    }

    #[test]
    fn value_state_channels_has_one_entry() {
        let channels = ValueState::channels();
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].0, "__value__");
    }
}
