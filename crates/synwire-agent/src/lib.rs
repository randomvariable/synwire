#![forbid(unsafe_code)]

//! # synwire-agent
//!
//! Agent runtime implementations for Synwire — the concrete backend layer.
//!
//! This crate provides real implementations of the agent runtime traits defined
//! in `synwire-core`, wiring abstract capabilities to live backends and services.
//!
//! ## Key modules
//!
//! | Module | Purpose |
//! |---|---|
//! | [`vfs`] | VFS providers (`LocalProvider`, `MemoryProvider`, `CompositeProvider`) |
//! | [`sandbox`] | Process sandboxing, isolation, and output capture |
//! | [`middleware`] | Request/response middleware pipeline for agent execution |
//! | [`strategies`] | Execution strategies (single-pass, iterative, FSM-based) |
//! | [`mcp`] | MCP client transport and tool bridge |
//! | [`sampling`] | `SamplingProvider` implementations for tool-internal LLM access |
//! | [`session`] | Session lifecycle, persistence, and resumption |
//! | [`sbfl`] | Spectrum-based fault localisation for diagnostic ranking |
//! | [`dataflow`] | Intra-procedural dataflow analysis over indexed code |
//! | [`call_graph`] | Call-graph construction and traversal |
//! | [`experience`] | Experience pool storage and retrieval |
//! | [`experience_sampling`] | Sampling strategies over the experience pool |
//! | [`tools`] | Agent-level tool implementations |
//!
//! All I/O operations are async-first. This crate compiles with zero `unsafe`.

pub mod call_graph;
pub mod dataflow;
pub mod experience;
pub mod experience_sampling;
pub mod mcp;
pub mod middleware;
pub mod sampling;
pub mod sandbox;
pub mod sbfl;
pub mod session;
pub mod strategies;
pub mod tools;
pub mod vfs;
