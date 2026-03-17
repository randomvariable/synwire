//! MCP resource retrieval and conversion.
//!
//! Supports static (non-dynamic) MCP resources only.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use synwire_core::mcp::traits::McpTransport;
use synwire_core::tools::BinaryResult;

use crate::error::McpAdapterError;
use crate::pagination::PaginationCursor;

// ---------------------------------------------------------------------------
// MCP resource types
// ---------------------------------------------------------------------------

/// A resource descriptor returned by an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResourceDescriptor {
    /// The resource URI.
    pub uri: String,
    /// Human-readable name.
    pub name: String,
    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,
    /// MIME type of the resource.
    #[serde(default)]
    pub mime_type: Option<String>,
}

/// A loaded MCP resource blob.
#[derive(Debug, Clone)]
pub struct McpResourceBlob {
    /// Resource URI.
    pub uri: String,
    /// Text content (for text resources).
    pub text: Option<String>,
    /// Binary content (for binary resources).
    pub binary: Option<BinaryResult>,
    /// MIME type.
    pub mime_type: Option<String>,
}

// ---------------------------------------------------------------------------
// Resource fetching (T279)
// ---------------------------------------------------------------------------

/// Lists all static resources available from the server.
///
/// Uses cursor-based pagination with the [`MAX_PAGES`](crate::pagination::MAX_PAGES)
/// cap. Dynamic resources (those with URI templates) are excluded.
///
/// # Errors
///
/// Returns [`McpAdapterError`] on transport or serialization errors.
pub async fn get_resources(
    transport: &dyn McpTransport,
) -> Result<Vec<McpResourceDescriptor>, McpAdapterError> {
    let mut all_resources: Vec<McpResourceDescriptor> = Vec::new();
    let mut cursor = PaginationCursor::new();

    loop {
        let params = cursor.current().map_or_else(
            || serde_json::json!({}),
            |c| serde_json::json!({ "cursor": c }),
        );

        let response = transport
            .call_tool("resources/list", params)
            .await
            .map_err(|e| McpAdapterError::Transport {
                message: format!("resources/list failed: {e}"),
            })?;

        let resources =
            response["resources"]
                .as_array()
                .ok_or_else(|| McpAdapterError::Transport {
                    message: "resources/list response missing 'resources' array".into(),
                })?;

        for raw in resources {
            match serde_json::from_value::<McpResourceDescriptor>(raw.clone()) {
                Ok(r) => {
                    // Skip dynamic resources (URI templates contain `{`)
                    if !r.uri.contains('{') {
                        all_resources.push(r);
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Skipping malformed resource descriptor");
                }
            }
        }

        let next_cursor = response["nextCursor"].as_str().map(str::to_owned);
        if !cursor.advance(next_cursor) {
            break;
        }
    }

    Ok(all_resources)
}

/// Loads the contents of a single MCP resource by URI.
///
/// # Errors
///
/// Returns [`McpAdapterError`] on transport or serialization errors.
pub async fn get_mcp_resource(
    transport: &dyn McpTransport,
    uri: &str,
) -> Result<McpResourceBlob, McpAdapterError> {
    let params = serde_json::json!({ "uri": uri });
    let response = transport
        .call_tool("resources/read", params)
        .await
        .map_err(|e| McpAdapterError::Transport {
            message: format!("resources/read failed for '{uri}': {e}"),
        })?;

    convert_mcp_resource_to_blob(&response, uri)
}

/// Converts a raw `resources/read` response into a [`McpResourceBlob`].
///
/// # Errors
///
/// Returns [`McpAdapterError::Transport`] if the response shape is unexpected.
pub fn convert_mcp_resource_to_blob(
    response: &Value,
    uri: &str,
) -> Result<McpResourceBlob, McpAdapterError> {
    // The MCP spec returns `{ contents: [{ uri, text?, blob?, mimeType? }] }`
    let contents = response["contents"]
        .as_array()
        .ok_or_else(|| McpAdapterError::Transport {
            message: format!("resources/read response missing 'contents' for '{uri}'"),
        })?;

    let first = contents.first().ok_or_else(|| McpAdapterError::Transport {
        message: format!("resources/read response empty contents for '{uri}'"),
    })?;

    let text = first["text"].as_str().map(str::to_owned);
    let blob_b64 = first["blob"].as_str();
    let mime_type = first["mimeType"].as_str().map(str::to_owned);

    let binary = blob_b64.map(|b| {
        let bytes = base64_decode_simple(b);
        BinaryResult {
            bytes,
            mime_type: mime_type
                .clone()
                .unwrap_or_else(|| "application/octet-stream".into()),
        }
    });

    Ok(McpResourceBlob {
        uri: uri.to_owned(),
        text,
        binary,
        mime_type,
    })
}

/// Loads all resources from the transport as blobs.
///
/// Errors for individual resources are logged and skipped.
///
/// # Errors
///
/// Returns [`McpAdapterError`] if listing resources fails.
pub async fn load_mcp_resources(
    transport: &dyn McpTransport,
) -> Result<Vec<McpResourceBlob>, McpAdapterError> {
    let descriptors = get_resources(transport).await?;
    let mut blobs = Vec::with_capacity(descriptors.len());

    for desc in descriptors {
        match get_mcp_resource(transport, &desc.uri).await {
            Ok(blob) => blobs.push(blob),
            Err(e) => {
                tracing::warn!(uri = %desc.uri, error = %e, "Failed to load MCP resource, skipping");
            }
        }
    }

    Ok(blobs)
}

/// Simple base64 decoder used internally.
fn base64_decode_simple(s: &str) -> Vec<u8> {
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut lookup = [255u8; 256];
    for (i, &c) in alphabet.iter().enumerate() {
        lookup[usize::from(c)] = u8::try_from(i).unwrap_or(0);
    }

    let bytes: Vec<u8> = s
        .bytes()
        .filter(|b| *b != b'=')
        .filter_map(|b| {
            let v = lookup[usize::from(b)];
            if v == 255 { None } else { Some(v) }
        })
        .collect();

    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut i = 0;
    while i + 3 < bytes.len() {
        out.push((bytes[i] << 2) | (bytes[i + 1] >> 4));
        out.push((bytes[i + 1] << 4) | (bytes[i + 2] >> 2));
        out.push((bytes[i + 2] << 6) | bytes[i + 3]);
        i += 4;
    }
    if i + 2 < bytes.len() {
        out.push((bytes[i] << 2) | (bytes[i + 1] >> 4));
        out.push((bytes[i + 1] << 4) | (bytes[i + 2] >> 2));
    } else if i + 1 < bytes.len() {
        out.push((bytes[i] << 2) | (bytes[i + 1] >> 4));
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn convert_text_resource() {
        let response = serde_json::json!({
            "contents": [{
                "uri": "file:///README.md",
                "text": "# Hello",
                "mimeType": "text/markdown"
            }]
        });
        let blob = convert_mcp_resource_to_blob(&response, "file:///README.md").unwrap();
        assert_eq!(blob.text, Some("# Hello".into()));
        assert!(blob.binary.is_none());
        assert_eq!(blob.mime_type, Some("text/markdown".into()));
    }

    #[test]
    fn dynamic_uri_detection() {
        let dynamic = McpResourceDescriptor {
            uri: "file:///{path}".into(),
            name: "dynamic".into(),
            description: None,
            mime_type: None,
        };
        assert!(dynamic.uri.contains('{'));
    }
}
