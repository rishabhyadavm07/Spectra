export type HttpMethod =
  "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS" | "HEAD";

export interface Workspace {
  id: string;
  name: string;
  active_environment_id: string | null;
  auth: AuthConfig;
  created_at: string;
}

export interface WorkspaceSavedAuth {
  id: string;
  workspace_id: string;
  name: string;
  auth: AuthConfig;
  created_at: string;
}

export interface HeaderEntry {
  key: string;
  value: string;
  enabled: boolean;
}

export interface ParamEntry {
  key: string;
  value: string;
  enabled: boolean;
}

export interface TabState {
  tabId: string;
  request: SpectraRequest;
  response: ResponseDto | null;
  error: string | null;
  sending: boolean;
  savedResponseName: string | null;
  lastPersisted: SpectraRequest;
}

export type ImportFormat = "curl" | "postman" | "openapi" | "har";

export interface ImportResult {
  imported_count: number;
  request_ids: string[];
  saved_response_count: number;
}

export type RequestBody =
  | { kind: "None" }
  | { kind: "Json"; content: string }
  | { kind: "Text"; content: string }
  | { kind: "Xml"; content: string }
  | { kind: "FormUrlEncoded"; fields: ParamEntry[] };

export type OAuth1SignatureMethod = "hmac_sha1" | "hmac_sha256" | "plain_text";
export type HawkAlgorithm = "SHA1" | "SHA256";

export type AddAuthDataTo = "request_headers" | "query_params";
export type ClientAuthentication = "send_as_basic_auth_header" | "send_in_body";
export type ParamTarget = "header" | "body";

export interface OAuth2ExtraParam {
  key: string;
  value: string;
  target: ParamTarget;
}

export interface OAuth2Options {
  add_to: AddAuthDataTo;
  header_prefix: string;
  auto_refresh: boolean;
  client_authentication: ClientAuthentication;
  token_request_params: OAuth2ExtraParam[];
  refresh_request_params: OAuth2ExtraParam[];
}

export function defaultOAuth2Options(): OAuth2Options {
  return {
    add_to: "request_headers",
    header_prefix: "Bearer",
    auto_refresh: true,
    client_authentication: "send_as_basic_auth_header",
    token_request_params: [],
    refresh_request_params: [],
  };
}

export type OAuth2Grant =
  | {
      type: "AuthorizationCode";
      client_id: string;
      client_secret: string | null;
      auth_url: string;
      token_url: string;
      redirect_uri: string;
      scope: string | null;
      options: OAuth2Options;
    }
  | {
      type: "AuthorizationCodePkce";
      client_id: string;
      auth_url: string;
      token_url: string;
      redirect_uri: string;
      scope: string | null;
      options: OAuth2Options;
    }
  | {
      type: "ClientCredentials";
      client_id: string;
      client_secret: string;
      token_url: string;
      scope: string | null;
      options: OAuth2Options;
    }
  | {
      type: "Password";
      client_id: string;
      client_secret: string | null;
      token_url: string;
      username: string;
      password: string;
      scope: string | null;
      options: OAuth2Options;
    }
  | {
      type: "RefreshToken";
      client_id: string;
      client_secret: string | null;
      token_url: string;
      refresh_token: string;
      options: OAuth2Options;
    }
  | {
      type: "DeviceCode";
      client_id: string;
      device_auth_url: string;
      token_url: string;
      scope: string | null;
      options: OAuth2Options;
    }
  | {
      type: "Implicit";
      client_id: string;
      auth_url: string;
      redirect_uri: string;
      scope: string | null;
      options: OAuth2Options;
    };

export type AuthConfig =
  | { type: "None" }
  | { type: "InheritFromParent" }
  | { type: "Basic"; username: string; password: string }
  | { type: "Bearer"; token: string }
  | {
      type: "ApiKey";
      key: string;
      value: string;
      location: "header" | "query" | "cookie";
    }
  | {
      type: "OAuth1";
      consumer_key: string;
      consumer_secret: string;
      token: string | null;
      token_secret: string | null;
      signature_method: OAuth1SignatureMethod;
    }
  | { type: "OAuth2"; grant: OAuth2Grant }
  | {
      type: "AwsSigV4";
      access_key: string;
      secret_key: string;
      region: string;
      service: string;
      session_token: string | null;
    }
  | { type: "Digest"; username: string; password: string }
  | { type: "Hawk"; id: string; key: string; algorithm: HawkAlgorithm }
  | { type: "SavedAuth"; saved_auth_id: string };

export interface OAuthToken {
  access_token: string;
  token_type: string;
  refresh_token: string | null;
  expires_at: string | null;
}

export interface NamedOAuthToken {
  name: string;
  token: OAuthToken;
  saved_at: string;
}

export type PendingUserAction =
  | { kind: "Browser"; auth_url: string }
  | { kind: "DeviceCode"; user_code: string; verification_url: string };

export type OAuthStatus =
  | { status: "NotStarted" }
  | { status: "Pending"; user_action: PendingUserAction }
  | { status: "Complete"; token: OAuthToken }
  | { status: "Failed"; error: string };

export interface RequestSummary {
  id: string;
  folder_id: string | null;
  name: string;
  method: HttpMethod;
  url: string;
}

export interface Folder {
  id: string;
  workspace_id: string;
  parent_folder_id: string | null;
  name: string;
  auth: AuthConfig;
  created_at: string;
}

export interface HistoryEntry {
  id: string;
  workspace_id: string;
  request_id: string;
  request_snapshot: SpectraRequest;
  response: ResponseDto | null;
  error: string | null;
  executed_at: string;
}

export interface RequestRun {
  history_id: string;
  response: ResponseDto;
}

export interface SavedResponse {
  id: string;
  workspace_id: string;
  request_id: string;
  name: string;
  response: ResponseDto;
  saved_at: string;
}

export interface SpectraRequest {
  id: string;
  workspace_id: string;
  folder_id: string | null;
  name: string;
  method: HttpMethod;
  url: string;
  headers: HeaderEntry[];
  params: ParamEntry[];
  body: RequestBody;
  auth: AuthConfig;
  /** Free-form documentation, capped at 50 words (enforced client-side with
   * a live counter, and defensively truncated server-side too — see
   * spectra-api's `set_notes`). Defaults to "" for requests saved before
   * this field existed. */
  notes: string;
  created_at: string;
  updated_at: string;
}

export interface ResponseDto {
  status: number;
  status_text: string;
  headers: Record<string, string>;
  body: string;
  size_bytes: number;
  duration_ms: number;
}

export interface VariableOutput {
  value: string;
  secret: boolean;
}

export interface VariableInput {
  value: string;
  secret: boolean;
}

/** Sentinel shown for a secret variable's value in place of its real value —
 * the backend never sends secret plaintext to the frontend. Sending this
 * value back unchanged on update tells the backend to keep the existing
 * Keychain-stored secret rather than overwrite it. */
export const UNCHANGED_SECRET_SENTINEL = "••••••••";

export interface Environment {
  id: string;
  workspace_id: string;
  name: string;
  variables: Record<string, VariableOutput>;
}

/** A secret variable whose Windows Credential Manager entry could not be found — the
 * detectable symptom of ~/.spectra having been copied to a different
 * machine/user account than the one that created the secret. */
export interface OrphanedSecret {
  environment_id: string;
  environment_name: string;
  variable_name: string;
}

export interface AppSettings {
  theme: string;
  ssl_verification: boolean;
  request_timeout_ms: number;
}
