//! Tests for callback handler trait.

use std::sync::{Arc, Mutex};

use crate::BoxFuture;
use crate::callbacks::CallbackHandler;
use crate::messages::Message;
use serde_json::Value;

/// A callback handler that records events for test assertions.
struct RecordingCallback {
    events: Arc<Mutex<Vec<String>>>,
    ignore_tool: bool,
    ignore_llm: bool,
}

impl RecordingCallback {
    fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let handler = Self {
            events: Arc::clone(&events),
            ignore_tool: false,
            ignore_llm: false,
        };
        (handler, events)
    }

    fn with_ignore_tool() -> (Self, Arc<Mutex<Vec<String>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let handler = Self {
            events: Arc::clone(&events),
            ignore_tool: true,
            ignore_llm: false,
        };
        (handler, events)
    }
}

impl CallbackHandler for RecordingCallback {
    fn on_llm_start<'a>(
        &'a self,
        model_type: &'a str,
        _messages: &'a [Message],
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            if let Ok(mut events) = self.events.lock() {
                events.push(format!("llm_start:{model_type}"));
            }
        })
    }

    fn on_llm_end<'a>(&'a self, _response: &'a Value) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            if let Ok(mut events) = self.events.lock() {
                events.push("llm_end".into());
            }
        })
    }

    fn on_tool_start<'a>(&'a self, tool_name: &'a str, _input: &'a Value) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            if let Ok(mut events) = self.events.lock() {
                events.push(format!("tool_start:{tool_name}"));
            }
        })
    }

    fn on_tool_end<'a>(&'a self, tool_name: &'a str, _output: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            if let Ok(mut events) = self.events.lock() {
                events.push(format!("tool_end:{tool_name}"));
            }
        })
    }

    fn on_tool_error<'a>(&'a self, tool_name: &'a str, error: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            if let Ok(mut events) = self.events.lock() {
                events.push(format!("tool_error:{tool_name}:{error}"));
            }
        })
    }

    fn on_chain_start<'a>(&'a self, chain_name: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            if let Ok(mut events) = self.events.lock() {
                events.push(format!("chain_start:{chain_name}"));
            }
        })
    }

    fn on_chain_end<'a>(&'a self, _output: &'a Value) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            if let Ok(mut events) = self.events.lock() {
                events.push("chain_end".into());
            }
        })
    }

    fn ignore_tool(&self) -> bool {
        self.ignore_tool
    }

    fn ignore_llm(&self) -> bool {
        self.ignore_llm
    }
}

#[tokio::test]
async fn on_tool_start_end_fire() {
    let (handler, events) = RecordingCallback::new();
    let input = serde_json::json!({"query": "rust"});

    handler.on_tool_start("search", &input).await;
    handler.on_tool_end("search", "found 10 results").await;

    let recorded = events.lock().unwrap();
    assert_eq!(recorded.len(), 2);
    assert_eq!(recorded[0], "tool_start:search");
    assert_eq!(recorded[1], "tool_end:search");
}

#[tokio::test]
async fn on_llm_start_end_fire() {
    let (handler, events) = RecordingCallback::new();
    let messages = vec![Message::human("Hello")];

    handler.on_llm_start("gpt-4", &messages).await;
    handler
        .on_llm_end(&serde_json::json!({"content": "Hi"}))
        .await;

    let recorded = events.lock().unwrap();
    assert_eq!(recorded.len(), 2);
    assert_eq!(recorded[0], "llm_start:gpt-4");
    assert_eq!(recorded[1], "llm_end");
}

#[tokio::test]
async fn ignore_tool_filter() {
    let (handler, events) = RecordingCallback::with_ignore_tool();

    assert!(handler.ignore_tool());
    assert!(!handler.ignore_llm());

    // Simulate what an executor would do: check ignore_tool before firing
    if !handler.ignore_tool() {
        handler
            .on_tool_start("search", &serde_json::json!({}))
            .await;
    }
    // LLM callbacks should still fire
    handler
        .on_llm_start("gpt-4", &[Message::human("test")])
        .await;

    let recorded = events.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0], "llm_start:gpt-4");
}

#[tokio::test]
async fn on_tool_error_fires() {
    let (handler, events) = RecordingCallback::new();
    handler.on_tool_error("search", "connection refused").await;

    let recorded = events.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0], "tool_error:search:connection refused");
}

#[tokio::test]
async fn default_noop_implementation_compiles() {
    // A handler with all defaults should do nothing without error.
    struct NoopCallback;
    impl CallbackHandler for NoopCallback {}

    let handler = NoopCallback;
    handler.on_llm_start("test", &[]).await;
    handler.on_llm_end(&serde_json::json!(null)).await;
    handler
        .on_tool_start("test", &serde_json::json!(null))
        .await;
    handler.on_tool_end("test", "output").await;
    handler.on_tool_error("test", "err").await;
    handler.on_retry(1, "err").await;
    handler.on_chain_start("chain").await;
    handler.on_chain_end(&serde_json::json!(null)).await;
    assert!(!handler.ignore_tool());
    assert!(!handler.ignore_llm());
}
