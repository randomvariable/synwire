//! Content-Length based codec for the Debug Adapter Protocol wire format.
//!
//! The DAP wire format is identical to LSP base protocol:
//!
//! ```text
//! Content-Length: <N>\r\n
//! \r\n
//! <N bytes of JSON>
//! ```

use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

use crate::error::DapError;

/// Content-Length based codec for the Debug Adapter Protocol.
///
/// Frames JSON messages with a `Content-Length` header, following the same
/// wire format used by the Language Server Protocol.
pub struct ContentLengthCodec {
    /// If we have parsed the `Content-Length` header, this holds the expected body length.
    content_length: Option<usize>,
}

impl ContentLengthCodec {
    /// Create a new codec instance.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            content_length: None,
        }
    }
}

impl Default for ContentLengthCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for ContentLengthCodec {
    type Item = serde_json::Value;
    type Error = DapError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // If we don't have the content length yet, try to parse the header.
        if self.content_length.is_none() {
            if let Some(header_end) = find_header_end(src) {
                let header = std::str::from_utf8(&src[..header_end])
                    .map_err(|e| DapError::Codec(format!("invalid header encoding: {e}")))?;

                let content_length = parse_content_length(header)?;
                self.content_length = Some(content_length);

                // Consume header + \r\n\r\n delimiter.
                let _ = src.split_to(header_end + 4);
            } else {
                return Ok(None); // Need more data.
            }
        }

        // If we have content length, try to read body.
        if let Some(len) = self.content_length
            && src.len() >= len
        {
            let body = src.split_to(len);
            self.content_length = None;
            let value: serde_json::Value = serde_json::from_slice(&body)?;
            return Ok(Some(value));
        }

        Ok(None) // Need more data.
    }
}

impl Encoder<serde_json::Value> for ContentLengthCodec {
    type Error = DapError;

    fn encode(&mut self, item: serde_json::Value, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let body = serde_json::to_vec(&item)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        dst.extend_from_slice(header.as_bytes());
        dst.extend_from_slice(&body);
        Ok(())
    }
}

/// Find the position of the `\r\n\r\n` header/body delimiter.
fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

/// Parse the `Content-Length` value from a header block.
fn parse_content_length(header: &str) -> Result<usize, DapError> {
    for line in header.lines() {
        if let Some(value) = line.strip_prefix("Content-Length:") {
            return value
                .trim()
                .parse::<usize>()
                .map_err(|e| DapError::Codec(format!("invalid Content-Length: {e}")));
        }
    }
    Err(DapError::Codec("missing Content-Length header".into()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use tokio_util::codec::{Decoder, Encoder};

    #[test]
    fn decode_single_message() {
        let mut codec = ContentLengthCodec::new();
        let body = r#"{"seq":1,"type":"response"}"#;
        let frame = format!("Content-Length: {}\r\n\r\n{body}", body.len());
        let mut buf = BytesMut::from(frame.as_str());

        let result = codec.decode(&mut buf).unwrap();
        assert!(result.is_some());
        let value = result.unwrap();
        assert_eq!(value["seq"], 1);
    }

    #[test]
    fn decode_partial_header() {
        let mut codec = ContentLengthCodec::new();
        let mut buf = BytesMut::from("Content-Len");

        let result = codec.decode(&mut buf).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn decode_partial_body() {
        let mut codec = ContentLengthCodec::new();
        let body = r#"{"seq":1,"type":"response"}"#;
        let frame = format!("Content-Length: {}\r\n\r\n", body.len());
        // Only send header, not body.
        let mut buf = BytesMut::from(frame.as_str());

        let result = codec.decode(&mut buf).unwrap();
        assert!(result.is_none());

        // Now add the body.
        buf.extend_from_slice(body.as_bytes());
        let result = codec.decode(&mut buf).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn decode_two_messages() {
        let mut codec = ContentLengthCodec::new();
        let body1 = r#"{"seq":1}"#;
        let body2 = r#"{"seq":2}"#;
        let frame = format!(
            "Content-Length: {}\r\n\r\n{body1}Content-Length: {}\r\n\r\n{body2}",
            body1.len(),
            body2.len()
        );
        let mut buf = BytesMut::from(frame.as_str());

        let r1 = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(r1["seq"], 1);
        let r2 = codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(r2["seq"], 2);
    }

    #[test]
    fn encode_message() {
        let mut codec = ContentLengthCodec::new();
        let value = serde_json::json!({"seq": 1, "type": "request"});
        let mut buf = BytesMut::new();
        codec.encode(value, &mut buf).unwrap();

        let s = std::str::from_utf8(&buf).unwrap();
        assert!(s.starts_with("Content-Length: "));
        assert!(s.contains("\r\n\r\n"));
        assert!(s.contains("\"seq\""));
    }

    #[test]
    fn missing_content_length_header() {
        let mut codec = ContentLengthCodec::new();
        let mut buf = BytesMut::from("X-Custom: value\r\n\r\n{}");
        let result = codec.decode(&mut buf);
        assert!(result.is_err());
    }
}
