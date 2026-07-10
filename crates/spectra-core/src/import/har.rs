//! HAR (HTTP Archive, http://www.softwareishard.com/blog/har-12-spec/) —
//! the format browser DevTools "Save all as HAR" and network-capture tools
//! export. One request is generated per `log.entries[]` item, in capture
//! order, all at the top level (HAR has no folder/grouping concept).
//! Cookies, timing, and response data are ignored — only the outgoing
//! request shape has a Spectra equivalent.
use super::{ImportedRequest, ImportedTree};
use crate::error::{ApiError, ApiResult};
use crate::model::{AuthConfig, HeaderEntry, HttpMethod, ParamEntry, RequestBody};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct HarFile {
    log: HarLog,
}

#[derive(Debug, Deserialize)]
struct HarLog {
    #[serde(default)]
    entries: Vec<HarEntry>,
}

#[derive(Debug, Deserialize)]
struct HarEntry {
    request: HarRequest,
}

#[derive(Debug, Deserialize)]
struct HarRequest {
    method: String,
    url: String,
    #[serde(default)]
    headers: Vec<HarNameValue>,
    #[serde(rename = "queryString", default)]
    query_string: Vec<HarNameValue>,
    #[serde(rename = "postData", default)]
    post_data: Option<HarPostData>,
}

#[derive(Debug, Deserialize)]
struct HarNameValue {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct HarPostData {
    #[serde(rename = "mimeType", default)]
    mime_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    params: Vec<HarNameValue>,
}

fn parse_method(s: &str) -> HttpMethod {
    match s.to_uppercase().as_str() {
        "POST" => HttpMethod::Post,
        "PUT" => HttpMethod::Put,
        "PATCH" => HttpMethod::Patch,
        "DELETE" => HttpMethod::Delete,
        "OPTIONS" => HttpMethod::Options,
        "HEAD" => HttpMethod::Head,
        _ => HttpMethod::Get,
    }
}

fn convert_body(post_data: &HarPostData) -> RequestBody {
    if !post_data.params.is_empty() {
        return RequestBody::FormUrlEncoded {
            fields: post_data
                .params
                .iter()
                .map(|p| ParamEntry { key: p.name.clone(), value: p.value.clone(), enabled: true })
                .collect(),
        };
    }
    let content = post_data.text.clone().unwrap_or_default();
    if content.is_empty() {
        return RequestBody::None;
    }
    match post_data.mime_type.as_deref() {
        Some(mt) if mt.contains("json") => RequestBody::Json { content },
        Some(mt) if mt.contains("xml") => RequestBody::Xml { content },
        _ if content.trim_start().starts_with(['{', '[']) => RequestBody::Json { content },
        _ => RequestBody::Text { content },
    }
}

/// Basic/Bearer auth is recoverable from a captured `Authorization` header
/// (the same signal `import::curl` uses); anything else HAR captures
/// (cookies, custom signing) has no faithful Spectra auth-type equivalent
/// from a single captured header value, so it's left as `None` — the header
/// itself is still imported verbatim in `headers`.
fn extract_auth(headers: &[HarNameValue]) -> AuthConfig {
    let Some(auth_header) = headers.iter().find(|h| h.name.eq_ignore_ascii_case("authorization")) else {
        return AuthConfig::None;
    };
    if let Some(token) = auth_header.value.strip_prefix("Bearer ") {
        return AuthConfig::Bearer { token: token.trim().to_string() };
    }
    if let Some(encoded) = auth_header.value.strip_prefix("Basic ") {
        if let Ok(decoded) = base64_decode(encoded.trim()) {
            if let Some((user, pass)) = decoded.split_once(':') {
                return AuthConfig::Basic { username: user.to_string(), password: pass.to_string() };
            }
        }
    }
    AuthConfig::None
}

fn base64_decode(s: &str) -> Result<String, ()> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(s).ok().and_then(|b| String::from_utf8(b).ok()).ok_or(())
}

