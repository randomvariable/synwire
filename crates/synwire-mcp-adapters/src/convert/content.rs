//! MCP content type ↔ Synwire type mapping.
//!
//! Maps the MCP content block variants (Text, Image, `ResourceLink`,
//! `EmbeddedResource`, `AudioContent`) to their Synwire equivalents.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use synwire_core::tools::{BinaryResult, ToolContentType, ToolOutput, ToolResultStatus};

use crate::error::McpAdapterError;

// ---------------------------------------------------------------------------
// MCP content block (wire format)
// ---------------------------------------------------------------------------

/// A single MCP content block as returned by a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[non_exhaustive]
pub enum McpContentBlock {
    /// Plain text content.
    Text {
        /// The text content.
        text: String,
    },
    /// Base64-encoded image data.
    #[serde(rename_all = "camelCase")]
    Image {
        /// MIME type (e.g. `image/png`).
        mime_type: String,
        /// Base64-encoded image data.
        data: String,
    },
    /// A link to an MCP resource URI (not embedded).
    #[serde(rename_all = "camelCase")]
    Resource {
        /// The resource URI.
        uri: String,
        /// Optional MIME type hint.
        mime_type: Option<String>,
        /// Optional display text.
        text: Option<String>,
    },
    /// An embedded MCP resource with full content.
    #[serde(rename_all = "camelCase")]
    EmbeddedResource {
        /// The resource URI.
        uri: String,
        /// MIME type.
        mime_type: Option<String>,
        /// Embedded text content (if text resource).
        text: Option<String>,
        /// Embedded binary content, base64-encoded (if binary resource).
        blob: Option<String>,
    },
    /// Audio content (not yet supported by the Synwire tool layer).
    #[serde(rename_all = "camelCase")]
    Audio {
        /// MIME type (e.g. `audio/wav`).
        mime_type: String,
        /// Base64-encoded audio data.
        data: String,
    },
}

/// The full response payload from an MCP tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolCallResponse {
    /// Content blocks returned by the tool.
    pub content: Vec<McpContentBlock>,
    /// Whether the tool reported an error.
    #[serde(default)]
    pub is_error: bool,
}

// ---------------------------------------------------------------------------
// Conversion
// ---------------------------------------------------------------------------

/// Converts an MCP tool call response value into a Synwire [`ToolOutput`].
///
/// # Behaviour
///
/// - `Text` → `content` field.
/// - `Image` → decoded bytes in `binary_results`.
/// - `Resource` → resource URI in `content`, metadata in `artifact`.
/// - `EmbeddedResource` → text content or decoded bytes.
/// - `Audio` → recorded as `UnsupportedContent` error.
/// - `isError: true` → `status` set to `Failure`.
///
/// # Errors
///
/// Returns [`McpAdapterError::Serialization`] if the response cannot be
/// deserialized.
pub fn convert_mcp_response_to_tool_output(raw: Value) -> Result<ToolOutput, McpAdapterError> {
    let response: McpToolCallResponse = serde_json::from_value(raw)?;

    let status = if response.is_error {
        ToolResultStatus::Failure
    } else {
        ToolResultStatus::Success
    };

    let mut text_parts: Vec<String> = Vec::new();
    let mut binary_results: Vec<BinaryResult> = Vec::new();
    let mut artifact: Option<Value> = None;
    let mut content_type = ToolContentType::Text;

    for block in response.content {
        match block {
            McpContentBlock::Text { text } => {
                text_parts.push(text);
                content_type = ToolContentType::Text;
            }
            McpContentBlock::Image { mime_type, data } => {
                let bytes = base64_decode(&data);
                binary_results.push(BinaryResult { bytes, mime_type });
                content_type = ToolContentType::Image;
            }
            McpContentBlock::Resource {
                uri,
                mime_type,
                text,
            } => {
                let desc = text.as_deref().unwrap_or(&uri);
                text_parts.push(format!("[resource: {desc}]"));
                let resource_meta = serde_json::json!({
                    "uri": uri,
                    "mime_type": mime_type,
                });
                artifact = Some(resource_meta);
            }
            McpContentBlock::EmbeddedResource {
                uri,
                mime_type,
                text,
                blob,
            } => {
                if let Some(t) = text {
                    text_parts.push(t);
                } else if let Some(b) = blob {
                    let mime = mime_type.unwrap_or_else(|| "application/octet-stream".into());
                    binary_results.push(BinaryResult {
                        bytes: base64_decode(&b),
                        mime_type: mime,
                    });
                    content_type = ToolContentType::File;
                }
                let _ = uri; // URI preserved in artifact
                if artifact.is_none() {
                    artifact = Some(serde_json::json!({ "uri": uri }));
                }
            }
            McpContentBlock::Audio { mime_type, .. } => {
                // AudioContent is not supported — record as unsupported text
                text_parts.push(format!("[unsupported audio content: {mime_type}]"));
            }
        }
    }

    Ok(ToolOutput {
        content: text_parts.join("\n"),
        artifact,
        binary_results,
        status,
        content_type: Some(content_type),
        ..ToolOutput::default()
    })
}

