//! Factory functions that produce `Tool` implementations backed by an [`LspClient`].
//!
//! Call [`lsp_tools`] with a shared client to get a capability-conditional
//! set of tools ready for agent registration.

use std::sync::Arc;

use serde_json::json;

use synwire_core::error::{SynwireError, ToolError};
use synwire_core::tools::{StructuredTool, Tool, ToolOutput, ToolSchema};

use crate::client::LspClient;
use crate::error::LspError;

// ── Helpers ──────────────────────────────────────────────────────────────────

#[allow(clippy::needless_pass_by_value)] // used in map_err closures
fn lsp_tool_err(e: LspError) -> SynwireError {
    SynwireError::Tool(ToolError::InvocationFailed {
        message: e.to_string(),
    })
}

const fn ok(content: String) -> ToolOutput {
    ToolOutput {
        content,
        artifact: None,
        binary_results: Vec::new(),
        status: synwire_core::tools::ToolResultStatus::Success,
        telemetry: None,
        content_type: None,
    }
}

fn extract_str(input: &serde_json::Value, key: &str) -> Result<String, SynwireError> {
    input
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(String::from)
        .ok_or_else(|| {
            SynwireError::Tool(ToolError::ValidationFailed {
                message: format!("missing required string parameter: {key}"),
            })
        })
}

fn extract_u32(input: &serde_json::Value, key: &str) -> Result<u32, SynwireError> {
    input
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| {
            SynwireError::Tool(ToolError::ValidationFailed {
                message: format!("missing required integer parameter: {key}"),
            })
        })
}

fn make_uri(path: &str) -> Result<lsp_types::Url, SynwireError> {
    lsp_types::Url::from_file_path(path).map_err(|()| {
        SynwireError::Tool(ToolError::ValidationFailed {
            message: format!("invalid file path: {path}"),
        })
    })
}

// ── Public factory ───────────────────────────────────────────────────────────

/// Build all LSP tools appropriate for the server's advertised capabilities.
///
/// Tools that are always available (status, diagnostics) are included
/// unconditionally. Feature-specific tools (hover, definition, references,
/// etc.) are only included when the server capability is present.
#[allow(clippy::too_many_lines, clippy::needless_pass_by_value)]
pub fn lsp_tools(client: Arc<LspClient>) -> Vec<Box<dyn Tool>> {
    let mut tools: Vec<Box<dyn Tool>> = Vec::new();

    // Always available.
    if let Some(tool) = build_lsp_status_tool(Arc::clone(&client)) {
        tools.push(Box::new(tool));
    }
    if let Some(tool) = build_lsp_diagnostics_tool(Arc::clone(&client)) {
        tools.push(Box::new(tool));
    }

    // Capability-conditional.
    let caps = client.capabilities();
    if let Some(ref caps) = caps {
        if caps.definition_provider.is_some() {
            if let Some(tool) = build_goto_definition_tool(Arc::clone(&client)) {
                tools.push(Box::new(tool));
            }
        }
        if caps.references_provider.is_some() {
            if let Some(tool) = build_find_references_tool(Arc::clone(&client)) {
                tools.push(Box::new(tool));
            }
        }
        if caps.hover_provider.is_some() {
            if let Some(tool) = build_hover_tool(Arc::clone(&client)) {
                tools.push(Box::new(tool));
            }
        }
        if caps.completion_provider.is_some() {
            if let Some(tool) = build_completion_tool(Arc::clone(&client)) {
                tools.push(Box::new(tool));
            }
        }
        if caps.document_symbol_provider.is_some() {
            if let Some(tool) = build_document_symbols_tool(Arc::clone(&client)) {
                tools.push(Box::new(tool));
            }
        }
        if caps.workspace_symbol_provider.is_some() {
            if let Some(tool) = build_workspace_symbols_tool(Arc::clone(&client)) {
                tools.push(Box::new(tool));
            }
        }
        if caps.code_action_provider.is_some() {
            if let Some(tool) = build_code_actions_tool(Arc::clone(&client)) {
                tools.push(Box::new(tool));
            }
        }
        if caps.document_formatting_provider.is_some() {
            if let Some(tool) = build_formatting_tool(Arc::clone(&client)) {
                tools.push(Box::new(tool));
            }
        }
        if caps.rename_provider.is_some() {
            if let Some(tool) = build_rename_tool(Arc::clone(&client)) {
                tools.push(Box::new(tool));
            }
        }
        if caps.signature_help_provider.is_some() {
            if let Some(tool) = build_signature_help_tool(Arc::clone(&client)) {
                tools.push(Box::new(tool));
            }
        }
    }

    tools
}

