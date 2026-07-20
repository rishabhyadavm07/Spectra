use super::{ImportedRequest, ImportedSavedResponse, ImportedTree};
use crate::error::{ApiError, ApiResult};
use crate::model::{
    ApiKeyLocation, AuthConfig, HawkAlgorithm, HeaderEntry, HttpMethod, OAuth1SignatureMethod, ParamEntry,
    RequestBody, ResponseDto,
};
use serde::Deserialize;
use std::collections::HashMap;

/// Postman Collection v2.1 format (the export format from "Export" on any
/// collection). Handles folders (nested `item` arrays), requests, headers,
/// URL-encoded/raw/form-data bodies, auth (Basic/Bearer/API Key/OAuth1/
/// AWS SigV4/Digest/Hawk map onto their direct Spectra equivalents; OAuth2
/// maps its `accessToken` onto a Bearer token, see `convert_auth` for why),
/// and saved example responses (`item.response[]` — Postman's "Examples,"
/// imported as Spectra `SavedResponse`s attached to the request they came
/// from). Other Postman features (scripts, tests, variables-with-scope,
/// protocolProfileBehavior, etc.) are silently dropped — they have no
/// Spectra equivalent yet.
#[derive(Debug, Deserialize)]
struct Collection {
    #[serde(default)]
    item: Vec<Item>,
}

#[derive(Debug, Deserialize)]
struct Item {
    name: String,
    #[serde(default)]
    item: Option<Vec<Item>>,
    #[serde(default)]
    request: Option<PostmanRequest>,
    #[serde(default)]
    response: Vec<PostmanResponse>,
}

/// One saved example response under `item.response[]` — Postman calls these
/// "Examples." `code`/`status` are both optional per the schema (a
/// hand-edited or older-exported collection can omit either), so both fall
/// back to a generic "0 Unknown" rather than failing the whole import over
/// one malformed example.
#[derive(Debug, Deserialize)]
struct PostmanResponse {
    #[serde(default = "default_response_name")]
    name: String,
    #[serde(default)]
    code: Option<u16>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    header: Vec<PostmanHeader>,
    #[serde(default)]
    body: Option<String>,
    #[serde(rename = "responseTime", default)]
    response_time: Option<serde_json::Value>,
}

fn default_response_name() -> String {
    "Saved Response".to_string()
}

fn convert_saved_response(resp: &PostmanResponse) -> ImportedSavedResponse {
    let headers: HashMap<String, String> =
        resp.header.iter().filter(|h| !h.disabled).map(|h| (h.key.clone(), h.value.clone())).collect();
    let body = resp.body.clone().unwrap_or_default();
    // Postman's responseTime can be a number, null, or (rarely) a string;
    // anything that isn't a plain non-negative number becomes 0 rather than
    // failing the import over a cosmetic field.
    let duration_ms = resp.response_time.as_ref().and_then(|v| v.as_u64()).unwrap_or(0);

    ImportedSavedResponse {
        name: resp.name.clone(),
        response: ResponseDto {
            status: resp.code.unwrap_or(0),
            status_text: resp.status.clone().unwrap_or_else(|| "Unknown".to_string()),
            headers,
            size_bytes: body.len(),
            body,
            duration_ms,
        },
    }
}

#[derive(Debug, Deserialize)]
struct PostmanRequest {
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    header: Vec<PostmanHeader>,
    #[serde(default)]
    url: Option<PostmanUrl>,
    #[serde(default)]
    body: Option<PostmanBody>,
    #[serde(default)]
    auth: Option<PostmanAuth>,
}

