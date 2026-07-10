use crate::error::{ApiError, ApiResult};
use crate::model::{NamedOAuthToken, OAuth2Grant, OAuth2Options, OAuthStatus, OAuthToken, PendingUserAction};
use crate::oauth2;
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

/// Per-request named-token state: every token fetched for this request so
/// far, plus which one (by name) is "current" — the one `cached_token`
/// returns and the one auth signing actually uses.
#[derive(Default, Clone)]
struct TokenSlots {
    tokens: Vec<NamedOAuthToken>,
    current_name: Option<String>,
}

/// In-memory OAuth flow + token state, keyed by request_id. Lives in
/// AppContext; deliberately never persisted to disk (tokens are secrets).
///
/// Two things are tracked per request: the live flow `status` (what the GUI
/// polls via get_oauth_status to show "Waiting for authorization…" etc.),
/// and a `TokenSlots` list of every named token fetched for that request —
/// matching Postman's "Current Token" picker, where a request can hold
/// several previously-fetched tokens (e.g. "Prod Token" / "Staging Token")
/// and pick which one to actually send.
#[derive(Default)]
pub struct OAuthStore {
    statuses: RwLock<HashMap<String, OAuthStatus>>,
    slots: RwLock<HashMap<String, TokenSlots>>,
}

impl OAuthStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn get(&self, request_id: &str) -> OAuthStatus {
        self.statuses
            .read()
            .await
            .get(request_id)
            .cloned()
            .unwrap_or(OAuthStatus::NotStarted)
    }

    pub async fn set(&self, request_id: &str, status: OAuthStatus) {
        self.statuses.write().await.insert(request_id.to_string(), status);
    }

    /// Records a newly-fetched token under `name` (defaulting to a
    /// timestamp-based name if none given, matching Postman's default) and
    /// makes it the current token for this request. Also updates the flow
    /// `status` to `Complete` so existing get_oauth_status polling keeps
    /// working unchanged.
    pub async fn save_token(&self, request_id: &str, name: Option<String>, token: OAuthToken) {
        let name = name.unwrap_or_else(|| format!("Token {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")));
        self.set(request_id, OAuthStatus::Complete { token: token.clone() }).await;

        let mut slots = self.slots.write().await;
        let entry = slots.entry(request_id.to_string()).or_default();
        entry.tokens.retain(|t| t.name != name);
        entry.tokens.push(NamedOAuthToken { name: name.clone(), token, saved_at: chrono::Utc::now() });
        entry.current_name = Some(name);
    }

    /// Every named token fetched so far for this request, newest first.
    pub async fn list_tokens(&self, request_id: &str) -> Vec<NamedOAuthToken> {
        let mut tokens = self.slots.read().await.get(request_id).map(|s| s.tokens.clone()).unwrap_or_default();
        tokens.sort_by(|a, b| b.saved_at.cmp(&a.saved_at));
        tokens
    }

    /// Marks an already-fetched token as current without re-running the
    /// flow — lets the user switch back to a previously-saved token.
    pub async fn select_token(&self, request_id: &str, name: &str) -> ApiResult<()> {
        let mut slots = self.slots.write().await;
        let entry = slots.get_mut(request_id).ok_or_else(|| ApiError::NotFound { entity: "oauth token", id: name.to_string() })?;
        if !entry.tokens.iter().any(|t| t.name == name) {
            return Err(ApiError::NotFound { entity: "oauth token", id: name.to_string() });
        }
        entry.current_name = Some(name.to_string());
        Ok(())
    }

    pub async fn delete_token(&self, request_id: &str, name: &str) {
        let mut slots = self.slots.write().await;
        if let Some(entry) = slots.get_mut(request_id) {
            entry.tokens.retain(|t| t.name != name);
            if entry.current_name.as_deref() == Some(name) {
                entry.current_name = entry.tokens.first().map(|t| t.name.clone());
            }
        }
    }

    /// The current token for this request (the one auth signing/sending
    /// actually uses), if any and not expired.
    pub async fn cached_token(&self, request_id: &str) -> Option<OAuthToken> {
        let slots = self.slots.read().await;
        let entry = slots.get(request_id)?;
        let current_name = entry.current_name.as_ref()?;
        let named = entry.tokens.iter().find(|t| &t.name == current_name)?;
        if named.token.is_expired() {
            None
        } else {
            Some(named.token.clone())
        }
    }

    /// The current token's `refresh_token`, but only when the token itself
    /// has expired — used to decide whether an auto-refresh attempt applies
    /// (a still-valid token is returned by `cached_token` instead; nothing
    /// to refresh if there was never a token or it didn't come with one).
    pub async fn expired_refresh_token(&self, request_id: &str) -> Option<String> {
        let slots = self.slots.read().await;
        let entry = slots.get(request_id)?;
        let current_name = entry.current_name.as_ref()?;
        let named = entry.tokens.iter().find(|t| &t.name == current_name)?;
        if named.token.is_expired() {
            named.token.refresh_token.clone()
        } else {
            None
        }
    }
}

