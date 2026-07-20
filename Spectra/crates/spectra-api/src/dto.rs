use serde::{Deserialize, Serialize};
use spectra_core::model::{AuthConfig, HeaderEntry, HttpMethod, ParamEntry, RequestBody};

/// What the frontend sends when creating/updating an environment variable.
/// A `secret: true` variable's `value` is write-only — it's pushed to
/// Keychain and never echoed back; `value` is empty/ignored on updates where
/// the user didn't change it (see `EDIT_SENTINEL` in environment.rs).
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct VariableInput {
    pub value: String,
    pub secret: bool,
}

/// What the frontend receives when listing/reading an environment. Secret
/// values are always masked — plaintext never leaves the Keychain except at
/// request-send time inside spectra-core.
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct VariableOutput {
    pub value: String,
    pub secret: bool,
}

/// Environment shape sent to the frontend — identical to the internal model
/// except `variables` is masked (no Keychain account strings, no plaintext).
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct EnvironmentOutput {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub variables: std::collections::HashMap<String, VariableOutput>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateWorkspaceInput {
    pub name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateRequestInput {
    pub workspace_id: String,
    pub folder_id: Option<String>,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetHeadersInput {
    pub request_id: String,
    pub headers: Vec<HeaderEntry>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetParamsInput {
    pub request_id: String,
    pub params: Vec<ParamEntry>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetBodyInput {
    pub request_id: String,
    pub body: RequestBody,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetAuthInput {
    pub request_id: String,
    pub auth: AuthConfig,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SendRequestInput {
    pub request_id: String,
    pub environment_id: Option<String>,
}

/// One secret variable found to be missing its Keychain entry — the
/// telltale sign `~/.spectra` was copied/restored onto a machine (or user
/// account) other than the one that originally created it, since Keychain
/// entries are local to the machine/keychain and never travel with the
/// JSON files (PRD Section 21 gap — see HANDOFF.md "Secrets portability").
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct OrphanedSecret {
    pub environment_id: String,
    pub environment_name: String,
    pub variable_name: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct AuthTypeDescriptor {
    pub type_id: &'static str,
    pub label: &'static str,
}

pub fn list_auth_types() -> Vec<AuthTypeDescriptor> {
    vec![
        AuthTypeDescriptor { type_id: "none", label: "None" },
        AuthTypeDescriptor { type_id: "basic", label: "Basic Auth" },
        AuthTypeDescriptor { type_id: "bearer", label: "Bearer Token" },
        AuthTypeDescriptor { type_id: "api_key", label: "API Key" },
        AuthTypeDescriptor { type_id: "oauth1", label: "OAuth 1.0" },
        AuthTypeDescriptor { type_id: "oauth2", label: "OAuth 2.0" },
        AuthTypeDescriptor { type_id: "aws_sigv4", label: "AWS Signature V4" },
        AuthTypeDescriptor { type_id: "digest", label: "Digest Auth" },
        AuthTypeDescriptor { type_id: "hawk", label: "Hawk" },
    ]
}