#[derive(Debug, Deserialize)]
struct PostmanHeader {
    key: String,
    value: String,
    #[serde(default)]
    disabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PostmanUrl {
    Raw(String),
    Detailed { raw: String },
}

impl PostmanUrl {
    fn raw(&self) -> String {
        match self {
            PostmanUrl::Raw(s) => s.clone(),
            PostmanUrl::Detailed { raw } => raw.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct PostmanBody {
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    raw: Option<String>,
    #[serde(default)]
    urlencoded: Vec<PostmanKv>,
    #[serde(default)]
    formdata: Vec<PostmanKv>,
    #[serde(default)]
    options: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct PostmanKv {
    key: String,
    #[serde(default)]
    value: String,
    #[serde(default)]
    disabled: bool,
}

#[derive(Debug, Deserialize)]
struct PostmanAuth {
    #[serde(rename = "type")]
    auth_type: String,
    #[serde(default)]
    basic: Vec<PostmanAuthField>,
    #[serde(default)]
    bearer: Vec<PostmanAuthField>,
    #[serde(default)]
    apikey: Vec<PostmanAuthField>,
    #[serde(default)]
    oauth1: Vec<PostmanAuthField>,
    #[serde(default)]
    oauth2: Vec<PostmanAuthField>,
    #[serde(default, rename = "awsv4")]
    awsv4: Vec<PostmanAuthField>,
    #[serde(default)]
    digest: Vec<PostmanAuthField>,
    #[serde(default)]
    hawk: Vec<PostmanAuthField>,
}

#[derive(Debug, Deserialize)]
struct PostmanAuthField {
    key: String,
    #[serde(default)]
    value: String,
}

fn field<'a>(fields: &'a [PostmanAuthField], key: &str) -> Option<&'a str> {
    fields.iter().find(|f| f.key == key).map(|f| f.value.as_str())
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

fn convert_auth(auth: &PostmanAuth) -> AuthConfig {
    match auth.auth_type.as_str() {
        "basic" => AuthConfig::Basic {
            username: field(&auth.basic, "username").unwrap_or_default().to_string(),
            password: field(&auth.basic, "password").unwrap_or_default().to_string(),
        },
        "bearer" => AuthConfig::Bearer { token: field(&auth.bearer, "token").unwrap_or_default().to_string() },
        "apikey" => AuthConfig::ApiKey {
            key: field(&auth.apikey, "key").unwrap_or_default().to_string(),
            value: field(&auth.apikey, "value").unwrap_or_default().to_string(),
            location: match field(&auth.apikey, "in") {
                Some("query") => ApiKeyLocation::Query,
                _ => ApiKeyLocation::Header,
            },
        },
        "oauth1" => AuthConfig::OAuth1 {
            consumer_key: field(&auth.oauth1, "consumerKey").unwrap_or_default().to_string(),
            consumer_secret: field(&auth.oauth1, "consumerSecret").unwrap_or_default().to_string(),
            token: field(&auth.oauth1, "token").filter(|s| !s.is_empty()).map(str::to_string),
            token_secret: field(&auth.oauth1, "tokenSecret").filter(|s| !s.is_empty()).map(str::to_string),
            signature_method: match field(&auth.oauth1, "signatureMethod") {
                Some("HMAC-SHA256") => OAuth1SignatureMethod::HmacSha256,
                Some("PLAINTEXT") => OAuth1SignatureMethod::PlainText,
                _ => OAuth1SignatureMethod::HmacSha1,
            },
        },
        // Postman's "oauth2" auth block describes a previously-completed
        // token exchange (accessToken + grant metadata for its own UI), not
        // a re-runnable grant config — Spectra's OAuth2Grant needs live
        // client_id/urls to fetch tokens itself, which Postman's export
        // doesn't carry. The one thing that *does* transfer faithfully is
        // the token value itself, so it's imported as a Bearer token rather
        // than invented/incomplete OAuth2 grant fields.
        "oauth2" => match field(&auth.oauth2, "accessToken") {
            Some(token) if !token.is_empty() => AuthConfig::Bearer { token: token.to_string() },
            _ => AuthConfig::None,
        },
        "awsv4" => AuthConfig::AwsSigV4 {
            access_key: field(&auth.awsv4, "accessKey").unwrap_or_default().to_string(),
            secret_key: field(&auth.awsv4, "secretKey").unwrap_or_default().to_string(),
            region: field(&auth.awsv4, "region").unwrap_or_default().to_string(),
            service: field(&auth.awsv4, "service").unwrap_or_default().to_string(),
            session_token: field(&auth.awsv4, "sessionToken").filter(|s| !s.is_empty()).map(str::to_string),
        },
        "digest" => AuthConfig::Digest {
            username: field(&auth.digest, "username").unwrap_or_default().to_string(),
            password: field(&auth.digest, "password").unwrap_or_default().to_string(),
        },
        "hawk" => AuthConfig::Hawk {
            id: field(&auth.hawk, "authId").unwrap_or_default().to_string(),
            key: field(&auth.hawk, "authKey").unwrap_or_default().to_string(),
            algorithm: match field(&auth.hawk, "algorithm") {
                Some("sha256") => HawkAlgorithm::Sha256,
                _ => HawkAlgorithm::Sha1,
            },
        },
        _ => AuthConfig::None,
    }
}

fn convert_body(body: &PostmanBody) -> RequestBody {
    match body.mode.as_deref() {
        Some("raw") => {
            let content = body.raw.clone().unwrap_or_default();
            let language = body
                .options
                .as_ref()
                .and_then(|o| o.get("raw"))
                .and_then(|r| r.get("language"))
                .and_then(|l| l.as_str());
            match language {
                Some("xml") => RequestBody::Xml { content },
                Some("json") | None if content.trim_start().starts_with(['{', '[']) => RequestBody::Json { content },
                _ => RequestBody::Text { content },
            }
        }
        Some("urlencoded") => RequestBody::FormUrlEncoded {
            fields: body
                .urlencoded
                .iter()
                .map(|kv| ParamEntry { key: kv.key.clone(), value: kv.value.clone(), enabled: !kv.disabled })
                .collect(),
        },
        Some("formdata") => RequestBody::FormUrlEncoded {
            fields: body
                .formdata
                .iter()
                .map(|kv| ParamEntry { key: kv.key.clone(), value: kv.value.clone(), enabled: !kv.disabled })
                .collect(),
        },
        _ => RequestBody::None,
    }
}

fn walk(items: &[Item], folder_path: &[String], out: &mut Vec<ImportedRequest>) {
    for item in items {
        if let Some(children) = &item.item {
            let mut next_path = folder_path.to_vec();
            next_path.push(item.name.clone());
            walk(children, &next_path, out);
            continue;
        }
        let Some(req) = &item.request else { continue };
        let headers: Vec<HeaderEntry> = req
            .header
            .iter()
            .map(|h| HeaderEntry { key: h.key.clone(), value: h.value.clone(), enabled: !h.disabled })
            .collect();
        let url = req.url.as_ref().map(|u| u.raw()).unwrap_or_default();
        let method = req.method.as_deref().map(parse_method).unwrap_or(HttpMethod::Get);
        let body = req.body.as_ref().map(convert_body).unwrap_or(RequestBody::None);
        let auth = req.auth.as_ref().map(convert_auth).unwrap_or(AuthConfig::None);
        let saved_responses: Vec<ImportedSavedResponse> = item.response.iter().map(convert_saved_response).collect();

        out.push(ImportedRequest {
            folder_path: folder_path.to_vec(),
            name: item.name.clone(),
            method,
            url,
            headers,
            params: Vec::new(),
            body,
            auth,
            saved_responses,
        });
    }
}

pub fn parse(input: &str) -> ApiResult<ImportedTree> {
    let collection: Collection =
        serde_json::from_str(input).map_err(|e| ApiError::Validation(format!("invalid Postman collection: {e}")))?;
    let mut requests = Vec::new();
    walk(&collection.item, &[], &mut requests);
    Ok(ImportedTree { requests })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_flat_collection() {
        let json = r#"{
            "info": { "name": "Test", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json" },
            "item": [
                {
                    "name": "Get users",
                    "request": {
                        "method": "GET",
                        "header": [{"key": "Accept", "value": "application/json"}],
                        "url": { "raw": "https://api.example.com/users" }
                    }
                }
            ]
        }"#;
        let tree = parse(json).unwrap();
        assert_eq!(tree.requests.len(), 1);
        assert_eq!(tree.requests[0].name, "Get users");
        assert_eq!(tree.requests[0].url, "https://api.example.com/users");
        assert!(tree.requests[0].folder_path.is_empty());
    }

    #[test]
    fn parses_nested_folders() {
        let json = r#"{
            "info": { "name": "Test", "schema": "x" },
            "item": [
                {
                    "name": "Users",
                    "item": [
                        {
                            "name": "Get user",
                            "request": { "method": "GET", "url": { "raw": "https://api.example.com/users/1" } }
                        }
                    ]
                }
            ]
        }"#;
        let tree = parse(json).unwrap();
        assert_eq!(tree.requests.len(), 1);
        assert_eq!(tree.requests[0].folder_path, vec!["Users".to_string()]);
    }

    fn request_with_auth(auth_json: &str) -> ImportedRequest {
        let json = format!(
            r#"{{
                "info": {{ "name": "Test", "schema": "x" }},
                "item": [
                    {{
                        "name": "Req",
                        "request": {{
                            "method": "GET",
                            "url": {{ "raw": "https://api.example.com/x" }},
                            "auth": {auth_json}
                        }}
                    }}
                ]
            }}"#
        );
        parse(&json).unwrap().requests.into_iter().next().unwrap()
    }

