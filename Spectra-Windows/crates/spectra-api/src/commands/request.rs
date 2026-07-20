use crate::dto::{CreateRequestInput, SendRequestInput, SetAuthInput, SetBodyInput, SetHeadersInput, SetParamsInput};
use spectra_core::model::{new_id, AuthConfig, HistoryEntry, HttpMethod, Request, RequestSummary, ResponseDto, VariableValue};
use spectra_core::{request_engine, AppContext, ApiResult};
use std::collections::HashMap;

pub async fn list_requests(
    ctx: &AppContext,
    workspace_id: String,
    folder_id: Option<String>,
) -> ApiResult<Vec<RequestSummary>> {
    let all = ctx.storage.list_requests(&workspace_id).await?;
    Ok(all
        .iter()
        .filter(|r| folder_id.is_none() || r.folder_id == folder_id)
        .map(RequestSummary::from)
        .collect())
}

pub async fn open_request(ctx: &AppContext, id: String) -> ApiResult<Request> {
    ctx.storage.find_request(&id).await
}

pub async fn create_request(ctx: &AppContext, input: CreateRequestInput) -> ApiResult<Request> {
    let now = chrono::Utc::now();
    let req = Request {
        id: new_id(),
        workspace_id: input.workspace_id,
        folder_id: input.folder_id,
        name: input.name,
        method: input.method,
        url: input.url,
        headers: Vec::new(),
        params: Vec::new(),
        body: Default::default(),
        // New requests inherit auth from their folder/workspace by default,
        // matching Postman/Insomnia's collection-auth UX — a user opts out
        // by explicitly choosing "None" in the Auth tab, rather than every
        // new request starting with no auth and silently ignoring whatever
        // the workspace/folder already has configured.
        auth: AuthConfig::InheritFromParent,
        notes: String::new(),
        created_at: now,
        updated_at: now,
    };
    ctx.storage.save_request(&req).await?;
    Ok(req)
}

pub async fn delete_request(ctx: &AppContext, id: String) -> ApiResult<()> {
    ctx.storage.find_and_delete_request(&id).await
}

pub async fn set_method(ctx: &AppContext, id: String, method: HttpMethod) -> ApiResult<Request> {
    let mut req = ctx.storage.find_request(&id).await?;
    req.method = method;
    req.updated_at = chrono::Utc::now();
    ctx.storage.save_request(&req).await?;
    Ok(req)
}

pub async fn set_url(ctx: &AppContext, id: String, url: String) -> ApiResult<Request> {
    let mut req = ctx.storage.find_request(&id).await?;
    req.url = url;
    req.updated_at = chrono::Utc::now();
    ctx.storage.save_request(&req).await?;
    Ok(req)
}

pub async fn set_name(ctx: &AppContext, id: String, name: String) -> ApiResult<Request> {
    let mut req = ctx.storage.find_request(&id).await?;
    req.name = name;
    req.updated_at = chrono::Utc::now();
    ctx.storage.save_request(&req).await?;
    Ok(req)
}

/// Max words allowed in a request's notes. Enforced here by truncation
/// (keeping only the first 50 words) rather than rejecting the call outright
/// — the GUI already prevents typing past 50 words with a live counter (see
/// RequestTabs.tsx), so this server-side cap exists purely as a defensive
/// backstop for callers that bypass that UI (e.g. an MCP client), and a
/// silent truncation is far less surprising to an automated caller than a
/// hard validation error would be.
const MAX_NOTES_WORDS: usize = 50;

fn truncate_to_word_limit(notes: &str, max_words: usize) -> String {
    let words: Vec<&str> = notes.split_whitespace().collect();
    if words.len() <= max_words {
        notes.to_string()
    } else {
        words[..max_words].join(" ")
    }
}

pub async fn set_notes(ctx: &AppContext, request_id: String, notes: String) -> ApiResult<Request> {
    let mut req = ctx.storage.find_request(&request_id).await?;
    req.notes = truncate_to_word_limit(&notes, MAX_NOTES_WORDS);
    req.updated_at = chrono::Utc::now();
    ctx.storage.save_request(&req).await?;
    Ok(req)
}

pub async fn set_headers(ctx: &AppContext, input: SetHeadersInput) -> ApiResult<Request> {
    let mut req = ctx.storage.find_request(&input.request_id).await?;
    req.headers = input.headers;
    req.updated_at = chrono::Utc::now();
    ctx.storage.save_request(&req).await?;
    Ok(req)
}

pub async fn set_params(ctx: &AppContext, input: SetParamsInput) -> ApiResult<Request> {
    let mut req = ctx.storage.find_request(&input.request_id).await?;
    req.params = input.params;
    req.updated_at = chrono::Utc::now();
    ctx.storage.save_request(&req).await?;
    Ok(req)
}

