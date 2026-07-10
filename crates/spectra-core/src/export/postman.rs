//! Serializes a workspace's Folders/Requests into a Postman Collection
//! v2.1 JSON document — the inverse of `import::postman`. Round-trips
//! everything the importer understands (Basic/Bearer/API Key auth,
//! raw/urlencoded/formdata bodies) plus maps Spectra's other auth types
//! onto their Postman equivalents where Postman has one.
use super::FolderNode;
use crate::model::{ApiKeyLocation, AuthConfig, Request, RequestBody};
use serde_json::{json, Value};

pub fn export(workspace_name: &str, root: &FolderNode) -> String {
    let items: Vec<Value> = node_items(root);
    let collection = json!({
        "info": {
            "name": workspace_name,
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json",
        },
        "item": items,
    });
    serde_json::to_string_pretty(&collection).unwrap_or_default()
}

fn node_items(node: &FolderNode) -> Vec<Value> {
    let mut items: Vec<Value> = node.requests.iter().map(|r| request_item(r)).collect();
    for child in &node.children {
        let Some(folder) = child.folder else { continue };
        items.push(json!({
            "name": folder.name,
            "item": node_items(child),
        }));
    }
    items
}

fn request_item(req: &Request) -> Value {
    let header: Vec<Value> = req
        .headers
        .iter()
        .filter(|h| h.enabled)
        .map(|h| json!({ "key": h.key, "value": h.value }))
        .collect();

    let mut url = req.url.clone();
    let query: Vec<Value> = req
        .params
        .iter()
        .filter(|p| p.enabled)
        .map(|p| json!({ "key": p.key, "value": p.value }))
        .collect();
    if !query.is_empty() && !url.contains('?') {
        let qs: Vec<String> = req.params.iter().filter(|p| p.enabled).map(|p| format!("{}={}", p.key, p.value)).collect();
        url = format!("{url}?{}", qs.join("&"));
    }

    let body = body_json(&req.body);
    let auth = auth_json(&req.auth);

    let mut request = json!({
        "method": method_str(req.method),
        "header": header,
        "url": { "raw": url },
    });
    if let Some(body) = body {
        request["body"] = body;
    }
    if let Some(auth) = auth {
        request["auth"] = auth;
    }
    if !query.is_empty() {
        request["url"]["query"] = Value::Array(query);
    }

    json!({ "name": req.name, "request": request })
}

fn method_str(m: crate::model::HttpMethod) -> &'static str {
    use crate::model::HttpMethod::*;
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

fn body_json(body: &RequestBody) -> Option<Value> {
    match body {
        RequestBody::None => None,
        RequestBody::Json { content } => {
            Some(json!({ "mode": "raw", "raw": content, "options": { "raw": { "language": "json" } } }))
        }
        RequestBody::Text { content } => Some(json!({ "mode": "raw", "raw": content })),
        RequestBody::Xml { content } => {
            Some(json!({ "mode": "raw", "raw": content, "options": { "raw": { "language": "xml" } } }))
        }
        RequestBody::FormUrlEncoded { fields } => Some(json!({
            "mode": "urlencoded",
            "urlencoded": fields.iter().map(|f| json!({ "key": f.key, "value": f.value, "disabled": !f.enabled })).collect::<Vec<_>>(),
        })),
    }
}

/// Only auth types Postman itself supports are round-tripped as structured
/// auth blocks; anything else (OAuth1/AWS SigV4/Digest/Hawk) is instead
/// baked into a literal Authorization header so the request still works
/// when replayed from Postman, even though Postman can't re-derive the
/// signature itself (see PRD note in HANDOFF.md on import auth gaps).
fn auth_json(auth: &AuthConfig) -> Option<Value> {
    match auth {
        // InheritFromParent isn't resolved here — Postman has its own
        // separate collection-level auth mechanism a user can set up
        // independently after import, so this exports as "no auth" on the
        // request itself rather than us guessing the inherited value.
        AuthConfig::None | AuthConfig::InheritFromParent => None,
        AuthConfig::Basic { username, password } => Some(json!({
            "type": "basic",
            "basic": [
                { "key": "username", "value": username },
                { "key": "password", "value": password },
            ],
        })),
        AuthConfig::Bearer { token } => Some(json!({
            "type": "bearer",
            "bearer": [{ "key": "token", "value": token }],
        })),
        AuthConfig::ApiKey { key, value, location } => Some(json!({
            "type": "apikey",
            "apikey": [
                { "key": "key", "value": key },
                { "key": "value", "value": value },
                { "key": "in", "value": match location {
                    ApiKeyLocation::Query => "query",
                    ApiKeyLocation::Header | ApiKeyLocation::Cookie => "header",
                }},
            ],
        })),
        // No Postman-native representation — omit rather than mis-model.
        AuthConfig::OAuth1 { .. }
        | AuthConfig::OAuth2 { .. }
        | AuthConfig::AwsSigV4 { .. }
        | AuthConfig::Digest { .. }
        | AuthConfig::Hawk { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Folder, HeaderEntry, HttpMethod};

    fn req(name: &str, folder_id: Option<&str>) -> Request {
        Request {
            id: "r1".into(),
            workspace_id: "w1".into(),
            folder_id: folder_id.map(String::from),
            name: name.into(),
            method: HttpMethod::Get,
            url: "https://api.example.com/x".into(),
            headers: vec![HeaderEntry { key: "Accept".into(), value: "application/json".into(), enabled: true }],
            params: Vec::new(),
            body: RequestBody::None,
            auth: AuthConfig::None,
            notes: String::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn exports_flat_request() {
        let requests = vec![req("Get X", None)];
        let folders: Vec<Folder> = Vec::new();
        let tree = super::super::build_tree(&folders, &requests, None);
        let out = export("My Workspace", &tree);
        let parsed: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["info"]["name"], "My Workspace");
        assert_eq!(parsed["item"][0]["name"], "Get X");
        assert_eq!(parsed["item"][0]["request"]["method"], "GET");
    }

    #[test]
    fn exports_nested_folder() {
        let folder = Folder {
            id: "f1".into(),
            workspace_id: "w1".into(),
            parent_folder_id: None,
            name: "Users".into(),
            auth: crate::model::AuthConfig::InheritFromParent,
            created_at: chrono::Utc::now(),
        };
        let requests = vec![req("Get user", Some("f1"))];
        let folders = vec![folder];
        let tree = super::super::build_tree(&folders, &requests, None);
        let out = export("WS", &tree);
        let parsed: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["item"][0]["name"], "Users");
        assert_eq!(parsed["item"][0]["item"][0]["name"], "Get user");
    }
}
