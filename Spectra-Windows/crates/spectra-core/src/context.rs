use crate::cookies::ClearableCookieStore;
use crate::oauth_flow::OAuthStore;
use crate::secrets::SecretStore;
use crate::storage::Storage;
use std::sync::Arc;

/// Shared context every spectra-api command operates against. Constructed
/// once at startup by spectra-tauri and (later) spectra-mcp — this is the
/// mechanism that guarantees GUI and MCP call the exact same backend state.
#[derive(Clone)]
pub struct AppContext {
    pub storage: Arc<Storage>,
    pub cookie_store: ClearableCookieStore,
    pub http: reqwest::Client,
    pub oauth_store: Arc<OAuthStore>,
    pub secrets: Arc<dyn SecretStore>,
}

impl AppContext {
    pub fn new(storage: Storage, cookie_store: ClearableCookieStore, http: reqwest::Client, secrets: Arc<dyn SecretStore>) -> Self {
        Self {
            storage: Arc::new(storage),
            cookie_store,
            http,
            oauth_store: Arc::new(OAuthStore::new()),
            secrets,
        }
    }
}
