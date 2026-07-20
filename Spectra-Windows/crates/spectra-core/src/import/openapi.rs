use super::{ImportedRequest, ImportedTree};
use crate::error::{ApiError, ApiResult};
use crate::model::{AuthConfig, HeaderEntry, HttpMethod, ParamEntry, RequestBody};
use serde_json::Value;

/// OpenAPI 3.x and Swagger 2.0 (JSON or YAML). One request is generated per
/// operation (path + method), grouped into folders by the operation's first
/// tag (untagged operations land at the top level). Path/query parameters
/// become `{{param_name}}`-style placeholders in the URL/params list rather
/// than resolved values — there's no "real" value in a spec, only a shape,
/// so this deliberately produces templated requests the user fills in
/// (matching how Postman's own OpenAPI import behaves). `$ref` pointers
/// (same-document only, e.g. `#/components/parameters/PageParam`) are
/// dereferenced for both parameters and request body schemas — see
/// `resolve_ref`. Path-item-level `parameters` are merged into every
/// operation under that path, with an operation's own parameter of the same
/// name taking precedence.
pub fn parse(input: &str) -> ApiResult<ImportedTree> {
    let doc: Value = if input.trim_start().starts_with('{') {
        serde_json::from_str(input).map_err(|e| ApiError::Validation(format!("invalid OpenAPI JSON: {e}")))?
    } else {
        serde_yaml::from_str::<serde_yaml::Value>(input)
            .map_err(|e| ApiError::Validation(format!("invalid OpenAPI YAML: {e}")))
            .and_then(|y| serde_json::to_value(y).map_err(|e| ApiError::Validation(e.to_string())))?
    };

    let is_swagger2 = doc.get("swagger").and_then(|v| v.as_str()).map(|s| s.starts_with('2')).unwrap_or(false);

    let base_url = if is_swagger2 {
        let scheme = doc
            .get("schemes")
            .and_then(|s| s.as_array())
            .and_then(|a| a.first())
            .and_then(|v| v.as_str())
            .unwrap_or("https");
        let host = doc.get("host").and_then(|v| v.as_str()).unwrap_or("{{host}}");
        let base_path = doc.get("basePath").and_then(|v| v.as_str()).unwrap_or("");
        format!("{scheme}://{host}{base_path}")
    } else {
        doc.get("servers")
            .and_then(|s| s.as_array())
            .and_then(|a| a.first())
            .and_then(|s| s.get("url"))
            .and_then(|u| u.as_str())
            .unwrap_or("{{base_url}}")
            .trim_end_matches('/')
            .to_string()
    };

    let paths = doc.get("paths").and_then(|p| p.as_object()).ok_or_else(|| {
        ApiError::Validation("no 'paths' object found — not a valid OpenAPI/Swagger document".into())
    })?;

    let mut requests = Vec::new();
    for (path, path_item) in paths {
        let Some(path_item) = path_item.as_object() else { continue };
        // Parameters declared directly on the path item (rather than per
        // operation) apply to every method under it — merged in before the
        // operation's own parameters so an operation can still override one
        // by name (matches how tooling like Postman's own importer treats
        // this ambiguity).
        let path_level_params = resolve_ref_array(&doc, path_item.get("parameters"));

        for method_str in ["get", "post", "put", "patch", "delete", "options", "head"] {
            let Some(operation) = path_item.get(method_str) else { continue };
            let method = parse_method(method_str);
            let name = operation
                .get("summary")
                .and_then(|v| v.as_str())
                .or_else(|| operation.get("operationId").and_then(|v| v.as_str()))
                .map(str::to_string)
                .unwrap_or_else(|| format!("{} {}", method_str.to_uppercase(), path));

            let folder_path = operation
                .get("tags")
                .and_then(|t| t.as_array())
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .map(|tag| vec![tag.to_string()])
                .unwrap_or_default();

            let mut headers = Vec::new();
            let mut params = Vec::new();
            let mut url_path = path.clone();
            let mut seen_param_names = std::collections::HashSet::new();

            let op_params = resolve_ref_array(&doc, operation.get("parameters"));
            for p in op_params.iter().chain(path_level_params.iter()) {
                let Some(param_name) = p.get("name").and_then(|v| v.as_str()) else { continue };
                if !seen_param_names.insert(param_name.to_string()) {
                    continue; // operation-level parameter already took precedence
                }
                let location = p.get("in").and_then(|v| v.as_str()).unwrap_or("query");
                match location {
                    "path" => {
                        url_path = url_path.replace(&format!("{{{param_name}}}"), &format!("{{{{{param_name}}}}}"));
                    }
                    "query" => params.push(ParamEntry {
                        key: param_name.to_string(),
                        value: format!("{{{{{param_name}}}}}"),
                        enabled: p.get("required").and_then(|v| v.as_bool()).unwrap_or(false),
                    }),
                    "header" => headers.push(HeaderEntry {
                        key: param_name.to_string(),
                        value: format!("{{{{{param_name}}}}}"),
                        enabled: p.get("required").and_then(|v| v.as_bool()).unwrap_or(false),
                    }),
                    _ => {}
                }
            }

            let body = extract_request_body_example(&doc, operation);

            requests.push(ImportedRequest {
                folder_path,
                name,
                method,
                url: format!("{base_url}{url_path}"),
                headers,
                params,
                body,
                auth: AuthConfig::None,
                saved_responses: Vec::new(),
            });
        }
    }

    Ok(ImportedTree { requests })
}