pub async fn set_body(ctx: &AppContext, input: SetBodyInput) -> ApiResult<Request> {
    let mut req = ctx.storage.find_request(&input.request_id).await?;
    req.body = input.body;
    req.updated_at = chrono::Utc::now();
    ctx.storage.save_request(&req).await?;
    Ok(req)
}

pub async fn set_auth(ctx: &AppContext, input: SetAuthInput) -> ApiResult<Request> {
    let mut req = ctx.storage.find_request(&input.request_id).await?;
    req.auth = input.auth;
    req.updated_at = chrono::Utc::now();
    ctx.storage.save_request(&req).await?;
    Ok(req)
}

pub async fn get_auth(ctx: &AppContext, request_id: String) -> ApiResult<spectra_core::model::AuthConfig> {
    Ok(ctx.storage.find_request(&request_id).await?.auth)
}

/// The auth that will actually be used when this request is sent — resolves
/// `InheritFromParent` per `resolve_effective_auth` rather than returning it
/// literally. Powers a "this request inherits Bearer auth from Workspace X"
/// style hint in the Auth tab, distinct from `get_auth`'s raw stored value.
pub async fn get_effective_auth(
    ctx: &AppContext,
    request_id: String,
) -> ApiResult<AuthConfig> {
    let req = ctx.storage.find_request(&request_id).await?;
    resolve_effective_auth(ctx, &req).await
}

pub async fn clear_auth(ctx: &AppContext, request_id: String) -> ApiResult<Request> {
    let mut req = ctx.storage.find_request(&request_id).await?;
    req.auth = Default::default();
    req.updated_at = chrono::Utc::now();
    ctx.storage.save_request(&req).await?;
    Ok(req)
}

pub async fn save_request(ctx: &AppContext, id: String) -> ApiResult<Request> {
    ctx.storage.find_request(&id).await
}

/// Resolves `AuthConfig::InheritFromParent` by walking folder -> ... ->
/// folder -> workspace, using the first concretely-configured auth found
/// (i.e. anything other than `None`/`InheritFromParent` itself). Falls back
/// to `AuthConfig::None` if nothing up the chain has one configured, or if
/// the chain is somehow broken (a folder_id that no longer resolves) —
/// sending "no auth" is always a safe default, never a hard error, so a
/// stale reference can't block a send.
///
/// This is the one place inheritance is resolved; `request_engine` and the
/// exporters never see `InheritFromParent` themselves (see the comments on
/// their `AuthConfig` match arms).
pub async fn resolve_effective_auth(ctx: &AppContext, req: &Request) -> ApiResult<AuthConfig> {
    let mut auth = req.auth.clone();

    if auth == AuthConfig::InheritFromParent {
        if let Some(folder_id) = &req.folder_id {
            let mut current = folder_id.clone();
            loop {
                let folder = ctx.storage.get_folder(&req.workspace_id, &current).await?;
                if folder.auth != AuthConfig::InheritFromParent {
                    auth = folder.auth;
                    break;
                }
                match folder.parent_folder_id {
                    Some(pid) => current = pid,
                    None => break,
                }
            }
        }

        if auth == AuthConfig::InheritFromParent {
            let ws = ctx.storage.get_workspace(&req.workspace_id).await?;
            auth = ws.auth;
        }
    }

    if let AuthConfig::SavedAuth { saved_auth_id } = auth {
        return match ctx.storage.get_saved_auth(&req.workspace_id, &saved_auth_id).await {
            Ok(saved) => Ok(saved.auth),
            Err(_) => Ok(AuthConfig::None),
        };
    }

    Ok(auth)
}

/// Returns a copy of `req` with its `auth` field replaced by the resolved
/// effective auth — the shape `request_engine`/history/OAuth commands need,
/// since they all operate on a `&Request` rather than an `AuthConfig` alone.
pub(crate) async fn with_effective_auth(ctx: &AppContext, req: Request) -> ApiResult<Request> {
    let auth = resolve_effective_auth(ctx, &req).await?;
    Ok(Request { auth, ..req })
}

/// Resolves the active environment's variables into plain strings for
/// substitution, fetching secret plaintext from Keychain just-in-time. This
/// is the only place secret values are ever pulled into process memory as
/// plaintext — they're never persisted anywhere outside the Keychain.
pub(crate) async fn resolve_env_vars(
    ctx: &AppContext,
    workspace_id: &str,
    environment_id: &Option<String>,
) -> ApiResult<HashMap<String, String>> {
    let env_vars = match environment_id {
        Some(env_id) => ctx.storage.get_environment(workspace_id, env_id).await?.variables,
        None => Default::default(),
    };
    let mut resolved = HashMap::with_capacity(env_vars.len());
    for (name, value) in env_vars {
        let plain = match value {
            VariableValue::Plain { value } => value,
            VariableValue::Secret { keychain_account } => ctx.secrets.get(&keychain_account)?.unwrap_or_default(),
        };
        resolved.insert(name, plain);
    }
    Ok(resolved)
}

