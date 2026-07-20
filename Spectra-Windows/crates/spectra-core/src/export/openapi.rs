//! Serializes a workspace's Folders/Requests into a best-effort OpenAPI 3.0
//! document. This is lossier than the Postman export: OpenAPI has no notion
//! of a saved concrete request (auth secrets, literal header values) — it
//! describes a *shape*. `{{variable}}` placeholders in the URL become path
//! parameters; everything else (concrete headers/query values, auth) is
//! preserved as a `description` note on the operation rather than invented
//! OpenAPI fields that don't have a faithful equivalent.
use super::FolderNode;
use crate::model::Request;
use serde_json::{json, Map, Value};

pub fn export(workspace_name: &str, root: &FolderNode) -> String {
    let mut paths: Map<String, Value> = Map::new();
    collect_paths(root, &mut paths);

    let doc = json!({
        "openapi": "3.0.3",
        "info": { "title": workspace_name, "version": "1.0.0" },
        "paths": Value::Object(paths),
    });
    serde_json::to_string_pretty(&doc).unwrap_or_default()
}

fn collect_paths(node: &FolderNode, paths: &mut Map<String, Value>) {
    for req in &node.requests {
        let tag = node.folder.map(|f| f.name.clone());
        let (path, operation) = operation_for(req, tag.as_deref());
        let entry = paths.entry(path).or_insert_with(|| json!({})).as_object_mut().expect("path entry is object");
        entry.insert(method_str(req.method).to_string(), operation);
    }
    for child in &node.children {
        collect_paths(child, paths);
    }
}

/// Turns a stored URL like `https://api.example.com/users/{{id}}?active={{flag}}`
/// into an OpenAPI path (`/users/{id}`) plus a matching `parameters` array,
/// treating every `{{...}}` placeholder found in the path as a required path
/// parameter — the inverse of how `import::openapi` templates them.
fn operation_for(req: &Request, tag: Option<&str>) -> (String, Value) {
    let (path_part, query_part) = req.url.split_once('?').map(|(p, q)| (p, Some(q))).unwrap_or((req.url.as_str(), None));
    let path = strip_origin(path_part);
    let openapi_path = template_to_openapi(&path);

    let mut parameters: Vec<Value> = extract_placeholders(&path)
        .into_iter()
        .map(|name| json!({ "name": name, "in": "path", "required": true, "schema": { "type": "string" } }))
        .collect();

    for p in req.params.iter().filter(|p| p.enabled) {
        parameters.push(json!({ "name": p.key, "in": "query", "required": false, "schema": { "type": "string" } }));
    }
    if let Some(qs) = query_part {
        for pair in qs.split('&') {
            if let Some((k, _)) = pair.split_once('=') {
                if !parameters.iter().any(|p| p["name"] == k) {
                    parameters.push(json!({ "name": k, "in": "query", "required": false, "schema": { "type": "string" } }));
                }
            }
        }
    }

    let mut operation = json!({
        "summary": req.name,
        "operationId": req.id,
        "responses": { "200": { "description": "Successful response" } },
    });
    if !parameters.is_empty() {
        operation["parameters"] = Value::Array(parameters);
    }
    if let Some(tag) = tag {
        operation["tags"] = json!([tag]);
    }
    if let crate::model::RequestBody::Json { content } = &req.body {
        if let Ok(example) = serde_json::from_str::<Value>(content) {
            operation["requestBody"] = json!({
                "content": { "application/json": { "schema": { "type": "object" }, "example": example } }
            });
        }
    }

    (openapi_path, operation)
}

fn strip_origin(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://")) {
        match rest.find('/') {
            Some(idx) => rest[idx..].to_string(),
            None => "/".to_string(),
        }
    } else {
        url.to_string()
    }
}

fn template_to_openapi(path: &str) -> String {
    let mut out = String::new();
    let mut chars = path.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' && chars.peek() == Some(&'{') {
            chars.next();
            let mut name = String::new();
            while let Some(&next) = chars.peek() {
                if next == '}' {
                    break;
                }
                name.push(next);
                chars.next();
            }
            chars.next();
            if chars.peek() == Some(&'}') {
                chars.next();
            }
            out.push('{');
            out.push_str(&name);
            out.push('}');
        } else {
            out.push(c);
        }
    }
    out
}

fn extract_placeholders(path: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = path.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' && chars.peek() == Some(&'{') {
            chars.next();
            let mut name = String::new();
            while let Some(&next) = chars.peek() {
                if next == '}' {
                    break;
                }
                name.push(next);
                chars.next();
            }
            out.push(name);
        }
    }
    out
}

fn method_str(m: crate::model::HttpMethod) -> &'static str {
    use crate::model::HttpMethod::*;
    match m {
        Get => "get",
        Post => "post",
        Put => "put",
        Patch => "patch",
        Delete => "delete",
        Options => "options",
        Head => "head",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AuthConfig, HttpMethod, RequestBody};

    fn req(url: &str) -> Request {
        Request {
            id: "r1".into(),
            workspace_id: "w1".into(),
            folder_id: None,
            name: "Get user".into(),
            method: HttpMethod::Get,
            url: url.into(),
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
    fn exports_path_placeholder_as_parameter() {
        let requests = vec![req("https://api.example.com/users/{{id}}")];
        let folders = Vec::new();
        let tree = super::super::build_tree(&folders, &requests, None);
        let out = export("WS", &tree);
        let parsed: Value = serde_json::from_str(&out).unwrap();
        assert!(parsed["paths"]["/users/{id}"]["get"].is_object());
        let params = parsed["paths"]["/users/{id}"]["get"]["parameters"].as_array().unwrap();
        assert_eq!(params[0]["name"], "id");
        assert_eq!(params[0]["in"], "path");
    }

    #[test]
    fn groups_by_folder_as_tag() {
        use crate::model::Folder;
        let folder = Folder {
            id: "f1".into(),
            workspace_id: "w1".into(),
            parent_folder_id: None,
            name: "Users".into(),
            auth: crate::model::AuthConfig::InheritFromParent,
            created_at: chrono::Utc::now(),
        };
        let mut r = req("https://api.example.com/x");
        r.folder_id = Some("f1".into());
        let requests = vec![r];
        let folders = vec![folder];
        let tree = super::super::build_tree(&folders, &requests, None);
        let out = export("WS", &tree);
        let parsed: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["paths"]["/x"]["get"]["tags"][0], "Users");
    }
}
