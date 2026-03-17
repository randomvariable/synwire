//! Core LSP client wrapping `async-lsp` transport and `ServerSocket`.
//!
//! The [`LspClient`] spawns a language-server child process, wires its
//! stdin/stdout through an `async-lsp` `MainLoop`, and exposes high-level
//! async methods for every LSP operation the framework needs.

use std::collections::HashMap;
use std::ops::ControlFlow;
use std::process::Stdio;
use std::sync::{Arc, RwLock};

use async_lsp::concurrency::ConcurrencyLayer;
use async_lsp::panic::CatchUnwindLayer;
use async_lsp::router::Router;
use async_lsp::tracing::TracingLayer;
use async_lsp::{LanguageClient, LanguageServer, ResponseError, ServerSocket};
use lsp_types::{
    ClientCapabilities, CodeActionContext, CodeActionParams, CompletionParams,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentFormattingParams, DocumentSymbolParams, FormattingOptions, GotoDefinitionParams,
    GotoDefinitionResponse, HoverParams, InitializeParams, InitializedParams, PartialResultParams,
    Position, PublishDiagnosticsParams, Range, ReferenceContext, ReferenceParams, RenameParams,
    ServerCapabilities, ShowMessageParams, SignatureHelpParams, TextDocumentContentChangeEvent,
    TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams, Url,
    VersionedTextDocumentIdentifier, WindowClientCapabilities, WorkDoneProgressParams,
    WorkspaceFolder, WorkspaceSymbolParams, WorkspaceSymbolResponse,
};
use tokio::task::JoinHandle;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tower::ServiceBuilder;

use crate::config::LspServerConfig;
use crate::document_sync::DocumentSyncManager;
use crate::error::LspError;

/// Event used internally to shut down the main loop.
struct StopMainLoop;

// ── Server state tracking ────────────────────────────────────────────────────

/// Lifecycle state of the language server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LspServerState {
    /// Child process spawned, waiting for `initialize` response.
    Starting,
    /// `initialize` succeeded, server is ready for requests.
    Running,
    /// Server is shutting down (`shutdown` sent).
    ShuttingDown,
    /// Server process has exited.
    Stopped,
}

impl std::fmt::Display for LspServerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Starting => f.write_str("starting"),
            Self::Running => f.write_str("running"),
            Self::ShuttingDown => f.write_str("shutting_down"),
            Self::Stopped => f.write_str("stopped"),
        }
    }
}

// ── Client handler (receives server-to-client notifications) ─────────────────

/// Shared diagnostics cache: URI -> list of diagnostics.
type DiagnosticsCache = Arc<RwLock<HashMap<Url, Vec<lsp_types::Diagnostic>>>>;

/// State struct that implements [`LanguageClient`] to handle server-to-client
/// notifications such as `publishDiagnostics` and `window/showMessage`.
struct ClientHandler {
    diagnostics: DiagnosticsCache,
}

impl LanguageClient for ClientHandler {
    type Error = ResponseError;
    type NotifyResult = ControlFlow<async_lsp::Result<()>>;

    fn publish_diagnostics(&mut self, params: PublishDiagnosticsParams) -> Self::NotifyResult {
        tracing::debug!(
            uri = %params.uri,
            count = params.diagnostics.len(),
            "Received publishDiagnostics"
        );
        if let Ok(mut cache) = self.diagnostics.write() {
            let _prev = cache.insert(params.uri, params.diagnostics);
        }
        ControlFlow::Continue(())
    }

    fn show_message(&mut self, params: ShowMessageParams) -> Self::NotifyResult {
        tracing::info!(
            typ = ?params.typ,
            message = %params.message,
            "LSP window/showMessage"
        );
        ControlFlow::Continue(())
    }

    fn log_message(&mut self, params: lsp_types::LogMessageParams) -> Self::NotifyResult {
        match params.typ {
            lsp_types::MessageType::ERROR => {
                tracing::error!(message = %params.message, "LSP server log");
            }
            lsp_types::MessageType::WARNING => {
                tracing::warn!(message = %params.message, "LSP server log");
            }
            lsp_types::MessageType::INFO => {
                tracing::info!(message = %params.message, "LSP server log");
            }
            _ => {
                tracing::debug!(message = %params.message, "LSP server log");
            }
        }
        ControlFlow::Continue(())
    }
}

impl ClientHandler {
    fn new_router(diagnostics: DiagnosticsCache) -> Router<Self> {
        let mut router = Router::from_language_client(Self { diagnostics });
        let _: &mut Router<Self> = router.event(Self::on_stop);
        router
    }

