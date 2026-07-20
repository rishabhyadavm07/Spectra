use spectra_core::import::{self, curl, har, openapi, postman, ImportFormat, ImportedRequest};
use spectra_core::model::{new_id, Folder, Request, SavedResponse};
use spectra_core::{AppContext, ApiError, ApiResult};
use std::collections::HashMap;

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct ImportResult {
    pub imported_count: usize,
    pub request_ids: Vec<String>,
    pub saved_response_count: usize,
}

pub async fn import_collection(
    ctx: &AppContext,
    workspace_id: String,
    content: String,
    format: Option<String>,
) -> ApiResult<ImportResult> {
    let format = match format.as_deref() {
        Some("curl") => ImportFormat::Curl,
        Some("postman") => ImportFormat::PostmanCollection,
        Some("openapi") => ImportFormat::OpenApi,
        Some("har") => ImportFormat::Har,
        Some(other) => return Err(ApiError::Validation(format!("unknown import format: {other}"))),
        None => import::detect_format(&content)
            .ok_or_else(|| ApiError::Validation("could not detect import format from content".into()))?,
    };

    let requests: Vec<ImportedRequest> = match format {
        ImportFormat::Curl => vec![curl::parse(&content)?],
        ImportFormat::PostmanCollection => postman::parse(&content)?.requests,
        ImportFormat::OpenApi => openapi::parse(&content)?.requests,
        ImportFormat::Har => har::parse(&content)?.requests,
    };

    if requests.is_empty() {
        return Err(ApiError::Validation("no requests found to import".into()));
    }

    let mut folder_cache: HashMap<Vec<String>, Option<String>> = HashMap::new();
    folder_cache.insert(Vec::new(), None);

    let mut request_ids = Vec::with_capacity(requests.len());
    let mut saved_response_count = 0usize;
    let now = chrono::Utc::now();

    for imported in requests {
        let folder_id = resolve_folder_path(ctx, &workspace_id, &imported.folder_path, &mut folder_cache).await?;

        let req = Request {
            id: new_id(),
            workspace_id: workspace_id.clone(),
            folder_id,
            name: imported.name,
            method: imported.method,
            url: imported.url,
            headers: imported.headers,
            params: imported.params,
            body: imported.body,
            auth: imported.auth,
            notes: String::new(),
            created_at: now,
            updated_at: now,
        };
        ctx.storage.save_request(&req).await?;

        for imported_response in imported.saved_responses {
            let saved = SavedResponse {
                id: new_id(),
                workspace_id: workspace_id.clone(),
                request_id: req.id.clone(),
                name: imported_response.name,
                response: imported_response.response,
                saved_at: now,
            };
            ctx.storage.save_saved_response(&saved).await?;
            saved_response_count += 1;
        }

        request_ids.push(req.id);
    }

    Ok(ImportResult { imported_count: request_ids.len(), request_ids, saved_response_count })
}

async fn resolve_folder_path(
    ctx: &AppContext,
    workspace_id: &str,
    path: &[String],
    cache: &mut HashMap<Vec<String>, Option<String>>,
) -> ApiResult<Option<String>> {
    let mut current_parent_id = None;
    let mut current_path = Vec::new();

    for segment in path {
        current_path.push(segment.clone());
        if let Some(id) = cache.get(&current_path) {
            current_parent_id = id.clone();
            continue;
        }

        let folder = Folder {
            id: new_id(),
            workspace_id: workspace_id.to_string(),
            parent_folder_id: current_parent_id,
            name: segment.clone(),
            auth: spectra_core::model::AuthConfig::InheritFromParent,
            created_at: chrono::Utc::now(),
        };
        ctx.storage.save_folder(&folder).await?;
        cache.insert(current_path.clone(), Some(folder.id.clone()));
        current_parent_id = Some(folder.id);
    }
    
    Ok(current_parent_id)
}