/// The core send loop: resolve variables → build request → fire → return
/// response. Identical whether called from spectra-tauri or spectra-mcp.
/// Every execution — success or failure — is recorded to History (PRD
/// Section 11: "every executed request"), so this is the single place that
/// both GUI Send and an MCP send_request call end up leaving a trail.
pub async fn send_request(ctx: &AppContext, input: SendRequestInput) -> ApiResult<ResponseDto> {
    let req = ctx.storage.find_request(&input.request_id).await?;
    let vars = resolve_env_vars(ctx, &req.workspace_id, &input.environment_id).await?;
    // The history snapshot below keeps `req.auth` as `InheritFromParent` if
    // that's what's stored — only the actual outgoing call uses the
    // resolved auth — so replaying history later re-resolves against
    // whatever the folder/workspace auth is *at replay time*, the same way
    // {{variables}} in a replayed request re-resolve against the live
    // environment rather than a frozen value.
    let effective = with_effective_auth(ctx, req.clone()).await?;
    let result = request_engine::execute(&ctx.http, &ctx.oauth_store, &effective, &vars).await;

    let entry = HistoryEntry {
        id: new_id(),
        workspace_id: req.workspace_id.clone(),
        request_id: req.id.clone(),
        request_snapshot: req.clone(),
        response: result.as_ref().ok().cloned(),
        error: result.as_ref().err().map(|e| e.to_string()),
        executed_at: chrono::Utc::now(),
    };
    // History is best-effort: a storage failure here shouldn't hide the
    // actual send result from the caller.
    let _ = ctx.storage.save_history_entry(&entry).await;

    result
}

/// Computes the headers Spectra would send without performing the HTTP call
/// — lets the GUI/MCP show auto-generated headers (auth, content-type) the
/// way Postman surfaces them, without hiding what actually goes on the wire.
pub async fn preview_headers(ctx: &AppContext, input: SendRequestInput) -> ApiResult<Vec<(String, String)>> {
    let req = ctx.storage.find_request(&input.request_id).await?;
    let vars = resolve_env_vars(ctx, &req.workspace_id, &input.environment_id).await?;
    let effective = with_effective_auth(ctx, req).await?;
    request_engine::preview_headers(&ctx.oauth_store, &effective, &vars).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_to_word_limit_leaves_short_notes_untouched() {
        let notes = "A short note about this endpoint.";
        assert_eq!(truncate_to_word_limit(notes, MAX_NOTES_WORDS), notes);
    }

    #[test]
    fn truncate_to_word_limit_caps_at_exactly_max_words() {
        let words: Vec<String> = (0..50).map(|i| format!("word{i}")).collect();
        let notes = words.join(" ");
        assert_eq!(truncate_to_word_limit(&notes, MAX_NOTES_WORDS), notes);
    }

    #[test]
    fn truncate_to_word_limit_drops_words_past_the_cap() {
        let words: Vec<String> = (0..75).map(|i| format!("word{i}")).collect();
        let notes = words.join(" ");
        let truncated = truncate_to_word_limit(&notes, MAX_NOTES_WORDS);
        let truncated_words: Vec<&str> = truncated.split_whitespace().collect();
        assert_eq!(truncated_words.len(), 50);
        assert_eq!(truncated_words, words[..50]);
    }

    #[test]
    fn truncate_to_word_limit_handles_empty_and_whitespace_only() {
        assert_eq!(truncate_to_word_limit("", MAX_NOTES_WORDS), "");
        assert_eq!(truncate_to_word_limit("   ", MAX_NOTES_WORDS), "   ");
    }

    /// Old on-disk `Request` JSON (pre-`notes` field) must still deserialize
    /// cleanly, with `notes` defaulting to an empty string — the same
    /// `#[serde(default)]` guarantee this project already relies on for
    /// `Workspace`/`Folder`'s `auth` field (see HANDOFF.md).
    #[test]
    fn request_without_notes_field_deserializes_with_empty_default() {
        let old_json = r#"{
            "id": "req1",
            "workspace_id": "ws1",
            "folder_id": null,
            "name": "Legacy Request",
            "method": "GET",
            "url": "https://example.com",
            "headers": [],
            "params": [],
            "body": { "kind": "None" },
            "auth": { "type": "None" },
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;
        let req: Request = serde_json::from_str(old_json).expect("old JSON without notes must still deserialize");
        assert_eq!(req.notes, "");
    }
}
