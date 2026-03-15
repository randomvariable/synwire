//! `OTel` attribute mapper trait and default `GenAI` implementation.

use crate::observability::gen_ai;
use serde_json::Value;
use std::collections::HashMap;

/// Trait for mapping domain-specific attributes to OpenTelemetry key-value
/// pairs.
pub trait OTelAttributeMapper: Send + Sync {
    /// Maps the given attributes into OTel-compatible key-value pairs.
    fn map_attributes(&self, input: &HashMap<String, Value>) -> Vec<(String, String)>;
}

/// Default attribute mapper that translates `GenAI` semantic convention keys.
///
/// Recognised input keys (matching `gen_ai::*` constants) are mapped directly.
/// Unrecognised keys are prefixed with `synwire.custom.`.
#[derive(Debug, Default)]
pub struct GenAIAttributeMapper;

impl GenAIAttributeMapper {
    /// Creates a new mapper.
    pub const fn new() -> Self {
        Self
    }
}

/// Known `GenAI` attribute keys.
const KNOWN_KEYS: &[&str] = &[
    gen_ai::OPERATION_NAME,
    gen_ai::PROVIDER_NAME,
    gen_ai::REQUEST_MODEL,
    gen_ai::REQUEST_TEMPERATURE,
    gen_ai::REQUEST_MAX_TOKENS,
    gen_ai::RESPONSE_MODEL,
    gen_ai::RESPONSE_FINISH_REASONS,
    gen_ai::RESPONSE_ID,
    gen_ai::USAGE_INPUT_TOKENS,
    gen_ai::USAGE_OUTPUT_TOKENS,
];

impl OTelAttributeMapper for GenAIAttributeMapper {
    fn map_attributes(&self, input: &HashMap<String, Value>) -> Vec<(String, String)> {
        let mut out = Vec::with_capacity(input.len());
        for (key, value) in input {
            let otel_key = if KNOWN_KEYS.contains(&key.as_str()) {
                key.clone()
            } else {
                format!("synwire.custom.{key}")
            };

            let otel_value = match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => String::new(),
                other => other.to_string(),
            };

            out.push((otel_key, otel_value));
        }
        out
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_known_keys_directly() {
        let mapper = GenAIAttributeMapper::new();
        let mut input = HashMap::new();
        let _ = input.insert(gen_ai::REQUEST_MODEL.to_owned(), json!("gpt-4"));
        let _ = input.insert(gen_ai::USAGE_INPUT_TOKENS.to_owned(), json!(100));

        let attrs = mapper.map_attributes(&input);
        assert_eq!(attrs.len(), 2);

        let model_attr = attrs.iter().find(|(k, _)| k == gen_ai::REQUEST_MODEL);
        assert!(model_attr.is_some());
        assert_eq!(model_attr.unwrap().1, "gpt-4");
    }

    #[test]
    fn prefixes_unknown_keys() {
        let mapper = GenAIAttributeMapper::new();
        let mut input = HashMap::new();
        let _ = input.insert("my.custom.key".to_owned(), json!("value"));

        let attrs = mapper.map_attributes(&input);
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0].0, "synwire.custom.my.custom.key");
    }

    #[test]
    fn handles_various_value_types() {
        let mapper = GenAIAttributeMapper::new();
        let mut input = HashMap::new();
        let _ = input.insert(gen_ai::REQUEST_TEMPERATURE.to_owned(), json!(0.7));
        let _ = input.insert("flag".to_owned(), json!(true));
        let _ = input.insert("empty".to_owned(), json!(null));

        let attrs = mapper.map_attributes(&input);
        assert_eq!(attrs.len(), 3);
    }
}
