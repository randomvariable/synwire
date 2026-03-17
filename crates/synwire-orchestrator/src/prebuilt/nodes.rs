//! Prebuilt utility nodes for common graph patterns.
//!
//! Nodes that need typed state access ([`IfElseNode`], [`LoopNode`],
//! [`IterationNode`]) are generic over `S: State`.
//!
//! Nodes that work purely at the JSON level ([`TemplateTransformNode`],
//! [`ListOperatorNode`], [`VariableAggregatorNode`], [`HttpRequestNode`],
//! [`QuestionClassifierNode`], [`ValidationNode`]) use [`State::to_value()`]
//! and [`State::from_value()`] to bridge between typed state and JSON.

use std::sync::Arc;

use crate::error::GraphError;
use crate::graph::state::{NodeFn, State};

// ---------------------------------------------------------------------------
// IfElseNode
// ---------------------------------------------------------------------------

/// A node that branches execution based on a condition.
///
/// Evaluates a predicate against the typed state and delegates to one of two
/// inner node functions.
pub struct IfElseNode<S: State> {
    condition: Box<dyn Fn(&S) -> bool + Send + Sync>,
    if_true: NodeFn<S>,
    if_false: NodeFn<S>,
}

impl<S: State> IfElseNode<S> {
    /// Creates a new `IfElseNode`.
    ///
    /// - `condition` -- predicate evaluated against the typed state
    /// - `if_true` -- executed when the condition returns `true`
    /// - `if_false` -- executed when the condition returns `false`
    pub fn new(
        condition: Box<dyn Fn(&S) -> bool + Send + Sync>,
        if_true: NodeFn<S>,
        if_false: NodeFn<S>,
    ) -> Self {
        Self {
            condition,
            if_true,
            if_false,
        }
    }

    /// Converts this node into a [`NodeFn<S>`].
    pub fn into_node_fn(self) -> NodeFn<S>
    where
        Self: 'static,
    {
        let condition = Arc::new(self.condition);
        let if_true = Arc::new(self.if_true);
        let if_false = Arc::new(self.if_false);
        Box::new(move |state: S| {
            let condition = Arc::clone(&condition);
            let if_true = Arc::clone(&if_true);
            let if_false = Arc::clone(&if_false);
            Box::pin(async move {
                if condition(&state) {
                    if_true(state).await
                } else {
                    if_false(state).await
                }
            })
        })
    }
}

impl<S: State> std::fmt::Debug for IfElseNode<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IfElseNode").finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// LoopNode
// ---------------------------------------------------------------------------

/// A node that repeats a body function until a predicate is satisfied or a
/// maximum iteration count is reached.
///
/// On each iteration, the body function transforms the state and then the
/// predicate is evaluated. The loop terminates when the predicate returns
/// `true` or `max_iterations` is reached.
pub struct LoopNode<S: State> {
    body: NodeFn<S>,
    predicate: Box<dyn Fn(&S) -> bool + Send + Sync>,
    max_iterations: usize,
}

impl<S: State> LoopNode<S> {
    /// Creates a new `LoopNode`.
    ///
    /// - `body` -- function executed on each iteration
    /// - `predicate` -- checked after each iteration; loop exits when `true`
    /// - `max_iterations` -- hard cap to prevent infinite loops
    pub fn new(
        body: NodeFn<S>,
        predicate: Box<dyn Fn(&S) -> bool + Send + Sync>,
        max_iterations: usize,
    ) -> Self {
        Self {
            body,
            predicate,
            max_iterations,
        }
    }

    /// Converts this node into a [`NodeFn<S>`].
    pub fn into_node_fn(self) -> NodeFn<S>
    where
        Self: 'static,
    {
        let body = Arc::new(self.body);
        let predicate = Arc::new(self.predicate);
        let max_iterations = self.max_iterations;
        Box::new(move |state: S| {
            let body = Arc::clone(&body);
            let predicate = Arc::clone(&predicate);
            Box::pin(async move {
                let mut current = state;
                for _ in 0..max_iterations {
                    current = body(current).await?;
                    if predicate(&current) {
                        return Ok(current);
                    }
                }
                Err(GraphError::MaxIterations {
                    limit: max_iterations,
                })
            })
        })
    }
}

impl<S: State> std::fmt::Debug for LoopNode<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoopNode")
            .field("max_iterations", &self.max_iterations)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// HttpRequestNode
// ---------------------------------------------------------------------------

/// A node that makes HTTP requests.
///
/// Reads `"url"` and optional `"method"`, `"headers"`, and `"body"` from the
/// state, performs the request, and writes the response to `"http_response"`.
pub struct HttpRequestNode {
    client: reqwest::Client,
}

