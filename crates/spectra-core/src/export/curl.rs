//! Serializes a single request into a `curl` command — the inverse of
//! `import::curl`. Folder structure has no meaning for cURL, so callers
//! export one request at a time (unlike Postman/OpenAPI, which export a
//! whole workspace tree).
use crate::model::{AuthConfig, HttpMethod, Request, RequestBody};

pub fn export(req: &Request) -> String {
    let mut parts = vec!["curl".to_string()];

    if req.method != HttpMethod::Get {
        parts.push("-X".to_string());
        parts.push(method_str(req.method).to_string());
    }

    for h in req.headers.iter().filter(|h| h.enabled) {
        parts.push("-H".to_string());
        parts.push(shell_quote(&format!("{}: {}", h.key, h.value)));
    }

    match &req.auth {
        AuthConfig::Basic { username, password } => {
            parts.push("-u".to_string());
            parts.push(shell_quote(&format!("{username}:{password}")));
        }
        AuthConfig::Bearer { token } => {
            parts.push("-H".to_string());
            parts.push(shell_quote(&format!("Authorization: Bearer {token}")));
        }
        AuthConfig::ApiKey { key, value, .. } => {
            parts.push("-H".to_string());
            parts.push(shell_quote(&format!("{key}: {value}")));
        }
        // No single-header/flag representation in plain curl — the request
        // still exports (URL/headers/body), just without the signature.
        // InheritFromParent isn't resolved here either — a lone exported
        // curl command has no folder/workspace context to inherit from.
        AuthConfig::None
        | AuthConfig::InheritFromParent
        | AuthConfig::OAuth1 { .. }
        | AuthConfig::OAuth2 { .. }
        | AuthConfig::AwsSigV4 { .. }
        | AuthConfig::Digest { .. }
        | AuthConfig::Hawk { .. } => {}
    }

    match &req.body {
        RequestBody::None => {}
        RequestBody::Json { content } => {
            parts.push("-H".to_string());
            parts.push(shell_quote("Content-Type: application/json"));
            parts.push("-d".to_string());
            parts.push(shell_quote(content));
        }
        RequestBody::Xml { content } => {
            parts.push("-H".to_string());
            parts.push(shell_quote("Content-Type: application/xml"));
            parts.push("-d".to_string());
            parts.push(shell_quote(content));
        }
        RequestBody::Text { content } => {
            parts.push("-d".to_string());
            parts.push(shell_quote(content));
        }
        RequestBody::FormUrlEncoded { fields } => {
            for f in fields.iter().filter(|f| f.enabled) {
                parts.push("-d".to_string());
                parts.push(shell_quote(&format!("{}={}", f.key, f.value)));
            }
        }
    }

    let mut url = req.url.clone();
    let enabled_params: Vec<_> = req.params.iter().filter(|p| p.enabled).collect();
    if !enabled_params.is_empty() && !url.contains('?') {
        let qs: Vec<String> = enabled_params.iter().map(|p| format!("{}={}", p.key, p.value)).collect();
        url = format!("{url}?{}", qs.join("&"));
    }
    parts.push(shell_quote(&url));

    parts.join(" ")
}

fn method_str(m: HttpMethod) -> &'static str {
    use HttpMethod::*;
    match m {
        Get => "GET",
        Post => "POST",
        Put => "PUT",
        Patch => "PATCH",
        Delete => "DELETE",
        Options => "OPTIONS",
        Head => "HEAD",
    }
}

/// Wraps a value in single quotes, escaping any embedded single quote the
/// POSIX-portable way (`'...'"'"'...'`) — safe for pasting into bash/zsh.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r#"'"'"'"#))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::HeaderEntry;

    fn base_req() -> Request {
        Request {
            id: "r1".into(),
            workspace_id: "w1".into(),
            folder_id: None,
            name: "Get X".into(),
            method: HttpMethod::Get,
            url: "https://api.example.com/x".into(),
            headers: Vec::new(),
            params: Vec::new(),
            body: RequestBody::None,
            auth: AuthConfig::None,
            notes: String::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn exports_simple_get() {
        let out = export(&base_req());
        assert_eq!(out, "curl 'https://api.example.com/x'");
    }

    #[test]
    fn exports_post_with_json_body() {
        let mut req = base_req();
        req.method = HttpMethod::Post;
        req.body = RequestBody::Json { content: r#"{"a":1}"#.to_string() };
        let out = export(&req);
        assert!(out.contains("-X POST"));
        assert!(out.contains("Content-Type: application/json"));
        assert!(out.contains(r#"{"a":1}"#));
    }

    #[test]
    fn exports_bearer_auth_as_header() {
        let mut req = base_req();
        req.auth = AuthConfig::Bearer { token: "abc123".into() };
        let out = export(&req);
        assert!(out.contains("Authorization: Bearer abc123"));
    }

    #[test]
    fn exports_enabled_headers_only() {
        let mut req = base_req();
        req.headers = vec![
            HeaderEntry { key: "X-On".into(), value: "1".into(), enabled: true },
            HeaderEntry { key: "X-Off".into(), value: "2".into(), enabled: false },
        ];
        let out = export(&req);
        assert!(out.contains("X-On: 1"));
        assert!(!out.contains("X-Off"));
    }
}
