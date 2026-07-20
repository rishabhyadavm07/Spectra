use crate::dto::CreateWorkspaceInput;
use spectra_core::model::{new_id, AuthConfig, Workspace};
use spectra_core::{AppContext, ApiResult};

pub async fn list_workspaces(ctx: &AppContext) -> ApiResult<Vec<Workspace>> {
    ctx.storage.list_workspaces().await
}

pub async fn create_workspace(ctx: &AppContext, input: CreateWorkspaceInput) -> ApiResult<Workspace> {
    let ws = Workspace {
        id: new_id(),
        name: input.name,
        active_environment_id: None,
        auth: AuthConfig::None,
        created_at: chrono::Utc::now(),
    };
    ctx.storage.save_workspace(&ws).await?;
    Ok(ws)
}

pub async fn open_workspace(ctx: &AppContext, id: String) -> ApiResult<Workspace> {
    ctx.storage.get_workspace(&id).await
}

pub async fn set_active_environment(
    ctx: &AppContext,
    workspace_id: String,
    environment_id: Option<String>,
) -> ApiResult<Workspace> {
    let mut ws = ctx.storage.get_workspace(&workspace_id).await?;
    ws.active_environment_id = environment_id;
    ctx.storage.save_workspace(&ws).await?;
    Ok(ws)
}

pub async fn set_workspace_auth(ctx: &AppContext, workspace_id: String, auth: AuthConfig) -> ApiResult<Workspace> {
    let mut ws = ctx.storage.get_workspace(&workspace_id).await?;
    ws.auth = auth;
    ctx.storage.save_workspace(&ws).await?;
    Ok(ws)
}
