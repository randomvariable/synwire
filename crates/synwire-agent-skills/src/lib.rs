//! Agent skills runtime for Synwire.
//!
//! This crate implements the [agentskills.io](https://agentskills.io)
//! specification for discoverable, composable agent skills, extended with
//! Synwire-specific runtime hints.
//!
//! # Overview
//!
//! A skill is a directory containing:
//! - `SKILL.md` — manifest (YAML frontmatter) + instructions body
//! - `scripts/` — optional runtime scripts
//! - `references/` — optional reference material
//! - `assets/` — optional static assets
//!
//! Skills are discovered from two locations:
//! - Global: `$DATA/<product>/skills/`
//! - Project-local: `.<product>/skills/`
//!
//! # Quick start
//!
//! ```no_run
//! use std::path::Path;
//! use synwire_agent_skills::{loader::SkillLoader, registry::SkillRegistry};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let loader = SkillLoader::new();
//! let entries = loader.scan(Path::new("/path/to/skills")).await?;
//!
//! let mut registry = SkillRegistry::new();
//! for entry in entries {
//!     registry.register(entry);
//! }
//!
//! for (name, desc) in registry.list_names_and_descriptions() {
//!     println!("{name}: {desc}");
//! }
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]

pub mod error;
pub mod loader;
pub mod manifest;
pub mod registry;
pub mod runtime;
