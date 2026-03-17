//! Tool-sequence runtime.
//!
//! Validates a declarative sequence of tool-call steps and returns them for the
//! caller to execute. This runtime does **not** dispatch tools itself; it acts
//! as a schema-level validator and pass-through.
//!
//! # Input format
//!
//! The `args` JSON must contain a `steps` array, where each element is an
//! object with a `tool` string and an `args` object:
//!
//! ```json
//! {
//!     "steps": [
//!         { "tool": "grep", "args": { "pattern": "fn main" } },
//!         { "tool": "read", "args": { "path": "src/lib.rs" } }
//!     ]
//! }
//! ```

use crate::error::SkillError;
use crate::runtime::{SkillExecutor, SkillInput, SkillOutput};

/// Executor that validates a tool-call sequence and returns it for external
/// dispatch.
#[derive(Debug, Default)]
pub struct SequenceRuntime {}

impl SequenceRuntime {
    /// Create a new [`SequenceRuntime`].
    pub const fn new() -> Self {
        Self {}
    }
}

impl SkillExecutor for SequenceRuntime {
    fn execute(&self, input: SkillInput) -> Result<SkillOutput, SkillError> {
        let steps = input
            .args
            .get("steps")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                SkillError::InvalidManifest(
                    "tool-sequence input must contain a 'steps' array".to_owned(),
                )
            })?;

        for (i, step) in steps.iter().enumerate() {
            let obj = step.as_object().ok_or_else(|| {
                SkillError::InvalidManifest(format!("step {i} must be a JSON object"))
            })?;

            let tool = obj.get("tool").ok_or_else(|| {
                SkillError::InvalidManifest(format!("step {i} is missing the 'tool' field"))
            })?;

            if !tool.is_string() {
                return Err(SkillError::InvalidManifest(format!(
                    "step {i} 'tool' field must be a string"
                )));
            }

            let args = obj.get("args").ok_or_else(|| {
                SkillError::InvalidManifest(format!("step {i} is missing the 'args' field"))
            })?;

            if !args.is_object() {
                return Err(SkillError::InvalidManifest(format!(
                    "step {i} 'args' field must be an object"
                )));
            }
        }

        let count = steps.len();
        Ok(SkillOutput {
            result: serde_json::json!({
                "steps": steps,
                "count": count,
            }),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::runtime::SkillInput;

    #[test]
    fn valid_steps_pass_through() {
        let runtime = SequenceRuntime::new();
        let input = SkillInput {
            args: serde_json::json!({
                "steps": [
                    { "tool": "grep", "args": { "pattern": "fn" } },
                    { "tool": "read", "args": { "path": "lib.rs" } }
                ]
            }),
        };
        let output = runtime.execute(input).expect("valid steps should succeed");
        assert_eq!(output.result["count"], 2);
        assert_eq!(output.result["steps"][0]["tool"], serde_json::json!("grep"));
    }

    #[test]
    fn missing_steps_returns_error() {
        let runtime = SequenceRuntime::new();
        let input = SkillInput {
            args: serde_json::json!({}),
        };
        let err = runtime
            .execute(input)
            .expect_err("missing steps should fail");
        assert!(matches!(err, SkillError::InvalidManifest(_)));
    }

    #[test]
    fn step_missing_tool_returns_error() {
        let runtime = SequenceRuntime::new();
        let input = SkillInput {
            args: serde_json::json!({
                "steps": [{ "args": { "x": 1 } }]
            }),
        };
        let err = runtime
            .execute(input)
            .expect_err("missing tool should fail");
        assert!(matches!(err, SkillError::InvalidManifest(_)));
    }

    #[test]
    fn step_args_not_object_returns_error() {
        let runtime = SequenceRuntime::new();
        let input = SkillInput {
            args: serde_json::json!({
                "steps": [{ "tool": "grep", "args": "not-an-object" }]
            }),
        };
        let err = runtime
            .execute(input)
            .expect_err("non-object args should fail");
        assert!(matches!(err, SkillError::InvalidManifest(_)));
    }

    #[test]
    fn empty_steps_is_valid() {
        let runtime = SequenceRuntime::new();
        let input = SkillInput {
            args: serde_json::json!({ "steps": [] }),
        };
        let output = runtime.execute(input).expect("empty steps should succeed");
        assert_eq!(output.result["count"], 0);
    }
}