impl HttpRequestNode {
    /// Creates a new `HttpRequestNode` with the given HTTP client.
    pub const fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// Creates a new `HttpRequestNode` with a default client.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::HttpRequest`] if the default client cannot be built.
    pub fn with_default_client() -> Result<Self, GraphError> {
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| GraphError::HttpRequest {
                message: format!("failed to build HTTP client: {e}"),
            })?;
        Ok(Self { client })
    }

    /// Converts this node into a [`NodeFn<S>`] for any state type.
    ///
    /// Uses `State::to_value()` and `State::from_value()` to bridge between
    /// the typed state and the JSON representation used internally.
    pub fn into_node_fn<S: State>(self) -> NodeFn<S>
    where
        Self: 'static,
    {
        let node = Arc::new(self);
        Box::new(move |state: S| {
            let node = Arc::clone(&node);
            Box::pin(async move {
                let value = state.to_value().map_err(|e| GraphError::InvalidUpdate {
                    message: format!("state serialisation failed: {e}"),
                })?;
                let result = node.invoke(value).await?;
                S::from_value(result).map_err(|e| GraphError::InvalidUpdate {
                    message: format!("state deserialisation failed: {e}"),
                })
            })
        })
    }

    async fn invoke(&self, mut state: serde_json::Value) -> Result<serde_json::Value, GraphError> {
        let url = state
            .get("url")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| GraphError::InvalidUpdate {
                message: "state must contain a 'url' string".into(),
            })?
            .to_owned();

        let method = state
            .get("method")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("GET")
            .to_uppercase();

        let mut request = match method.as_str() {
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            "PATCH" => self.client.patch(&url),
            "HEAD" => self.client.head(&url),
            _ => self.client.get(&url),
        };

        // Apply headers if present.
        if let Some(headers) = state.get("headers").and_then(serde_json::Value::as_object) {
            for (key, val) in headers {
                if let Some(val_str) = val.as_str() {
                    request = request.header(key.as_str(), val_str);
                }
            }
        }

        // Apply body if present.
        if let Some(body) = state.get("body") {
            request = request.json(body);
        }

        let response = request.send().await.map_err(|e| GraphError::HttpRequest {
            message: e.to_string(),
        })?;

        let status = response.status().as_u16();
        let body_text = response.text().await.map_err(|e| GraphError::HttpRequest {
            message: format!("failed to read response body: {e}"),
        })?;

        let http_response = serde_json::json!({
            "status": status,
            "body": body_text,
        });

        if let Some(obj) = state.as_object_mut() {
            let _prev = obj.insert("http_response".to_owned(), http_response);
        }

        Ok(state)
    }
}

impl std::fmt::Debug for HttpRequestNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpRequestNode").finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// ValidationNode
// ---------------------------------------------------------------------------

/// A node that validates state contains required keys.
///
/// Checks that all specified keys exist in the top-level state object.
/// Optionally checks that values are non-null and non-empty strings.
pub struct ValidationNode {
    required_keys: Vec<String>,
    reject_empty: bool,
}

impl ValidationNode {
    /// Creates a new `ValidationNode` that checks for the given keys.
    pub const fn new(required_keys: Vec<String>) -> Self {
        Self {
            required_keys,
            reject_empty: false,
        }
    }

    /// Also rejects keys whose values are `null` or empty strings.
    #[must_use]
    pub const fn with_reject_empty(mut self, reject: bool) -> Self {
        self.reject_empty = reject;
        self
    }

    /// Converts this node into a [`NodeFn<S>`] for any state type.
    ///
    /// Uses `State::to_value()` and `State::from_value()` to bridge between
    /// the typed state and the JSON representation used internally.
    pub fn into_node_fn<S: State>(self) -> NodeFn<S>
    where
        Self: 'static,
    {
        let node = Arc::new(self);
        Box::new(move |state: S| {
            let node = Arc::clone(&node);
            Box::pin(async move {
                let value = state.to_value().map_err(|e| GraphError::InvalidUpdate {
                    message: format!("state serialisation failed: {e}"),
                })?;
                let result = node.invoke(value)?;
                S::from_value(result).map_err(|e| GraphError::InvalidUpdate {
                    message: format!("state deserialisation failed: {e}"),
                })
            })
        })
    }

    fn invoke(&self, state: serde_json::Value) -> Result<serde_json::Value, GraphError> {
        let obj = state.as_object().ok_or_else(|| GraphError::Validation {
            message: "state must be a JSON object".into(),
        })?;

        for key in &self.required_keys {
            let val = obj.get(key).ok_or_else(|| GraphError::Validation {
                message: format!("missing required key: '{key}'"),
            })?;

            if self.reject_empty {
                if val.is_null() {
                    return Err(GraphError::Validation {
                        message: format!("key '{key}' is null"),
                    });
                }
                if let Some(s) = val.as_str()
                    && s.is_empty()
                {
                    return Err(GraphError::Validation {
                        message: format!("key '{key}' is an empty string"),
                    });
                }
            }
        }

        Ok(state)
    }
}

