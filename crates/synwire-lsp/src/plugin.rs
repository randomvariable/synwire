//! LSP plugin implementing the synwire agent `Plugin` trait.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use synwire_core::agents::plugin::Plugin;
use synwire_core::agents::signal::{Action, SignalKind, SignalRoute};
use synwire_core::tools::Tool;

use crate::client::LspClient;
use crate::config::LspPluginConfig;
use crate::registry::LanguageServerRegistry;
use crate::tools::lsp_tools;

/// Plugin that integrates LSP language servers into a synwire agent.
///
/// Manages multiple [`LspClient`] instances (one per language) and
/// aggregates their tools into the agent's tool registry.
pub struct LspPlugin {
    clients: Arc<RwLock<HashMap<String, Arc<LspClient>>>>,
    registry: Arc<LanguageServerRegistry>,
    config: LspPluginConfig,
}

impl std::fmt::Debug for LspPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let client_count = self.clients.read().map(|c| c.len()).unwrap_or(0);
        f.debug_struct("LspPlugin")
            .field("clients", &client_count)
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl LspPlugin {
    /// Create a new LSP plugin with the given configuration.
    #[must_use]
    pub fn new(config: LspPluginConfig) -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            registry: Arc::new(LanguageServerRegistry::default_registry()),
            config,
        }
    }

    /// Create a new LSP plugin with default configuration.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(LspPluginConfig::default())
    }

    /// Create a new LSP plugin with a custom server registry.
    #[must_use]
    pub fn with_registry(config: LspPluginConfig, registry: LanguageServerRegistry) -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            registry: Arc::new(registry),
            config,
        }
    }

    /// Register a running [`LspClient`] under a language identifier.
    ///
    /// The client should already be initialized.
    pub fn register_client(&self, language_id: &str, client: Arc<LspClient>) {
        if let Ok(mut clients) = self.clients.write() {
            let _prev = clients.insert(language_id.into(), client);
        }
    }

    /// Retrieve a client by language identifier.
    #[must_use]
    pub fn client(&self, language_id: &str) -> Option<Arc<LspClient>> {
        self.clients
            .read()
            .ok()
            .and_then(|c| c.get(language_id).cloned())
    }

    /// Reference to the language server registry.
    #[must_use]
    pub fn registry(&self) -> &LanguageServerRegistry {
        &self.registry
    }

    /// Reference to the plugin configuration.
    #[must_use]
    pub const fn config(&self) -> &LspPluginConfig {
        &self.config
    }
}

impl Default for LspPlugin {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[allow(clippy::unnecessary_literal_bound)] // constrained by Plugin trait signature
impl Plugin for LspPlugin {
    fn name(&self) -> &str {
        "lsp"
    }

    fn tools(&self) -> Vec<Arc<dyn Tool>> {
        let Ok(clients) = self.clients.read() else {
            return Vec::new();
        };

        let mut all_tools: Vec<Arc<dyn Tool>> = Vec::new();
        for client in clients.values() {
            let client_tools = lsp_tools(Arc::clone(client));
            all_tools.extend(client_tools.into_iter().map(Arc::from));
        }
        drop(clients);
        all_tools
    }

    fn signal_routes(&self) -> Vec<SignalRoute> {
        vec![
            SignalRoute::new(
                SignalKind::Custom("lsp_diagnostics_changed".into()),
                Action::Continue,
                0,
            ),
            SignalRoute::new(
                SignalKind::Custom("lsp_server_crashed".into()),
                Action::Continue,
                0,
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_name() {
        let plugin = LspPlugin::with_defaults();
        assert_eq!(plugin.name(), "lsp");
    }

    #[test]
    fn tools_empty_when_no_clients() {
        let plugin = LspPlugin::with_defaults();
        assert!(plugin.tools().is_empty());
    }

    #[test]
    fn signal_routes_present() {
        let plugin = LspPlugin::with_defaults();
        let routes = plugin.signal_routes();
        assert_eq!(routes.len(), 2);
    }

    #[test]
    fn default_config() {
        let plugin = LspPlugin::default();
        assert!(plugin.config().auto_start);
        assert!(plugin.config().auto_shutdown);
    }
}
