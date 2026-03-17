//! # synwire-orchestrator
//!
//! Synwire's graph-based orchestration engine.
//!
//! Provides [`StateGraph`](graph::StateGraph) for building state machines
//! with nodes and edges, compiled into
//! [`CompiledGraph`](graph::CompiledGraph) for execution via the Pregel
//! engine. Supports channels, checkpointing, interrupts, and streaming.
//!
//! # Quick start
//!
//! ```rust,ignore
//! use synwire_orchestrator::graph::{StateGraph, ValueState};
//! use synwire_orchestrator::constants::END;
//! use synwire_orchestrator::func::sync_node;
//!
//! let mut graph = StateGraph::<ValueState>::new();
//! graph.add_node("echo", sync_node(|s| Ok(s)))?;
//! graph.set_entry_point("echo").add_edge("echo", END);
//! let compiled = graph.compile()?;
//! let input = ValueState(serde_json::json!({"msg": "hi"}));
//! let result = compiled.invoke(input).await?;
//! ```

#![forbid(unsafe_code)]

pub mod channels;
pub mod config;
pub mod constants;
pub mod error;
pub mod func;
pub mod graph;
pub mod managed;
pub mod messages;
pub mod metrics;
pub mod prebuilt;
pub mod registry;
pub mod types;
