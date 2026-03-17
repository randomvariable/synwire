//! SKILL.md frontmatter parser following the agentskills.io specification.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::SkillError;

/// The runtime environment for a skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum SkillRuntime {
    /// Lua scripting runtime.
    Lua,
    /// Rhai scripting runtime.
    Rhai,
    /// WebAssembly runtime.
    Wasm,
    /// A sequence of tool invocations.
    ToolSequence,
    /// An external process.
    External,
}

/// A directive to create a new tool from a skill script.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CreateToolDirective {
    /// The name of the tool to create.
    pub name: String,
    /// A description of what the tool does.
    pub description: String,
    /// The runtime that executes the tool.
    pub runtime: SkillRuntime,
    /// The script source for the tool.
    pub script: String,
}

/// Raw YAML frontmatter as deserialized from SKILL.md.
#[derive(Debug, Deserialize)]
struct RawFrontmatter {
    name: String,
    description: String,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    compatibility: Option<String>,
    #[serde(default)]
    metadata: HashMap<String, String>,
    #[serde(rename = "allowed-tools", default)]
    allowed_tools: Option<String>,
    #[serde(default)]
    runtime: Option<SkillRuntime>,
}

/// A parsed and validated SKILL.md manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SkillManifest {
    /// The skill name: 1–64 chars, lowercase letters, digits, hyphens.
    pub name: String,
    /// Human-readable description: 1–1024 chars.
    pub description: String,
    /// SPDX license identifier.
    pub license: Option<String>,
    /// Semver compatibility expression.
    pub compatibility: Option<String>,
    /// Arbitrary string key-value metadata.
    pub metadata: HashMap<String, String>,
    /// Tools this skill is permitted to invoke.
    pub allowed_tools: Vec<String>,
    /// Optional runtime hint (Synwire extension).
    pub runtime: Option<SkillRuntime>,
}

