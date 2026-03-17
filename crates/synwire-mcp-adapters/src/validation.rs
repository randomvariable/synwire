//! JSON Schema validation of tool arguments before invocation.
//!
//! Uses the `jsonschema` crate to validate tool arguments against the
//! tool's declared schema before the call is forwarded to the MCP server.

use serde_json::Value;

use crate::error::McpAdapterError;

// ---------------------------------------------------------------------------
// Schema validation (T285)
// ---------------------------------------------------------------------------

/// Validates `arguments` against the JSON Schema `schema`.
///
/// Returns `Ok(())` if the arguments are valid, or
/// [`McpAdapterError::SchemaValidation`] with a description of the first
/// validation error.
///
/// # Notes
///
/// If `schema` is not a valid JSON Schema object the function returns
/// `SchemaValidation` with a compilation error message.
pub fn validate_tool_arguments(
    tool_name: &str,
    schema: &Value,
    arguments: &Value,
) -> Result<(), McpAdapterError> {
    let validator =
        jsonschema::validator_for(schema).map_err(|e| McpAdapterError::SchemaValidation {
            tool: tool_name.to_owned(),
            reason: format!("schema compilation failed: {e}"),
        })?;

    if !validator.is_valid(arguments) {
        let reason = validator
            .validate(arguments)
            .err()
            .map_or_else(|| "validation failed".to_owned(), |e| e.to_string());

        return Err(McpAdapterError::SchemaValidation {
            tool: tool_name.to_owned(),
            reason,
        });
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn simple_schema() -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" },
                "count": { "type": "integer" }
            },
            "required": ["query"]
        })
    }

    #[test]
    fn valid_args_pass() {
        let args = serde_json::json!({ "query": "hello" });
        assert!(validate_tool_arguments("search", &simple_schema(), &args).is_ok());
    }

    #[test]
    fn missing_required_field_fails() {
        let args = serde_json::json!({ "count": 5 });
        let result = validate_tool_arguments("search", &simple_schema(), &args);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("search") || err.contains("validation"));
    }

    #[test]
    fn wrong_type_fails() {
        let args = serde_json::json!({ "query": 42 });
        let result = validate_tool_arguments("search", &simple_schema(), &args);
        assert!(result.is_err());
    }

    #[test]
    fn extra_fields_allowed_by_default() {
        let args = serde_json::json!({ "query": "hi", "extra": true });
        assert!(validate_tool_arguments("search", &simple_schema(), &args).is_ok());
    }

    #[test]
    fn additional_properties_false_rejects_extras() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": { "q": { "type": "string" } },
            "required": ["q"],
            "additionalProperties": false
        });
        let args = serde_json::json!({ "q": "ok", "extra": "bad" });
        let result = validate_tool_arguments("strict-tool", &schema, &args);
        assert!(result.is_err());
    }
}
