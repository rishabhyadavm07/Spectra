//! MCP tool surface for Spectra. Every tool here is a thin wrapper over the
//! exact same `spectra-api` command functions `spectra-tauri` calls — this is
//! the sibling wrapper crate described in the PRD: an AI agent driving these
//! tools can never diverge in behavior from the GUI, because both end up
//! calling the same function in `spectra-api`.

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use spectra_api::commands::{environment, export, folder, history, import, oauth, request, saved_response, workspace};
use spectra_api::dto::{
    CreateRequestInput, CreateWorkspaceInput, SendRequestInput, SetAuthInput, SetBodyInput, SetHeadersInput,
    SetParamsInput, VariableInput,
};
use spectra_core::model::{AuthConfig, HeaderEntry, HttpMethod, ParamEntry, RequestBody, ResponseDto};
use spectra_core::AppContext;
use std::collections::HashMap;

/// Tool outputs are returned as JSON text rather than rmcp's structured
/// `Json<T>` wrapper: several of our result types are top-level arrays or
/// `()`, and the MCP spec requires a structured outputSchema's root type to
/// be `object` — arrays/units panic at tool-registration time. Serializing
/// to a JSON string ourselves sidesteps that constraint while still giving
/// the calling agent fully parseable JSON in the tool result text.
fn map_err<T: Serialize>(r: spectra_core::ApiResult<T>) -> Result<String, String> {
    r.map_err(|e| e.to_string()).and_then(|v| serde_json::to_string_pretty(&v).map_err(|e| e.to_string()))
}

fn to_json<T: Serialize>(v: &T) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|e| e.to_string())
}

/// Truncate `dto.body` to at most `max` bytes, appending a trailer if truncated.
fn truncate_body(dto: &mut ResponseDto, max: usize) {
    let total = dto.body.len();
    if total > max {
        // Find a safe UTF-8 boundary
        let mut end = max;
        while end > 0 && !dto.body.is_char_boundary(end) {
            end -= 1;
        }
        dto.body.truncate(end);
        dto.body.push_str(&format!(
            "\n... [TRUNCATED — showing first {} of {} bytes. Use analyze_response for targeted search.]",
            end, total
        ));
    }
}

#[derive(Clone)]
pub struct SpectraServer {
    ctx: AppContext,
    tool_router: ToolRouter<Self>,
}