// ── Tool builders ────────────────────────────────────────────────────────────

fn build_lsp_status_tool(client: Arc<LspClient>) -> Option<StructuredTool> {
    StructuredTool::builder()
        .name("lsp.status")
        .description(
            "Report the current state of the LSP server (starting, running, stopped).\n\
             When to use: Check whether the language server is alive before making requests.\n\
             Returns: A JSON object with the server state and capability summary.",
        )
        .schema(ToolSchema {
            name: "lsp.status".into(),
            description: "Report LSP server state and capabilities.".into(),
            parameters: json!({"type": "object", "properties": {}}),
        })
        .func(move |_| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let state = client.status();
                let caps = client.capabilities();
                let result = json!({
                    "state": state.to_string(),
                    "has_capabilities": caps.is_some(),
                    "open_documents": client.documents().len(),
                });
                Ok(ok(
                    serde_json::to_string_pretty(&result).map_err(SynwireError::from)?
                ))
            })
        })
        .build()
        .ok()
}

fn build_lsp_diagnostics_tool(client: Arc<LspClient>) -> Option<StructuredTool> {
    StructuredTool::builder()
        .name("lsp.diagnostics")
        .description(
            "Retrieve compiler/linter diagnostics (errors, warnings) for a file.\n\
             When to use: After editing code, check for problems reported by the language server.\n\
             Parameters: file_path (absolute path to the file).\n\
             Returns: A JSON array of diagnostic objects with severity, range, and message.",
        )
        .schema(ToolSchema {
            name: "lsp.diagnostics".into(),
            description: "Get diagnostics for a file.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Absolute path to the file"}
                },
                "required": ["file_path"]
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let path = extract_str(&input, "file_path")?;
                let uri = make_uri(&path)?;
                let diags = client.diagnostics(&uri);
                let result = serde_json::to_string_pretty(&diags).map_err(SynwireError::from)?;
                Ok(ok(result))
            })
        })
        .build()
        .ok()
}

fn build_goto_definition_tool(client: Arc<LspClient>) -> Option<StructuredTool> {
    StructuredTool::builder()
        .name("lsp.definition")
        .description(
            "Jump to the definition of the symbol at a given position.\n\
             When to use: To find where a function, type, or variable is defined.\n\
             Parameters: file_path, line (0-indexed), character (0-indexed).\n\
             Returns: JSON with location(s) of the definition.",
        )
        .schema(ToolSchema {
            name: "lsp.definition".into(),
            description: "Go to definition of symbol at position.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Absolute path to the file"},
                    "line": {"type": "integer", "description": "0-indexed line number"},
                    "character": {"type": "integer", "description": "0-indexed character offset"}
                },
                "required": ["file_path", "line", "character"]
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let path = extract_str(&input, "file_path")?;
                let line = extract_u32(&input, "line")?;
                let character = extract_u32(&input, "character")?;
                let uri = make_uri(&path)?;
                let result = client
                    .goto_definition(&uri, line, character)
                    .await
                    .map_err(lsp_tool_err)?;
                let json = serde_json::to_string_pretty(&result).map_err(SynwireError::from)?;
                Ok(ok(json))
            })
        })
        .build()
        .ok()
}

fn build_find_references_tool(client: Arc<LspClient>) -> Option<StructuredTool> {
    StructuredTool::builder()
        .name("lsp.references")
        .description(
            "Find all references to the symbol at a given position.\n\
             When to use: To see everywhere a function, type, or variable is used.\n\
             Parameters: file_path, line (0-indexed), character (0-indexed).\n\
             Returns: JSON array of reference locations.",
        )
        .schema(ToolSchema {
            name: "lsp.references".into(),
            description: "Find all references to symbol at position.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Absolute path to the file"},
                    "line": {"type": "integer", "description": "0-indexed line number"},
                    "character": {"type": "integer", "description": "0-indexed character offset"}
                },
                "required": ["file_path", "line", "character"]
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let path = extract_str(&input, "file_path")?;
                let line = extract_u32(&input, "line")?;
                let character = extract_u32(&input, "character")?;
                let uri = make_uri(&path)?;
                let result = client
                    .find_references(&uri, line, character)
                    .await
                    .map_err(lsp_tool_err)?;
                let json = serde_json::to_string_pretty(&result).map_err(SynwireError::from)?;
                Ok(ok(json))
            })
        })
        .build()
        .ok()
}