/// Resolves a JSON Pointer `$ref` (e.g. `#/components/parameters/PageParam`
/// or the Swagger 2 equivalent `#/parameters/PageParam`) against the root
/// document. Only same-document refs are supported — external file refs
/// (`other.yaml#/...`) have no meaning for a single pasted spec and are left
/// unresolved (the raw `$ref` object is returned as-is, so callers that only
/// look for `name`/`in`/`schema` fields simply find nothing and skip it
/// rather than panicking).
fn resolve_ref<'a>(doc: &'a Value, value: &'a Value) -> &'a Value {
    resolve_ref_depth(doc, value, 0)
}

fn resolve_ref_depth<'a>(doc: &'a Value, value: &'a Value, depth: u8) -> &'a Value {
    if depth > 16 {
        return value; // cycle guard — same-document specs shouldn't nest this deep
    }
    let Some(ref_path) = value.get("$ref").and_then(|r| r.as_str()) else {
        return value;
    };
    let Some(pointer) = ref_path.strip_prefix('#') else {
        return value; // external file ref — not resolvable from this document alone
    };
    match doc.pointer(pointer) {
        Some(target) => resolve_ref_depth(doc, target, depth + 1),
        None => value,
    }
}

/// Resolves an array of parameters (or path items), dereferencing any
/// `$ref` entries against the root document.
fn resolve_ref_array(doc: &Value, arr: Option<&Value>) -> Vec<Value> {
    arr.and_then(|v| v.as_array())
        .map(|items| items.iter().map(|item| resolve_ref(doc, item).clone()).collect())
        .unwrap_or_default()
}

fn parse_method(s: &str) -> HttpMethod {
    match s {
        "post" => HttpMethod::Post,
        "put" => HttpMethod::Put,
        "patch" => HttpMethod::Patch,
        "delete" => HttpMethod::Delete,
        "options" => HttpMethod::Options,
        "head" => HttpMethod::Head,
        _ => HttpMethod::Get,
    }
}

/// Best-effort extraction of a JSON request body placeholder from an
/// OpenAPI 3.x `requestBody.content.application/json.example`, or else a
/// property-name skeleton built from `schema` (dereferencing `$ref` against
/// the root document first) — good enough to give the user a starting shape
/// rather than an empty body for POST/PUT/PATCH operations.
fn extract_request_body_example(doc: &Value, operation: &Value) -> RequestBody {
    let request_body = resolve_ref(doc, match operation.get("requestBody") {
        Some(rb) => rb,
        None => return RequestBody::None,
    });
    let Some(json_content) = request_body.get("content").and_then(|c| c.get("application/json")) else {
        return RequestBody::None;
    };

    if let Some(example) = json_content.get("example") {
        return RequestBody::Json { content: serde_json::to_string_pretty(example).unwrap_or_default() };
    }

    if let Some(schema) = json_content.get("schema") {
        let skeleton = schema_skeleton(doc, schema, 0);
        return RequestBody::Json { content: serde_json::to_string_pretty(&skeleton).unwrap_or_default() };
    }

    RequestBody::Json { content: "{}".to_string() }
}

/// Builds a placeholder JSON value from an OpenAPI schema object: objects
/// become `{ property: placeholder }` for each declared property, arrays
/// become a single-element array of their item skeleton, and scalar types
/// become a `{{property_name}}`-style hint string. `$ref` is dereferenced at
/// every level so nested `components/schemas` definitions still produce a
/// real shape instead of an empty object.
fn schema_skeleton(doc: &Value, schema: &Value, depth: u8) -> Value {
    if depth > 8 {
        return serde_json::json!({});
    }
    let schema = resolve_ref(doc, schema);

    if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
        let mut obj = serde_json::Map::new();
        for (name, prop_schema) in props {
            obj.insert(name.clone(), scalar_or_nested(doc, prop_schema, name, depth));
        }
        return Value::Object(obj);
    }

    if schema.get("type").and_then(|t| t.as_str()) == Some("array") {
        if let Some(items) = schema.get("items") {
            return Value::Array(vec![schema_skeleton(doc, items, depth + 1)]);
        }
        return Value::Array(vec![]);
    }

    serde_json::json!({})
}