// --- Tool parameter shapes -------------------------------------------------
// One struct per tool that takes more than zero arguments. Fields mirror the
// spectra-tauri command signatures 1:1 (see crates/spectra-tauri/src/commands.rs).

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateWorkspaceParams {
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WorkspaceIdParams {
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListRequestsParams {
    pub workspace_id: String,
    pub folder_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RequestIdParams {
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateRequestParams {
    pub workspace_id: String,
    pub folder_id: Option<String>,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetMethodParams {
    pub id: String,
    pub method: HttpMethod,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetUrlParams {
    pub id: String,
    pub url: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetNameParams {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetNotesParams {
    pub request_id: String,
    pub notes: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetHeadersParams {
    pub request_id: String,
    pub headers: Vec<HeaderEntry>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetParamsParams {
    pub request_id: String,
    pub params: Vec<ParamEntry>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetBodyParams {
    pub request_id: String,
    pub body: RequestBody,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetAuthParams {
    pub request_id: String,
    pub auth: AuthConfig,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RequestIdOnlyParams {
    pub request_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FetchOAuthTokenParams {
    pub request_id: String,
    /// Name to save this token under (e.g. "Prod Token"). Omit to auto-generate a timestamp-based name.
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OAuthTokenNameParams {
    pub request_id: String,
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendRequestParams {
    pub request_id: String,
    pub environment_id: Option<String>,
    /// If set, truncate the response body to this many bytes. Useful for large
    /// payloads that would blow an agent's token budget.
    pub max_body_bytes: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AutomationScreenshotParams {
    pub request_id: String,
    /// Absolute (or cwd-relative) file path to save the screenshot to, e.g. "/tmp/request.png".
    pub save_path: String,
    pub environment_id: Option<String>,
    /// If true, force the GUI to re-send the request even if it already has a response.
    pub force_send: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AutomationFocusParams {
    pub request_id: String,
    pub environment_id: Option<String>,
    /// If true, force the GUI to re-send the request even if it already has a response.
    pub force_send: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListEnvironmentsParams {
    pub workspace_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateEnvironmentParams {
    pub workspace_id: String,
    pub name: String,
    pub variables: HashMap<String, VariableInput>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateEnvironmentParams {
    pub workspace_id: String,
    pub id: String,
    pub name: String,
    pub variables: HashMap<String, VariableInput>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteEnvironmentParams {
    pub workspace_id: String,
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckSecretsHealthParams {
    pub workspace_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetActiveEnvironmentParams {
    pub workspace_id: String,
    pub environment_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetWorkspaceAuthParams {
    pub workspace_id: String,
    pub auth: AuthConfig,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListFoldersParams {
    pub workspace_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateFolderParams {
    pub workspace_id: String,
    pub parent_folder_id: Option<String>,
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetFolderAuthParams {
    pub workspace_id: String,
    pub id: String,
    pub auth: AuthConfig,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenameFolderParams {
    pub workspace_id: String,
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveFolderParams {
    pub workspace_id: String,
    pub id: String,
    pub new_parent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteFolderParams {
    pub workspace_id: String,
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveRequestParams {
    pub request_id: String,
    pub target_folder_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListHistoryParams {
    pub workspace_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HistoryEntryParams {
    pub workspace_id: String,
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListHistoryForRequestParams {
    pub workspace_id: String,
    pub request_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ConvertHistoryToRequestParams {
    pub workspace_id: String,
    pub id: String,
    pub target_folder_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListSavedResponsesParams {
    pub workspace_id: String,
    pub request_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SaveResponseParams {
    pub workspace_id: String,
    pub request_id: String,
    pub name: String,
    pub response: ResponseDto,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteSavedResponseParams {
    pub workspace_id: String,
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImportCollectionParams {
    pub workspace_id: String,
    pub content: String,
    /// One of "curl", "postman", "openapi", "har". Omit to auto-detect from content.
    pub format: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportWorkspaceParams {
    pub workspace_id: String,
    /// One of "postman", "openapi". cURL export applies to a single request — use export_request.
    pub format: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportRequestParams {
    pub request_id: String,
    /// Only "curl" is supported here. For a whole collection use export_workspace.
    pub format: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetUiLineNumbersParams {
    pub show: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzeResponseParams {
    pub request_id: String,
    pub query: String,
    pub environment_id: Option<String>,
    /// Max context characters to include around each match (default: 120).
    pub context_chars: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendAndScreenshotParams {
    pub request_id: String,
    /// Absolute file path to save the screenshot (e.g. "/tmp/shot.png").
    pub save_path: String,
    pub environment_id: Option<String>,
    /// If set, truncate the response body in the returned JSON.
    pub max_body_bytes: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchResponseParams {
    pub request_id: String,
    /// Text to search for in the response body (case-insensitive substring).
    pub query: String,
    /// Absolute file path to save the screenshot after scrolling to the match.
    pub save_path: String,
    pub environment_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QuickTestParams {
    pub url: String,
    /// HTTP method, defaults to GET.
    pub method: Option<HttpMethod>,
    /// Optional headers as key-value pairs.
    pub headers: Option<Vec<HeaderEntry>>,
    /// Optional request body.
    pub body: Option<RequestBody>,
    /// If provided, take a screenshot and save it here.
    pub save_path: Option<String>,
    /// Max response body bytes to return (default: 8192).
    pub max_body_bytes: Option<usize>,
}

#[tool_router(router = tool_router)]
impl SpectraServer {
    pub fn new(ctx: AppContext) -> Self {
        Self { ctx, tool_router: Self::tool_router() }
    }

    // --- Workspaces ---------------------------------------------------

    #[tool(description = "List all workspaces.")]
    async fn list_workspaces(&self) -> Result<String, String> {
        map_err(workspace::list_workspaces(&self.ctx).await)
    }

    #[tool(description = "Create a new workspace.")]
    async fn create_workspace(&self, params: Parameters<CreateWorkspaceParams>) -> Result<String, String> {
        map_err(workspace::create_workspace(&self.ctx, CreateWorkspaceInput { name: params.0.name }).await)
    }

    #[tool(description = "Open (fetch) a workspace by id.")]
    async fn open_workspace(&self, params: Parameters<WorkspaceIdParams>) -> Result<String, String> {
        map_err(workspace::open_workspace(&self.ctx, params.0.id).await)
    }

    #[tool(
        description = "Set (or clear, by omitting environment_id) the active environment for a workspace. Requests sent without an explicit environment_id use this one."
    )]
    async fn set_active_environment(
        &self,
        params: Parameters<SetActiveEnvironmentParams>,
    ) -> Result<String, String> {
        map_err(workspace::set_active_environment(&self.ctx, params.0.workspace_id, params.0.environment_id).await)
    }

    #[tool(
        description = "Set the workspace-level auth every request/folder in it inherits by default (via a request's auth being \"InheritFromParent\"), unless overridden by a folder or the request itself. This is the top of the inheritance chain."
    )]
    async fn set_workspace_auth(&self, params: Parameters<SetWorkspaceAuthParams>) -> Result<String, String> {
        map_err(workspace::set_workspace_auth(&self.ctx, params.0.workspace_id, params.0.auth).await)
    }

    // --- Requests -------------------------------------------------------

    #[tool(description = "List requests in a workspace, optionally filtered to one folder.")]
    async fn list_requests(&self, params: Parameters<ListRequestsParams>) -> Result<String, String> {
        map_err(request::list_requests(&self.ctx, params.0.workspace_id, params.0.folder_id).await)
    }

    #[tool(description = "Open (fetch) a request by id, including headers/params/body/auth.")]
    async fn open_request(&self, params: Parameters<RequestIdParams>) -> Result<String, String> {
        map_err(request::open_request(&self.ctx, params.0.id).await)
    }

    #[tool(description = "Create a new request (method + URL only; use set_headers/set_params/set_body/set_auth to fill in the rest).")]
    async fn create_request(&self, params: Parameters<CreateRequestParams>) -> Result<String, String> {
        let p = params.0;
        map_err(request::create_request(
            &self.ctx,
            CreateRequestInput { workspace_id: p.workspace_id, folder_id: p.folder_id, name: p.name, method: p.method, url: p.url },
        ).await)
    }

    #[tool(description = "Delete a request by id.")]
    async fn delete_request(&self, params: Parameters<RequestIdParams>) -> Result<String, String> {
        map_err(request::delete_request(&self.ctx, params.0.id).await)
    }

    #[tool(description = "Change a request's HTTP method.")]
    async fn set_method(&self, params: Parameters<SetMethodParams>) -> Result<String, String> {
        map_err(request::set_method(&self.ctx, params.0.id, params.0.method).await)
    }

    #[tool(description = "Change a request's URL.")]
    async fn set_url(&self, params: Parameters<SetUrlParams>) -> Result<String, String> {
        map_err(request::set_url(&self.ctx, params.0.id, params.0.url).await)
    }

    #[tool(description = "Rename a request.")]
    async fn set_name(&self, params: Parameters<SetNameParams>) -> Result<String, String> {
        map_err(request::set_name(&self.ctx, params.0.id, params.0.name).await)
    }

    #[tool(
        description = "Set a request's free-form notes (documentation). Capped at 50 words — text beyond the 50th word is silently truncated rather than rejected."
    )]
    async fn set_notes(&self, params: Parameters<SetNotesParams>) -> Result<String, String> {
        map_err(request::set_notes(&self.ctx, params.0.request_id, params.0.notes).await)
    }

    #[tool(description = "Replace a request's headers list.")]
    async fn set_headers(&self, params: Parameters<SetHeadersParams>) -> Result<String, String> {
        map_err(request::set_headers(&self.ctx, SetHeadersInput { request_id: params.0.request_id, headers: params.0.headers }).await)
    }

    #[tool(description = "Replace a request's query params list.")]
    async fn set_params(&self, params: Parameters<SetParamsParams>) -> Result<String, String> {
        map_err(request::set_params(&self.ctx, SetParamsInput { request_id: params.0.request_id, params: params.0.params }).await)
    }

    #[tool(description = "Replace a request's body (None/Json/Text/Xml/FormUrlEncoded).")]
    async fn set_body(&self, params: Parameters<SetBodyParams>) -> Result<String, String> {
        map_err(request::set_body(&self.ctx, SetBodyInput { request_id: params.0.request_id, body: params.0.body }).await)
    }

    #[tool(description = "Set a request's auth configuration (None/Basic/Bearer/ApiKey/OAuth1/OAuth2/AwsSigV4/Digest/Hawk).")]
    async fn set_auth(&self, params: Parameters<SetAuthParams>) -> Result<String, String> {
        map_err(request::set_auth(&self.ctx, SetAuthInput { request_id: params.0.request_id, auth: params.0.auth }).await)
    }

    #[tool(description = "Get a request's raw stored auth configuration (may be \"InheritFromParent\" — use get_effective_auth to see what's actually sent).")]
    async fn get_auth(&self, params: Parameters<RequestIdOnlyParams>) -> Result<String, String> {
        map_err(request::get_auth(&self.ctx, params.0.request_id).await)
    }

    #[tool(
        description = "Get the auth that will actually be used when this request is sent, resolving \"InheritFromParent\" by walking up the folder chain to the workspace."
    )]
    async fn get_effective_auth(&self, params: Parameters<RequestIdOnlyParams>) -> Result<String, String> {
        map_err(request::get_effective_auth(&self.ctx, params.0.request_id).await)
    }

    #[tool(description = "Reset a request's auth configuration to None.")]
    async fn clear_auth(&self, params: Parameters<RequestIdOnlyParams>) -> Result<String, String> {
        map_err(request::clear_auth(&self.ctx, params.0.request_id).await)
    }

    #[tool(description = "List the auth type ids/labels supported by set_auth.")]
    async fn list_auth_types(&self) -> String {
        to_json(&spectra_api::dto::list_auth_types())
    }

    #[tool(
        description = "Send a request over the network (resolving {{variables}} from the given or active environment), recording it to History. This performs a real HTTP call."
    )]
    async fn send_request(&self, params: Parameters<SendRequestParams>) -> Result<String, String> {
        let mut res = request::send_request(
            &self.ctx,
            SendRequestInput { request_id: params.0.request_id, environment_id: params.0.environment_id },
        )
        .await;
        
        if let Ok(ref mut dto) = res {
            if let Some(max) = params.0.max_body_bytes {
                truncate_body(dto, max);
            }
        }
        map_err(res)
    }

    #[tool(
        description = "Send a request over the network and return its response body with line numbers prefixed to each line (e.g. '1 | ...'). Useful for reading large JSON/text payloads and referencing specific lines."
    )]
    async fn send_request_with_lines(&self, params: Parameters<SendRequestParams>) -> Result<String, String> {
        let res = request::send_request(
            &self.ctx,
            SendRequestInput { request_id: params.0.request_id, environment_id: params.0.environment_id },
        )
        .await;
        
        match res {
            Ok(mut dto) => {
                let lines: Vec<String> = dto.body.lines().enumerate().map(|(i, line)| format!("{} | {}", i + 1, line)).collect();
                dto.body = lines.join("\n");
                if let Some(max) = params.0.max_body_bytes {
                    truncate_body(&mut dto, max);
                }
                map_err(Ok(dto))
            }
            Err(e) => map_err::<ResponseDto>(Err(e)),
        }
    }

    #[tool(
        description = "Compute the exact headers that would be sent for a request (including computed auth headers) without performing the HTTP call."
    )]
    async fn preview_headers(&self, params: Parameters<SendRequestParams>) -> Result<String, String> {
        map_err(
            request::preview_headers(
                &self.ctx,
                SendRequestInput { request_id: params.0.request_id, environment_id: params.0.environment_id },
            )
            .await,
        )
    }

    #[tool(
        description = "Clear all persisted session cookies from the HTTP client."
    )]
    async fn clear_cookies(&self) -> String {
        self.ctx.cookie_store.clear();
        to_json(&())
    }

    // --- Automation (drives the already-running GUI process) ------------

    #[tool(
        description = "Ask the already-running Spectra GUI app to open a request's tab, send it (if it has no response yet or force_send is true), wait for the response to render, and take a screenshot of the window. Requires the GUI to already be running — returns a clear error if it isn't."
    )]
    async fn automation_screenshot_request(
        &self,
        params: Parameters<AutomationScreenshotParams>,
    ) -> Result<String, String> {
        let p = params.0;
        let saved_path =
            crate::automation_client::request_screenshot(p.request_id, p.save_path, p.environment_id, p.force_send.unwrap_or(false)).await?;
        Ok(to_json(&serde_json::json!({ "success": true, "saved_path": saved_path })))
    }

    #[tool(
        description = "Ask the already-running Spectra GUI app to open a request's tab and send it (if it has no response yet or force_send is true), without taking a screenshot. Returns a status report: which workspace/request it resolved to, whether it actually rendered a response or error, and the send error if any. Useful to navigate the GUI to a request (e.g. during a screen-share) or to verify the request resolves and sends correctly before calling automation_screenshot_request. Requires the GUI to already be running — returns a clear error if it isn't."
    )]
    async fn automation_focus_request(
        &self,
        params: Parameters<AutomationFocusParams>,
    ) -> Result<String, String> {
        let p = params.0;
        let report = crate::automation_client::request_focus(p.request_id, p.environment_id, p.force_send.unwrap_or(false)).await?;
        Ok(to_json(&report))
    }

    #[tool(
        description = "Ask the already-running Spectra GUI app to toggle line numbers in its editors. Useful to call before taking a screenshot so you can reference lines in the UI."
    )]
    async fn set_ui_line_numbers(&self, params: Parameters<SetUiLineNumbersParams>) -> Result<String, String> {
        crate::automation_client::set_ui_line_numbers(params.0.show).await?;
        Ok(to_json(&"Line numbers toggled in GUI."))
    }

    #[tool(
        description = "Search a response body without downloading the full body over the MCP token channel. Returns matches with line numbers and short context."
    )]
    async fn analyze_response(&self, params: Parameters<AnalyzeResponseParams>) -> Result<String, String> {
        let p = params.0;
        let dto = request::send_request(
            &self.ctx,
            SendRequestInput { request_id: p.request_id, environment_id: p.environment_id },
        )
        .await
        .map_err(|e| e.to_string())?;

        let query = p.query.to_lowercase();
        let context_chars = p.context_chars.unwrap_or(120);
        let mut matches = Vec::new();
        let mut total_matches = 0;
        let lines: Vec<&str> = dto.body.lines().collect();
        let total_lines = lines.len();

        for (i, line) in lines.into_iter().enumerate() {
            let lower_line = line.to_lowercase();
            if lower_line.contains(&query) {
                total_matches += 1;
                if matches.len() < 50 {
                    // Extract a snippet around the match
                    let match_idx = lower_line.find(&query).unwrap();
                    let start = match_idx.saturating_sub(context_chars / 2);
                    let end = (match_idx + query.len() + context_chars / 2).min(line.len());
                    
                    let mut snippet = String::new();
                    if start > 0 { snippet.push_str("..."); }
                    // find char boundaries
                    let mut safe_start = start;
                    while safe_start < line.len() && !line.is_char_boundary(safe_start) { safe_start += 1; }
                    let mut safe_end = end;
                    while safe_end > 0 && !line.is_char_boundary(safe_end) { safe_end -= 1; }
                    
                    snippet.push_str(&line[safe_start..safe_end]);
                    if end < line.len() { snippet.push_str("..."); }

                    matches.push(serde_json::json!({
                        "line_number": i + 1,
                        "content": snippet.trim()
                    }));
                }
            }
        }

        Ok(to_json(&serde_json::json!({
            "status": dto.status,
            "duration_ms": dto.duration_ms,
            "total_lines": total_lines,
            "total_matches": total_matches,
            "showing_matches": matches.len(),
            "matches": matches
        })))
    }

    #[tool(
        description = "Send a request and capture a screenshot of the result in one step. Saves 2 round trips compared to focusing then screenshotting."
    )]
    async fn send_and_screenshot(
        &self,
        params: Parameters<SendAndScreenshotParams>,
    ) -> Result<String, String> {
        let p = params.0;
        let (saved_path, report) =
            crate::automation_client::request_send_and_screenshot(p.request_id, p.save_path, p.environment_id).await?;
        
        let res = serde_json::json!({
            "success": true,
            "saved_path": saved_path,
            "report": report
        });
        
        // Optional body truncation if the report contains the body (which it doesn't currently, 
        // but we keep the parameter for future parity or if we want to fetch the response here).
        // For now we just return the report which tells us if it sent.
        
        Ok(to_json(&res))
    }

    #[tool(
        description = "Open a request's tab, search for text in the response, scroll to it, and take a screenshot."
    )]
    async fn search_response(
        &self,
        params: Parameters<SearchResponseParams>,
    ) -> Result<String, String> {
        let p = params.0;
        let result = crate::automation_client::request_search_response(
            p.request_id, 
            p.query, 
            p.save_path, 
            p.environment_id
        ).await?;
        
        Ok(to_json(&result))
    }

    #[tool(
        description = "Create a temporary workspace and request, send it, optionally screenshot it, and return the response. Perfect for throwaway tests."
    )]
    async fn quick_test(
        &self,
        params: Parameters<QuickTestParams>,
    ) -> Result<String, String> {
        let p = params.0;
        
        // 1. Find or create scratch workspace
        let ws_name = "Quick Test";
        let mut ws_id = None;
        let all_ws = workspace::list_workspaces(&self.ctx).await.unwrap_or_default();
        for ws in all_ws {
            if ws.name == ws_name {
                ws_id = Some(ws.id);
                break;
            }
        }
        
        let ws_id = if let Some(id) = ws_id {
            id
        } else {
            let ws = workspace::create_workspace(&self.ctx, CreateWorkspaceInput {
                name: ws_name.to_string(),
            }).await.map_err(|e| e.to_string())?;
            ws.id
        };

        // 2. Create the request
        let method = p.method.unwrap_or(HttpMethod::Get);
        let req = request::create_request(&self.ctx, CreateRequestInput {
            workspace_id: ws_id,
            folder_id: None,
            name: format!("{:?} {}", method, p.url),
            method,
            url: p.url,
        }).await.map_err(|e| e.to_string())?;

        if let Some(headers) = p.headers {
            let _ = request::set_headers(&self.ctx, SetHeadersInput {
                request_id: req.id.clone(),
                headers,
            }).await;
        }

        if let Some(body) = p.body {
            let _ = request::set_body(&self.ctx, SetBodyInput {
                request_id: req.id.clone(),
                body,
            }).await;
        }

        // 3. Send it
        let mut dto = request::send_request(
            &self.ctx,
            SendRequestInput { request_id: req.id.clone(), environment_id: None },
        )
        .await
        .map_err(|e| {
            // Cleanup on send failure
            let _ = request::delete_request(&self.ctx, req.id.clone());
            e.to_string()
        })?;
        
        // 4. Truncate body
        let max_bytes = p.max_body_bytes.unwrap_or(8192);
        truncate_body(&mut dto, max_bytes);

        // 5. Screenshot if requested
        let screenshot_path = if let Some(save_path) = p.save_path {
            match crate::automation_client::request_screenshot(req.id.clone(), save_path, None, true).await {
                Ok(path) => Some(path),
                Err(e) => Some(format!("Screenshot failed: {}", e))
            }
        } else {
            None
        };

        // 6. Cleanup
        let _ = request::delete_request(&self.ctx, req.id);

        Ok(to_json(&serde_json::json!({
            "response": dto,
            "screenshot": screenshot_path
        })))
    }

    // --- OAuth2 interactive flows ---------------------------------------

    #[tool(
        description = "Start an interactive OAuth2 flow (Authorization Code, Authorization Code + PKCE, or Device Code) for a request already configured with that grant type. Opens the system browser or returns a device code to show the user. Non-interactive grants (Client Credentials/Password/Refresh Token) don't need this — just call send_request."
    )]
    async fn start_oauth_flow(&self, params: Parameters<RequestIdOnlyParams>) -> Result<String, String> {
        map_err(oauth::start_oauth_flow(&self.ctx, params.0.request_id).await)
    }

    #[tool(description = "Poll the status of an in-progress OAuth2 flow for a request.")]
    async fn get_oauth_status(&self, params: Parameters<RequestIdOnlyParams>) -> Result<String, String> {
        map_err(oauth::get_oauth_status(&self.ctx, params.0.request_id).await)
    }

    #[tool(description = "Cancel an in-progress OAuth2 flow for a request.")]
    async fn cancel_oauth_flow(&self, params: Parameters<RequestIdOnlyParams>) -> Result<String, String> {
        map_err(oauth::cancel_oauth_flow(&self.ctx, params.0.request_id).await)
    }

    #[tool(
        description = "Fetch a fresh OAuth2 token on demand for a non-interactive grant (Client Credentials/Password/Refresh Token) and save it under a name — the \"Get New Access Token\" action. For interactive grants use start_oauth_flow instead."
    )]
    async fn fetch_oauth_token(&self, params: Parameters<FetchOAuthTokenParams>) -> Result<String, String> {
        map_err(oauth::fetch_oauth_token(&self.ctx, params.0.request_id, params.0.name).await)
    }

    #[tool(description = "List every named OAuth2 token saved so far for a request, newest first.")]
    async fn list_oauth_tokens(&self, params: Parameters<RequestIdOnlyParams>) -> Result<String, String> {
        map_err(oauth::list_oauth_tokens(&self.ctx, params.0.request_id).await)
    }

    #[tool(description = "Mark a previously-saved named OAuth2 token as the current one to use for a request, without re-running the flow.")]
    async fn select_oauth_token(&self, params: Parameters<OAuthTokenNameParams>) -> Result<String, String> {
        map_err(oauth::select_oauth_token(&self.ctx, params.0.request_id, params.0.name).await)
    }

    #[tool(description = "Delete a named OAuth2 token saved for a request.")]
    async fn delete_oauth_token(&self, params: Parameters<OAuthTokenNameParams>) -> Result<String, String> {
        map_err(oauth::delete_oauth_token(&self.ctx, params.0.request_id, params.0.name).await)
    }

    // --- Environments -----------------------------------------------------

    #[tool(description = "List environments in a workspace. Secret variable values are always masked.")]
    async fn list_environments(&self, params: Parameters<ListEnvironmentsParams>) -> Result<String, String> {
        map_err(environment::list_environments(&self.ctx, params.0.workspace_id).await)
    }

    #[tool(
        description = "Create an environment with variables. Set a variable's secret=true to store its value in the macOS Keychain instead of plain JSON."
    )]
    async fn create_environment(&self, params: Parameters<CreateEnvironmentParams>) -> Result<String, String> {
        map_err(environment::create_environment(&self.ctx, params.0.workspace_id, params.0.name, params.0.variables).await)
    }

    #[tool(
        description = "Update an environment's name/variables. To leave a secret variable's value unchanged, pass back the masked sentinel value exactly as returned by list_environments/create_environment rather than a new value."
    )]
    async fn update_environment(&self, params: Parameters<UpdateEnvironmentParams>) -> Result<String, String> {
        let p = params.0;
        map_err(environment::update_environment(&self.ctx, p.workspace_id, p.id, p.name, p.variables).await)
    }

    #[tool(description = "Delete an environment (and any Keychain-backed secrets it owned).")]
    async fn delete_environment(&self, params: Parameters<DeleteEnvironmentParams>) -> Result<String, String> {
        map_err(environment::delete_environment(&self.ctx, params.0.workspace_id, params.0.id).await)
    }

    #[tool(
        description = "Check every environment in a workspace for secret variables whose macOS Keychain entry can't be found. This is the detectable symptom of copying/restoring ~/.spectra onto a different machine or user account than the one that created the secrets — the entries never travel with the JSON files. Read-only."
    )]
    async fn check_secrets_health(&self, params: Parameters<CheckSecretsHealthParams>) -> Result<String, String> {
        map_err(environment::check_secrets_health(&self.ctx, params.0.workspace_id).await)
    }

    // --- Folders ----------------------------------------------------------

    #[tool(description = "List folders in a workspace (flat list; use parent_folder_id to reconstruct the tree).")]
    async fn list_folders(&self, params: Parameters<ListFoldersParams>) -> Result<String, String> {
        map_err(folder::list_folders(&self.ctx, params.0.workspace_id).await)
    }

    #[tool(description = "Create a folder, optionally nested under a parent folder.")]
    async fn create_folder(&self, params: Parameters<CreateFolderParams>) -> Result<String, String> {
        map_err(folder::create_folder(&self.ctx, params.0.workspace_id, params.0.parent_folder_id, params.0.name).await)
    }

    #[tool(
        description = "Set a folder-level auth override every request (and sub-folder) under it inherits by default, unless overridden further down the chain. Pass auth type \"InheritFromParent\" to remove this folder's own override and fall back to its parent folder/workspace instead."
    )]
    async fn set_folder_auth(&self, params: Parameters<SetFolderAuthParams>) -> Result<String, String> {
        map_err(folder::set_folder_auth(&self.ctx, params.0.workspace_id, params.0.id, params.0.auth).await)
    }

    #[tool(description = "Rename a folder.")]
    async fn rename_folder(&self, params: Parameters<RenameFolderParams>) -> Result<String, String> {
        map_err(folder::rename_folder(&self.ctx, params.0.workspace_id, params.0.id, params.0.name).await)
    }

    #[tool(description = "Move a folder under a new parent (or to the root, by omitting new_parent_id).")]
    async fn move_folder(&self, params: Parameters<MoveFolderParams>) -> Result<String, String> {
        map_err(folder::move_folder(&self.ctx, params.0.workspace_id, params.0.id, params.0.new_parent_id).await)
    }

    #[tool(
        description = "Delete a folder. Requests and sub-folders directly inside it are reparented to its parent rather than being deleted."
    )]
    async fn delete_folder(&self, params: Parameters<DeleteFolderParams>) -> Result<String, String> {
        map_err(folder::delete_folder(&self.ctx, params.0.workspace_id, params.0.id).await)
    }

    #[tool(description = "Move a request into a folder (or to the workspace root, by omitting target_folder_id).")]
    async fn move_request(&self, params: Parameters<MoveRequestParams>) -> Result<String, String> {
        map_err(folder::move_request(&self.ctx, params.0.request_id, params.0.target_folder_id).await)
    }

    // --- History ------------------------------------------------------------

    #[tool(description = "List every recorded send (success or failure) for a workspace.")]
    async fn list_history(&self, params: Parameters<ListHistoryParams>) -> Result<String, String> {
        map_err(history::list_history(&self.ctx, params.0.workspace_id).await)
    }

    #[tool(description = "Delete a history entry.")]
    async fn delete_history_entry(&self, params: Parameters<HistoryEntryParams>) -> Result<String, String> {
        map_err(history::delete_history_entry(&self.ctx, params.0.workspace_id, params.0.id).await)
    }

    #[tool(
        description = "List the last 5 recorded sends (newest-first) for one specific request — a filtered view over the same on-disk history list_history returns, not a separate retention policy."
    )]
    async fn list_history_for_request(&self, params: Parameters<ListHistoryForRequestParams>) -> Result<String, String> {
        map_err(history::list_history_for_request(&self.ctx, params.0.workspace_id, params.0.request_id).await)
    }

    #[tool(
        description = "Re-send the exact request as it was captured in a history entry (not the live, possibly-since-edited saved request). Performs a real HTTP call and records a new history entry."
    )]
    async fn replay_history_entry(&self, params: Parameters<HistoryEntryParams>) -> Result<String, String> {
        map_err(history::replay_history_entry(&self.ctx, params.0.workspace_id, params.0.id).await)
    }

    #[tool(description = "Save a history entry's request snapshot as a new standalone saved request.")]
    async fn convert_history_to_request(&self, params: Parameters<ConvertHistoryToRequestParams>) -> Result<String, String> {
        map_err(history::convert_history_to_request(&self.ctx, params.0.workspace_id, params.0.id, params.0.target_folder_id).await)
    }

    // --- Saved responses ------------------------------------------------------

    #[tool(description = "List saved (curated) example responses attached to a request.")]
    async fn list_saved_responses(&self, params: Parameters<ListSavedResponsesParams>) -> Result<String, String> {
        map_err(saved_response::list_saved_responses(&self.ctx, params.0.workspace_id, params.0.request_id).await)
    }

    #[tool(description = "Save a named example response under a request (e.g. \"Compliant Response\").")]
    async fn save_response(&self, params: Parameters<SaveResponseParams>) -> Result<String, String> {
        let p = params.0;
        map_err(saved_response::save_response(&self.ctx, p.workspace_id, p.request_id, p.name, p.response).await)
    }

    #[tool(description = "Delete a saved response.")]
    async fn delete_saved_response(&self, params: Parameters<DeleteSavedResponseParams>) -> Result<String, String> {
        map_err(saved_response::delete_saved_response(&self.ctx, params.0.workspace_id, params.0.id).await)
    }

    // --- Import -----------------------------------------------------------

    #[tool(
        description = "Import a collection (cURL command, Postman Collection v2.1 JSON, OpenAPI 3.x/Swagger 2.0 JSON or YAML, or a HAR/HTTP Archive JSON export) into a workspace as real folders/requests. Pass format explicitly or omit it to auto-detect from content."
    )]
    async fn import_collection(&self, params: Parameters<ImportCollectionParams>) -> Result<String, String> {
        map_err(import::import_collection(&self.ctx, params.0.workspace_id, params.0.content, params.0.format).await)
    }

    #[tool(
        description = "Export a whole workspace's folder/request tree as a Postman Collection v2.1 JSON document (format=\"postman\") or a best-effort OpenAPI 3.0 document (format=\"openapi\"). For a single request as a curl command, use export_request instead."
    )]
    async fn export_workspace(&self, params: Parameters<ExportWorkspaceParams>) -> Result<String, String> {
        export::export_workspace(&self.ctx, params.0.workspace_id, params.0.format).await.map_err(|e| e.to_string())
    }

    #[tool(
        description = "Export a single request as a curl command (format=\"curl\"). For a whole workspace as Postman/OpenAPI, use export_workspace instead."
    )]
    async fn export_request(&self, params: Parameters<ExportRequestParams>) -> Result<String, String> {
        export::export_request(&self.ctx, params.0.request_id, params.0.format).await.map_err(|e| e.to_string())
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for SpectraServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("spectra-mcp", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Spectra is a Postman/Insomnia-style API client. These tools drive the exact same backend an end \
                 user's GUI uses, so anything done here is reflected in their app and vice versa. Typical flow: \
                 list_workspaces (or create_workspace) -> create_request -> \
                 set_headers/set_params/set_body/set_auth -> send_request. Use environments \
                 (list_environments/create_environment/set_active_environment) for {{variable}} substitution, and \
                 import_collection to bulk-load cURL/Postman/OpenAPI collections.",
            )
    }
}
