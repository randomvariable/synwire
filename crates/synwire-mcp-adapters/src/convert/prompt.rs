//! MCP prompt retrieval and conversion.
//!
//! Converts MCP prompt responses to Synwire message types.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::McpAdapterError;

// ---------------------------------------------------------------------------
// MCP prompt types
// ---------------------------------------------------------------------------

/// Role of a message in an MCP prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpPromptRole {
    /// User role.
    User,
    /// Assistant role.
    Assistant,
}

/// A single content block within an MCP prompt message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[non_exhaustive]
pub enum McpPromptContent {
    /// Plain text content.
    Text {
        /// The text.
        text: String,
    },
    /// Image content.
    Image {
        /// MIME type.
        mime_type: String,
        /// Base64 data.
        data: String,
    },
    /// Resource reference.
    Resource {
        /// Resource URI.
        uri: String,
        /// Optional text content.
        text: Option<String>,
    },
}

/// A single message in an MCP prompt response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptMessage {
    /// Message role.
    pub role: McpPromptRole,
    /// Message content.
    pub content: McpPromptContent,
}

/// A converted Synwire prompt message with role and text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertedPromptMessage {
    /// Message role ("user" or "assistant").
    pub role: String,
    /// Extracted text content.
    pub text: String,
    /// Raw content for non-text blocks (image data, resource URI, etc.).
    pub raw_content: Option<Value>,
}

// ---------------------------------------------------------------------------
// Prompt retrieval (T280)
// ---------------------------------------------------------------------------

/// Fetches a named prompt from an MCP server.
///
/// Sends `prompts/get` with the given prompt name and optional arguments.
/// Returns the list of messages as [`ConvertedPromptMessage`]s.
///
/// # Errors
///
/// Returns [`McpAdapterError`] on transport or serialization errors.
pub async fn get_prompt(
    transport: &dyn synwire_core::mcp::traits::McpTransport,
    prompt_name: &str,
    arguments: Option<Value>,
) -> Result<Vec<ConvertedPromptMessage>, McpAdapterError> {
    let params = serde_json::json!({
        "name": prompt_name,
        "arguments": arguments.unwrap_or_else(|| serde_json::json!({})),
    });

    let response = transport
        .call_tool("prompts/get", params)
        .await
        .map_err(|e| McpAdapterError::Transport {
            message: format!("prompts/get failed for '{prompt_name}': {e}"),
        })?;

    let messages = response["messages"]
        .as_array()
        .ok_or_else(|| McpAdapterError::Transport {
            message: format!("prompts/get response missing 'messages' for '{prompt_name}'"),
        })?;

    let converted: Vec<ConvertedPromptMessage> = messages
        .iter()
        .filter_map(
            |m| match serde_json::from_value::<McpPromptMessage>(m.clone()) {
                Ok(msg) => Some(convert_mcp_prompt_message(msg)),
                Err(e) => {
                    tracing::warn!(error = %e, "Skipping malformed prompt message");
                    None
                }
            },
        )
        .collect();

    Ok(converted)
}

/// Converts a single [`McpPromptMessage`] into a [`ConvertedPromptMessage`].
///
/// Role is mapped directly. Content is extracted as text where possible;
/// non-text content is preserved in `raw_content`.
pub fn convert_mcp_prompt_message(message: McpPromptMessage) -> ConvertedPromptMessage {
    let role = match message.role {
        McpPromptRole::User => "user".to_owned(),
        McpPromptRole::Assistant => "assistant".to_owned(),
    };

    match message.content {
        McpPromptContent::Text { text } => ConvertedPromptMessage {
            role,
            text,
            raw_content: None,
        },
        McpPromptContent::Image { mime_type, data } => ConvertedPromptMessage {
            role,
            text: format!("[image: {mime_type}]"),
            raw_content: Some(serde_json::json!({
                "type": "image",
                "mime_type": mime_type,
                "data": data,
            })),
        },
        McpPromptContent::Resource { uri, text } => {
            let display = text.clone().unwrap_or_else(|| uri.clone());
            ConvertedPromptMessage {
                role,
                text: display,
                raw_content: Some(serde_json::json!({
                    "type": "resource",
                    "uri": uri,
                    "text": text,
                })),
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn text_message_converts() {
        let msg = McpPromptMessage {
            role: McpPromptRole::User,
            content: McpPromptContent::Text {
                text: "Hello!".into(),
            },
        };
        let converted = convert_mcp_prompt_message(msg);
        assert_eq!(converted.role, "user");
        assert_eq!(converted.text, "Hello!");
        assert!(converted.raw_content.is_none());
    }

    #[test]
    fn assistant_role_maps() {
        let msg = McpPromptMessage {
            role: McpPromptRole::Assistant,
            content: McpPromptContent::Text { text: "ok".into() },
        };
        let converted = convert_mcp_prompt_message(msg);
        assert_eq!(converted.role, "assistant");
    }

    #[test]
    fn image_content_preserves_raw() {
        let msg = McpPromptMessage {
            role: McpPromptRole::User,
            content: McpPromptContent::Image {
                mime_type: "image/png".into(),
                data: "AAAA".into(),
            },
        };
        let converted = convert_mcp_prompt_message(msg);
        assert!(converted.text.contains("image/png"));
        assert!(converted.raw_content.is_some());
    }
}