impl std::fmt::Debug for ValidationNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidationNode")
            .field("required_keys", &self.required_keys)
            .field("reject_empty", &self.reject_empty)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// TemplateTransformNode
// ---------------------------------------------------------------------------

/// A node that applies simple string template substitution to state.
///
/// Replaces `{key}` placeholders in the template with values from the state.
/// The rendered result is stored under the `output_key`.
pub struct TemplateTransformNode {
    template: String,
    output_key: String,
}

impl TemplateTransformNode {
    /// Creates a new `TemplateTransformNode`.
    ///
    /// - `template` -- a string with `{key}` placeholders
    /// - `output_key` -- the state key where the rendered result is stored
    pub fn new(template: impl Into<String>, output_key: impl Into<String>) -> Self {
        Self {
            template: template.into(),
            output_key: output_key.into(),
        }
    }

    /// Converts this node into a [`NodeFn<S>`] for any state type.
    ///
    /// Uses `State::to_value()` and `State::from_value()` to bridge between
    /// the typed state and the JSON representation used internally.
    pub fn into_node_fn<S: State>(self) -> NodeFn<S>
    where
        Self: 'static,
    {
        let node = Arc::new(self);
        Box::new(move |state: S| {
            let node = Arc::clone(&node);
            Box::pin(async move {
                let value = state.to_value().map_err(|e| GraphError::InvalidUpdate {
                    message: format!("state serialisation failed: {e}"),
                })?;
                let result = node.invoke(value)?;
                S::from_value(result).map_err(|e| GraphError::InvalidUpdate {
                    message: format!("state deserialisation failed: {e}"),
                })
            })
        })
    }

    fn invoke(&self, mut state: serde_json::Value) -> Result<serde_json::Value, GraphError> {
        let obj = state.as_object().ok_or_else(|| GraphError::Template {
            message: "state must be a JSON object".into(),
        })?;

        let mut result = self.template.clone();
        for (key, val) in obj {
            let placeholder = format!("{{{key}}}");
            let replacement = match val {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => String::new(),
                other => other.to_string(),
            };
            result = result.replace(&placeholder, &replacement);
        }

        if let Some(obj) = state.as_object_mut() {
            let _prev = obj.insert(self.output_key.clone(), serde_json::Value::String(result));
        }

        Ok(state)
    }
}

impl std::fmt::Debug for TemplateTransformNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TemplateTransformNode")
            .field("template", &self.template)
            .field("output_key", &self.output_key)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// ListOperatorNode
// ---------------------------------------------------------------------------

/// Operation to perform on a list.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum ListOperation {
    /// Sort the list (lexicographic for strings, numeric for numbers).
    Sort,
    /// Reverse the list.
    Reverse,
    /// Remove duplicate adjacent values (requires sorted input for full dedup).
    Deduplicate,
    /// Take the first N items.
    Slice {
        /// Number of items to take.
        count: usize,
    },
}

/// A boxed predicate over JSON values.
type JsonPredicate = Box<dyn Fn(&serde_json::Value) -> bool + Send + Sync>;

/// A node that performs operations on a JSON array in the state.
///
/// Reads an array from `source_key`, applies the operation, and writes the
/// result to `output_key`.
pub struct ListOperatorNode {
    source_key: String,
    output_key: String,
    operation: ListOperation,
    filter_predicate: Option<JsonPredicate>,
}

impl ListOperatorNode {
    /// Creates a new `ListOperatorNode`.
    pub fn new(
        source_key: impl Into<String>,
        output_key: impl Into<String>,
        operation: ListOperation,
    ) -> Self {
        Self {
            source_key: source_key.into(),
            output_key: output_key.into(),
            operation,
            filter_predicate: None,
        }
    }

    /// Adds a filter predicate that is applied before the main operation.
    ///
    /// Only items for which the predicate returns `true` are kept.
    #[must_use]
    pub fn with_filter(
        mut self,
        predicate: Box<dyn Fn(&serde_json::Value) -> bool + Send + Sync>,
    ) -> Self {
        self.filter_predicate = Some(predicate);
        self
    }

    /// Converts this node into a [`NodeFn<S>`] for any state type.
    ///
    /// Uses `State::to_value()` and `State::from_value()` to bridge between
    /// the typed state and the JSON representation used internally.
    pub fn into_node_fn<S: State>(self) -> NodeFn<S>
    where
        Self: 'static,
    {
        let node = Arc::new(self);
        Box::new(move |state: S| {
            let node = Arc::clone(&node);
            Box::pin(async move {
                let value = state.to_value().map_err(|e| GraphError::InvalidUpdate {
                    message: format!("state serialisation failed: {e}"),
                })?;
                let result = node.invoke(value)?;
                S::from_value(result).map_err(|e| GraphError::InvalidUpdate {
                    message: format!("state deserialisation failed: {e}"),
                })
            })
        })
    }

