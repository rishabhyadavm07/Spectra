use spectra_core::model::{new_id, AuthConfig, Folder};
use spectra_core::{AppContext, ApiResult};

pub async fn list_folders(ctx: &AppContext, workspace_id: String) -> ApiResult<Vec<Folder>> {
    ctx.storage.list_folders(&workspace_id).await
}

pub async fn create_folder(
    ctx: &AppContext,
    workspace_id: String,
    parent_folder_id: Option<String>,
    name: String,
) -> ApiResult<Folder> {
    let folder = Folder {
        id: new_id(),
        workspace_id,
        parent_folder_id,
        name,
        auth: AuthConfig::InheritFromParent,
        created_at: chrono::Utc::now(),
    };
    ctx.storage.save_folder(&folder).await?;
    Ok(folder)
}

pub async fn rename_folder(ctx: &AppContext, workspace_id: String, id: String, name: String) -> ApiResult<Folder> {
    let mut folder = ctx.storage.get_folder(&workspace_id, &id).await?;
    folder.name = name;
    ctx.storage.save_folder(&folder).await?;
    Ok(folder)
}

/// Sets a folder-level auth override every request (and sub-folder) under
/// it inherits by default, unless overridden further down the chain. Pass
/// `AuthConfig::InheritFromParent` to remove this folder's own override and
/// fall back to its parent folder/workspace instead.
pub async fn set_folder_auth(ctx: &AppContext, workspace_id: String, id: String, auth: spectra_core::model::AuthConfig) -> ApiResult<Folder> {
    let mut folder = ctx.storage.get_folder(&workspace_id, &id).await?;
    folder.auth = auth;
    ctx.storage.save_folder(&folder).await?;
    Ok(folder)
}

pub async fn move_folder(
    ctx: &AppContext,
    workspace_id: String,
    id: String,
    new_parent_id: Option<String>,
) -> ApiResult<Folder> {
    let mut folder = ctx.storage.get_folder(&workspace_id, &id).await?;
    folder.parent_folder_id = new_parent_id;
    ctx.storage.save_folder(&folder).await?;
    Ok(folder)
}

pub async fn delete_folder(ctx: &AppContext, workspace_id: String, id: String) -> ApiResult<()> {
    // Requests and sub-folders directly inside this folder move up to its
    // parent rather than being deleted — matches the PRD's "no data loss"
    // spirit for a destructive-sounding action the user might not expect
    // to cascade-delete their saved requests.
    let folder = ctx.storage.get_folder(&workspace_id, &id).await?;
    for mut req in ctx.storage.list_requests(&workspace_id).await? {
        if req.folder_id.as_deref() == Some(id.as_str()) {
            req.folder_id = folder.parent_folder_id.clone();
            ctx.storage.save_request(&req).await?;
        }
    }
    for mut child in ctx.storage.list_folders(&workspace_id).await? {
        if child.parent_folder_id.as_deref() == Some(id.as_str()) {
            child.parent_folder_id = folder.parent_folder_id.clone();
            ctx.storage.save_folder(&child).await?;
        }
    }
    ctx.storage.delete_folder(&workspace_id, &id).await
}

pub async fn move_request(ctx: &AppContext, request_id: String, target_folder_id: Option<String>) -> ApiResult<()> {
    let mut req = ctx.storage.find_request(&request_id).await?;
    req.folder_id = target_folder_id;
    req.updated_at = chrono::Utc::now();
    ctx.storage.save_request(&req).await
}