fn build_hover_tool(client: Arc<LspClient>) -> Option<StructuredTool> {
    StructuredTool::builder()
        .name("lsp.hover")
        .description(
            "Get hover information (type, documentation) for the symbol at a position.\n\
             When to use: To inspect the type or documentation of a symbol.\n\
             Parameters: file_path, line (0-indexed), character (0-indexed).\n\
             Returns: Hover content (typically markdown).",
        )
        .schema(ToolSchema {
            name: "lsp.hover".into(),
            description: "Get hover info for symbol at position.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Absolute path to the file"},
                    "line": {"type": "integer", "description": "0-indexed line number"},
                    "character": {"type": "integer", "description": "0-indexed character offset"}
                },
                "required": ["file_path", "line", "character"]
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let path = extract_str(&input, "file_path")?;
                let line = extract_u32(&input, "line")?;
                let character = extract_u32(&input, "character")?;
                let uri = make_uri(&path)?;
                let result = client
                    .hover(&uri, line, character)
                    .await
                    .map_err(lsp_tool_err)?;
                let json = serde_json::to_string_pretty(&result).map_err(SynwireError::from)?;
                Ok(ok(json))
            })
        })
        .build()
        .ok()
}

fn build_completion_tool(client: Arc<LspClient>) -> Option<StructuredTool> {
    StructuredTool::builder()
        .name("lsp.completion")
        .description(
            "Get completion suggestions at a position.\n\
             When to use: To see what the language server suggests at a cursor position.\n\
             Parameters: file_path, line (0-indexed), character (0-indexed).\n\
             Returns: JSON with completion items.",
        )
        .schema(ToolSchema {
            name: "lsp.completion".into(),
            description: "Get completion suggestions at position.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Absolute path to the file"},
                    "line": {"type": "integer", "description": "0-indexed line number"},
                    "character": {"type": "integer", "description": "0-indexed character offset"}
                },
                "required": ["file_path", "line", "character"]
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let path = extract_str(&input, "file_path")?;
                let line = extract_u32(&input, "line")?;
                let character = extract_u32(&input, "character")?;
                let uri = make_uri(&path)?;
                let result = client
                    .completion(&uri, line, character)
                    .await
                    .map_err(lsp_tool_err)?;
                let json = serde_json::to_string_pretty(&result).map_err(SynwireError::from)?;
                Ok(ok(json))
            })
        })
        .build()
        .ok()
}

fn build_document_symbols_tool(client: Arc<LspClient>) -> Option<StructuredTool> {
    StructuredTool::builder()
        .name("lsp.symbols")
        .description(
            "List all symbols (functions, types, variables) in a document.\n\
             When to use: To get an overview of a file's structure.\n\
             Parameters: file_path.\n\
             Returns: JSON array of symbol information.",
        )
        .schema(ToolSchema {
            name: "lsp.symbols".into(),
            description: "List all symbols in a document.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Absolute path to the file"}
                },
                "required": ["file_path"]
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let path = extract_str(&input, "file_path")?;
                let uri = make_uri(&path)?;
                let result = client.document_symbols(&uri).await.map_err(lsp_tool_err)?;
                let json = serde_json::to_string_pretty(&result).map_err(SynwireError::from)?;
                Ok(ok(json))
            })
        })
        .build()
        .ok()
}

fn build_workspace_symbols_tool(client: Arc<LspClient>) -> Option<StructuredTool> {
    StructuredTool::builder()
        .name("lsp.workspace_symbols")
        .description(
            "Search for symbols across the entire workspace.\n\
             When to use: To find a function or type by name across all project files.\n\
             Parameters: query (search string).\n\
             Returns: JSON array of symbol information.",
        )
        .schema(ToolSchema {
            name: "lsp.workspace_symbols".into(),
            description: "Search for symbols across the workspace.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Symbol name or partial name to search for"}
                },
                "required": ["query"]
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let query = extract_str(&input, "query")?;
                let result = client.workspace_symbols(&query).await.map_err(lsp_tool_err)?;
                let json = serde_json::to_string_pretty(&result).map_err(SynwireError::from)?;
                Ok(ok(json))
            })
        })
        .build()
        .ok()
}

