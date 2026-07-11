//! Thin Tauri command wrappers. Each function here does argument marshaling
//! only and calls straight into spectra-api — no business logic lives here.
//! Exception: `automation_tab_ready` near the bottom, which is GUI-only
//! automation glue rather than a spectra-api command — see the module doc
//! on `crate::automation` for why.

use crate::automation::{AutomationState, TabReadyReport};
use spectra_api::commands::{environment, export, folder, history, import, oauth, request, saved_response, workspace, settings};
use spectra_api::dto::{
    CreateRequestInput, CreateWorkspaceInput, EnvironmentOutput, OrphanedSecret, SendRequestInput, SetAuthInput,
    SetBodyInput, SetHeadersInput, SetParamsInput, VariableInput,
};
use spectra_core::model::{
    AuthConfig, Folder, HistoryEntry, HttpMethod, NamedOAuthToken, OAuthStatus, PendingUserAction, Request,
    RequestRun, RequestSummary, ResponseDto, SavedResponse, Workspace,
};
use spectra_core::AppContext;
use std::collections::HashMap;
use tauri::State;

type CmdResult<T> = Result<T, String>;

fn map_err<T>(r: spectra_core::ApiResult<T>) -> CmdResult<T> {
    r.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_workspaces(ctx: State<'_, AppContext>) -> CmdResult<Vec<Workspace>> {
    map_err(workspace::list_workspaces(&ctx).await)
}

#[tauri::command]
pub async fn create_workspace(ctx: State<'_, AppContext>, name: String) -> CmdResult<Workspace> {
    map_err(workspace::create_workspace(&ctx, CreateWorkspaceInput { name }).await)
}

#[tauri::command]
pub async fn open_workspace(ctx: State<'_, AppContext>, id: String) -> CmdResult<Workspace> {
    map_err(workspace::open_workspace(&ctx, id).await)
}

#[tauri::command]
pub async fn list_requests(
    ctx: State<'_, AppContext>,
    workspace_id: String,
    folder_id: Option<String>,
) -> CmdResult<Vec<RequestSummary>> {
    map_err(request::list_requests(&ctx, workspace_id, folder_id).await)
}

#[tauri::command]
pub async fn open_request(ctx: State<'_, AppContext>, id: String) -> CmdResult<Request> {
    map_err(request::open_request(&ctx, id).await)
}

#[tauri::command]
pub async fn create_request(
    ctx: State<'_, AppContext>,
    workspace_id: String,
    folder_id: Option<String>,
    name: String,
    method: HttpMethod,
    url: String,
) -> CmdResult<Request> {
    map_err(request::create_request(
        &ctx,
        CreateRequestInput { workspace_id, folder_id, name, method, url },
    ).await)
}

#[tauri::command]
pub async fn delete_request(ctx: State<'_, AppContext>, id: String) -> CmdResult<()> {
    map_err(request::delete_request(&ctx, id).await)
}

#[tauri::command]
pub async fn set_method(ctx: State<'_, AppContext>, id: String, method: HttpMethod) -> CmdResult<Request> {
    map_err(request::set_method(&ctx, id, method).await)
}

#[tauri::command]
pub async fn set_url(ctx: State<'_, AppContext>, id: String, url: String) -> CmdResult<Request> {
    map_err(request::set_url(&ctx, id, url).await)
}

#[tauri::command]
pub async fn set_name(ctx: State<'_, AppContext>, id: String, name: String) -> CmdResult<Request> {
    map_err(request::set_name(&ctx, id, name).await)
}

#[tauri::command]
pub async fn set_notes(ctx: State<'_, AppContext>, request_id: String, notes: String) -> CmdResult<Request> {
    map_err(request::set_notes(&ctx, request_id, notes).await)
}

#[tauri::command]
pub async fn set_headers(
    ctx: State<'_, AppContext>,
    request_id: String,
    headers: Vec<spectra_core::model::HeaderEntry>,
) -> CmdResult<Request> {
    map_err(request::set_headers(&ctx, SetHeadersInput { request_id, headers }).await)
}

#[tauri::command]
pub async fn set_params(
    ctx: State<'_, AppContext>,
    request_id: String,
    params: Vec<spectra_core::model::ParamEntry>,
) -> CmdResult<Request> {
    map_err(request::set_params(&ctx, SetParamsInput { request_id, params }).await)
}

#[tauri::command]
pub async fn set_body(
    ctx: State<'_, AppContext>,
    request_id: String,
    body: spectra_core::model::RequestBody,
) -> CmdResult<Request> {
    map_err(request::set_body(&ctx, SetBodyInput { request_id, body }).await)
}

#[tauri::command]
pub async fn set_auth(ctx: State<'_, AppContext>, request_id: String, auth: AuthConfig) -> CmdResult<Request> {
    map_err(request::set_auth(&ctx, SetAuthInput { request_id, auth }).await)
}

#[tauri::command]
pub async fn get_auth(ctx: State<'_, AppContext>, request_id: String) -> CmdResult<AuthConfig> {
    map_err(request::get_auth(&ctx, request_id).await)
}

#[tauri::command]
pub async fn get_effective_auth(ctx: State<'_, AppContext>, request_id: String) -> CmdResult<AuthConfig> {
    map_err(request::get_effective_auth(&ctx, request_id).await)
}

#[tauri::command]
pub async fn clear_auth(ctx: State<'_, AppContext>, request_id: String) -> CmdResult<Request> {
    map_err(request::clear_auth(&ctx, request_id).await)
}

#[tauri::command]
pub async fn list_auth_types() -> Vec<spectra_api::dto::AuthTypeDescriptor> {
    spectra_api::dto::list_auth_types()
}

#[tauri::command]
pub async fn send_request(
    ctx: State<'_, AppContext>,
    request_id: String,
    environment_id: Option<String>,
) -> CmdResult<ResponseDto> {
    map_err(request::send_request(&ctx, SendRequestInput { request_id, environment_id }).await)
}

#[tauri::command]
pub async fn preview_headers(
    ctx: State<'_, AppContext>,
    request_id: String,
    environment_id: Option<String>,
) -> CmdResult<Vec<(String, String)>> {
    map_err(request::preview_headers(&ctx, SendRequestInput { request_id, environment_id }).await)
}

#[tauri::command]
pub async fn clear_cookies(ctx: State<'_, AppContext>) -> CmdResult<()> {
    ctx.cookie_store.clear();
    Ok(())
}

#[tauri::command]
pub async fn start_oauth_flow(ctx: State<'_, AppContext>, request_id: String) -> CmdResult<PendingUserAction> {
    map_err(oauth::start_oauth_flow(&ctx, request_id).await)
}

#[tauri::command]
pub async fn finish_oauth_flow(ctx: State<'_, AppContext>, url: String) -> CmdResult<()> {
    map_err(oauth::finish_oauth_flow(&ctx, url).await)
}

#[tauri::command]
pub async fn get_oauth_status(ctx: State<'_, AppContext>, request_id: String) -> CmdResult<OAuthStatus> {
    map_err(oauth::get_oauth_status(&ctx, request_id).await)
}

#[tauri::command]
pub async fn cancel_oauth_flow(ctx: State<'_, AppContext>, request_id: String) -> CmdResult<()> {
    map_err(oauth::cancel_oauth_flow(&ctx, request_id).await)
}

#[tauri::command]
pub async fn fetch_oauth_token(
    ctx: State<'_, AppContext>,
    request_id: String,
    name: Option<String>,
) -> CmdResult<NamedOAuthToken> {
    map_err(oauth::fetch_oauth_token(&ctx, request_id, name).await)
}

#[tauri::command]
pub async fn list_oauth_tokens(ctx: State<'_, AppContext>, request_id: String) -> CmdResult<Vec<NamedOAuthToken>> {
    map_err(oauth::list_oauth_tokens(&ctx, request_id).await)
}

#[tauri::command]
pub async fn select_oauth_token(ctx: State<'_, AppContext>, request_id: String, name: String) -> CmdResult<()> {
    map_err(oauth::select_oauth_token(&ctx, request_id, name).await)
}

#[tauri::command]
pub async fn delete_oauth_token(ctx: State<'_, AppContext>, request_id: String, name: String) -> CmdResult<()> {
    map_err(oauth::delete_oauth_token(&ctx, request_id, name).await)
}

#[tauri::command]
pub async fn list_environments(ctx: State<'_, AppContext>, workspace_id: String) -> CmdResult<Vec<EnvironmentOutput>> {
    map_err(environment::list_environments(&ctx, workspace_id).await)
}

#[tauri::command]
pub async fn create_environment(
    ctx: State<'_, AppContext>,
    workspace_id: String,
    name: String,
    variables: HashMap<String, VariableInput>,
) -> CmdResult<EnvironmentOutput> {
    map_err(environment::create_environment(&ctx, workspace_id, name, variables).await)
}

#[tauri::command]
pub async fn update_environment(
    ctx: State<'_, AppContext>,
    workspace_id: String,
    id: String,
    name: String,
    variables: HashMap<String, VariableInput>,
) -> CmdResult<EnvironmentOutput> {
    map_err(environment::update_environment(&ctx, workspace_id, id, name, variables).await)
}

#[tauri::command]
pub async fn delete_environment(ctx: State<'_, AppContext>, workspace_id: String, id: String) -> CmdResult<()> {
    map_err(environment::delete_environment(&ctx, workspace_id, id).await)
}

#[tauri::command]
pub async fn check_secrets_health(ctx: State<'_, AppContext>, workspace_id: String) -> CmdResult<Vec<OrphanedSecret>> {
    map_err(environment::check_secrets_health(&ctx, workspace_id).await)
}

#[tauri::command]
pub async fn set_active_environment(
    ctx: State<'_, AppContext>,
    workspace_id: String,
    environment_id: Option<String>,
) -> CmdResult<Workspace> {
    map_err(workspace::set_active_environment(&ctx, workspace_id, environment_id).await)
}

#[tauri::command]
pub async fn set_workspace_auth(ctx: State<'_, AppContext>, workspace_id: String, auth: AuthConfig) -> CmdResult<Workspace> {
    map_err(workspace::set_workspace_auth(&ctx, workspace_id, auth).await)
}

#[tauri::command]
pub async fn list_folders(ctx: State<'_, AppContext>, workspace_id: String) -> CmdResult<Vec<Folder>> {
    map_err(folder::list_folders(&ctx, workspace_id).await)
}

#[tauri::command]
pub async fn create_folder(
    ctx: State<'_, AppContext>,
    workspace_id: String,
    parent_folder_id: Option<String>,
    name: String,
) -> CmdResult<Folder> {
    map_err(folder::create_folder(&ctx, workspace_id, parent_folder_id, name).await)
}

#[tauri::command]
pub async fn set_folder_auth(ctx: State<'_, AppContext>, workspace_id: String, id: String, auth: AuthConfig) -> CmdResult<Folder> {
    map_err(folder::set_folder_auth(&ctx, workspace_id, id, auth).await)
}

#[tauri::command]
pub async fn rename_folder(ctx: State<'_, AppContext>, workspace_id: String, id: String, name: String) -> CmdResult<Folder> {
    map_err(folder::rename_folder(&ctx, workspace_id, id, name).await)
}

#[tauri::command]
pub async fn move_folder(
    ctx: State<'_, AppContext>,
    workspace_id: String,
    id: String,
    new_parent_id: Option<String>,
) -> CmdResult<Folder> {
    map_err(folder::move_folder(&ctx, workspace_id, id, new_parent_id).await)
}

#[tauri::command]
pub async fn delete_folder(ctx: State<'_, AppContext>, workspace_id: String, id: String) -> CmdResult<()> {
    map_err(folder::delete_folder(&ctx, workspace_id, id).await)
}

#[tauri::command]
pub async fn move_request(ctx: State<'_, AppContext>, request_id: String, target_folder_id: Option<String>) -> CmdResult<()> {
    map_err(folder::move_request(&ctx, request_id, target_folder_id).await)
}

#[tauri::command]
pub async fn list_history(ctx: State<'_, AppContext>, workspace_id: String) -> CmdResult<Vec<HistoryEntry>> {
    map_err(history::list_history(&ctx, workspace_id).await)
}

#[tauri::command]
pub async fn delete_history_entry(ctx: State<'_, AppContext>, workspace_id: String, id: String) -> CmdResult<()> {
    map_err(history::delete_history_entry(&ctx, workspace_id, id).await)
}

#[tauri::command]
pub async fn list_history_for_request(ctx: State<'_, AppContext>, workspace_id: String, request_id: String) -> CmdResult<Vec<HistoryEntry>> {
    map_err(history::list_history_for_request(&ctx, workspace_id, request_id).await)
}

#[tauri::command]
pub async fn replay_history_entry(
    ctx: State<'_, AppContext>,
    workspace_id: String,
    id: String,
) -> CmdResult<RequestRun> {
    map_err(history::replay_history_entry(&ctx, workspace_id, id).await)
}

#[tauri::command]
pub async fn convert_history_to_request(
    ctx: State<'_, AppContext>,
    workspace_id: String,
    id: String,
    target_folder_id: Option<String>,
) -> CmdResult<Request> {
    map_err(history::convert_history_to_request(&ctx, workspace_id, id, target_folder_id).await)
}

#[tauri::command]
pub async fn list_saved_responses(
    ctx: State<'_, AppContext>,
    workspace_id: String,
    request_id: String,
) -> CmdResult<Vec<SavedResponse>> {
    map_err(saved_response::list_saved_responses(&ctx, workspace_id, request_id).await)
}

#[tauri::command]
pub async fn save_response(
    ctx: State<'_, AppContext>,
    workspace_id: String,
    request_id: String,
    name: String,
    response: ResponseDto,
) -> CmdResult<SavedResponse> {
    map_err(saved_response::save_response(&ctx, workspace_id, request_id, name, response).await)
}

#[tauri::command]
pub async fn delete_saved_response(ctx: State<'_, AppContext>, workspace_id: String, id: String) -> CmdResult<()> {
    map_err(saved_response::delete_saved_response(&ctx, workspace_id, id).await)
}

#[tauri::command]
pub async fn import_collection(
    ctx: State<'_, AppContext>,
    workspace_id: String,
    content: String,
    format: Option<String>,
) -> CmdResult<import::ImportResult> {
    map_err(import::import_collection(&ctx, workspace_id, content, format).await)
}

#[tauri::command]
pub async fn export_workspace(ctx: State<'_, AppContext>, workspace_id: String, format: String) -> CmdResult<String> {
    map_err(export::export_workspace(&ctx, workspace_id, format).await)
}

#[tauri::command]
pub async fn export_request(ctx: State<'_, AppContext>, request_id: String, format: String) -> CmdResult<String> {
    map_err(export::export_request(&ctx, request_id, format).await)
}

/// Called by the frontend once it has opened/switched to the tab for
/// `request_id` (in response to the `automation://prepare-request` event)
/// and either rendered a response/error or given up trying — i.e. it's now
/// safe for the automation IPC server to act (screenshot, or just report
/// status for a focus-only call). Not a spectra-api command: this is purely
/// "wake up the waiting automation task with a status report," it doesn't
/// read or write any domain state.
#[tauri::command]
pub async fn automation_tab_ready(state: State<'_, AutomationState>, report: TabReadyReport) -> CmdResult<()> {
    state.signal_ready(report).await;
    Ok(())
}

#[tauri::command]
pub async fn automation_search_ready(state: State<'_, AutomationState>, report: crate::automation::SearchReadyReport) -> CmdResult<()> {
    state.signal_search_ready(report).await;
    Ok(())
}

#[tauri::command]
pub async fn get_settings(ctx: State<'_, AppContext>) -> CmdResult<spectra_core::model::AppSettings> {
    map_err(settings::get_settings(&ctx).await)
}

#[tauri::command]
pub async fn save_settings(ctx: State<'_, AppContext>, settings: spectra_core::model::AppSettings) -> CmdResult<()> {
    map_err(settings::save_settings(&ctx, settings).await)
}
