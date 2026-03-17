//! In-memory skill registry with progressive disclosure support.

use crate::loader::SkillEntry;

/// An in-memory registry of loaded skills.
///
/// Supports progressive disclosure: callers can list skill names and
/// descriptions cheaply, then retrieve the full body only when a skill is
/// activated.
#[derive(Debug, Default)]
pub struct SkillRegistry {
    entries: Vec<SkillEntry>,
}

impl SkillRegistry {
    /// Create an empty [`SkillRegistry`].
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Register a skill entry.
    pub fn register(&mut self, entry: SkillEntry) {
        self.entries.push(entry);
    }

    /// Find a skill by its manifest name.
    pub fn find_by_name(&self, name: &str) -> Option<&SkillEntry> {
        self.entries.iter().find(|e| e.manifest.name == name)
    }

    /// Return a list of `(name, description)` pairs for all registered skills.
    ///
    /// Suitable for presenting a compact skill catalogue to an LLM at startup
    /// without transmitting full skill bodies.
    pub fn list_names_and_descriptions(&self) -> Vec<(&str, &str)> {
        self.entries
            .iter()
            .map(|e| (e.manifest.name.as_str(), e.manifest.description.as_str()))
            .collect()
    }

    /// Return the full body of a skill by name.
    ///
    /// Returns `None` if no skill with that name is registered.
    pub fn get_full_body(&self, name: &str) -> Option<&str> {
        self.find_by_name(name).map(|e| e.body.as_str())
    }
}