    #[test]
    fn parses_oauth1_auth() {
        let req = request_with_auth(
            r#"{
                "type": "oauth1",
                "oauth1": [
                    { "key": "consumerKey", "value": "ck" },
                    { "key": "consumerSecret", "value": "cs" },
                    { "key": "token", "value": "tok" },
                    { "key": "tokenSecret", "value": "toksec" },
                    { "key": "signatureMethod", "value": "HMAC-SHA256" }
                ]
            }"#,
        );
        match req.auth {
            AuthConfig::OAuth1 { consumer_key, consumer_secret, token, token_secret, signature_method } => {
                assert_eq!(consumer_key, "ck");
                assert_eq!(consumer_secret, "cs");
                assert_eq!(token.as_deref(), Some("tok"));
                assert_eq!(token_secret.as_deref(), Some("toksec"));
                assert_eq!(signature_method, OAuth1SignatureMethod::HmacSha256);
            }
            other => panic!("expected OAuth1, got {other:?}"),
        }
    }

    #[test]
    fn parses_oauth2_auth_as_bearer() {
        let req = request_with_auth(
            r#"{ "type": "oauth2", "oauth2": [{ "key": "accessToken", "value": "abc123" }] }"#,
        );
        match req.auth {
            AuthConfig::Bearer { token } => assert_eq!(token, "abc123"),
            other => panic!("expected Bearer, got {other:?}"),
        }
    }

