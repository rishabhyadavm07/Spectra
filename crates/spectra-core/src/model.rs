use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type Id = String;

pub fn new_id() -> Id {
    ulid::Ulid::new().to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Workspace {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub active_environment_id: Option<Id>,
    /// Auth every request in this workspace inherits by default (via
    /// `AuthConfig::InheritFromParent`), unless overridden by a folder or
    /// the request itself. The top of the inheritance chain — nothing above
    /// the workspace to fall back to.
    #[serde(default)]
    pub auth: AuthConfig,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Options,
    Head,
}

impl HttpMethod {
    pub fn as_reqwest(&self) -> reqwest::Method {
        match self {
            HttpMethod::Get => reqwest::Method::GET,
            HttpMethod::Post => reqwest::Method::POST,
            HttpMethod::Put => reqwest::Method::PUT,
            HttpMethod::Patch => reqwest::Method::PATCH,
            HttpMethod::Delete => reqwest::Method::DELETE,
            HttpMethod::Options => reqwest::Method::OPTIONS,
            HttpMethod::Head => reqwest::Method::HEAD,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct HeaderEntry {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ParamEntry {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind")]
pub enum RequestBody {
    None,
    Json { content: String },
    Text { content: String },
    Xml { content: String },
    FormUrlEncoded { fields: Vec<ParamEntry> },
}

impl Default for RequestBody {
    fn default() -> Self {
        RequestBody::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
#[serde(tag = "type")]
pub enum AuthConfig {
    /// Explicitly no auth — distinct from `InheritFromParent`: this
    /// deliberately sends no Authorization header even if a parent folder
    /// or the workspace has one configured.
    None,
    /// Use the nearest configured auth walking up folder -> ... -> folder
    /// -> workspace (falling back to `None` if nothing up the chain has one
    /// configured) — matches Postman's "inherit from parent" collection
    /// auth behavior. New requests default to this; `None` must be chosen
    /// explicitly to opt out of inheritance.
    InheritFromParent,
    Basic { username: String, password: String },
    Bearer { token: String },
    ApiKey { key: String, value: String, location: ApiKeyLocation },
    OAuth1 {
        consumer_key: String,
        consumer_secret: String,
        token: Option<String>,
        token_secret: Option<String>,
        signature_method: OAuth1SignatureMethod,
    },
    OAuth2 { grant: OAuth2Grant },
    AwsSigV4 {
        access_key: String,
        secret_key: String,
        region: String,
        service: String,
        session_token: Option<String>,
    },
    Digest { username: String, password: String },
    Hawk { id: String, key: String, algorithm: HawkAlgorithm },
}

impl Default for AuthConfig {
    fn default() -> Self {
        AuthConfig::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyLocation {
    Header,
    Query,
    Cookie,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OAuth1SignatureMethod {
    HmacSha1,
    HmacSha256,
    PlainText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HawkAlgorithm {
    Sha1,
    Sha256,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AddAuthDataTo {
    RequestHeaders,
    QueryParams,
}

impl Default for AddAuthDataTo {
    fn default() -> Self {
        AddAuthDataTo::RequestHeaders
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClientAuthentication {
    SendAsBasicAuthHeader,
    SendInBody,
}

impl Default for ClientAuthentication {
    fn default() -> Self {
        ClientAuthentication::SendAsBasicAuthHeader
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ParamTarget {
    Header,
    Body,
}

/// One extra key/value pair a user wants included on the token or refresh
/// request beyond the standard OAuth2 parameters — Postman's "Advanced"
/// section calls these Token Request / Refresh Request params.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
pub struct OAuth2ExtraParam {
    pub key: String,
    pub value: String,
    pub target: ParamTarget,
}

fn default_header_prefix() -> String {
    "Bearer".to_string()
}

fn default_auto_refresh() -> bool {
    true
}

/// Settings shared across every OAuth2 grant type — matches the fields
/// Postman's OAuth 2.0 auth panel exposes regardless of which grant is
/// selected (see AuthPanel.tsx for the corresponding UI).
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
pub struct OAuth2Options {
    #[serde(default)]
    pub add_to: AddAuthDataTo,
    #[serde(default = "default_header_prefix")]
    pub header_prefix: String,
    #[serde(default = "default_auto_refresh")]
    pub auto_refresh: bool,
    #[serde(default)]
    pub client_authentication: ClientAuthentication,
    #[serde(default)]
    pub token_request_params: Vec<OAuth2ExtraParam>,
    #[serde(default)]
    pub refresh_request_params: Vec<OAuth2ExtraParam>,
}

impl Default for OAuth2Options {
    fn default() -> Self {
        Self {
            add_to: AddAuthDataTo::default(),
            header_prefix: default_header_prefix(),
            auto_refresh: default_auto_refresh(),
            client_authentication: ClientAuthentication::default(),
            token_request_params: Vec::new(),
            refresh_request_params: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
#[serde(tag = "type")]
pub enum OAuth2Grant {
    AuthorizationCode {
        client_id: String,
        client_secret: Option<String>,
        auth_url: String,
        token_url: String,
        redirect_uri: String,
        scope: Option<String>,
        #[serde(default)]
        options: OAuth2Options,
    },
    AuthorizationCodePkce {
        client_id: String,
        auth_url: String,
        token_url: String,
        redirect_uri: String,
        scope: Option<String>,
        #[serde(default)]
        options: OAuth2Options,
    },
    ClientCredentials {
        client_id: String,
        client_secret: String,
        token_url: String,
        scope: Option<String>,
        #[serde(default)]
        options: OAuth2Options,
    },
    Password {
        client_id: String,
        client_secret: Option<String>,
        token_url: String,
        username: String,
        password: String,
        scope: Option<String>,
        #[serde(default)]
        options: OAuth2Options,
    },
    RefreshToken {
        client_id: String,
        client_secret: Option<String>,
        token_url: String,
        refresh_token: String,
        #[serde(default)]
        options: OAuth2Options,
    },
    DeviceCode {
        client_id: String,
        device_auth_url: String,
        token_url: String,
        scope: Option<String>,
        #[serde(default)]
        options: OAuth2Options,
    },
    Implicit {
        client_id: String,
        auth_url: String,
        redirect_uri: String,
        scope: Option<String>,
        #[serde(default)]
        options: OAuth2Options,
    },
}

impl OAuth2Grant {
    pub fn options(&self) -> &OAuth2Options {
        match self {
            OAuth2Grant::AuthorizationCode { options, .. }
            | OAuth2Grant::AuthorizationCodePkce { options, .. }
            | OAuth2Grant::ClientCredentials { options, .. }
            | OAuth2Grant::Password { options, .. }
            | OAuth2Grant::RefreshToken { options, .. }
            | OAuth2Grant::DeviceCode { options, .. }
            | OAuth2Grant::Implicit { options, .. } => options,
        }
    }

    /// `(client_id, client_secret, token_url)` for whichever grant this is,
    /// used to build a refresh-token request generically regardless of
    /// which grant type originally fetched the (now-expired) token. Device
    /// Code and Implicit have no token endpoint of their own to refresh
    /// against (device flow tokens aren't normally refreshable via this
    /// path, and Implicit never reaches token exchange at all) — those
    /// return `None`, so auto-refresh silently falls back to a fresh fetch.
    pub fn refresh_context(&self) -> Option<(&str, Option<&str>, &str)> {
        match self {
            OAuth2Grant::AuthorizationCode { client_id, client_secret, token_url, .. } => {
                Some((client_id, client_secret.as_deref(), token_url))
            }
            OAuth2Grant::AuthorizationCodePkce { client_id, token_url, .. } => Some((client_id, None, token_url)),
            OAuth2Grant::ClientCredentials { client_id, client_secret, token_url, .. } => {
                Some((client_id, Some(client_secret.as_str()), token_url))
            }
            OAuth2Grant::Password { client_id, client_secret, token_url, .. } => {
                Some((client_id, client_secret.as_deref(), token_url))
            }
            OAuth2Grant::RefreshToken { client_id, client_secret, token_url, .. } => {
                Some((client_id, client_secret.as_deref(), token_url))
            }
            OAuth2Grant::DeviceCode { .. } | OAuth2Grant::Implicit { .. } => None,
        }
    }
}

/// A fetched OAuth2 token, cached per-request in memory (AppContext-scoped).
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct OAuthToken {
    pub access_token: String,
    pub token_type: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl OAuthToken {
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            // 30s skew buffer so we don't hand back a token that expires mid-flight.
            Some(exp) => chrono::Utc::now() + chrono::Duration::seconds(30) >= exp,
            None => false,
        }
    }
}

/// A fetched token given a user-facing name, so a request can hold several
/// at once (e.g. "Prod Token" / "Staging Token") with one marked current —
/// matches Postman's "Current Token" picker. Memory-only, same as
/// `OAuthToken` itself: never persisted to disk, cleared on app restart.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct NamedOAuthToken {
    pub name: String,
    pub token: OAuthToken,
    pub saved_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "status")]
pub enum OAuthStatus {
    NotStarted,
    Pending { user_action: PendingUserAction },
    Complete { token: OAuthToken },
    Failed { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind")]
pub enum PendingUserAction {
    Browser { auth_url: String },
    DeviceCode { user_code: String, verification_url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub id: Id,
    pub workspace_id: Id,
    pub folder_id: Option<Id>,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<HeaderEntry>,
    #[serde(default)]
    pub params: Vec<ParamEntry>,
    #[serde(default)]
    pub body: RequestBody,
    #[serde(default)]
    pub auth: AuthConfig,
    /// Free-form documentation for this request, capped at 50 words (see
    /// `commands/request.rs::set_notes` for how the cap is enforced).
    /// `#[serde(default)]` so requests saved before this field existed
    /// deserialize cleanly as an empty string rather than failing to load —
    /// same pattern as `Workspace`/`Folder`'s `auth` field (see HANDOFF.md).
    #[serde(default)]
    pub notes: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RequestSummary {
    pub id: Id,
    pub folder_id: Option<Id>,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
}

impl From<&Request> for RequestSummary {
    fn from(r: &Request) -> Self {
        RequestSummary {
            id: r.id.clone(),
            folder_id: r.folder_id.clone(),
            name: r.name.clone(),
            method: r.method,
            url: r.url.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RequestRun {
    pub history_id: Id,
    pub response: ResponseDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ResponseDto {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub size_bytes: usize,
    pub duration_ms: u64,
}

/// A named response saved under a request for reference/documentation
/// purposes (e.g. "Compliant Response" / "Non-Compliant Response" examples).
/// Distinct from History: saved responses are curated and persist
/// indefinitely; history is an automatic, timestamped execution log.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SavedResponse {
    pub id: Id,
    pub workspace_id: Id,
    pub request_id: Id,
    pub name: String,
    pub response: ResponseDto,
    pub saved_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct HistoryEntry {
    pub id: Id,
    pub workspace_id: Id,
    pub request_id: Id,
    pub request_snapshot: Request,
    pub response: Option<ResponseDto>,
    pub error: Option<String>,
    pub executed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Folder {
    pub id: Id,
    pub workspace_id: Id,
    pub parent_folder_id: Option<Id>,
    pub name: String,
    /// Auth every request (and sub-folder) under this folder inherits by
    /// default, unless overridden further down the chain. `InheritFromParent`
    /// here means "use my own parent folder's auth, or the workspace's".
    #[serde(default)]
    pub auth: AuthConfig,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum VariableScope {
    Global,
    #[default]
    Workspace,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Environment {
    pub id: Id,
    pub workspace_id: Id,
    pub name: String,
    pub variables: HashMap<String, VariableValue>,
}

/// A variable is either a plain value stored inline in the environment's
/// JSON file, or a secret whose plaintext lives only in the macOS Keychain —
/// the JSON only ever holds the Keychain account reference, never the value
/// itself (PRD Section 21: never persist secrets as plaintext files).
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind")]
pub enum VariableValue {
    Plain { value: String },
    Secret { keychain_account: String },
}

impl VariableValue {
    pub fn is_secret(&self) -> bool {
        matches!(self, VariableValue::Secret { .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AppSettings {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_ssl_verification")]
    pub ssl_verification: bool,
    #[serde(default = "default_request_timeout_ms")]
    pub request_timeout_ms: u64,
}

fn default_theme() -> String { "light".to_string() }
fn default_ssl_verification() -> bool { true }
fn default_request_timeout_ms() -> u64 { 30000 }

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            ssl_verification: default_ssl_verification(),
            request_timeout_ms: default_request_timeout_ms(),
        }
    }
}

