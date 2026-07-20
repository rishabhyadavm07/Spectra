use crate::error::{ApiError, ApiResult};
use crate::model::{ClientAuthentication, OAuth2ExtraParam, OAuth2Grant, OAuth2Options, OAuthToken, ParamTarget};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default = "default_token_type")]
    token_type: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

fn default_token_type() -> String {
    "Bearer".to_string()
}

impl From<TokenResponse> for OAuthToken {
    fn from(r: TokenResponse) -> Self {
        OAuthToken {
            access_token: r.access_token,
            token_type: r.token_type,
            refresh_token: r.refresh_token,
            expires_at: r.expires_in.map(|s| chrono::Utc::now() + chrono::Duration::seconds(s)),
        }
    }
}

/// Posts a token/refresh request, honoring `options.client_authentication`
/// (HTTP Basic auth header vs. `client_id`/`client_secret` in the form body
/// — some authorization servers, notably Azure AD/Entra ID as seen in the
/// Databricks OAuth2 setup, reject one or the other) and appending any
/// user-configured advanced extra params to the form body or as extra
/// headers per their `target`.
async fn post_token_request(
    http: &reqwest::Client,
    token_url: &str,
    client_id: &str,
    client_secret: Option<&str>,
    mut params: Vec<(&str, &str)>,
    options: &OAuth2Options,
    extra_params: &[OAuth2ExtraParam],
) -> ApiResult<OAuthToken> {
    let mut req = http.post(token_url).header("Accept", "application/json");

    match (options.client_authentication, client_secret) {
        (ClientAuthentication::SendAsBasicAuthHeader, Some(secret)) => {
            req = req.basic_auth(client_id, Some(secret));
        }
        (ClientAuthentication::SendAsBasicAuthHeader, None) => {
            // No secret to authenticate with (e.g. a public client) — fall
            // back to sending client_id in the body, same as SendInBody.
            params.push(("client_id", client_id));
        }
        (ClientAuthentication::SendInBody, _) => {
            params.push(("client_id", client_id));
            if let Some(secret) = client_secret {
                params.push(("client_secret", secret));
            }
        }
    }

    for extra in extra_params {
        match extra.target {
            ParamTarget::Body => params.push((extra.key.as_str(), extra.value.as_str())),
            ParamTarget::Header => req = req.header(&extra.key, &extra.value),
        }
    }

    let resp = req.form(&params).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(ApiError::Auth(format!("token endpoint returned {status}: {body}")));
    }

    let parsed: TokenResponse = resp
        .json()
        .await
        .map_err(|e| ApiError::Auth(format!("invalid token response: {e}")))?;
    Ok(parsed.into())
}

/// Fetches a token for grants that require no browser/device interaction.
/// Returns None for interactive grants (caller must use the oauth_flow module).
pub async fn fetch_token_noninteractive(
    http: &reqwest::Client,
    grant: &OAuth2Grant,
) -> ApiResult<Option<OAuthToken>> {
    match grant {
        OAuth2Grant::ClientCredentials { client_id, client_secret, token_url, scope, options } => {
            let mut params = vec![("grant_type", "client_credentials")];
            if let Some(s) = scope {
                params.push(("scope", s.as_str()));
            }
            Ok(Some(
                post_token_request(
                    http,
                    token_url,
                    client_id,
                    Some(client_secret.as_str()),
                    params,
                    options,
                    &options.token_request_params,
                )
                .await?,
            ))
        }
        OAuth2Grant::Password { client_id, client_secret, token_url, username, password, scope, options } => {
            let mut params = vec![("grant_type", "password"), ("username", username.as_str()), ("password", password.as_str())];
            if let Some(s) = scope {
                params.push(("scope", s.as_str()));
            }
            Ok(Some(
                post_token_request(
                    http,
                    token_url,
                    client_id,
                    client_secret.as_deref(),
                    params,
                    options,
                    &options.token_request_params,
                )
                .await?,
            ))
        }
        OAuth2Grant::RefreshToken { client_id, client_secret, token_url, refresh_token, options } => {
            let params = vec![("grant_type", "refresh_token"), ("refresh_token", refresh_token.as_str())];
            Ok(Some(
                post_token_request(
                    http,
                    token_url,
                    client_id,
                    client_secret.as_deref(),
                    params,
                    options,
                    &options.refresh_request_params,
                )
                .await?,
            ))
        }
        OAuth2Grant::AuthorizationCode { .. }
        | OAuth2Grant::AuthorizationCodePkce { .. }
        | OAuth2Grant::DeviceCode { .. }
        | OAuth2Grant::Implicit { .. } => Ok(None),
    }
}