fn generate_pkce_pair() -> (String, String) {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

fn generate_state() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Starts the interactive OAuth2 flow for a request's configured grant.
/// For AuthorizationCode/Pkce: opens the system browser and spins up a
/// loopback server on the redirect_uri's port to catch the callback.
/// For DeviceCode: hits the device authorization endpoint and returns the
/// user code immediately, then polls the token endpoint in the background.
/// Updates `store` as the flow progresses; caller polls via get_oauth_status.
pub async fn start_flow(
    http: reqwest::Client,
    store: Arc<OAuthStore>,
    request_id: String,
    grant: OAuth2Grant,
) -> ApiResult<PendingUserAction> {
    match grant.clone() {
        OAuth2Grant::AuthorizationCode { client_id, client_secret, auth_url, token_url, redirect_uri, scope, options } => {
            let state = generate_state();
            let mut url = reqwest::Url::parse(&auth_url).map_err(|e| ApiError::Validation(e.to_string()))?;
            {
                let mut q = url.query_pairs_mut();
                q.append_pair("response_type", "code");
                q.append_pair("client_id", &client_id);
                q.append_pair("redirect_uri", &redirect_uri);
                q.append_pair("state", &state);
                if let Some(s) = &scope {
                    q.append_pair("scope", s);
                }
            }
            let action = PendingUserAction::Browser { auth_url: url.to_string() };
            store.set(&request_id, OAuthStatus::Pending { user_action: action.clone() }).await;

            let _ = open::that(url.to_string());

            tokio::spawn(run_loopback_and_exchange(
                http,
                store,
                request_id,
                redirect_uri,
                state,
                token_url,
                client_id,
                client_secret,
                None,
                options,
            ));

            Ok(action)
        }
        OAuth2Grant::AuthorizationCodePkce { client_id, auth_url, token_url, redirect_uri, scope, options } => {
            let state = generate_state();
            let (verifier, challenge) = generate_pkce_pair();
            let mut url = reqwest::Url::parse(&auth_url).map_err(|e| ApiError::Validation(e.to_string()))?;
            {
                let mut q = url.query_pairs_mut();
                q.append_pair("response_type", "code");
                q.append_pair("client_id", &client_id);
                q.append_pair("redirect_uri", &redirect_uri);
                q.append_pair("state", &state);
                q.append_pair("code_challenge", &challenge);
                q.append_pair("code_challenge_method", "S256");
                if let Some(s) = &scope {
                    q.append_pair("scope", s);
                }
            }
            let action = PendingUserAction::Browser { auth_url: url.to_string() };
            store.set(&request_id, OAuthStatus::Pending { user_action: action.clone() }).await;

            let _ = open::that(url.to_string());

            tokio::spawn(run_loopback_and_exchange(
                http,
                store,
                request_id,
                redirect_uri,
                state,
                token_url,
                client_id,
                None,
                Some(verifier),
                options,
            ));

            Ok(action)
        }
        OAuth2Grant::DeviceCode { client_id, device_auth_url, token_url, scope, .. } => {
            let dc = oauth2::start_device_code(&http, &device_auth_url, &client_id, scope.as_deref()).await?;
            let verification_url = dc
                .verification_uri_complete
                .or(dc.verification_uri)
                .unwrap_or_default();
            let action = PendingUserAction::DeviceCode {
                user_code: dc.user_code.clone(),
                verification_url: verification_url.clone(),
            };
            store.set(&request_id, OAuthStatus::Pending { user_action: action.clone() }).await;

            let _ = open::that(&verification_url);

            tokio::spawn(async move {
                let result = oauth2::poll_device_code(
                    &http,
                    &token_url,
                    &client_id,
                    &dc.device_code,
                    dc.interval,
                    std::time::Duration::from_secs(600),
                )
                .await;
                match result {
                    Ok(token) => store.save_token(&request_id, None, token).await,
                    Err(e) => store.set(&request_id, OAuthStatus::Failed { error: e.to_string() }).await,
                }
            });

            Ok(action)
        }
        OAuth2Grant::Implicit { .. } => Err(ApiError::Unsupported(
            "Implicit grant returns tokens directly in the URL fragment, which a headless \
             loopback server cannot observe; use Authorization Code + PKCE instead."
                .into(),
        )),
        OAuth2Grant::ClientCredentials { .. } | OAuth2Grant::Password { .. } | OAuth2Grant::RefreshToken { .. } => {
            Err(ApiError::Unsupported(
                "this grant does not require interactive flow; call send_request directly".into(),
            ))
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_loopback_and_exchange(
    http: reqwest::Client,
    store: Arc<OAuthStore>,
    request_id: String,
    redirect_uri: String,
    expected_state: String,
    token_url: String,
    client_id: String,
    client_secret: Option<String>,
    code_verifier: Option<String>,
    options: OAuth2Options,
) {
    let result = catch_callback_and_exchange(
        &http,
        &redirect_uri,
        &expected_state,
        &token_url,
        &client_id,
        client_secret.as_deref(),
        code_verifier.as_deref(),
        &options,
    )
    .await;

    match result {
        Ok(token) => store.save_token(&request_id, None, token).await,
        Err(e) => store.set(&request_id, OAuthStatus::Failed { error: e.to_string() }).await,
    }
}

#[allow(clippy::too_many_arguments)]
async fn catch_callback_and_exchange(
    http: &reqwest::Client,
    redirect_uri: &str,
    expected_state: &str,
    token_url: &str,
    client_id: &str,
    client_secret: Option<&str>,
    code_verifier: Option<&str>,
    options: &OAuth2Options,
) -> ApiResult<OAuthToken> {
    let url = reqwest::Url::parse(redirect_uri).map_err(|e| ApiError::Validation(e.to_string()))?;
    let port = url.port().unwrap_or(80);
    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|e| ApiError::IoError(format!("failed to bind loopback callback on port {port}: {e}")))?;

    let (mut stream, _) = tokio::time::timeout(std::time::Duration::from_secs(300), listener.accept())
        .await
        .map_err(|_| ApiError::Auth("timed out waiting for OAuth browser redirect".into()))?
        .map_err(|e| ApiError::IoError(e.to_string()))?;

    let mut buf = vec![0u8; 8192];
    let n = stream.read(&mut buf).await.map_err(|e| ApiError::IoError(e.to_string()))?;
    let request_line = String::from_utf8_lossy(&buf[..n]);
    let first_line = request_line.lines().next().unwrap_or("");
    let path = first_line.split_whitespace().nth(1).unwrap_or("/");

    let callback_url = reqwest::Url::parse(&format!("http://127.0.0.1:{port}{path}"))
        .map_err(|e| ApiError::Validation(e.to_string()))?;
    let params: HashMap<String, String> = callback_url.query_pairs().into_owned().collect();

    let body = "<html><body>Spectra: authentication complete, you can close this tab.</body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;

    if let Some(err) = params.get("error") {
        return Err(ApiError::Auth(format!("OAuth authorization failed: {err}")));
    }
    let state = params.get("state").ok_or_else(|| ApiError::Auth("missing state in callback".into()))?;
    if state != expected_state {
        return Err(ApiError::Auth("OAuth state mismatch; possible CSRF".into()));
    }
    let code = params.get("code").ok_or_else(|| ApiError::Auth("missing authorization code in callback".into()))?;

    oauth2::exchange_authorization_code(
        http,
        token_url,
        client_id,
        client_secret,
        code,
        redirect_uri,
        code_verifier,
        options,
    )
    .await
}
