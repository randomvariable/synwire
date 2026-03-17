//! DAP plugin for the Synwire agent runtime.
//!
//! Integrates debug adapter management with the agent lifecycle,
//! contributing DAP tools and signal routes for debug events.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use synwire_core::agents::plugin::Plugin;
use synwire_core::agents::signal::{Action, SignalKind, SignalRoute};
use synwire_core::tools::Tool;

use crate::client::{DapClient, DapSessionState};
use crate::config::{DapAdapterConfig, DapPluginConfig};
use crate::error::DapError;
use crate::registry::DebugAdapterRegistry;
use crate::tools::create_tools;
use crate::transport::EventHandler;

/// Plugin providing Debug Adapter Protocol integration for agents.
///
/// Manages debug adapter client lifecycles, registers DAP tools into
/// the agent's tool registry, and routes debug events (stopped, terminated)
/// as signals.
pub struct DapPlugin {
    clients: Arc<RwLock<HashMap<String, Arc<DapClient>>>>,
    registry: Arc<DebugAdapterRegistry>,
    config: DapPluginConfig,
    tools: Vec<Arc<dyn Tool>>,
}

impl DapPlugin {
    /// Create a new DAP plugin with the given configuration.
    ///
    /// The plugin starts with no active clients. Use [`start_adapter`](Self::start_adapter)
    /// to spawn debug adapter processes.
    #[must_use]
    pub fn new(config: DapPluginConfig) -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            registry: Arc::new(DebugAdapterRegistry::with_builtins()),
            config,
            tools: Vec::new(),
        }
    }

    /// Create a plugin with a custom adapter registry.
    #[must_use]
    pub fn with_registry(config: DapPluginConfig, registry: DebugAdapterRegistry) -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            registry: Arc::new(registry),
            config,
            tools: Vec::new(),
        }
    }

    /// Start a debug adapter and register its tools.
    ///
    /// The adapter is identified by `session_id`. If a session with the same
    /// ID already exists, it is disconnected first.
    ///
    /// # Errors
    ///
    /// Returns [`DapError`] if the adapter fails to spawn or initialize.
    pub async fn start_adapter(
        &mut self,
        session_id: &str,
        adapter_config: &DapAdapterConfig,
    ) -> Result<Arc<DapClient>, DapError> {
        // Disconnect existing session if present.
        {
            let clients = self.clients.read().await;
            if let Some(existing) = clients.get(session_id) {
                let _ = existing.disconnect().await;
            }
        }

        let clients_ref = Arc::clone(&self.clients);
        let session_id_owned = session_id.to_string();

        // Create event handler that updates client state on debug events.
        let event_handler: EventHandler = Arc::new(move |event: serde_json::Value| {
            let event_name = event
                .get("event")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string();

            let clients_ref = Arc::clone(&clients_ref);
            let session_id_owned = session_id_owned.clone();

            // Spawn a task to handle state updates asynchronously.
            drop(tokio::spawn(async move {
                let clients = clients_ref.read().await;
                if let Some(client) = clients.get(&session_id_owned) {
                    match event_name.as_str() {
                        "stopped" => {
                            client.set_status(DapSessionState::Stopped).await;
                            tracing::debug!(
                                session_id = %session_id_owned,
                                "Debug session stopped (breakpoint/step)"
                            );
                        }
                        "terminated" => {
                            client.set_status(DapSessionState::Terminated).await;
                            tracing::debug!(
                                session_id = %session_id_owned,
                                "Debug session terminated"
                            );
                        }
                        "continued" => {
                            client.set_status(DapSessionState::Running).await;
                        }
                        _ => {}
                    }
                }
            }));
        });

        let client = DapClient::start(adapter_config, event_handler)?;
        client.initialize().await?;

        let client = Arc::new(client);

        // Register client.
        {
            let mut clients = self.clients.write().await;
            let _ = clients.insert(session_id.to_string(), Arc::clone(&client));
        }

        // Create tools for this client.
        let new_tools = create_tools(Arc::clone(&client))
            .map_err(|e| DapError::InitializationFailed(format!("failed to create tools: {e}")))?;
        self.tools = new_tools;

        tracing::debug!(session_id, "DAP adapter started and initialized");

        Ok(client)
    }

    /// Get a client by session ID.
    pub async fn client(&self, session_id: &str) -> Option<Arc<DapClient>> {
        let clients = self.clients.read().await;
        clients.get(session_id).cloned()
    }

    /// Disconnect all active sessions (called on plugin shutdown).
    pub async fn disconnect_all(&self) {
        let clients = self.clients.read().await;
        for (session_id, client) in clients.iter() {
            if let Err(e) = client.disconnect().await {
                tracing::warn!(
                    session_id,
                    error = %e,
                    "Failed to disconnect DAP session"
                );
            }
        }
    }

    /// Access the adapter registry.
    #[must_use]
    pub fn registry(&self) -> &DebugAdapterRegistry {
        &self.registry
    }

    /// Access the plugin configuration.
    #[must_use]
    pub const fn config(&self) -> &DapPluginConfig {
        &self.config
    }
}

impl Plugin for DapPlugin {
    #[allow(clippy::unnecessary_literal_bound)] // Trait signature requires `&str`.
    fn name(&self) -> &str {
        "dap"
    }

    fn tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.clone()
    }

    fn signal_routes(&self) -> Vec<SignalRoute> {
        vec![
            SignalRoute::new(
                SignalKind::Custom("dap_stopped".into()),
                Action::Continue,
                0,
            ),
            SignalRoute::new(
                SignalKind::Custom("dap_terminated".into()),
                Action::Continue,
                0,
            ),
        ]
    }
}