    #[allow(clippy::unused_self, clippy::missing_const_for_fn)] // required by async-lsp's Router::event signature
    fn on_stop(&mut self, _: StopMainLoop) -> ControlFlow<async_lsp::Result<()>> {
        ControlFlow::Break(Ok(()))
    }
}

// ── LspClient ────────────────────────────────────────────────────────────────

/// High-level LSP client that manages a child language-server process.
///
/// Use [`LspClient::start`] to spawn a server and then [`LspClient::initialize`]
/// to perform the LSP handshake.
pub struct LspClient {
    /// Socket for sending requests/notifications to the server.
    server: Arc<tokio::sync::Mutex<ServerSocket>>,
    /// Current lifecycle state.
    state: Arc<RwLock<LspServerState>>,
    /// Server capabilities from the `initialize` response.
    capabilities: Arc<RwLock<Option<ServerCapabilities>>>,
    /// Diagnostics cache, populated by `publishDiagnostics` notifications.
    diagnostics: DiagnosticsCache,
    /// Document sync manager.
    documents: DocumentSyncManager,
    /// Handle to the main loop task.
    _mainloop_handle: JoinHandle<()>,
    /// Handle to the child process (kept alive for the process lifetime).
    _child: tokio::process::Child,
}

impl std::fmt::Debug for LspClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LspClient")
            .field("state", &self.status())
            .finish_non_exhaustive()
    }
}

/// Convert an `async_lsp::Error` into an [`LspError`].
fn lsp_err(method: &str, err: &async_lsp::Error) -> LspError {
    LspError::RequestFailed {
        method: method.into(),
        message: err.to_string(),
    }
}

impl LspClient {
    /// Spawn a language server child process and wire up the async-lsp transport.
    ///
    /// After calling `start`, the server is in [`LspServerState::Starting`].
    /// Call [`initialize`](Self::initialize) to complete the LSP handshake.
    ///
    /// # Errors
    ///
    /// Returns [`LspError::BinaryNotFound`] if the command cannot be resolved,
    /// or [`LspError::Transport`] if the child process fails to spawn.
    #[allow(clippy::unused_async)] // kept async for API consistency; spawns async tasks internally
    pub fn start(config: &LspServerConfig) -> Result<Self, LspError> {
        // Verify the binary exists.
        let _binary_path = which::which(&config.command).map_err(|_| LspError::BinaryNotFound {
            binary: config.command.clone(),
        })?;

        // Spawn the child process.
        let mut cmd = tokio::process::Command::new(&config.command);
        let _: &mut tokio::process::Command = cmd
            .args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);

        for (k, v) in &config.env {
            let _: &mut tokio::process::Command = cmd.env(k, v);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| LspError::Transport(e.to_string()))?;

        let child_stdout = child
            .stdout
            .take()
            .ok_or_else(|| LspError::Transport("child stdout not available".into()))?;
        let child_stdin = child
            .stdin
            .take()
            .ok_or_else(|| LspError::Transport("child stdin not available".into()))?;

        // Build the async-lsp main loop with our ClientHandler for server-to-client messages.
        let diagnostics: DiagnosticsCache = Arc::new(RwLock::new(HashMap::new()));

        let (mainloop, server) = {
            let diag = Arc::clone(&diagnostics);
            async_lsp::MainLoop::new_client(|_server| {
                ServiceBuilder::new()
                    .layer(TracingLayer::default())
                    .layer(CatchUnwindLayer::default())
                    .layer(ConcurrencyLayer::default())
                    .service(ClientHandler::new_router(diag))
            })
        };

        // Bridge tokio IO types to futures IO types via tokio-util compat.
        let stdout_compat = child_stdout.compat();
        let stdin_compat = child_stdin.compat_write();

        let mainloop_handle = tokio::spawn(async move {
            if let Err(e) = mainloop.run_buffered(stdout_compat, stdin_compat).await {
                tracing::warn!(error = %e, "LSP main loop exited with error");
            }
        });

        let documents = DocumentSyncManager::new(256);

