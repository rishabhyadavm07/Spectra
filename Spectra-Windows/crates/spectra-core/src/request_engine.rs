use crate::auth_signing::{self, AwsSigV4Params, HawkParams, OAuth1Params};
use crate::error::{ApiError, ApiResult};
use crate::model::{ApiKeyLocation, AuthConfig, HawkAlgorithm, OAuth1SignatureMethod, Request, RequestBody, ResponseDto};
use crate::oauth2;
use crate::oauth_flow::OAuthStore;
use crate::variables::{find_unresolved, resolve_string};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Builds and executes the outgoing HTTP call for a stored Request.
/// This is the single implementation both the GUI (via spectra-tauri) and
/// MCP (via spectra-mcp) drive through spectra-api::commands::request::send_request.
pub async fn execute(
    http: &reqwest::Client,
    oauth_store: &Arc<OAuthStore>,
    request: &Request,
    vars: &HashMap<String, String>,
) -> ApiResult<ResponseDto> {
    // Fail fast with a clear message naming the missing variable(s), rather
    // than letting an unsubstituted `{{...}}` reach URL parsing / reqwest and
    // surface as an opaque "invalid URL" / network error.
    let unresolved = find_unresolved(&request.url, vars);
    if !unresolved.is_empty() {
        return Err(ApiError::Validation(format!(
            "unresolved variable(s) in URL: {}",
            unresolved.join(", ")
        )));
    }

    let url = resolve_string(&request.url, vars);

    let query: Vec<(String, String)> = request
        .params
        .iter()
        .filter(|p| p.enabled)
        .map(|p| (resolve_string(&p.key, vars), resolve_string(&p.value, vars)))
        .collect();

    let resolved_headers: Vec<(String, String)> = request
        .headers
        .iter()
        .filter(|h| h.enabled)
        .map(|h| (resolve_string(&h.key, vars), resolve_string(&h.value, vars)))
        .collect();

    let body_bytes: Vec<u8> = match &request.body {
        RequestBody::None => Vec::new(),
        RequestBody::Json { content } | RequestBody::Text { content } | RequestBody::Xml { content } => {
            resolve_string(content, vars).into_bytes()
        }
        RequestBody::FormUrlEncoded { fields } => {
            let form: Vec<(String, String)> = fields
                .iter()
                .filter(|f| f.enabled)
                .map(|f| (resolve_string(&f.key, vars), resolve_string(&f.value, vars)))
                .collect();
            urlencoded_form(&form).into_bytes()
        }
    };

    let auth = resolve_auth(http, oauth_store, &request.id, &request.auth, vars).await?;

    let start = Instant::now();
    let resp = send_once(http, request, &url, &query, &resolved_headers, &body_bytes, &auth).await?;

    // Digest is the one scheme that needs a real round trip: send unauthenticated,
    // read the challenge off a 401, then retry once with the computed response header.
    let resp = if let ResolvedAuth::Digest { username, password } = &auth {
        if resp.status().as_u16() == 401 {
            if let Some(challenge) = resp
                .headers()
                .get("www-authenticate")
                .and_then(|v| v.to_str().ok())
                .and_then(auth_signing::parse_digest_challenge)
            {
                let path = reqwest::Url::parse(&url).map(|u| u.path().to_string()).unwrap_or_else(|_| "/".into());
                let header = auth_signing::digest_header(
                    username,
                    password,
                    request.method.as_reqwest().as_str(),
                    &path,
                    &challenge,
                );
                send_once(
                    http,
                    request,
                    &url,
                    &query,
                    &resolved_headers,
                    &body_bytes,
                    &ResolvedAuth::Authorization(header),
                )
                .await?
            } else {
                resp
            }
        } else {
            resp
        }
    } else {
        resp
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    let status = resp.status().as_u16();
    let status_text = resp.status().canonical_reason().unwrap_or("").to_string();
    let headers: HashMap<String, String> = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let bytes = resp.bytes().await?;
    let size_bytes = bytes.len();
    let body = String::from_utf8_lossy(&bytes).to_string();

    Ok(ResponseDto { status, status_text, headers, body, size_bytes, duration_ms })
}

/// Computes the headers Spectra will actually send on the wire — auth header,
/// content-type, resolved variable substitutions — without performing the
/// HTTP call. Used by the GUI/MCP to show "auto-generated" headers the way
/// Postman surfaces them, so nothing is invisible to the user.
///
/// OAuth2 is special-cased: fetching a fresh token is a side-effecting network
/// call, so the preview only reflects a token already cached from a prior
/// send/authorize; otherwise it reports the header as pending.
pub async fn preview_headers(
    oauth_store: &Arc<OAuthStore>,
    request: &Request,
    vars: &HashMap<String, String>,
) -> ApiResult<Vec<(String, String)>> {
    let url = resolve_string(&request.url, vars);
    let query: Vec<(String, String)> = request
        .params
        .iter()
        .filter(|p| p.enabled)
        .map(|p| (resolve_string(&p.key, vars), resolve_string(&p.value, vars)))
        .collect();

    // Computed before the OAuth2/Digest early returns below so a pending
    // token or not-yet-challenged Digest auth doesn't hide the request's
    // own custom headers from the preview — only the Authorization header
    // itself is genuinely unknown at this point for those two schemes.
    let custom_headers: Vec<(String, String)> = request
        .headers
        .iter()
        .filter(|h| h.enabled)
        .map(|h| (resolve_string(&h.key, vars), resolve_string(&h.value, vars)))
        .collect();

    let auth = match &request.auth {
        AuthConfig::OAuth2 { grant } => match oauth_store.cached_token(&request.id).await {
            Some(token) => ResolvedAuth::OAuth2Token {
                token: token.access_token,
                header_prefix: grant.options().header_prefix.clone(),
                add_to: grant.options().add_to,
            },
            None => {
                let mut headers = custom_headers;
                headers.push(("Authorization".to_string(), "(pending — click Authorize or Send)".to_string()));
                return Ok(headers);
            }
        },
        AuthConfig::Digest { .. } => {
            let mut headers = custom_headers;
            headers.push(("Authorization".to_string(), "(computed after first 401 challenge on Send)".to_string()));
            return Ok(headers);
        }
        other => resolve_auth_sync(other, vars),
    };

    let body_bytes: Vec<u8> = match &request.body {
        RequestBody::None => Vec::new(),
        RequestBody::Json { content } | RequestBody::Text { content } | RequestBody::Xml { content } => {
            resolve_string(content, vars).into_bytes()
        }
        RequestBody::FormUrlEncoded { fields } => {
            let form: Vec<(String, String)> = fields
                .iter()
                .filter(|f| f.enabled)
                .map(|f| (resolve_string(&f.key, vars), resolve_string(&f.value, vars)))
                .collect();
            urlencoded_form(&form).into_bytes()
        }
    };

    let mut builder = reqwest::Client::new().request(request.method.as_reqwest(), &url);
    if !query.is_empty() {
        builder = builder.query(&query);
    }
    for (k, v) in headers_excluding_manual_auth(&custom_headers, &auth) {
        builder = builder.header(k, v);
    }
    match &request.body {
        RequestBody::Json { .. } => builder = builder.header("Content-Type", "application/json"),
        RequestBody::Xml { .. } => builder = builder.header("Content-Type", "application/xml"),
        RequestBody::FormUrlEncoded { .. } => {
            builder = builder.header("Content-Type", "application/x-www-form-urlencoded")
        }
        RequestBody::None | RequestBody::Text { .. } => {}
    }
    builder = apply_resolved_auth(builder, request, &url, &query, &body_bytes, &auth)?;

    let built = builder.build().map_err(|e| ApiError::Validation(e.to_string()))?;
    Ok(built
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect())
}

fn resolve_auth_sync(auth: &AuthConfig, vars: &HashMap<String, String>) -> ResolvedAuth {
    match auth {
        // By the time auth reaches here it must already be resolved
        // concretely — the command layer walks folder -> workspace
        // inheritance before ever calling into request_engine (see
        // spectra-api::commands::request::resolve_effective_auth). Reaching
        // this arm means nothing up the chain had a real auth configured,
        // which behaves the same as an explicit None.
        AuthConfig::None | AuthConfig::InheritFromParent => ResolvedAuth::None,
        AuthConfig::Basic { username, password } => {
            ResolvedAuth::Basic { username: resolve_string(username, vars), password: resolve_string(password, vars) }
        }
        AuthConfig::Bearer { token } => ResolvedAuth::Bearer { token: resolve_string(token, vars) },
        AuthConfig::ApiKey { key, value, location } => {
            let key = resolve_string(key, vars);
            let value = resolve_string(value, vars);
            match location {
                ApiKeyLocation::Header => ResolvedAuth::ApiKeyHeader { key, value },
                ApiKeyLocation::Query => ResolvedAuth::ApiKeyQuery { key, value },
                ApiKeyLocation::Cookie => ResolvedAuth::ApiKeyCookie { key, value },
            }
        }
        AuthConfig::OAuth1 { consumer_key, consumer_secret, token, token_secret, signature_method } => {
            ResolvedAuth::OAuth1 {
                consumer_key: resolve_string(consumer_key, vars),
                consumer_secret: resolve_string(consumer_secret, vars),
                token: token.as_ref().map(|t| resolve_string(t, vars)),
                token_secret: token_secret.as_ref().map(|t| resolve_string(t, vars)),
                signature_method: *signature_method,
            }
        }
        AuthConfig::AwsSigV4 { access_key, secret_key, region, service, session_token } => ResolvedAuth::AwsSigV4 {
            access_key: resolve_string(access_key, vars),
            secret_key: resolve_string(secret_key, vars),
            region: resolve_string(region, vars),
            service: resolve_string(service, vars),
            session_token: session_token.as_ref().map(|t| resolve_string(t, vars)),
        },
        AuthConfig::Hawk { id, key, algorithm } => {
            ResolvedAuth::Hawk { id: resolve_string(id, vars), key: resolve_string(key, vars), algorithm: *algorithm }
        }
        // Digest and OAuth2 are handled by their callers before reaching here.
        AuthConfig::Digest { .. } | AuthConfig::OAuth2 { .. } | AuthConfig::SavedAuth { .. } => ResolvedAuth::None,
    }
}

fn apply_resolved_auth(
    builder: reqwest::RequestBuilder,
    request: &Request,
    url: &str,
    query: &[(String, String)],
    body: &[u8],
    auth: &ResolvedAuth,
) -> ApiResult<reqwest::RequestBuilder> {
    Ok(match auth {
        ResolvedAuth::None => builder,
        ResolvedAuth::Basic { username, password } => builder.basic_auth(username, Some(password)),
        ResolvedAuth::Bearer { token } => builder.bearer_auth(token),
        ResolvedAuth::OAuth2Token { token, header_prefix, add_to } => match add_to {
            crate::model::AddAuthDataTo::QueryParams => builder.query(&[("access_token", token)]),
            crate::model::AddAuthDataTo::RequestHeaders => {
                let value = if header_prefix.is_empty() { token.clone() } else { format!("{header_prefix} {token}") };
                builder.header("Authorization", value)
            }
        },
        ResolvedAuth::ApiKeyHeader { key, value } => builder.header(key, value),
        ResolvedAuth::ApiKeyQuery { key, value } => builder.query(&[(key, value)]),
        ResolvedAuth::ApiKeyCookie { key, value } => builder.header("Cookie", format!("{key}={value}")),
        ResolvedAuth::Digest { .. } => builder,
        ResolvedAuth::Authorization(value) => builder.header("Authorization", value),
        ResolvedAuth::OAuth1 { consumer_key, consumer_secret, token, token_secret, signature_method } => {
            let header = auth_signing::oauth1_header(
                &OAuth1Params {
                    consumer_key: consumer_key.clone(),
                    consumer_secret: consumer_secret.clone(),
                    token: token.clone(),
                    token_secret: token_secret.clone(),
                    signature_method: *signature_method,
                },
                request.method.as_reqwest().as_str(),
                url,
                query,
            );
            builder.header("Authorization", header)
        }
        ResolvedAuth::AwsSigV4 { access_key, secret_key, region, service, session_token } => {
            let parsed = reqwest::Url::parse(url).map_err(|e| ApiError::Validation(e.to_string()))?;
            let host = parsed.host_str().unwrap_or_default().to_string();
            let path = parsed.path().to_string();
            let query_str = parsed.query().unwrap_or("").to_string();
            let signed = auth_signing::aws_sigv4_headers(
                &AwsSigV4Params {
                    access_key: access_key.clone(),
                    secret_key: secret_key.clone(),
                    region: region.clone(),
                    service: service.clone(),
                    session_token: session_token.clone(),
                },
                request.method.as_reqwest().as_str(),
                &host,
                &path,
                &query_str,
                body,
            );
            let mut b = builder;
            for (k, v) in signed.headers {
                b = b.header(k, v);
            }
            b
        }
        ResolvedAuth::Hawk { id, key, algorithm } => {
            let parsed = reqwest::Url::parse(url).map_err(|e| ApiError::Validation(e.to_string()))?;
            let host = parsed.host_str().unwrap_or_default().to_string();
            let port = parsed.port_or_known_default().unwrap_or(443);
            let path = parsed.path().to_string();
            let header = auth_signing::hawk_header(
                &HawkParams { id: id.clone(), key: key.clone(), algorithm: *algorithm },
                request.method.as_reqwest().as_str(),
                &host,
                port,
                &path,
            );
            builder.header("Authorization", header)
        }
    })
}

fn urlencoded_form(fields: &[(String, String)]) -> String {
    fields
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

enum ResolvedAuth {
    None,
    Basic { username: String, password: String },
    Bearer { token: String },
    /// An OAuth2 token, carrying the grant's `header_prefix` (e.g. "Bearer",
    /// but some APIs use others) and `add_to` (Authorization header vs. a
    /// query param) — unlike plain `Bearer`, these are user-configurable
    /// per Postman's OAuth2 panel rather than hardcoded.
    OAuth2Token { token: String, header_prefix: String, add_to: crate::model::AddAuthDataTo },
    ApiKeyHeader { key: String, value: String },
    ApiKeyQuery { key: String, value: String },
    ApiKeyCookie { key: String, value: String },
    Digest { username: String, password: String },
    /// A fully-computed `Authorization` header value (used for the Digest
    /// retry pass once we have a challenge).
    Authorization(String),
    OAuth1 {
        consumer_key: String,
        consumer_secret: String,
        token: Option<String>,
        token_secret: Option<String>,
        signature_method: OAuth1SignatureMethod,
    },
    AwsSigV4 {
        access_key: String,
        secret_key: String,
        region: String,
        service: String,
        session_token: Option<String>,
    },
    Hawk { id: String, key: String, algorithm: HawkAlgorithm },
}

async fn resolve_auth(
    http: &reqwest::Client,
    oauth_store: &Arc<OAuthStore>,
    request_id: &str,
    auth: &AuthConfig,
    vars: &HashMap<String, String>,
) -> ApiResult<ResolvedAuth> {
    match auth {
        // See the comment on the same arm in resolve_auth_sync — inheritance
        // is resolved by the command layer before this is ever called.
        AuthConfig::None | AuthConfig::InheritFromParent | AuthConfig::SavedAuth { .. } => Ok(ResolvedAuth::None),
        AuthConfig::Basic { username, password } => Ok(ResolvedAuth::Basic {
            username: resolve_string(username, vars),
            password: resolve_string(password, vars),
        }),
        AuthConfig::Bearer { token } => Ok(ResolvedAuth::Bearer { token: resolve_string(token, vars) }),
        AuthConfig::ApiKey { key, value, location } => {
            let key = resolve_string(key, vars);
            let value = resolve_string(value, vars);
            Ok(match location {
                ApiKeyLocation::Header => ResolvedAuth::ApiKeyHeader { key, value },
                ApiKeyLocation::Query => ResolvedAuth::ApiKeyQuery { key, value },
                ApiKeyLocation::Cookie => ResolvedAuth::ApiKeyCookie { key, value },
            })
        }
        AuthConfig::Digest { username, password } => Ok(ResolvedAuth::Digest {
            username: resolve_string(username, vars),
            password: resolve_string(password, vars),
        }),
        AuthConfig::OAuth1 { consumer_key, consumer_secret, token, token_secret, signature_method } => {
            Ok(ResolvedAuth::OAuth1 {
                consumer_key: resolve_string(consumer_key, vars),
                consumer_secret: resolve_string(consumer_secret, vars),
                token: token.as_ref().map(|t| resolve_string(t, vars)),
                token_secret: token_secret.as_ref().map(|t| resolve_string(t, vars)),
                signature_method: *signature_method,
            })
        }
        AuthConfig::AwsSigV4 { access_key, secret_key, region, service, session_token } => {
            Ok(ResolvedAuth::AwsSigV4 {
                access_key: resolve_string(access_key, vars),
                secret_key: resolve_string(secret_key, vars),
                region: resolve_string(region, vars),
                service: resolve_string(service, vars),
                session_token: session_token.as_ref().map(|t| resolve_string(t, vars)),
            })
        }
        AuthConfig::Hawk { id, key, algorithm } => Ok(ResolvedAuth::Hawk {
            id: resolve_string(id, vars),
            key: resolve_string(key, vars),
            algorithm: *algorithm,
        }),
        AuthConfig::OAuth2 { grant } => {
            let options = grant.options();
            if let Some(token) = oauth_store.cached_token(request_id).await {
                return Ok(ResolvedAuth::OAuth2Token {
                    token: token.access_token,
                    header_prefix: options.header_prefix.clone(),
                    add_to: options.add_to,
                });
            }

            // Auto-refresh: if the last-fetched token expired but came with
            // a refresh_token, use it instead of re-running the full grant
            // (matches Postman's "Auto-refresh Token" toggle).
            if options.auto_refresh {
                if let Some(refresh_token) = oauth_store.expired_refresh_token(request_id).await {
                    if let Some(refreshed) = oauth2::refresh_grant_token(http, grant, &refresh_token).await? {
                        oauth_store.save_token(request_id, None, refreshed.clone()).await;
                        return Ok(ResolvedAuth::OAuth2Token {
                            token: refreshed.access_token,
                            header_prefix: options.header_prefix.clone(),
                            add_to: options.add_to,
                        });
                    }
                }
            }

            Err(ApiError::Auth("No access token. Please fetch an access token first.".into()))
        }
    }
}

/// Filters a manually-added `Authorization` header out of the resolved
/// header list whenever a real auth scheme is configured. A manual
/// `Authorization` header and a configured auth scheme both ultimately call
/// `.header("Authorization", ...)` on the same `reqwest::RequestBuilder` —
/// reqwest appends rather than replaces, so without this the request goes
/// out with two `Authorization` headers and the server sees whichever one
/// it reads first (often the stale/unresolved manual one, not the Auth
/// tab's actual computed value — this is exactly how a request with a
/// leftover `Authorization: Bearer {{unresolved_var}}` header ends up
/// rejecting a perfectly valid token configured in the Auth tab). The Auth
/// tab is always the authoritative source once it's configured, matching
/// Postman/Insomnia's behavior of superseding a manual Authorization header
/// with computed auth.
fn headers_excluding_manual_auth<'a>(
    headers: &'a [(String, String)],
    auth: &ResolvedAuth,
) -> Vec<&'a (String, String)> {
    let skip_manual_auth_header = !matches!(auth, ResolvedAuth::None);
    headers.iter().filter(|(k, _)| !(skip_manual_auth_header && k.eq_ignore_ascii_case("authorization"))).collect()
}

