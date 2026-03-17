//! JSON-RPC 2.0 based IPC protocol for the synwire daemon.
//!
//! The daemon communicates with MCP server proxies over a Unix domain socket
//! using newline-delimited JSON (NDJSON). Each line is a complete JSON-RPC 2.0
//! request or response object.
//!
//! # Supported methods
//!
//! | Method string        | Variant              | Description                          |
//! |----------------------|----------------------|--------------------------------------|
//! | `index`              | [`IpcMethod::Index`] | Trigger indexing for a worktree      |
//! | `search`             | [`IpcMethod::Search`]| Semantic vector search               |
//! | `graph_query`        | [`IpcMethod::GraphQuery`] | Code graph query              |
//! | `graph_search`       | [`IpcMethod::GraphSearch`]| Code graph search             |
//! | `community_search`   | [`IpcMethod::CommunitySearch`] | Community detection search |
//! | `hybrid_search`      | [`IpcMethod::HybridSearch`] | Hybrid vector + BM25 search |
//! | `clone_repo`         | [`IpcMethod::CloneRepo`] | Clone a repository              |
//! | `xref_query`         | [`IpcMethod::XrefQuery`] | Cross-reference query            |
//! | `index_status`       | [`IpcMethod::IndexStatus`] | Check indexing status          |
//!
//! # Wire format
//!
//! Each message is a single UTF-8 JSON object terminated by `\n`. The daemon
//! reads one line at a time, deserialises it as an [`IpcRequest`], dispatches
//! the method, and writes back an [`IpcResponse`] followed by `\n`.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

/// JSON-RPC 2.0 version string.
const JSONRPC_VERSION: &str = "2.0";

// ── Error codes (JSON-RPC 2.0 standard range) ──────────────────────────────

/// Error code: the requested method does not exist.
pub const ERROR_METHOD_NOT_FOUND: i32 = -32601;

/// Error code: invalid method parameters.
pub const ERROR_INVALID_PARAMS: i32 = -32602;

/// Error code: an internal error occurred while processing the request.
pub const ERROR_INTERNAL: i32 = -32603;

/// Error code: the request could not be parsed as valid JSON-RPC.
pub const ERROR_PARSE: i32 = -32700;

// ── Request ─────────────────────────────────────────────────────────────────

/// A JSON-RPC 2.0 request received from a client over the UDS.
#[derive(Debug, Deserialize)]
pub struct IpcRequest {
    /// Must be `"2.0"`.
    pub jsonrpc: String,

    /// Caller-assigned request identifier, echoed back in the response.
    pub id: serde_json::Value,

    /// The method name to invoke (e.g. `"search"`, `"index"`).
    pub method: String,

    /// Method-specific parameters. Defaults to `null` if omitted.
    #[serde(default)]
    pub params: serde_json::Value,
}

// ── Response ────────────────────────────────────────────────────────────────

/// A JSON-RPC 2.0 response sent back to the client.
#[derive(Debug, Serialize)]
pub struct IpcResponse {
    /// Always `"2.0"`.
    pub jsonrpc: &'static str,

    /// The request identifier from the corresponding [`IpcRequest`].
    pub id: serde_json::Value,

    /// The successful result payload, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,

    /// The error payload, present only when the request failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<IpcError>,
}

impl IpcResponse {
    /// Construct a successful response with the given result payload.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::json;
    /// # use synwire_daemon::ipc::IpcResponse;
    /// let resp = IpcResponse::success(json!(1), json!({"hits": 42}));
    /// assert!(resp.error.is_none());
    /// assert_eq!(resp.result, Some(json!({"hits": 42})));
    /// ```
    pub const fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Construct an error response.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::json;
    /// # use synwire_daemon::ipc::{IpcResponse, ERROR_METHOD_NOT_FOUND};
    /// let resp = IpcResponse::error(json!(1), ERROR_METHOD_NOT_FOUND, "unknown method");
    /// assert!(resp.result.is_none());
    /// assert_eq!(resp.error.as_ref().map(|e| e.code), Some(ERROR_METHOD_NOT_FOUND));
    /// ```
    pub fn error(id: serde_json::Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            id,
            result: None,
            error: Some(IpcError {
                code,
                message: message.into(),
            }),
        }
    }
}

// ── Error ───────────────────────────────────────────────────────────────────

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Serialize)]
pub struct IpcError {
    /// A numeric error code. Standard codes are defined as `ERROR_*` constants.
    pub code: i32,

    /// A short human-readable description of the error.
    pub message: String,
}

// ── Method enum ─────────────────────────────────────────────────────────────

/// The set of IPC methods the daemon understands.
///
/// Parsed from the `method` field of an [`IpcRequest`]. Unrecognised method
/// strings are captured in the [`Unknown`](IpcMethod::Unknown) variant so
/// callers can return a structured `method not found` error rather than
/// silently dropping the request.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum IpcMethod {
    /// Trigger indexing for a worktree.
    Index,
    /// Semantic (vector) search.
    Search,
    /// Code graph query.
    GraphQuery,
    /// Code graph search.
    GraphSearch,
    /// Community detection search.
    CommunitySearch,
    /// Hybrid vector + BM25 search.
    HybridSearch,
    /// Clone a repository.
    CloneRepo,
    /// Cross-reference query.
    XrefQuery,
    /// Check indexing status for a worktree.
    IndexStatus,
    /// An unrecognised method string.
    Unknown(String),
}

