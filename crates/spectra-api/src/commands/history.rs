use spectra_core::model::{new_id, HistoryEntry, Request, RequestRun};
use spectra_core::{request_engine, AppContext, ApiResult};

pub async fn list_history(ctx: &AppContext, workspace_id: String) -> ApiResult<Vec<HistoryEntry>> {
    ctx.storage.list_history(&workspace_id).await
}

/// The last 5 history entries for one request, newest-first — a filtered
/// view over the same on-disk history `list_history` returns, not a
/// separate retention policy (see `Storage::list_history_for_request`).
pub async fn list_history_for_request(ctx: &AppContext, workspace_id: String, request_id: String) -> ApiResult<Vec<HistoryEntry>> {
    ctx.storage.list_history_for_request(&workspace_id, &request_id).await
}

pub async fn delete_history_entry(ctx: &AppContext, workspace_id: String, id: String) -> ApiResult<()> {
    ctx.storage.delete_history_entry(&workspace_id, &id).await
}

/// Re-runs the exact request as it was at the time of the history entry
/// (the snapshot), not the live (possibly since-edited) saved request.
pub async fn replay_history_entry(ctx: &AppContext, workspace_id: String, id: String) -> ApiResult<RequestRun> {
    let entry = ctx.storage.get_history_entry(&workspace_id, &id).await?;
    let vars: std::collections::HashMap<String, String> = Default::default();
    let result = request_engine::execute(&ctx.http, &ctx.oauth_store, &entry.request_snapshot, &vars).await;

    let new_entry = HistoryEntry {
        id: new_id(),
        workspace_id: workspace_id.clone(),
        request_id: entry.request_id.clone(),
        request_snapshot: entry.request_snapshot.clone(),
        response: result.as_ref().ok().cloned(),
        error: result.as_ref().err().map(|e| e.to_string()),
        executed_at: chrono::Utc::now(),
    };
    let _ = ctx.storage.save_history_entry(&new_entry).await;

    result.map(|response| RequestRun { history_id: new_entry.id, response })
}

/// Saves the history snapshot as a new standalone request in the workspace,
/// letting a one-off/replayed call become a reusable saved request.
pub async fn convert_history_to_request(
    ctx: &AppContext,
    workspace_id: String,
    id: String,
    target_folder_id: Option<String>,
) -> ApiResult<Request> {
    let entry = ctx.storage.get_history_entry(&workspace_id, &id).await?;
    let now = chrono::Utc::now();
    let mut req = entry.request_snapshot;
    req.id = new_id();
    req.folder_id = target_folder_id;
    req.created_at = now;
    req.updated_at = now;
    ctx.storage.save_request(&req).await?;
    Ok(req)
}
