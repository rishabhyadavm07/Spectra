import { invoke } from "@tauri-apps/api/core";
import type {
  AuthConfig,
  Environment,
  Folder,
  HeaderEntry,
  HistoryEntry,
  HttpMethod,
  ImportFormat,
  ImportResult,
  NamedOAuthToken,
  OAuthStatus,
  OrphanedSecret,
  ParamEntry,
  PendingUserAction,
  RequestBody,
  RequestRun,
  RequestSummary,
  ResponseDto,
  SavedResponse,
  SpectraRequest,
  VariableInput,
  Workspace,
  AppSettings,
} from "./types";

// Thin wrappers over Tauri invoke — mirror spectra-api's command surface
// 1:1 so the frontend never encodes logic the MCP server wouldn't also get.
export const api = {
  listWorkspaces: () => invoke<Workspace[]>("list_workspaces"),
  createWorkspace: (name: string) =>
    invoke<Workspace>("create_workspace", { name }),
  openWorkspace: (id: string) => invoke<Workspace>("open_workspace", { id }),

  listRequests: (workspaceId: string, folderId?: string) =>
    invoke<RequestSummary[]>("list_requests", {
      workspaceId,
      folderId: folderId ?? null,
    }),
  openRequest: (id: string) => invoke<SpectraRequest>("open_request", { id }),
  createRequest: (
    workspaceId: string,
    name: string,
    method: HttpMethod,
    url: string,
  ) =>
    invoke<SpectraRequest>("create_request", {
      workspaceId,
      folderId: null,
      name,
      method,
      url,
    }),
  deleteRequest: (id: string) => invoke<void>("delete_request", { id }),
  setMethod: (id: string, method: HttpMethod) =>
    invoke<SpectraRequest>("set_method", { id, method }),
  setUrl: (id: string, url: string) =>
    invoke<SpectraRequest>("set_url", { id, url }),
  setName: (id: string, name: string) =>
    invoke<SpectraRequest>("set_name", { id, name }),
  setNotes: (requestId: string, notes: string) =>
    invoke<SpectraRequest>("set_notes", { requestId, notes }),
  setHeaders: (requestId: string, headers: HeaderEntry[]) =>
    invoke<SpectraRequest>("set_headers", { requestId, headers }),
  setParams: (requestId: string, params: ParamEntry[]) =>
    invoke<SpectraRequest>("set_params", { requestId, params }),
  setBody: (requestId: string, body: RequestBody) =>
    invoke<SpectraRequest>("set_body", { requestId, body }),
  setAuth: (requestId: string, auth: AuthConfig) =>
    invoke<SpectraRequest>("set_auth", { requestId, auth }),
  getEffectiveAuth: (requestId: string) =>
    invoke<AuthConfig>("get_effective_auth", { requestId }),

  sendRequest: (requestId: string, environmentId?: string) =>
    invoke<ResponseDto>("send_request", {
      requestId,
      environmentId: environmentId ?? null,
    }),
  previewHeaders: (requestId: string, environmentId?: string) =>
    invoke<[string, string][]>("preview_headers", {
      requestId,
      environmentId: environmentId ?? null,
    }),
  clearCookies: () => invoke<void>("clear_cookies", {}),

  startOAuthFlow: (requestId: string) =>
    invoke<PendingUserAction>("start_oauth_flow", { requestId }),
  getOAuthStatus: (requestId: string) =>
    invoke<OAuthStatus>("get_oauth_status", { requestId }),
  cancelOAuthFlow: (requestId: string) =>
    invoke<void>("cancel_oauth_flow", { requestId }),
  fetchOAuthToken: (requestId: string, name?: string) =>
    invoke<NamedOAuthToken>("fetch_oauth_token", {
      requestId,
      name: name ?? null,
    }),
  listOAuthTokens: (requestId: string) =>
    invoke<NamedOAuthToken[]>("list_oauth_tokens", { requestId }),
  selectOAuthToken: (requestId: string, name: string) =>
    invoke<void>("select_oauth_token", { requestId, name }),
  deleteOAuthToken: (requestId: string, name: string) =>
    invoke<void>("delete_oauth_token", { requestId, name }),

  listEnvironments: (workspaceId: string) =>
    invoke<Environment[]>("list_environments", { workspaceId }),
  createEnvironment: (
    workspaceId: string,
    name: string,
    variables: Record<string, VariableInput>,
  ) =>
    invoke<Environment>("create_environment", { workspaceId, name, variables }),
  updateEnvironment: (
    workspaceId: string,
    id: string,
    name: string,
    variables: Record<string, VariableInput>,
  ) =>
    invoke<Environment>("update_environment", {
      workspaceId,
      id,
      name,
      variables,
    }),
  deleteEnvironment: (workspaceId: string, id: string) =>
    invoke<void>("delete_environment", { workspaceId, id }),
  checkSecretsHealth: (workspaceId: string) =>
    invoke<OrphanedSecret[]>("check_secrets_health", { workspaceId }),
  setActiveEnvironment: (workspaceId: string, environmentId: string | null) =>
    invoke<Workspace>("set_active_environment", { workspaceId, environmentId }),
  setWorkspaceAuth: (workspaceId: string, auth: AuthConfig) =>
    invoke<Workspace>("set_workspace_auth", { workspaceId, auth }),

  listFolders: (workspaceId: string) =>
    invoke<Folder[]>("list_folders", { workspaceId }),
  createFolder: (
    workspaceId: string,
    parentFolderId: string | null,
    name: string,
  ) => invoke<Folder>("create_folder", { workspaceId, parentFolderId, name }),
  setFolderAuth: (workspaceId: string, id: string, auth: AuthConfig) =>
    invoke<Folder>("set_folder_auth", { workspaceId, id, auth }),
  renameFolder: (workspaceId: string, id: string, name: string) =>
    invoke<Folder>("rename_folder", { workspaceId, id, name }),
  moveFolder: (workspaceId: string, id: string, newParentId: string | null) =>
    invoke<Folder>("move_folder", { workspaceId, id, newParentId }),
  deleteFolder: (workspaceId: string, id: string) =>
    invoke<void>("delete_folder", { workspaceId, id }),
  moveRequest: (requestId: string, targetFolderId: string | null) =>
    invoke<void>("move_request", { requestId, targetFolderId }),

  listHistory: (workspaceId: string) =>
    invoke<HistoryEntry[]>("list_history", { workspaceId }),
  listHistoryForRequest: (workspaceId: string, requestId: string) =>
    invoke<HistoryEntry[]>("list_history_for_request", {
      workspaceId,
      requestId,
    }),
  deleteHistoryEntry: (workspaceId: string, id: string) =>
    invoke<void>("delete_history_entry", { workspaceId, id }),
  replayHistoryEntry: (workspaceId: string, id: string) =>
    invoke<RequestRun>("replay_history_entry", { workspaceId, id }),
  convertHistoryToRequest: (
    workspaceId: string,
    id: string,
    targetFolderId: string | null,
  ) =>
    invoke<SpectraRequest>("convert_history_to_request", {
      workspaceId,
      id,
      targetFolderId,
    }),

  listSavedResponses: (workspaceId: string, requestId: string) =>
    invoke<SavedResponse[]>("list_saved_responses", { workspaceId, requestId }),
  saveResponse: (
    workspaceId: string,
    requestId: string,
    name: string,
    response: ResponseDto,
  ) =>
    invoke<SavedResponse>("save_response", {
      workspaceId,
      requestId,
      name,
      response,
    }),
  deleteSavedResponse: (workspaceId: string, id: string) =>
    invoke<void>("delete_saved_response", { workspaceId, id }),

  importCollection: (
    workspaceId: string,
    content: string,
    format?: ImportFormat,
  ) =>
    invoke<ImportResult>("import_collection", {
      workspaceId,
      content,
      format: format ?? null,
    }),

  exportWorkspace: (workspaceId: string, format: "postman" | "openapi") =>
    invoke<string>("export_workspace", { workspaceId, format }),
  exportRequest: (requestId: string, format: "curl") =>
    invoke<string>("export_request", { requestId, format }),

  /** Signals the Rust-side automation IPC server (see HANDOFF.md's MCP
   * server section) that this request's tab is open, sent (if it needed to
   * be), and its response (or a send error) has rendered — safe to act on
   * now (screenshot, or just report status for a focus-only call). Called
   * only from the `automation://prepare-request` handler in App.tsx. */
  automationTabReady: (report: AutomationTabReadyReport) =>
    invoke<void>("automation_tab_ready", { report }),
  automationSearchReady: (report: AutomationSearchReadyReport) =>
    invoke<void>("automation_search_ready", { report }),

  getSettings: () => invoke<AppSettings>("get_settings"),
  saveSettings: (settings: AppSettings) =>
    invoke<void>("save_settings", { settings }),
};

/** Mirrors the Rust `TabReadyReport` struct — see crates/spectra-tauri/src/automation/mod.rs. */
export interface AutomationTabReadyReport {
  request_id: string;
  workspace_id: string | null;
  name: string | null;
  url: string | null;
  rendered: boolean;
  send_error: string | null;
}

export interface AutomationSearchReadyReport {
  request_id: string;
  match_count: number;
  first_match_line: number | null;
}