/// Refreshes an expired token using `refresh_token`, generically across
/// whichever grant originally fetched it (see `OAuth2Grant::refresh_context`
/// for which grants support this). Returns `None` when the grant has no
/// token endpoint to refresh against, so callers fall back to a fresh fetch
/// via `fetch_token_noninteractive` instead of erroring.
pub async fn refresh_grant_token(
    http: &reqwest::Client,
    grant: &OAuth2Grant,
    refresh_token: &str,
) -> ApiResult<Option<OAuthToken>> {
    let Some((client_id, client_secret, token_url)) = grant.refresh_context() else {
        return Ok(None);
    };
    let params = vec![("grant_type", "refresh_token"), ("refresh_token", refresh_token)];
    let options = grant.options();
    Ok(Some(
        post_token_request(http, token_url, client_id, client_secret, params, options, &options.refresh_request_params)
            .await?,
    ))
}

/// Exchanges an authorization code for a token (used by the loopback callback flow).
#[allow(clippy::too_many_arguments)]
pub async fn exchange_authorization_code(
    http: &reqwest::Client,
    token_url: &str,
    client_id: &str,
    client_secret: Option<&str>,
    code: &str,
    redirect_uri: &str,
    code_verifier: Option<&str>,
    options: &OAuth2Options,
) -> ApiResult<OAuthToken> {
    let mut params = vec![("grant_type", "authorization_code"), ("code", code), ("redirect_uri", redirect_uri)];
    if let Some(verifier) = code_verifier {
        params.push(("code_verifier", verifier));
    }
    post_token_request(http, token_url, client_id, client_secret, params, options, &options.token_request_params).await
}

#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: Option<String>,
    #[serde(alias = "verification_uri_complete")]
    pub verification_uri_complete: Option<String>,
    #[serde(default = "default_interval")]
    pub interval: u64,
}

fn default_interval() -> u64 {
    5
}

pub async fn start_device_code(
    http: &reqwest::Client,
    device_auth_url: &str,
    client_id: &str,
    scope: Option<&str>,
) -> ApiResult<DeviceCodeResponse> {
    let mut params: HashMap<&str, &str> = HashMap::new();
    params.insert("client_id", client_id);
    if let Some(s) = scope {
        params.insert("scope", s);
    }
    let resp = http
        .post(device_auth_url)
        .header("Accept", "application/json")
        .form(&params)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(ApiError::Auth(format!("device auth endpoint returned {status}: {body}")));
    }
    Ok(resp.json().await.map_err(|e| ApiError::Auth(e.to_string()))?)
}

/// Polls the token endpoint until the user completes the device-code flow,
/// the flow expires, or `deadline` elapses.
pub async fn poll_device_code(
    http: &reqwest::Client,
    token_url: &str,
    client_id: &str,
    device_code: &str,
    interval_secs: u64,
    timeout: std::time::Duration,
) -> ApiResult<OAuthToken> {
    let start = std::time::Instant::now();
    let mut interval = interval_secs.max(1);
    loop {
        if start.elapsed() > timeout {
            return Err(ApiError::Auth("device code flow timed out".into()));
        }
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;

        let params = [
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("client_id", client_id),
            ("device_code", device_code),
        ];
        let resp = http
            .post(token_url)
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await?;

        if resp.status().is_success() {
            let parsed: TokenResponse = resp.json().await.map_err(|e| ApiError::Auth(e.to_string()))?;
            return Ok(parsed.into());
        }

        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        match body.get("error").and_then(|e| e.as_str()) {
            Some("authorization_pending") => continue,
            Some("slow_down") => {
                interval += 5;
                continue;
            }
            Some(other) => return Err(ApiError::Auth(format!("device code flow failed: {other}"))),
            None => return Err(ApiError::Auth("device code flow failed: unknown error".into())),
        }
    }
}