async fn send_once(
    http: &reqwest::Client,
    request: &Request,
    url: &str,
    query: &[(String, String)],
    headers: &[(String, String)],
    body: &[u8],
    auth: &ResolvedAuth,
) -> ApiResult<reqwest::Response> {
    let mut builder = http.request(request.method.as_reqwest(), url);
    if !query.is_empty() {
        builder = builder.query(query);
    }
    for (k, v) in headers_excluding_manual_auth(headers, auth) {
        builder = builder.header(k, v);
    }
    match &request.body {
        RequestBody::Json { .. } => builder = builder.header("Content-Type", "application/json"),
        RequestBody::Xml { .. } => builder = builder.header("Content-Type", "application/xml"),
        RequestBody::FormUrlEncoded { .. } => {
            builder = builder.header("Content-Type", "application/x-www-form-urlencoded")
        }
        RequestBody::None | RequestBody::Text { .. } => {}
    }
    if !matches!(request.body, RequestBody::None) {
        builder = builder.body(body.to_vec());
    }

    builder = apply_resolved_auth(builder, request, url, query, body, auth)?;

    Ok(builder.send().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_manual_authorization_header_when_auth_is_configured() {
        let headers = vec![
            ("Authorization".to_string(), "Bearer {{token}}".to_string()),
            ("Accept".to_string(), "application/json".to_string()),
        ];
        let auth = ResolvedAuth::Bearer { token: "real-token".to_string() };
        let kept = headers_excluding_manual_auth(&headers, &auth);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].0, "Accept");
    }

    #[test]
    fn drops_manual_authorization_header_case_insensitively() {
        let headers = vec![("authorization".to_string(), "stale".to_string())];
        let auth = ResolvedAuth::Basic { username: "u".to_string(), password: "p".to_string() };
        assert!(headers_excluding_manual_auth(&headers, &auth).is_empty());
    }

    #[test]
    fn keeps_manual_authorization_header_when_auth_is_none() {
        let headers = vec![("Authorization".to_string(), "Bearer manual-token".to_string())];
        let kept = headers_excluding_manual_auth(&headers, &ResolvedAuth::None);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].1, "Bearer manual-token");
    }

    #[test]
    fn keeps_other_headers_untouched_regardless_of_auth() {
        let headers = vec![
            ("X-Custom".to_string(), "value".to_string()),
            ("Content-Type".to_string(), "text/plain".to_string()),
        ];
        let auth = ResolvedAuth::Bearer { token: "t".to_string() };
        let kept = headers_excluding_manual_auth(&headers, &auth);
        assert_eq!(kept.len(), 2);
    }
}
