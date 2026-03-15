//! Builder-pattern test fixtures for common Synwire types.

use std::collections::HashMap;

use serde_json::Value;
use synwire_core::documents::Document;
use synwire_core::messages::{Message, MessageContent, ToolCall};
use synwire_core::prompts::PromptTemplate;
use synwire_core::tools::ToolSchema;

/// Builder for constructing [`Message`] test fixtures.
#[derive(Debug, Default)]
pub struct MessageBuilder {
    id: Option<String>,
    name: Option<String>,
    content: Option<String>,
    role: Option<String>,
    tool_calls: Vec<ToolCall>,
    tool_call_id: Option<String>,
}

impl MessageBuilder {
    /// Creates a new empty [`MessageBuilder`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the message ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the sender name.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the message content text.
    #[must_use]
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Adds a tool call to the message.
    #[must_use]
    pub fn tool_call(mut self, call: ToolCall) -> Self {
        self.tool_calls.push(call);
        self
    }

    /// Sets the tool call ID (for tool messages).
    #[must_use]
    pub fn tool_call_id(mut self, id: impl Into<String>) -> Self {
        self.tool_call_id = Some(id.into());
        self
    }

    /// Sets the role (for chat messages).
    #[must_use]
    pub fn role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    /// Builds a human message.
    pub fn build_human(self) -> Message {
        Message::Human {
            id: self.id,
            name: self.name,
            content: MessageContent::Text(self.content.unwrap_or_default()),
            additional_kwargs: HashMap::new(),
        }
    }

    /// Builds an AI message.
    pub fn build_ai(self) -> Message {
        Message::AI {
            id: self.id,
            name: self.name,
            content: MessageContent::Text(self.content.unwrap_or_default()),
            tool_calls: self.tool_calls,
            invalid_tool_calls: Vec::new(),
            usage: None,
            response_metadata: None,
            additional_kwargs: HashMap::new(),
        }
    }

    /// Builds a system message.
    pub fn build_system(self) -> Message {
        Message::System {
            id: self.id,
            name: self.name,
            content: MessageContent::Text(self.content.unwrap_or_default()),
            additional_kwargs: HashMap::new(),
        }
    }

    /// Builds a tool response message.
    pub fn build_tool(self) -> Message {
        Message::tool(
            self.content.unwrap_or_default(),
            self.tool_call_id.unwrap_or_default(),
        )
    }
}

/// Builder for constructing [`Document`] test fixtures.
#[derive(Debug, Default)]
pub struct DocumentBuilder {
    id: Option<String>,
    page_content: String,
    metadata: HashMap<String, Value>,
}

impl DocumentBuilder {
    /// Creates a new [`DocumentBuilder`] with the given content.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            page_content: content.into(),
            ..Default::default()
        }
    }

    /// Sets the document ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Adds a metadata entry.
    #[must_use]
    pub fn metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        let _ = self.metadata.insert(key.into(), value);
        self
    }

    /// Builds the [`Document`].
    pub fn build(self) -> Document {
        Document {
            id: self.id,
            page_content: self.page_content,
            metadata: self.metadata,
        }
    }
}

/// Builder for constructing [`PromptTemplate`] test fixtures.
#[derive(Debug)]
pub struct PromptTemplateBuilder {
    template: String,
    input_variables: Vec<String>,
}

impl PromptTemplateBuilder {
    /// Creates a new [`PromptTemplateBuilder`] with the given template string.
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
            input_variables: Vec::new(),
        }
    }

    /// Adds an input variable.
    #[must_use]
    pub fn variable(mut self, name: impl Into<String>) -> Self {
        self.input_variables.push(name.into());
        self
    }

    /// Builds the [`PromptTemplate`].
    pub fn build(self) -> PromptTemplate {
        PromptTemplate::new(self.template, self.input_variables)
    }
}

/// Builder for constructing [`ToolSchema`] test fixtures.
#[derive(Debug)]
pub struct ToolSchemaBuilder {
    name: String,
    description: String,
    parameters: Value,
}

impl ToolSchemaBuilder {
    /// Creates a new [`ToolSchemaBuilder`] with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
        }
    }

    /// Sets the description.
    #[must_use]
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Sets the parameters JSON Schema.
    #[must_use]
    pub fn parameters(mut self, params: Value) -> Self {
        self.parameters = params;
        self
    }

    /// Builds the [`ToolSchema`].
    pub fn build(self) -> ToolSchema {
        ToolSchema {
            name: self.name,
            description: self.description,
            parameters: self.parameters,
        }
    }
}