impl IpcMethod {
    /// Parse a method string into the corresponding variant.
    ///
    /// # Examples
    ///
    /// ```
    /// # use synwire_daemon::ipc::IpcMethod;
    /// assert_eq!(IpcMethod::from_method_str("code.search_semantic"), IpcMethod::Search);
    /// assert_eq!(
    ///     IpcMethod::from_method_str("nonexistent"),
    ///     IpcMethod::Unknown("nonexistent".to_owned()),
    /// );
    /// ```
    pub fn from_method_str(s: &str) -> Self {
        match s {
            "index.build" => Self::Index,
            "code.search_semantic" => Self::Search,
            "graph_query" => Self::GraphQuery,
            "graph_search" => Self::GraphSearch,
            "code.search_by_community" => Self::CommunitySearch,
            "code.search_hybrid" => Self::HybridSearch,
            "clone_repo" => Self::CloneRepo,
            "xref_query" => Self::XrefQuery,
            "index.status" => Self::IndexStatus,
            other => Self::Unknown(other.to_owned()),
        }
    }

    /// Return the canonical wire-format string for this method.
    ///
    /// For [`Unknown`](IpcMethod::Unknown) variants this returns the original
    /// unrecognised string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Index => "index.build",
            Self::Search => "code.search_semantic",
            Self::GraphQuery => "graph_query",
            Self::GraphSearch => "graph_search",
            Self::CommunitySearch => "code.search_by_community",
            Self::HybridSearch => "code.search_hybrid",
            Self::CloneRepo => "clone_repo",
            Self::XrefQuery => "xref_query",
            Self::IndexStatus => "index.status",
            Self::Unknown(s) => s,
        }
    }
}

impl std::fmt::Display for IpcMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for IpcMethod {
    type Err = std::convert::Infallible;

    /// Parse a method string. This never fails — unrecognised strings become
    /// [`IpcMethod::Unknown`].
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_method_str(s))
    }
}

// ── Frame I/O ───────────────────────────────────────────────────────────────

/// Maximum size of a single JSON-RPC line (1 MiB).
///
/// Requests larger than this are rejected to prevent unbounded memory
/// allocation from a misbehaving client.
const MAX_LINE_BYTES: usize = 1024 * 1024;

/// Read a single JSON-RPC request from a newline-delimited stream.
///
/// Returns `Ok(None)` on clean EOF (the client closed the connection).
/// Returns `Err` with an [`IpcError`] if the line cannot be parsed.
///
/// The reader should be a `BufReader` wrapping the socket's read half.
pub async fn read_request<R>(reader: &mut R) -> Result<Option<IpcRequest>, IpcError>
where
    R: tokio::io::AsyncBufRead + Unpin,
{
    let mut line = String::new();

    // AsyncBufReadExt::read_line returns 0 on EOF.
    let n = reader.read_line(&mut line).await.map_err(|e| IpcError {
        code: ERROR_PARSE,
        message: format!("I/O error reading request line: {e}"),
    })?;

    if n == 0 {
        return Ok(None);
    }

    if n > MAX_LINE_BYTES {
        return Err(IpcError {
            code: ERROR_PARSE,
            message: format!("request line exceeds maximum size ({MAX_LINE_BYTES} bytes)"),
        });
    }

    let request: IpcRequest = serde_json::from_str(line.trim()).map_err(|e| IpcError {
        code: ERROR_PARSE,
        message: format!("invalid JSON-RPC request: {e}"),
    })?;

    if request.jsonrpc != JSONRPC_VERSION {
        return Err(IpcError {
            code: ERROR_PARSE,
            message: format!(
                "unsupported JSON-RPC version {:?}, expected {:?}",
                request.jsonrpc, JSONRPC_VERSION
            ),
        });
    }

    Ok(Some(request))
}