fn derive_name(url: &str) -> String {
    let without_query = url.split('?').next().unwrap_or(url);
    let last_segment = without_query.trim_end_matches('/').rsplit('/').next().unwrap_or(url);
    if last_segment.is_empty() { "Imported Request".to_string() } else { last_segment.to_string() }
}

pub fn parse(input: &str) -> ApiResult<ImportedTree> {
    let har: HarFile = serde_json::from_str(input).map_err(|e| ApiError::Validation(format!("invalid HAR file: {e}")))?;

    let mut requests = Vec::with_capacity(har.log.entries.len());
    for entry in &har.log.entries {
        let req = &entry.request;
        let method = parse_method(&req.method);
        let headers: Vec<HeaderEntry> =
            req.headers.iter().map(|h| HeaderEntry { key: h.name.clone(), value: h.value.clone(), enabled: true }).collect();
        let params: Vec<ParamEntry> =
            req.query_string.iter().map(|q| ParamEntry { key: q.name.clone(), value: q.value.clone(), enabled: true }).collect();
        let body = req.post_data.as_ref().map(convert_body).unwrap_or(RequestBody::None);
        let auth = extract_auth(&req.headers);

        requests.push(ImportedRequest {
            folder_path: Vec::new(),
            name: derive_name(&req.url),
            method,
            url: req.url.clone(),
            headers,
            params,
            body,
            auth,
            saved_responses: Vec::new(),
        });
    }

    if requests.is_empty() {
        return Err(ApiError::Validation("no requests found in HAR file".into()));
    }

    Ok(ImportedTree { requests })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_get() {
        let har = r#"{
            "log": {
                "entries": [
                    {
                        "request": {
                            "method": "GET",
                            "url": "https://api.example.com/users",
                            "headers": [{ "name": "Accept", "value": "application/json" }],
                            "queryString": []
                        }
                    }
                ]
            }
        }"#;
        let tree = parse(har).unwrap();
        assert_eq!(tree.requests.len(), 1);
        assert_eq!(tree.requests[0].method, HttpMethod::Get);
        assert_eq!(tree.requests[0].url, "https://api.example.com/users");
        assert_eq!(tree.requests[0].headers[0].key, "Accept");
    }

    #[test]
    fn parses_post_with_json_body() {
        let har = r#"{
            "log": {
                "entries": [
                    {
                        "request": {
                            "method": "POST",
                            "url": "https://api.example.com/users",
                            "headers": [],
                            "queryString": [],
                            "postData": { "mimeType": "application/json", "text": "{\"name\":\"a\"}" }
                        }
                    }
                ]
            }
        }"#;
        let tree = parse(har).unwrap();
        match &tree.requests[0].body {
            RequestBody::Json { content } => assert_eq!(content, r#"{"name":"a"}"#),
            other => panic!("expected Json body, got {other:?}"),
        }
    }

    #[test]
    fn extracts_bearer_auth_from_header() {
        let har = r#"{
            "log": {
                "entries": [
                    {
                        "request": {
                            "method": "GET",
                            "url": "https://api.example.com/x",
                            "headers": [{ "name": "Authorization", "value": "Bearer abc123" }],
                            "queryString": []
                        }
                    }
                ]
            }
        }"#;
        let tree = parse(har).unwrap();
        match &tree.requests[0].auth {
            AuthConfig::Bearer { token } => assert_eq!(token, "abc123"),
            other => panic!("expected Bearer, got {other:?}"),
        }
    }

    #[test]
    fn parses_query_string_params() {
        let har = r#"{
            "log": {
                "entries": [
                    {
                        "request": {
                            "method": "GET",
                            "url": "https://api.example.com/x?foo=bar",
                            "headers": [],
                            "queryString": [{ "name": "foo", "value": "bar" }]
                        }
                    }
                ]
            }
        }"#;
        let tree = parse(har).unwrap();
        assert_eq!(tree.requests[0].params.len(), 1);
        assert_eq!(tree.requests[0].params[0].key, "foo");
        assert_eq!(tree.requests[0].params[0].value, "bar");
    }

    #[test]
    fn empty_entries_is_an_error() {
        let har = r#"{ "log": { "entries": [] } }"#;
        assert!(parse(har).is_err());
    }
}
