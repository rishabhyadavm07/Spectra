use spectra_core::model::WorkspaceSavedAuth;
use spectra_core::{AppContext, ApiResult};

pub async fn list_saved_auths(ctx: &AppContext, workspace_id: String) -> ApiResult<Vec<WorkspaceSavedAuth>> {
    ctx.storage.list_saved_auths(&workspace_id).await
}

pub async fn get_saved_auth(ctx: &AppContext, workspace_id: String, id: String) -> ApiResult<WorkspaceSavedAuth> {
    ctx.storage.get_saved_auth(&workspace_id, &id).await
}

pub async fn save_saved_auth(ctx: &AppContext, auth: WorkspaceSavedAuth) -> ApiResult<()> {
    ctx.storage.save_saved_auth(&auth).await
}

pub async fn delete_saved_auth(ctx: &AppContext, workspace_id: String, id: String) -> ApiResult<()> {
    ctx.storage.delete_saved_auth(&workspace_id, &id).await
}