    fn invoke(&self, mut state: serde_json::Value) -> Result<serde_json::Value, GraphError> {
        let arr = state
            .get(&self.source_key)
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| GraphError::InvalidUpdate {
                message: format!("'{}' must be a JSON array", self.source_key),
            })?
            .clone();

        let mut items: Vec<serde_json::Value> = if let Some(ref pred) = self.filter_predicate {
            arr.into_iter().filter(|v| pred(v)).collect()
        } else {
            arr
        };

        match self.operation {
            ListOperation::Sort => {
                items.sort_by(|a, b| {
                    // Try numeric comparison first, fall back to string.
                    let na = a.as_f64();
                    let nb = b.as_f64();
                    if let (Some(fa), Some(fb)) = (na, nb) {
                        fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
                    } else {
                        a.to_string().cmp(&b.to_string())
                    }
                });
            }
            ListOperation::Reverse => {
                items.reverse();
            }
            ListOperation::Deduplicate => {
                items.dedup_by(|a, b| a == b);
            }
            ListOperation::Slice { count } => {
                items.truncate(count);
            }
        }

        if let Some(obj) = state.as_object_mut() {
            let _prev = obj.insert(self.output_key.clone(), serde_json::Value::Array(items));
        }

        Ok(state)
    }
}

impl std::fmt::Debug for ListOperatorNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListOperatorNode")
            .field("source_key", &self.source_key)
            .field("output_key", &self.output_key)
            .field("operation", &self.operation)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// VariableAggregatorNode
// ---------------------------------------------------------------------------

/// A node that collects values from multiple state keys into a single array.
///
/// Reads each key in `source_keys` from the state and assembles them into
/// an array stored under `output_key`.
pub struct VariableAggregatorNode {
    source_keys: Vec<String>,
    output_key: String,
}

impl VariableAggregatorNode {
    /// Creates a new `VariableAggregatorNode`.
    pub fn new(source_keys: Vec<String>, output_key: impl Into<String>) -> Self {
        Self {
            source_keys,
            output_key: output_key.into(),
        }
    }

    /// Converts this node into a [`NodeFn<S>`] for any state type.
    ///
    /// Uses `State::to_value()` and `State::from_value()` to bridge between
    /// the typed state and the JSON representation used internally.
    pub fn into_node_fn<S: State>(self) -> NodeFn<S>
    where
        Self: 'static,
    {
        let node = Arc::new(self);
        Box::new(move |state: S| {
            let node = Arc::clone(&node);
            Box::pin(async move {
                let value = state.to_value().map_err(|e| GraphError::InvalidUpdate {
                    message: format!("state serialisation failed: {e}"),
                })?;
                let result = node.invoke(value);
                S::from_value(result).map_err(|e| GraphError::InvalidUpdate {
                    message: format!("state deserialisation failed: {e}"),
                })
            })
        })
    }

    fn invoke(&self, mut state: serde_json::Value) -> serde_json::Value {
        let mut collected = Vec::with_capacity(self.source_keys.len());
        for key in &self.source_keys {
            if let Some(val) = state.get(key) {
                collected.push(val.clone());
            }
        }

        if let Some(obj) = state.as_object_mut() {
            let _prev = obj.insert(self.output_key.clone(), serde_json::Value::Array(collected));
        }

        state
    }
}

impl std::fmt::Debug for VariableAggregatorNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VariableAggregatorNode")
            .field("source_keys", &self.source_keys)
            .field("output_key", &self.output_key)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// QuestionClassifierNode
// ---------------------------------------------------------------------------

/// A node that classifies input text into one of several categories.
///
/// This is a stub implementation that uses keyword matching rather than
/// a real LLM call. To use an LLM-backed classifier, build a custom node
/// with a [`BaseChatModel`](synwire_core::language_models::BaseChatModel).
///
/// State contract:
/// - Reads from `input_key` (default `"question"`)
/// - Writes the matched category name to `output_key` (default `"category"`)
pub struct QuestionClassifierNode {
    categories: Vec<(String, Vec<String>)>,
    default_category: String,
    input_key: String,
    output_key: String,
}

impl QuestionClassifierNode {
    /// Creates a new `QuestionClassifierNode`.
    ///
    /// - `categories` -- list of `(category_name, keywords)` pairs
    /// - `default_category` -- returned when no keywords match
    pub fn new(
        categories: Vec<(String, Vec<String>)>,
        default_category: impl Into<String>,
    ) -> Self {
        Self {
            categories,
            default_category: default_category.into(),
            input_key: "question".to_owned(),
            output_key: "category".to_owned(),
        }
    }

    /// Sets the state key to read the input question from.
    #[must_use]
    pub fn with_input_key(mut self, key: impl Into<String>) -> Self {
        self.input_key = key.into();
        self
    }

    /// Sets the state key to write the classification result to.
    #[must_use]
    pub fn with_output_key(mut self, key: impl Into<String>) -> Self {
        self.output_key = key.into();
        self
    }

