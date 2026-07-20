use super::ImportedRequest;
use crate::error::{ApiError, ApiResult};
use crate::model::{AuthConfig, HeaderEntry, HttpMethod, ParamEntry, RequestBody};

/// Parses a single `curl ...` command into one request. Supports the flags
/// people actually paste from a browser's "Copy as cURL": -X/--request,
/// -H/--header, -d/--data(-raw/-binary/--urlencode), -u/--user, --url, and a
/// bare trailing URL. Anything else (e.g. --compressed, -k) is ignored
/// rather than rejected, since it doesn't affect the request shape we store.
pub fn parse(input: &str) -> ApiResult<ImportedRequest> {
    let normalized = input.trim().replace("\\\n", " ").replace('\n', " ");
    let tokens = shell_words::split(&normalized)
        .map_err(|e| ApiError::Validation(format!("could not parse curl command: {e}")))?;

    let mut iter = tokens.into_iter().peekable();
    match iter.next() {
        Some(first) if first == "curl" => {}
        _ => return Err(ApiError::Validation("input does not start with 'curl'".into())),
    }

    let mut method: Option<HttpMethod> = None;
    let mut url: Option<String> = None;
    let mut headers = Vec::new();
    let mut basic_auth: Option<(String, String)> = None;
    let mut data_parts: Vec<String> = Vec::new();
    let mut form_fields: Vec<ParamEntry> = Vec::new();
    let mut explicit_form = false;

    while let Some(tok) = iter.next() {
        match tok.as_str() {
            "-X" | "--request" => {
                if let Some(m) = iter.next() {
                    method = parse_method(&m);
                }
            }
            "-H" | "--header" => {
                if let Some(h) = iter.next() {
                    if let Some((k, v)) = h.split_once(':') {
                        headers.push(HeaderEntry { key: k.trim().to_string(), value: v.trim().to_string(), enabled: true });
                    }
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-ascii" => {
                if let Some(d) = iter.next() {
                    data_parts.push(d);
                }
            }
            "--data-urlencode" => {
                if let Some(d) = iter.next() {
                    data_parts.push(d);
                }
            }
            "-F" | "--form" => {
                explicit_form = true;
                if let Some(f) = iter.next() {
                    if let Some((k, v)) = f.split_once('=') {
                        form_fields.push(ParamEntry { key: k.to_string(), value: v.to_string(), enabled: true });
                    }
                }
            }
            "-u" | "--user" => {
                if let Some(u) = iter.next() {
                    if let Some((user, pass)) = u.split_once(':') {
                        basic_auth = Some((user.to_string(), pass.to_string()));
                    } else {
                        basic_auth = Some((u, String::new()));
                    }
                }
            }
            "--url" => {
                if let Some(u) = iter.next() {
                    url = Some(u);
                }
            }
            "-G" | "--get" => {
                method = Some(HttpMethod::Get);
            }
            // Flags that take no value and don't affect request shape.
            "-k" | "--insecure" | "-s" | "--silent" | "-v" | "--verbose" | "-i" | "--include" | "--compressed"
            | "-L" | "--location" => {}
            other if other.starts_with('-') => {
                // Unknown flag — if the next token looks like its argument
                // (doesn't itself start with '-') and isn't the URL, skip it
                // too so we don't misinterpret an option's value as the URL.
                if let Some(next) = iter.peek() {
                    if !next.starts_with('-') && !looks_like_url(next) {
                        iter.next();
                    }
                }
            }
            _ => {
                if url.is_none() {
                    url = Some(tok);
                }
            }
        }
    }

    let url = url.ok_or_else(|| ApiError::Validation("no URL found in curl command".into()))?;

    let body = if explicit_form && !form_fields.is_empty() {
        RequestBody::FormUrlEncoded { fields: form_fields }
    } else if !data_parts.is_empty() {
        let joined = data_parts.join("&");
        let content_type = headers.iter().find(|h| h.key.eq_ignore_ascii_case("content-type")).map(|h| h.value.clone());
        match content_type.as_deref() {
            Some(ct) if ct.contains("json") => RequestBody::Json { content: joined },
            Some(ct) if ct.contains("xml") => RequestBody::Xml { content: joined },
            _ if looks_like_json(&joined) => RequestBody::Json { content: joined },
            _ => RequestBody::Text { content: joined },
        }
    } else {
        RequestBody::None
    };

    // -d implies POST unless the method was set explicitly or -G was passed.
    let method = method.unwrap_or_else(|| {
        if matches!(body, RequestBody::None) { HttpMethod::Get } else { HttpMethod::Post }
    });

    let auth = match basic_auth {
        Some((username, password)) => AuthConfig::Basic { username, password },
        None => headers
            .iter()
            .find(|h| h.key.eq_ignore_ascii_case("authorization"))
            .and_then(|h| h.value.strip_prefix("Bearer ").map(|t| AuthConfig::Bearer { token: t.trim().to_string() }))
            .unwrap_or(AuthConfig::None),
    };

    let name = derive_name(&url);

    Ok(ImportedRequest {
        folder_path: Vec::new(),
        name,
        method,
        url,
        headers,
        params: Vec::new(),
        body,
        auth,
        saved_responses: Vec::new(),
    })
}

fn parse_method(s: &str) -> Option<HttpMethod> {
    match s.to_uppercase().as_str() {
        "GET" => Some(HttpMethod::Get),
        "POST" => Some(HttpMethod::Post),
        "PUT" => Some(HttpMethod::Put),
        "PATCH" => Some(HttpMethod::Patch),
        "DELETE" => Some(HttpMethod::Delete),
        "OPTIONS" => Some(HttpMethod::Options),
        "HEAD" => Some(HttpMethod::Head),
        _ => None,
    }
}

fn looks_like_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

fn looks_like_json(s: &str) -> bool {
    let t = s.trim();
    (t.starts_with('{') && t.ends_with('}')) || (t.starts_with('[') && t.ends_with(']'))
}

fn derive_name(url: &str) -> String {
    let without_query = url.split('?').next().unwrap_or(url);
    let last_segment = without_query.trim_end_matches('/').rsplit('/').next().unwrap_or(url);
    if last_segment.is_empty() { "Imported Request".to_string() } else { last_segment.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_get() {
        let req = parse("curl https://api.example.com/users").unwrap();
        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(req.url, "https://api.example.com/users");
    }

    #[test]
    fn parses_post_with_json_body_and_headers() {
        let cmd = r#"curl -X POST https://api.example.com/users -H "Content-Type: application/json" -d '{"name":"a"}'"#;
        let req = parse(cmd).unwrap();
        assert_eq!(req.method, HttpMethod::Post);
        assert!(req.headers.iter().any(|h| h.key == "Content-Type"));
        match req.body {
            RequestBody::Json { content } => assert_eq!(content, r#"{"name":"a"}"#),
            other => panic!("expected Json body, got {other:?}"),
        }
    }

    #[test]
    fn parses_basic_auth() {
        let req = parse("curl -u alice:secret https://api.example.com/x").unwrap();
        match req.auth {
            AuthConfig::Basic { username, password } => {
                assert_eq!(username, "alice");
                assert_eq!(password, "secret");
            }
            other => panic!("expected Basic auth, got {other:?}"),
        }
    }

    #[test]
    fn data_implies_post_method() {
        let req = parse(r#"curl https://api.example.com/x -d "a=b""#).unwrap();
        assert_eq!(req.method, HttpMethod::Post);
    }
}
