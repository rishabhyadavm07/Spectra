use spectra_core::model::{new_id, ResponseDto, SavedResponse};
use spectra_core::{AppContext, ApiResult};

pub async fn list_saved_responses(
    ctx: &AppContext,
    workspace_id: String,
    request_id: String,
) -> ApiResult<Vec<SavedResponse>> {
    ctx.storage.list_saved_responses(&workspace_id, &request_id).await
}

pub async fn save_response(
    ctx: &AppContext,
    workspace_id: String,
    request_id: String,
    name: String,
    response: ResponseDto,
) -> ApiResult<SavedResponse> {
    let saved =
        SavedResponse { id: new_id(), workspace_id, request_id, name, response, saved_at: chrono::Utc::now() };
    ctx.storage.save_saved_response(&saved).await?;
    Ok(saved)
}

pub async fn delete_saved_response(ctx: &AppContext, workspace_id: String, id: String) -> ApiResult<()> {
    ctx.storage.delete_saved_response(&workspace_id, &id).await
}