    /// Converts this node into a [`NodeFn<S>`] for any state type.
    ///
    /// Uses `State::to_value()` and `State::from_value()` to bridge between
    /// the typed state and the JSON representation used internally.
    pub fn into_node_fn<S: State>(self) -> NodeFn<S>
    where
        Self: 'static,
    {
        let node = Arc::new(self);
        Box::new(move |state: S| {
            let node = Arc::clone(&node);
            Box::pin(async move {
                let value = state.to_value().map_err(|e| GraphError::InvalidUpdate {
                    message: format!("state serialisation failed: {e}"),
                })?;
                let result = node.invoke(value);
                S::from_value(result).map_err(|e| GraphError::InvalidUpdate {
                    message: format!("state deserialisation failed: {e}"),
                })
            })
        })
    }

    fn invoke(&self, mut state: serde_json::Value) -> serde_json::Value {
        let input = state
            .get(&self.input_key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_lowercase();

        let mut best_category = &self.default_category;
        let mut best_count: usize = 0;

        for (category, keywords) in &self.categories {
            let count = keywords
                .iter()
                .filter(|kw| input.contains(&kw.to_lowercase()))
                .count();
            if count > best_count {
                best_count = count;
                best_category = category;
            }
        }

        if let Some(obj) = state.as_object_mut() {
            let _prev = obj.insert(
                self.output_key.clone(),
                serde_json::Value::String(best_category.clone()),
            );
        }

        state
    }
}

impl std::fmt::Debug for QuestionClassifierNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuestionClassifierNode")
            .field("categories", &self.categories.len())
            .field("default_category", &self.default_category)
            .field("input_key", &self.input_key)
            .field("output_key", &self.output_key)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// IterationNode
// ---------------------------------------------------------------------------

/// A node that iterates over a collection, applying a body function to each
/// item and collecting results.
///
/// Reads an array from `source_key`, invokes the body function for each
/// element (placing the element in a temporary `"__current_item"` key),
/// and collects results into `output_key`.
///
/// Uses `State::to_value()` and `State::from_value()` to access JSON fields
/// for the source/output keys, while keeping the body function generic.
pub struct IterationNode<S: State> {
    source_key: String,
    output_key: String,
    body: NodeFn<S>,
}

impl<S: State> IterationNode<S> {
    /// Creates a new `IterationNode`.
    ///
    /// - `source_key` -- state key containing the array to iterate over
    /// - `output_key` -- state key where collected results are stored
    /// - `body` -- function applied to each item
    pub fn new(
        source_key: impl Into<String>,
        output_key: impl Into<String>,
        body: NodeFn<S>,
    ) -> Self {
        Self {
            source_key: source_key.into(),
            output_key: output_key.into(),
            body,
        }
    }

    /// Converts this node into a [`NodeFn<S>`].
    pub fn into_node_fn(self) -> NodeFn<S>
    where
        Self: 'static,
    {
        let node = Arc::new(self);
        Box::new(move |state: S| {
            let node = Arc::clone(&node);
            Box::pin(async move { node.invoke(state).await })
        })
    }

    async fn invoke(&self, state: S) -> Result<S, GraphError> {
        let state_value = state.to_value().map_err(|e| GraphError::InvalidUpdate {
            message: format!("state serialisation failed: {e}"),
        })?;

        let items = state_value
            .get(&self.source_key)
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| GraphError::InvalidUpdate {
                message: format!("'{}' must be a JSON array", self.source_key),
            })?
            .clone();

        let mut results = Vec::with_capacity(items.len());

        for item in items {
            // Build a sub-state with the current item injected.
            let mut sub_value = state_value.clone();
            if let Some(obj) = sub_value.as_object_mut() {
                let _prev = obj.insert("__current_item".to_owned(), item);
            }

            let sub_state = S::from_value(sub_value).map_err(|e| GraphError::InvalidUpdate {
                message: format!("state deserialisation failed: {e}"),
            })?;

            let result_state = (self.body)(sub_state).await?;
            let result_value = result_state
                .to_value()
                .map_err(|e| GraphError::InvalidUpdate {
                    message: format!("state serialisation failed: {e}"),
                })?;

            // Extract the __current_item from the result as the "output".
            let output = result_value
                .get("__current_item")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            results.push(output);
        }

        let mut out = state_value;
        if let Some(obj) = out.as_object_mut() {
            let _prev = obj.insert(self.output_key.clone(), serde_json::Value::Array(results));
            // Clean up temporary key if present.
            let _prev = obj.remove("__current_item");
        }

        S::from_value(out).map_err(|e| GraphError::InvalidUpdate {
            message: format!("state deserialisation failed: {e}"),
        })
    }
}