/// Parse a `SKILL.md` file, extracting and validating the YAML frontmatter.
///
/// The frontmatter must be enclosed between two `---` delimiter lines at the
/// start of the file. Everything after the closing `---` is the skill body and
/// is not processed by this function.
///
/// # Errors
///
/// Returns [`SkillError::InvalidManifest`] when:
/// - The file does not contain `---` delimiters.
/// - The `name` field fails validation (empty, too long, or contains invalid
///   characters).
/// - The `description` field is empty or exceeds 1024 characters.
///
/// Returns [`SkillError::Yaml`] when the frontmatter cannot be parsed as YAML.
///
/// # Examples
///
/// ```
/// use synwire_agent_skills::manifest::parse_skill_md;
///
/// let content = r#"---
/// name: my-skill
/// description: "Does something useful"
/// ---
/// ## Instructions
/// Use this skill to do something.
/// "#;
///
/// let manifest = parse_skill_md(content).unwrap();
/// assert_eq!(manifest.name, "my-skill");
/// ```
pub fn parse_skill_md(content: &str) -> Result<SkillManifest, SkillError> {
    // Split off the frontmatter block.
    let after_first = content
        .strip_prefix("---")
        .and_then(|s| s.strip_prefix('\n').or_else(|| s.strip_prefix("\r\n")))
        .ok_or_else(|| {
            SkillError::InvalidManifest(
                "SKILL.md must begin with a '---' frontmatter delimiter".to_owned(),
            )
        })?;

    let end = after_first.find("\n---").ok_or_else(|| {
        SkillError::InvalidManifest(
            "SKILL.md frontmatter is not closed with a '---' delimiter".to_owned(),
        )
    })?;

    let frontmatter = &after_first[..end];
    let raw: RawFrontmatter = serde_yaml::from_str(frontmatter)?;

    validate_name(&raw.name)?;
    validate_description(&raw.description)?;

    let allowed_tools = raw
        .allowed_tools
        .as_deref()
        .map(|s| {
            s.split_whitespace()
                .filter(|t| !t.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();

    Ok(SkillManifest {
        name: raw.name,
        description: raw.description,
        license: raw.license,
        compatibility: raw.compatibility,
        metadata: raw.metadata,
        allowed_tools,
        runtime: raw.runtime,
    })
}

/// Validate a skill name.
fn validate_name(name: &str) -> Result<(), SkillError> {
    if name.is_empty() {
        return Err(SkillError::InvalidManifest(
            "skill name must not be empty".to_owned(),
        ));
    }
    if name.len() > 64 {
        return Err(SkillError::InvalidManifest(format!(
            "skill name '{name}' exceeds 64 characters"
        )));
    }
    let valid = name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !valid {
        return Err(SkillError::InvalidManifest(format!(
            "skill name '{name}' contains invalid characters (only lowercase letters, digits, and hyphens are allowed)"
        )));
    }
    Ok(())
}

/// Validate a skill description.
fn validate_description(description: &str) -> Result<(), SkillError> {
    if description.is_empty() {
        return Err(SkillError::InvalidManifest(
            "skill description must not be empty".to_owned(),
        ));
    }
    if description.len() > 1024 {
        return Err(SkillError::InvalidManifest(format!(
            "skill description exceeds 1024 characters (got {})",
            description.len()
        )));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    const VALID_SKILL: &str = r#"---
name: my-skill
description: "Does something useful"
license: MIT
allowed-tools: read write grep
runtime: lua
metadata:
  author: test
---
## Instructions
Use this skill to do something.
"#;

    #[test]
    fn valid_frontmatter_parses_correctly() {
        let manifest = parse_skill_md(VALID_SKILL).expect("should parse valid SKILL.md");
        assert_eq!(manifest.name, "my-skill");
        assert_eq!(manifest.description, "Does something useful");
        assert_eq!(manifest.license.as_deref(), Some("MIT"));
        assert_eq!(
            manifest.allowed_tools,
            vec!["read".to_owned(), "write".to_owned(), "grep".to_owned()]
        );
        assert_eq!(manifest.runtime, Some(SkillRuntime::Lua));
        assert_eq!(
            manifest.metadata.get("author").map(String::as_str),
            Some("test")
        );
    }

    #[test]
    fn invalid_name_uppercase_returns_error() {
        let content = "---\nname: MySkill\ndescription: \"desc\"\n---\n";
        let err = parse_skill_md(content).expect_err("uppercase name should fail");
        assert!(
            matches!(err, SkillError::InvalidManifest(_)),
            "expected InvalidManifest, got {err}"
        );
    }

    #[test]
    fn empty_description_returns_error() {
        let content = "---\nname: my-skill\ndescription: \"\"\n---\n";
        let err = parse_skill_md(content).expect_err("empty description should fail");
        assert!(
            matches!(err, SkillError::InvalidManifest(_)),
            "expected InvalidManifest, got {err}"
        );
    }

    #[test]
    fn missing_delimiters_returns_error() {
        let content = "name: my-skill\ndescription: desc\n";
        let err = parse_skill_md(content).expect_err("missing delimiters should fail");
        assert!(
            matches!(err, SkillError::InvalidManifest(_)),
            "expected InvalidManifest, got {err}"
        );
    }

    #[test]
    fn allowed_tools_split_correctly() {
        let content =
            "---\nname: my-skill\ndescription: \"desc\"\nallowed-tools: read write grep\n---\n";
        let manifest = parse_skill_md(content).expect("should parse");
        assert_eq!(
            manifest.allowed_tools,
            vec!["read".to_owned(), "write".to_owned(), "grep".to_owned()]
        );
    }

    #[test]
    fn runtime_lua_parses_correctly() {
        let content = "---\nname: my-skill\ndescription: \"desc\"\nruntime: lua\n---\n";
        let manifest = parse_skill_md(content).expect("should parse");
        assert_eq!(manifest.runtime, Some(SkillRuntime::Lua));
    }
}
