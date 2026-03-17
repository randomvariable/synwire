//! MCP transport implementations provided by the adapters layer.

pub mod websocket;

pub use websocket::WebSocketMcpTransport;