impl<S: State> std::fmt::Debug for IterationNode<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IterationNode")
            .field("source_key", &self.source_key)
            .field("output_key", &self.output_key)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::channels::BaseChannel;
    use crate::func::sync_node;
    use crate::graph::value_state::ValueState;

    // -----------------------------------------------------------------------
    // T061: A non-ValueState type to prove generics work.
    // -----------------------------------------------------------------------

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct TestCounterState {
        counter: i32,
        label: String,
    }

    impl State for TestCounterState {
        fn channels() -> Vec<(String, Box<dyn BaseChannel>)> {
            vec![]
        }

        fn from_channels(
            _channels: &HashMap<String, Box<dyn BaseChannel>>,
        ) -> Result<Self, GraphError> {
            Err(GraphError::Checkpoint {
                message: "not supported for TestCounterState".into(),
            })
        }
    }

    // -----------------------------------------------------------------------
    // IfElseNode tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_if_else_true_branch() {
        let node = IfElseNode::new(
            Box::new(|state: &ValueState| {
                state.0.get("flag").and_then(serde_json::Value::as_bool) == Some(true)
            }),
            sync_node(|mut s: ValueState| {
                if let Some(obj) = s.0.as_object_mut() {
                    let _prev = obj.insert(
                        "result".to_owned(),
                        serde_json::Value::String("true_branch".into()),
                    );
                }
                Ok(s)
            }),
            sync_node(|mut s: ValueState| {
                if let Some(obj) = s.0.as_object_mut() {
                    let _prev = obj.insert(
                        "result".to_owned(),
                        serde_json::Value::String("false_branch".into()),
                    );
                }
                Ok(s)
            }),
        );

        let node_fn = node.into_node_fn();
        let state = ValueState(serde_json::json!({"flag": true}));
        let result = node_fn(state).await.unwrap();
        assert_eq!(result.0["result"], "true_branch");
    }

    #[tokio::test]
    async fn test_if_else_false_branch() {
        let node = IfElseNode::new(
            Box::new(|state: &ValueState| {
                state.0.get("flag").and_then(serde_json::Value::as_bool) == Some(true)
            }),
            sync_node(|mut s: ValueState| {
                if let Some(obj) = s.0.as_object_mut() {
                    let _prev = obj.insert(
                        "result".to_owned(),
                        serde_json::Value::String("true_branch".into()),
                    );
                }
                Ok(s)
            }),
            sync_node(|mut s: ValueState| {
                if let Some(obj) = s.0.as_object_mut() {
                    let _prev = obj.insert(
                        "result".to_owned(),
                        serde_json::Value::String("false_branch".into()),
                    );
                }
                Ok(s)
            }),
        );

        let node_fn = node.into_node_fn();
        let state = ValueState(serde_json::json!({"flag": false}));
        let result = node_fn(state).await.unwrap();
        assert_eq!(result.0["result"], "false_branch");
    }

    /// T061: `IfElseNode` with a custom state type.
    #[tokio::test]
    async fn test_if_else_custom_state() {
        let node = IfElseNode::new(
            Box::new(|state: &TestCounterState| state.counter > 5),
            sync_node(|mut s: TestCounterState| {
                s.label = "high".to_owned();
                Ok(s)
            }),
            sync_node(|mut s: TestCounterState| {
                s.label = "low".to_owned();
                Ok(s)
            }),
        );

        let node_fn = node.into_node_fn();

        let high = node_fn(TestCounterState {
            counter: 10,
            label: String::new(),
        })
        .await
        .unwrap();
        assert_eq!(high.label, "high");

        // Rebuild node for second call (into_node_fn consumes self).
        let node = IfElseNode::new(
            Box::new(|state: &TestCounterState| state.counter > 5),
            sync_node(|mut s: TestCounterState| {
                s.label = "high".to_owned();
                Ok(s)
            }),
            sync_node(|mut s: TestCounterState| {
                s.label = "low".to_owned();
                Ok(s)
            }),
        );
        let node_fn = node.into_node_fn();

        let low = node_fn(TestCounterState {
            counter: 2,
            label: String::new(),
        })
        .await
        .unwrap();
        assert_eq!(low.label, "low");
    }

    // -----------------------------------------------------------------------
    // LoopNode tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_loop_node_terminates_on_predicate() {
        let node = LoopNode::new(
            sync_node(|mut s: ValueState| {
                let count =
                    s.0.get("count")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                if let Some(obj) = s.0.as_object_mut() {
                    let _prev = obj.insert("count".to_owned(), serde_json::json!(count + 1));
                }
                Ok(s)
            }),
            Box::new(|state: &ValueState| {
                state.0.get("count").and_then(serde_json::Value::as_u64) == Some(3)
            }),
            10,
        );

        let node_fn = node.into_node_fn();
        let state = ValueState(serde_json::json!({"count": 0}));
        let result = node_fn(state).await.unwrap();
        assert_eq!(result.0["count"], 3);
    }

    #[tokio::test]
    async fn test_loop_node_max_iterations() {
        let node = LoopNode::new(
            sync_node(Ok),
            Box::new(|_: &ValueState| false), // Never satisfied
            5,
        );

        let node_fn = node.into_node_fn();
        let state = ValueState(serde_json::json!({}));
        let err = node_fn(state).await.unwrap_err();
        assert!(err.to_string().contains('5'));
    }

    /// T061: `LoopNode` with a custom state type.
    #[tokio::test]
    async fn test_loop_node_custom_state() {
        let node = LoopNode::new(
            sync_node(|mut s: TestCounterState| {
                s.counter += 1;
                Ok(s)
            }),
            Box::new(|state: &TestCounterState| state.counter >= 3),
            10,
        );

        let node_fn = node.into_node_fn();
        let result = node_fn(TestCounterState {
            counter: 0,
            label: "loop_test".to_owned(),
        })
        .await
        .unwrap();
        assert_eq!(result.counter, 3);
        assert_eq!(result.label, "loop_test");
    }

    // -----------------------------------------------------------------------
    // ValidationNode tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_validation_node_passes() {
        let node = ValidationNode::new(vec!["name".into(), "age".into()]);
        let node_fn = node.into_node_fn();
        let state = ValueState(serde_json::json!({"name": "Alice", "age": 30}));
        let result = node_fn(state).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validation_node_missing_key() {
        let node = ValidationNode::new(vec!["name".into(), "email".into()]);
        let node_fn = node.into_node_fn();
        let state = ValueState(serde_json::json!({"name": "Alice"}));
        let err = node_fn(state).await.unwrap_err();
        assert!(err.to_string().contains("email"));
    }

    #[tokio::test]
    async fn test_validation_node_reject_empty() {
        let node = ValidationNode::new(vec!["name".into()]).with_reject_empty(true);
        let node_fn = node.into_node_fn();
        let state = ValueState(serde_json::json!({"name": ""}));
        let err = node_fn(state).await.unwrap_err();
        assert!(err.to_string().contains("empty string"));
    }

    #[tokio::test]
    async fn test_validation_node_reject_null() {
        let node = ValidationNode::new(vec!["name".into()]).with_reject_empty(true);
        let node_fn = node.into_node_fn();
        let state = ValueState(serde_json::json!({"name": null}));
        let err = node_fn(state).await.unwrap_err();
        assert!(err.to_string().contains("null"));
    }

    // -----------------------------------------------------------------------
    // TemplateTransformNode tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_template_transform_node() {
        let node =
            TemplateTransformNode::new("Hello, {name}! You are {age} years old.", "greeting");
        let node_fn = node.into_node_fn();
        let state = ValueState(serde_json::json!({"name": "Alice", "age": 30}));
        let result = node_fn(state).await.unwrap();
        assert_eq!(result.0["greeting"], "Hello, Alice! You are 30 years old.");
    }

    #[tokio::test]
    async fn test_template_transform_missing_placeholder() {
        let node = TemplateTransformNode::new("Hello, {name}! Contact: {email}", "output");
        let node_fn = node.into_node_fn();
        let state = ValueState(serde_json::json!({"name": "Alice"}));
        let result = node_fn(state).await.unwrap();
        // {email} is not replaced since the key doesn't exist.
        assert_eq!(result.0["output"], "Hello, Alice! Contact: {email}");
    }

    // -----------------------------------------------------------------------
    // ListOperatorNode tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_list_operator_sort() {
        let node = ListOperatorNode::new("items", "sorted", ListOperation::Sort);
        let node_fn = node.into_node_fn();
        let state = ValueState(serde_json::json!({"items": [3, 1, 2]}));
        let result = node_fn(state).await.unwrap();
        let sorted: Vec<i64> = result.0["sorted"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(serde_json::Value::as_i64)
            .collect();
        assert_eq!(sorted, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_list_operator_reverse() {
        let node = ListOperatorNode::new("items", "reversed", ListOperation::Reverse);
        let node_fn = node.into_node_fn();
        let state = ValueState(serde_json::json!({"items": [1, 2, 3]}));
        let result = node_fn(state).await.unwrap();
        let reversed: Vec<i64> = result.0["reversed"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(serde_json::Value::as_i64)
            .collect();
        assert_eq!(reversed, vec![3, 2, 1]);
    }

    #[tokio::test]
    async fn test_list_operator_deduplicate() {
        let node = ListOperatorNode::new("items", "deduped", ListOperation::Deduplicate);
        let node_fn = node.into_node_fn();
        let state = ValueState(serde_json::json!({"items": [1, 1, 2, 2, 3]}));
        let result = node_fn(state).await.unwrap();
        let deduped: Vec<i64> = result.0["deduped"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(serde_json::Value::as_i64)
            .collect();
        assert_eq!(deduped, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_list_operator_slice() {
        let node = ListOperatorNode::new("items", "sliced", ListOperation::Slice { count: 2 });
        let node_fn = node.into_node_fn();
        let state = ValueState(serde_json::json!({"items": [10, 20, 30, 40]}));
        let result = node_fn(state).await.unwrap();
        let sliced: Vec<i64> = result.0["sliced"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(serde_json::Value::as_i64)
            .collect();
        assert_eq!(sliced, vec![10, 20]);
    }

    #[tokio::test]
    async fn test_list_operator_with_filter() {
        let node = ListOperatorNode::new("items", "filtered", ListOperation::Sort)
            .with_filter(Box::new(|v| v.as_i64().is_some_and(|n| n > 2)));
        let node_fn = node.into_node_fn();
        let state = ValueState(serde_json::json!({"items": [5, 1, 3, 2, 4]}));
        let result = node_fn(state).await.unwrap();
        let filtered: Vec<i64> = result.0["filtered"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(serde_json::Value::as_i64)
            .collect();
        assert_eq!(filtered, vec![3, 4, 5]);
    }

    // -----------------------------------------------------------------------
    // VariableAggregatorNode tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_variable_aggregator() {
        let node =
            VariableAggregatorNode::new(vec!["a".into(), "b".into(), "c".into()], "collected");
        let node_fn = node.into_node_fn();
        let state = ValueState(serde_json::json!({"a": 1, "b": "two", "c": [3]}));
        let result = node_fn(state).await.unwrap();
        let collected = result.0["collected"].as_array().unwrap();
        assert_eq!(collected.len(), 3);
        assert_eq!(collected[0], 1);
        assert_eq!(collected[1], "two");
    }

    #[tokio::test]
    async fn test_variable_aggregator_missing_keys() {
        let node = VariableAggregatorNode::new(vec!["a".into(), "missing".into()], "collected");
        let node_fn = node.into_node_fn();
        let state = ValueState(serde_json::json!({"a": 1}));
        let result = node_fn(state).await.unwrap();
        let collected = result.0["collected"].as_array().unwrap();
        assert_eq!(collected.len(), 1);
    }

    // -----------------------------------------------------------------------
    // QuestionClassifierNode tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_question_classifier() {
        let node = QuestionClassifierNode::new(
            vec![
                (
                    "tech".into(),
                    vec!["rust".into(), "code".into(), "programming".into()],
                ),
                (
                    "math".into(),
                    vec!["calculate".into(), "equation".into(), "formula".into()],
                ),
            ],
            "general",
        );
        let node_fn = node.into_node_fn();

        let state = ValueState(serde_json::json!({"question": "How do I write Rust code?"}));
        let result = node_fn(state).await.unwrap();
        assert_eq!(result.0["category"], "tech");
    }

    #[tokio::test]
    async fn test_question_classifier_default() {
        let node = QuestionClassifierNode::new(
            vec![("tech".into(), vec!["rust".into(), "code".into()])],
            "general",
        );
        let node_fn = node.into_node_fn();

        let state = ValueState(serde_json::json!({"question": "What is the weather today?"}));
        let result = node_fn(state).await.unwrap();
        assert_eq!(result.0["category"], "general");
    }

    #[tokio::test]
    async fn test_question_classifier_custom_keys() {
        let node = QuestionClassifierNode::new(vec![("a".into(), vec!["hello".into()])], "other")
            .with_input_key("text")
            .with_output_key("class");
        let node_fn = node.into_node_fn();

        let state = ValueState(serde_json::json!({"text": "hello world"}));
        let result = node_fn(state).await.unwrap();
        assert_eq!(result.0["class"], "a");
    }

    // -----------------------------------------------------------------------
    // IterationNode tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_iteration_node() {
        let node = IterationNode::new(
            "items",
            "results",
            sync_node(|mut s: ValueState| {
                // Double the current item if it's a number.
                if let Some(n) =
                    s.0.get("__current_item")
                        .and_then(serde_json::Value::as_i64)
                    && let Some(obj) = s.0.as_object_mut()
                {
                    let _prev = obj.insert("__current_item".to_owned(), serde_json::json!(n * 2));
                }
                Ok(s)
            }),
        );
        let node_fn = node.into_node_fn();
        let state = ValueState(serde_json::json!({"items": [1, 2, 3]}));
        let result = node_fn(state).await.unwrap();
        let results: Vec<i64> = result.0["results"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(serde_json::Value::as_i64)
            .collect();
        assert_eq!(results, vec![2, 4, 6]);
    }

    #[tokio::test]
    async fn test_iteration_node_empty_collection() {
        let node = IterationNode::new("items", "results", sync_node(Ok));
        let node_fn = node.into_node_fn();
        let state = ValueState(serde_json::json!({"items": []}));
        let result = node_fn(state).await.unwrap();
        let results = result.0["results"].as_array().unwrap();
        assert!(results.is_empty());
    }
}