/// Minimal base64 decoder (standard alphabet, no padding validation).
fn base64_decode(s: &str) -> Vec<u8> {
    use std::collections::HashMap;
    // Build alphabet
    let alphabet: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut lookup: HashMap<u8, u8> = HashMap::new();
    for (i, &c) in alphabet.iter().enumerate() {
        let _ = lookup.insert(c, u8::try_from(i).unwrap_or(0));
    }

    let bytes: Vec<u8> = s
        .bytes()
        .filter(|b| *b != b'=')
        .filter_map(|b| lookup.get(&b).copied())
        .collect();

    let mut output = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut i = 0;
    while i + 3 < bytes.len() {
        output.push((bytes[i] << 2) | (bytes[i + 1] >> 4));
        output.push((bytes[i + 1] << 4) | (bytes[i + 2] >> 2));
        output.push((bytes[i + 2] << 6) | bytes[i + 3]);
        i += 4;
    }
    if i + 2 < bytes.len() {
        output.push((bytes[i] << 2) | (bytes[i + 1] >> 4));
        output.push((bytes[i + 1] << 4) | (bytes[i + 2] >> 2));
    } else if i + 1 < bytes.len() {
        output.push((bytes[i] << 2) | (bytes[i + 1] >> 4));
    }
    output
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn text_content_converts() {
        let raw = serde_json::json!({
            "content": [{ "type": "text", "text": "hello world" }],
            "isError": false
        });
        let output = convert_mcp_response_to_tool_output(raw).unwrap();
        assert_eq!(output.content, "hello world");
        assert_eq!(output.status, ToolResultStatus::Success);
    }

    #[test]
    fn is_error_sets_failure_status() {
        let raw = serde_json::json!({
            "content": [{ "type": "text", "text": "oops" }],
            "isError": true
        });
        let output = convert_mcp_response_to_tool_output(raw).unwrap();
        assert_eq!(output.status, ToolResultStatus::Failure);
    }

    #[test]
    fn audio_content_becomes_unsupported_text() {
        let raw = serde_json::json!({
            "content": [{ "type": "audio", "mimeType": "audio/wav", "data": "AAAA" }],
            "isError": false
        });
        let output = convert_mcp_response_to_tool_output(raw).unwrap();
        assert!(output.content.contains("unsupported audio content"));
    }

    #[test]
    fn multiple_text_blocks_joined() {
        let raw = serde_json::json!({
            "content": [
                { "type": "text", "text": "line 1" },
                { "type": "text", "text": "line 2" }
            ],
            "isError": false
        });
        let output = convert_mcp_response_to_tool_output(raw).unwrap();
        assert!(output.content.contains("line 1"));
        assert!(output.content.contains("line 2"));
    }
}