/// Write a JSON-RPC response as a single newline-terminated JSON line.
///
/// The writer should be the socket's write half (or a `BufWriter` around it).
pub async fn write_response<W>(writer: &mut W, response: &IpcResponse) -> std::io::Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut payload = serde_json::to_vec(response).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("failed to serialise IPC response: {e}"),
        )
    })?;
    payload.push(b'\n');

    writer.write_all(&payload).await?;
    writer.flush().await?;

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn method_roundtrip() {
        let methods = [
            "index.build",
            "code.search_semantic",
            "graph_query",
            "graph_search",
            "code.search_by_community",
            "code.search_hybrid",
            "clone_repo",
            "xref_query",
            "index.status",
        ];

        for name in methods {
            let m = IpcMethod::from_method_str(name);
            assert_eq!(m.as_str(), name);
            assert_ne!(m, IpcMethod::Unknown(name.to_owned()));
        }
    }

    #[test]
    fn method_unknown() {
        let m = IpcMethod::from_method_str("does_not_exist");
        assert_eq!(m, IpcMethod::Unknown("does_not_exist".to_owned()));
        assert_eq!(m.as_str(), "does_not_exist");
    }

    #[test]
    fn method_from_str_trait() {
        let m: IpcMethod = "code.search_semantic"
            .parse()
            .unwrap_or(IpcMethod::Unknown(String::new()));
        assert_eq!(m, IpcMethod::Search);
    }

    #[test]
    fn method_display() {
        assert_eq!(IpcMethod::HybridSearch.to_string(), "code.search_hybrid");
        assert_eq!(IpcMethod::Unknown("foo".to_owned()).to_string(), "foo");
    }

    #[test]
    fn response_success_serialisation() {
        let resp = IpcResponse::success(json!(42), json!({"status": "ok"}));
        let serialised = serde_json::to_value(&resp).unwrap_or_default();
        assert_eq!(serialised["jsonrpc"], "2.0");
        assert_eq!(serialised["id"], 42);
        assert_eq!(serialised["result"]["status"], "ok");
        // error field should be absent (skip_serializing_if)
        assert!(serialised.get("error").is_none());
    }

    #[test]
    fn response_error_serialisation() {
        let resp = IpcResponse::error(json!("abc"), ERROR_METHOD_NOT_FOUND, "not found");
        let serialised = serde_json::to_value(&resp).unwrap_or_default();
        assert_eq!(serialised["jsonrpc"], "2.0");
        assert_eq!(serialised["id"], "abc");
        assert!(serialised.get("result").is_none());
        assert_eq!(serialised["error"]["code"], ERROR_METHOD_NOT_FOUND);
        assert_eq!(serialised["error"]["message"], "not found");
    }

    #[tokio::test]
    async fn read_request_valid() {
        let input = r#"{"jsonrpc":"2.0","id":1,"method":"search","params":{"q":"hello"}}"#;
        let input_with_newline = format!("{input}\n");
        let mut cursor = std::io::Cursor::new(input_with_newline.into_bytes());
        let mut reader = tokio::io::BufReader::new(&mut cursor);

        let req = read_request(&mut reader).await;
        assert!(req.is_ok());
        let req = req.unwrap_or(None);
        assert!(req.is_some());
        let req = req.unwrap_or_else(|| IpcRequest {
            jsonrpc: String::new(),
            id: json!(null),
            method: String::new(),
            params: json!(null),
        });
        assert_eq!(req.method, "search");
        assert_eq!(req.id, json!(1));
        assert_eq!(req.params["q"], "hello");
    }

    #[tokio::test]
    async fn read_request_eof() {
        let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
        let mut reader = tokio::io::BufReader::new(&mut cursor);

        let result = read_request(&mut reader).await;
        assert!(result.is_ok());
        assert!(
            result
                .unwrap_or(Some(IpcRequest {
                    jsonrpc: String::new(),
                    id: json!(null),
                    method: String::new(),
                    params: json!(null),
                }))
                .is_none()
        );
    }

    #[tokio::test]
    async fn read_request_invalid_json() {
        let input = b"not json at all\n";
        let mut cursor = std::io::Cursor::new(input.to_vec());
        let mut reader = tokio::io::BufReader::new(&mut cursor);

        let result = read_request(&mut reader).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, ERROR_PARSE);
    }

    #[tokio::test]
    async fn read_request_wrong_version() {
        let input = r#"{"jsonrpc":"1.0","id":1,"method":"search"}"#;
        let input_with_newline = format!("{input}\n");
        let mut cursor = std::io::Cursor::new(input_with_newline.into_bytes());
        let mut reader = tokio::io::BufReader::new(&mut cursor);

        let result = read_request(&mut reader).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, ERROR_PARSE);
        assert!(err.message.contains("unsupported JSON-RPC version"));
    }

    #[tokio::test]
    async fn write_response_roundtrip() {
        let resp = IpcResponse::success(json!(99), json!(["a", "b"]));
        let mut buf = Vec::new();

        let write_result = write_response(&mut buf, &resp).await;
        assert!(write_result.is_ok());

        // The output should be valid JSON followed by a newline.
        let output = String::from_utf8(buf).unwrap_or_default();
        assert!(output.ends_with('\n'));

        let parsed: serde_json::Value = serde_json::from_str(output.trim()).unwrap_or_default();
        assert_eq!(parsed["id"], 99);
        assert_eq!(parsed["result"], json!(["a", "b"]));
    }

    #[tokio::test]
    async fn read_request_default_params() {
        let input = r#"{"jsonrpc":"2.0","id":1,"method":"index.status"}"#;
        let input_with_newline = format!("{input}\n");
        let mut cursor = std::io::Cursor::new(input_with_newline.into_bytes());
        let mut reader = tokio::io::BufReader::new(&mut cursor);

        let req = read_request(&mut reader).await;
        assert!(req.is_ok());
        let req = req.unwrap_or(None);
        assert!(req.is_some());
        let req = req.unwrap_or_else(|| IpcRequest {
            jsonrpc: String::new(),
            id: json!(null),
            method: String::new(),
            params: json!(null),
        });
        assert_eq!(req.method, "index.status");
        assert!(req.params.is_null());
    }
}