fn build_code_actions_tool(client: Arc<LspClient>) -> Option<StructuredTool> {
    StructuredTool::builder()
        .name("lsp.code_actions")
        .description(
            "Get available code actions (quick fixes, refactorings) for a range.\n\
             When to use: To see what automated fixes or refactorings the server offers.\n\
             Parameters: file_path, start_line, start_character, end_line, end_character.\n\
             Returns: JSON array of code action objects.",
        )
        .schema(ToolSchema {
            name: "lsp.code_actions".into(),
            description: "Get code actions for a range.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Absolute path to the file"},
                    "start_line": {"type": "integer", "description": "0-indexed start line"},
                    "start_character": {"type": "integer", "description": "0-indexed start character"},
                    "end_line": {"type": "integer", "description": "0-indexed end line"},
                    "end_character": {"type": "integer", "description": "0-indexed end character"}
                },
                "required": ["file_path", "start_line", "start_character", "end_line", "end_character"]
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let path = extract_str(&input, "file_path")?;
                let start_line = extract_u32(&input, "start_line")?;
                let start_char = extract_u32(&input, "start_character")?;
                let end_line = extract_u32(&input, "end_line")?;
                let end_char = extract_u32(&input, "end_character")?;
                let uri = make_uri(&path)?;
                let range = lsp_types::Range {
                    start: lsp_types::Position::new(start_line, start_char),
                    end: lsp_types::Position::new(end_line, end_char),
                };
                let result = client.code_actions(&uri, range).await.map_err(lsp_tool_err)?;
                let json = serde_json::to_string_pretty(&result).map_err(SynwireError::from)?;
                Ok(ok(json))
            })
        })
        .build()
        .ok()
}

fn build_formatting_tool(client: Arc<LspClient>) -> Option<StructuredTool> {
    StructuredTool::builder()
        .name("lsp.formatting")
        .description(
            "Format an entire document using the language server's formatter.\n\
             When to use: To auto-format code according to the project's style.\n\
             Parameters: file_path.\n\
             Returns: JSON array of text edits to apply.",
        )
        .schema(ToolSchema {
            name: "lsp.formatting".into(),
            description: "Format a document.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Absolute path to the file"}
                },
                "required": ["file_path"]
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let path = extract_str(&input, "file_path")?;
                let uri = make_uri(&path)?;
                let result = client.formatting(&uri).await.map_err(lsp_tool_err)?;
                let json = serde_json::to_string_pretty(&result).map_err(SynwireError::from)?;
                Ok(ok(json))
            })
        })
        .build()
        .ok()
}

fn build_rename_tool(client: Arc<LspClient>) -> Option<StructuredTool> {
    StructuredTool::builder()
        .name("lsp.rename")
        .description(
            "Rename a symbol across the workspace.\n\
             When to use: To rename a function, variable, or type and update all references.\n\
             Parameters: file_path, line, character, new_name.\n\
             Returns: JSON workspace edit describing all changes.",
        )
        .schema(ToolSchema {
            name: "lsp.rename".into(),
            description: "Rename a symbol across the workspace.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Absolute path to the file"},
                    "line": {"type": "integer", "description": "0-indexed line number"},
                    "character": {"type": "integer", "description": "0-indexed character offset"},
                    "new_name": {"type": "string", "description": "The new name for the symbol"}
                },
                "required": ["file_path", "line", "character", "new_name"]
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let path = extract_str(&input, "file_path")?;
                let line = extract_u32(&input, "line")?;
                let character = extract_u32(&input, "character")?;
                let new_name = extract_str(&input, "new_name")?;
                let uri = make_uri(&path)?;
                let result = client
                    .rename(&uri, line, character, &new_name)
                    .await
                    .map_err(lsp_tool_err)?;
                let json = serde_json::to_string_pretty(&result).map_err(SynwireError::from)?;
                Ok(ok(json))
            })
        })
        .build()
        .ok()
}

fn build_signature_help_tool(client: Arc<LspClient>) -> Option<StructuredTool> {
    StructuredTool::builder()
        .name("lsp.signature_help")
        .description(
            "Get signature help (parameter info) at a position.\n\
             When to use: To see the parameter list and documentation of a function being called.\n\
             Parameters: file_path, line (0-indexed), character (0-indexed).\n\
             Returns: JSON with signature help information.",
        )
        .schema(ToolSchema {
            name: "lsp.signature_help".into(),
            description: "Get signature help at position.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Absolute path to the file"},
                    "line": {"type": "integer", "description": "0-indexed line number"},
                    "character": {"type": "integer", "description": "0-indexed character offset"}
                },
                "required": ["file_path", "line", "character"]
            }),
        })
        .func(move |input| {
            let client = Arc::clone(&client);
            Box::pin(async move {
                let path = extract_str(&input, "file_path")?;
                let line = extract_u32(&input, "line")?;
                let character = extract_u32(&input, "character")?;
                let uri = make_uri(&path)?;
                let result = client
                    .signature_help(&uri, line, character)
                    .await
                    .map_err(lsp_tool_err)?;
                let json = serde_json::to_string_pretty(&result).map_err(SynwireError::from)?;
                Ok(ok(json))
            })
        })
        .build()
        .ok()
}