fn scalar_or_nested(doc: &Value, prop_schema: &Value, name: &str, depth: u8) -> Value {
    let prop_schema = resolve_ref(doc, prop_schema);
    match prop_schema.get("type").and_then(|t| t.as_str()) {
        Some("object") => schema_skeleton(doc, prop_schema, depth + 1),
        Some("array") => Value::Array(vec![
            prop_schema.get("items").map(|i| schema_skeleton(doc, i, depth + 1)).unwrap_or_else(|| serde_json::json!({}))
        ]),
        _ => Value::String(format!("{{{{{name}}}}}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_openapi3_paths() {
        let spec = r#"{
            "openapi": "3.0.0",
            "servers": [{ "url": "https://api.example.com" }],
            "paths": {
                "/users/{id}": {
                    "get": {
                        "summary": "Get user",
                        "parameters": [{ "name": "id", "in": "path", "required": true }]
                    }
                }
            }
        }"#;
        let tree = parse(spec).unwrap();
        assert_eq!(tree.requests.len(), 1);
        assert_eq!(tree.requests[0].name, "Get user");
        assert_eq!(tree.requests[0].url, "https://api.example.com/users/{{id}}");
        assert_eq!(tree.requests[0].method, HttpMethod::Get);
    }

    #[test]
    fn groups_by_first_tag() {
        let spec = r#"{
            "openapi": "3.0.0",
            "paths": {
                "/x": { "get": { "tags": ["Widgets"], "summary": "List" } }
            }
        }"#;
        let tree = parse(spec).unwrap();
        assert_eq!(tree.requests[0].folder_path, vec!["Widgets".to_string()]);
    }

    #[test]
    fn resolves_ref_parameter() {
        let spec = r##"{
            "openapi": "3.0.0",
            "servers": [{ "url": "https://api.example.com" }],
            "components": {
                "parameters": {
                    "PageParam": { "name": "page", "in": "query", "required": true }
                }
            },
            "paths": {
                "/items": {
                    "get": {
                        "summary": "List items",
                        "parameters": [{ "$ref": "#/components/parameters/PageParam" }]
                    }
                }
            }
        }"##;
        let tree = parse(spec).unwrap();
        assert_eq!(tree.requests[0].params.len(), 1);
        assert_eq!(tree.requests[0].params[0].key, "page");
        assert_eq!(tree.requests[0].params[0].value, "{{page}}");
        assert!(tree.requests[0].params[0].enabled);
    }

    #[test]
    fn resolves_ref_request_body_schema() {
        let spec = r##"{
            "openapi": "3.0.0",
            "servers": [{ "url": "https://api.example.com" }],
            "components": {
                "schemas": {
                    "NewUser": {
                        "type": "object",
                        "properties": { "name": { "type": "string" }, "age": { "type": "integer" } }
                    }
                }
            },
            "paths": {
                "/users": {
                    "post": {
                        "summary": "Create user",
                        "requestBody": {
                            "content": {
                                "application/json": { "schema": { "$ref": "#/components/schemas/NewUser" } }
                            }
                        }
                    }
                }
            }
        }"##;
        let tree = parse(spec).unwrap();
        match &tree.requests[0].body {
            RequestBody::Json { content } => {
                let parsed: Value = serde_json::from_str(content).unwrap();
                assert_eq!(parsed["name"], "{{name}}");
                assert_eq!(parsed["age"], "{{age}}");
            }
            other => panic!("expected Json body, got {other:?}"),
        }
    }

    #[test]
    fn path_level_parameters_apply_to_all_methods() {
        let spec = r#"{
            "openapi": "3.0.0",
            "servers": [{ "url": "https://api.example.com" }],
            "paths": {
                "/items/{id}": {
                    "parameters": [{ "name": "id", "in": "path", "required": true }],
                    "get": { "summary": "Get item" },
                    "delete": { "summary": "Delete item" }
                }
            }
        }"#;
        let tree = parse(spec).unwrap();
        assert_eq!(tree.requests.len(), 2);
        assert!(tree.requests.iter().all(|r| r.url == "https://api.example.com/items/{{id}}"));
    }
}