        Ok(Self {
            server: Arc::new(tokio::sync::Mutex::new(server)),
            state: Arc::new(RwLock::new(LspServerState::Starting)),
            capabilities: Arc::new(RwLock::new(None)),
            diagnostics,
            documents,
            _mainloop_handle: mainloop_handle,
            _child: child,
        })
    }

    /// Perform the LSP `initialize` / `initialized` handshake.
    ///
    /// After success the client transitions to [`LspServerState::Running`].
    ///
    /// # Errors
    ///
    /// Returns [`LspError::InitializationFailed`] if the server rejects the
    /// handshake, or [`LspError::NotReady`] if the client is not in
    /// `Starting` state.
    pub async fn initialize(&self) -> Result<(), LspError> {
        {
            let state = self.state.read().map_err(|e| LspError::NotReady {
                state: e.to_string(),
            })?;
            if *state != LspServerState::Starting {
                return Err(LspError::NotReady {
                    state: state.to_string(),
                });
            }
        }

        let init_params = InitializeParams {
            capabilities: ClientCapabilities {
                window: Some(WindowClientCapabilities {
                    work_done_progress: Some(true),
                    ..WindowClientCapabilities::default()
                }),
                ..ClientCapabilities::default()
            },
            ..InitializeParams::default()
        };

        let init_result = {
            let mut server = self.server.lock().await;
            server
                .initialize(init_params)
                .await
                .map_err(|e| LspError::InitializationFailed(e.to_string()))?
        };

        // Cache server capabilities.
        if let Ok(mut caps) = self.capabilities.write() {
            *caps = Some(init_result.capabilities);
        }

        // Send initialized notification.
        {
            let mut server = self.server.lock().await;
            let _res = server.initialized(InitializedParams {});
        }

        // Transition to Running.
        if let Ok(mut state) = self.state.write() {
            *state = LspServerState::Running;
        }

        tracing::info!("LSP server initialized successfully");
        Ok(())
    }

    /// Perform the LSP `initialize` handshake with a specific workspace root.
    ///
    /// # Errors
    ///
    /// Same as [`initialize`](Self::initialize).
    #[allow(deprecated)] // root_uri is deprecated in LSP spec but widely used
    pub async fn initialize_with_root(&self, root_uri: &Url) -> Result<(), LspError> {
        {
            let state = self.state.read().map_err(|e| LspError::NotReady {
                state: e.to_string(),
            })?;
            if *state != LspServerState::Starting {
                return Err(LspError::NotReady {
                    state: state.to_string(),
                });
            }
        }

        let init_params = InitializeParams {
            root_uri: Some(root_uri.clone()),
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: root_uri.clone(),
                name: "root".into(),
            }]),
            capabilities: ClientCapabilities {
                window: Some(WindowClientCapabilities {
                    work_done_progress: Some(true),
                    ..WindowClientCapabilities::default()
                }),
                ..ClientCapabilities::default()
            },
            ..InitializeParams::default()
        };

        let init_result = {
            let mut server = self.server.lock().await;
            server
                .initialize(init_params)
                .await
                .map_err(|e| LspError::InitializationFailed(e.to_string()))?
        };

        if let Ok(mut caps) = self.capabilities.write() {
            *caps = Some(init_result.capabilities);
        }

        {
            let mut server = self.server.lock().await;
            let _res = server.initialized(InitializedParams {});
        }

        if let Ok(mut state) = self.state.write() {
            *state = LspServerState::Running;
        }

        tracing::info!("LSP server initialized with root {root_uri}");
        Ok(())
    }

    /// Send the `shutdown` and `exit` sequence.
    ///
    /// # Errors
    ///
    /// Returns [`LspError::RequestFailed`] if the shutdown request fails.
    pub async fn shutdown(&self) -> Result<(), LspError> {
        if let Ok(mut state) = self.state.write() {
            *state = LspServerState::ShuttingDown;
        }

        let result = {
            let mut server = self.server.lock().await;
            server
                .shutdown(())
                .await
                .map_err(|ref e| lsp_err("shutdown", e))
        };

        {
            let mut server = self.server.lock().await;
            let _res = server.exit(());
            let _res = server.emit(StopMainLoop);
        }

        if let Ok(mut state) = self.state.write() {
            *state = LspServerState::Stopped;
        }

        result
    }

    /// Current lifecycle state of the server.
    #[must_use]
    pub fn status(&self) -> LspServerState {
        self.state
            .read()
            .map(|s| *s)
            .unwrap_or(LspServerState::Stopped)
    }

    /// Server capabilities received from the `initialize` response, if any.
    #[must_use]
    pub fn capabilities(&self) -> Option<ServerCapabilities> {
        self.capabilities.read().ok().and_then(|c| c.clone())
    }

    /// Reference to the internal document sync manager.
    #[must_use]
    pub const fn documents(&self) -> &DocumentSyncManager {
        &self.documents
    }

    // ── Guard helper ────────────────────────────────────────────────────

    fn require_running(&self) -> Result<(), LspError> {
        let state = self.status();
        if state != LspServerState::Running {
            return Err(LspError::NotReady {
                state: state.to_string(),
            });
        }
        Ok(())
    }

    // ── Document synchronisation ────────────────────────────────────────

    /// Send `textDocument/didOpen`.
    ///
    /// # Errors
    ///
    /// Returns [`LspError::NotReady`] if the server is not running, or
    /// [`LspError::NotificationFailed`] on transport errors.
    pub async fn open_document(
        &self,
        uri: &Url,
        language_id: &str,
        text: &str,
    ) -> Result<(), LspError> {
        self.require_running()?;
        let _tracked = self
            .documents
            .open(uri.clone(), language_id.into(), text.into());

        let mut server = self.server.lock().await;
        server
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: language_id.into(),
                    version: 0,
                    text: text.into(),
                },
            })
            .map_err(|e| LspError::NotificationFailed {
                method: "textDocument/didOpen".into(),
                message: e.to_string(),
            })
    }

    /// Send `textDocument/didClose`.
    ///
    /// # Errors
    ///
    /// Returns [`LspError::NotReady`] if the server is not running.
    pub async fn close_document(&self, uri: &Url) -> Result<(), LspError> {
        self.require_running()?;
        self.documents.close(uri);

        let mut server = self.server.lock().await;
        server
            .did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
            })
            .map_err(|e| LspError::NotificationFailed {
                method: "textDocument/didClose".into(),
                message: e.to_string(),
            })
    }

    /// Send `textDocument/didChange` with full-sync semantics.
    ///
    /// # Errors
    ///
    /// Returns [`LspError::DocumentNotOpen`] if the document is not tracked.
    pub async fn change_document(&self, uri: &Url, text: &str) -> Result<(), LspError> {
        self.require_running()?;
        let version =
            self.documents
                .change(uri, text.into())
                .ok_or_else(|| LspError::DocumentNotOpen {
                    uri: uri.to_string(),
                })?;

        let mut server = self.server.lock().await;
        server
            .did_change(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: text.into(),
                }],
            })
            .map_err(|e| LspError::NotificationFailed {
                method: "textDocument/didChange".into(),
                message: e.to_string(),
            })
    }

    // ── LSP requests ────────────────────────────────────────────────────

    /// `textDocument/hover`.
    ///
    /// # Errors
    ///
    /// Returns [`LspError::RequestFailed`] on server error.
    pub async fn hover(
        &self,
        uri: &Url,
        line: u32,
        character: u32,
    ) -> Result<Option<lsp_types::Hover>, LspError> {
        self.require_running()?;
        let mut server = self.server.lock().await;
        server
            .hover(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position::new(line, character),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .map_err(|ref e| lsp_err("textDocument/hover", e))
    }

    /// `textDocument/definition`.
    ///
    /// # Errors
    ///
    /// Returns [`LspError::RequestFailed`] on server error.
    pub async fn goto_definition(
        &self,
        uri: &Url,
        line: u32,
        character: u32,
    ) -> Result<Option<GotoDefinitionResponse>, LspError> {
        self.require_running()?;
        let mut server = self.server.lock().await;
        server
            .definition(GotoDefinitionParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position::new(line, character),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await
            .map_err(|ref e| lsp_err("textDocument/definition", e))
    }

    /// `textDocument/references`.
    ///
    /// # Errors
    ///
    /// Returns [`LspError::RequestFailed`] on server error.
    pub async fn find_references(
        &self,
        uri: &Url,
        line: u32,
        character: u32,
    ) -> Result<Option<Vec<lsp_types::Location>>, LspError> {
        self.require_running()?;
        let mut server = self.server.lock().await;
        server
            .references(ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position::new(line, character),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: ReferenceContext {
                    include_declaration: true,
                },
            })
            .await
            .map_err(|ref e| lsp_err("textDocument/references", e))
    }

    /// `textDocument/completion`.
    ///
    /// # Errors
    ///
    /// Returns [`LspError::RequestFailed`] on server error.
    pub async fn completion(
        &self,
        uri: &Url,
        line: u32,
        character: u32,
    ) -> Result<Option<lsp_types::CompletionResponse>, LspError> {
        self.require_running()?;
        let mut server = self.server.lock().await;
        server
            .completion(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position::new(line, character),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: None,
            })
            .await
            .map_err(|ref e| lsp_err("textDocument/completion", e))
    }

    /// `textDocument/documentSymbol`.
    ///
    /// # Errors
    ///
    /// Returns [`LspError::RequestFailed`] on server error.
    pub async fn document_symbols(
        &self,
        uri: &Url,
    ) -> Result<Option<lsp_types::DocumentSymbolResponse>, LspError> {
        self.require_running()?;
        let mut server = self.server.lock().await;
        server
            .document_symbol(DocumentSymbolParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await
            .map_err(|ref e| lsp_err("textDocument/documentSymbol", e))
    }

    /// `workspace/symbol`.
    ///
    /// Returns only the "flat" `SymbolInformation` variant. If the server
    /// returns the newer `WorkspaceSymbol` variant, those entries are omitted.
    ///
    /// # Errors
    ///
    /// Returns [`LspError::RequestFailed`] on server error.
    #[allow(deprecated)] // SymbolInformation is deprecated in newer LSP specs
    pub async fn workspace_symbols(
        &self,
        query: &str,
    ) -> Result<Option<Vec<lsp_types::SymbolInformation>>, LspError> {
        self.require_running()?;
        let response = {
            let mut server = self.server.lock().await;
            server
                .symbol(WorkspaceSymbolParams {
                    query: query.into(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await
                .map_err(|ref e| lsp_err("workspace/symbol", e))?
        };

        Ok(response.map(|resp| match resp {
            WorkspaceSymbolResponse::Flat(symbols) => symbols,
            WorkspaceSymbolResponse::Nested(_) => Vec::new(),
        }))
    }

    /// `textDocument/codeAction`.
    ///
    /// # Errors
    ///
    /// Returns [`LspError::RequestFailed`] on server error.
    pub async fn code_actions(
        &self,
        uri: &Url,
        range: Range,
    ) -> Result<Option<Vec<lsp_types::CodeActionOrCommand>>, LspError> {
        self.require_running()?;
        let mut server = self.server.lock().await;
        server
            .code_action(CodeActionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                range,
                context: CodeActionContext {
                    diagnostics: Vec::new(),
                    only: None,
                    trigger_kind: None,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await
            .map_err(|ref e| lsp_err("textDocument/codeAction", e))
    }

    /// `textDocument/formatting`.
    ///
    /// # Errors
    ///
    /// Returns [`LspError::RequestFailed`] on server error.
    pub async fn formatting(
        &self,
        uri: &Url,
    ) -> Result<Option<Vec<lsp_types::TextEdit>>, LspError> {
        self.require_running()?;
        let mut server = self.server.lock().await;
        server
            .formatting(DocumentFormattingParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                options: FormattingOptions {
                    tab_size: 4,
                    insert_spaces: true,
                    ..FormattingOptions::default()
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .map_err(|ref e| lsp_err("textDocument/formatting", e))
    }

    /// `textDocument/rename`.
    ///
    /// # Errors
    ///
    /// Returns [`LspError::RequestFailed`] on server error.
    pub async fn rename(
        &self,
        uri: &Url,
        line: u32,
        character: u32,
        new_name: &str,
    ) -> Result<Option<lsp_types::WorkspaceEdit>, LspError> {
        self.require_running()?;
        let mut server = self.server.lock().await;
        server
            .rename(RenameParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position::new(line, character),
                },
                new_name: new_name.into(),
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .map_err(|ref e| lsp_err("textDocument/rename", e))
    }

    /// `textDocument/signatureHelp`.
    ///
    /// # Errors
    ///
    /// Returns [`LspError::RequestFailed`] on server error.
    pub async fn signature_help(
        &self,
        uri: &Url,
        line: u32,
        character: u32,
    ) -> Result<Option<lsp_types::SignatureHelp>, LspError> {
        self.require_running()?;
        let mut server = self.server.lock().await;
        server
            .signature_help(SignatureHelpParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position::new(line, character),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                context: None,
            })
            .await
            .map_err(|ref e| lsp_err("textDocument/signatureHelp", e))
    }

    // ── Diagnostics ─────────────────────────────────────────────────────

    /// Return cached diagnostics for the given URI.
    ///
    /// Diagnostics are populated asynchronously by `publishDiagnostics`
    /// notifications from the server.
    #[must_use]
    pub fn diagnostics(&self, uri: &Url) -> Vec<lsp_types::Diagnostic> {
        self.diagnostics
            .read()
            .ok()
            .and_then(|cache| cache.get(uri).cloned())
            .unwrap_or_default()
    }

    /// Return cached diagnostics for all URIs.
    #[must_use]
    pub fn all_diagnostics(&self) -> HashMap<Url, Vec<lsp_types::Diagnostic>> {
        self.diagnostics
            .read()
            .ok()
            .map(|cache| cache.clone())
            .unwrap_or_default()
    }
}