    #[test]
    fn parses_awsv4_auth() {
        let req = request_with_auth(
            r#"{
                "type": "awsv4",
                "awsv4": [
                    { "key": "accessKey", "value": "AKIA" },
                    { "key": "secretKey", "value": "secret" },
                    { "key": "region", "value": "us-east-1" },
                    { "key": "service", "value": "execute-api" }
                ]
            }"#,
        );
        match req.auth {
            AuthConfig::AwsSigV4 { access_key, secret_key, region, service, session_token } => {
                assert_eq!(access_key, "AKIA");
                assert_eq!(secret_key, "secret");
                assert_eq!(region, "us-east-1");
                assert_eq!(service, "execute-api");
                assert_eq!(session_token, None);
            }
            other => panic!("expected AwsSigV4, got {other:?}"),
        }
    }

    #[test]
    fn parses_digest_auth() {
        let req = request_with_auth(
            r#"{ "type": "digest", "digest": [{ "key": "username", "value": "u" }, { "key": "password", "value": "p" }] }"#,
        );
        match req.auth {
            AuthConfig::Digest { username, password } => {
                assert_eq!(username, "u");
                assert_eq!(password, "p");
            }
            other => panic!("expected Digest, got {other:?}"),
        }
    }

    #[test]
    fn parses_hawk_auth() {
        let req = request_with_auth(
            r#"{
                "type": "hawk",
                "hawk": [
                    { "key": "authId", "value": "id1" },
                    { "key": "authKey", "value": "key1" },
                    { "key": "algorithm", "value": "sha256" }
                ]
            }"#,
        );
        match req.auth {
            AuthConfig::Hawk { id, key, algorithm } => {
                assert_eq!(id, "id1");
                assert_eq!(key, "key1");
                assert_eq!(algorithm, HawkAlgorithm::Sha256);
            }
            other => panic!("expected Hawk, got {other:?}"),
        }
    }

    #[test]
    fn parses_saved_example_responses() {
        let json = r#"{
            "info": { "name": "Test", "schema": "x" },
            "item": [
                {
                    "name": "Get user",
                    "request": { "method": "GET", "url": { "raw": "https://api.example.com/users/1" } },
                    "response": [
                        {
                            "name": "Success",
                            "code": 200,
                            "status": "OK",
                            "header": [{ "key": "Content-Type", "value": "application/json" }],
                            "body": "{\"id\": 1, \"name\": \"Alice\"}",
                            "responseTime": 42
                        },
                        {
                            "name": "Not Found",
                            "code": 404,
                            "status": "Not Found",
                            "header": [],
                            "body": "{\"error\": \"not found\"}"
                        }
                    ]
                }
            ]
        }"#;
        let tree = parse(json).unwrap();
        assert_eq!(tree.requests.len(), 1);
        let saved = &tree.requests[0].saved_responses;
        assert_eq!(saved.len(), 2);

        assert_eq!(saved[0].name, "Success");
        assert_eq!(saved[0].response.status, 200);
        assert_eq!(saved[0].response.status_text, "OK");
        assert_eq!(saved[0].response.headers.get("Content-Type").map(String::as_str), Some("application/json"));
        assert_eq!(saved[0].response.body, r#"{"id": 1, "name": "Alice"}"#);
        assert_eq!(saved[0].response.duration_ms, 42);

        assert_eq!(saved[1].name, "Not Found");
        assert_eq!(saved[1].response.status, 404);
        assert_eq!(saved[1].response.duration_ms, 0); // no responseTime field on this one
    }

    #[test]
    fn requests_without_saved_responses_have_empty_vec() {
        let json = r#"{
            "info": { "name": "Test", "schema": "x" },
            "item": [
                { "name": "Get users", "request": { "method": "GET", "url": { "raw": "https://api.example.com/users" } } }
            ]
        }"#;
        let tree = parse(json).unwrap();
        assert!(tree.requests[0].saved_responses.is_empty());
    }

    #[test]
    fn saved_response_missing_code_and_status_falls_back_gracefully() {
        let json = r#"{
            "info": { "name": "Test", "schema": "x" },
            "item": [
                {
                    "name": "Req",
                    "request": { "method": "GET", "url": { "raw": "https://api.example.com/x" } },
                    "response": [{ "name": "Minimal", "header": [], "body": "" }]
                }
            ]
        }"#;
        let tree = parse(json).unwrap();
        let saved = &tree.requests[0].saved_responses;
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].response.status, 0);
        assert_eq!(saved[0].response.status_text, "Unknown");
    }

    #[test]
    fn saved_response_disabled_headers_are_excluded() {
        let json = r#"{
            "info": { "name": "Test", "schema": "x" },
            "item": [
                {
                    "name": "Req",
                    "request": { "method": "GET", "url": { "raw": "https://api.example.com/x" } },
                    "response": [{
                        "name": "Example",
                        "code": 200,
                        "header": [
                            { "key": "X-Enabled", "value": "yes" },
                            { "key": "X-Disabled", "value": "no", "disabled": true }
                        ],
                        "body": ""
                    }]
                }
            ]
        }"#;
        let tree = parse(json).unwrap();
        let headers = &tree.requests[0].saved_responses[0].response.headers;
        assert_eq!(headers.get("X-Enabled").map(String::as_str), Some("yes"));
        assert!(!headers.contains_key("X-Disabled"));
    }
}
