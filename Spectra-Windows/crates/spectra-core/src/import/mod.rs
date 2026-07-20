//! Import support for bringing existing work into Spectra (PRD Section 14).
//! Each format module (curl, postman, openapi, har) parses its input into
//! the common `ImportedTree` shape below; the command layer then persists
//! that tree as real Folders/Requests via the same storage calls the GUI
//! uses — import never bypasses the normal creation path.

pub mod curl;
pub mod har;
pub mod openapi;
pub mod postman;

use crate::model::{AuthConfig, HeaderEntry, HttpMethod, ParamEntry, RequestBody, ResponseDto};

/// One request as parsed from an import source, not yet assigned an id or
/// persisted. `folder_path` is a list of folder names from root to the
/// request's containing folder (empty = top-level), resolved into real
/// Folder records by the caller so nested collections round-trip.
#[derive(Debug, Clone)]
pub struct ImportedRequest {
    pub folder_path: Vec<String>,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<HeaderEntry>,
    pub params: Vec<ParamEntry>,
    pub body: RequestBody,
    pub auth: AuthConfig,
    /// Saved example responses attached to this request in the source
    /// collection (Postman's `item.response[]` — an example response saved
    /// alongside a request, distinct from a live send). Only Postman
    /// currently populates this; cURL/OpenAPI/HAR have no equivalent concept
    /// to import, so they always leave it empty.
    pub saved_responses: Vec<ImportedSavedResponse>,
}

/// One saved example response as parsed from an import source, not yet
/// assigned an id or attached to a real (persisted) request.
#[derive(Debug, Clone)]
pub struct ImportedSavedResponse {
    pub name: String,
    pub response: ResponseDto,
}

#[derive(Debug, Clone, Default)]
pub struct ImportedTree {
    pub requests: Vec<ImportedRequest>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportFormat {
    Curl,
    PostmanCollection,
    OpenApi,
    Har,
}

/// Sniffs the format from raw text content when the caller doesn't already
/// know it (e.g. a generic "paste anything" import box). Best-effort only —
/// prefer an explicit format when the caller has one (e.g. from a file
/// picker's extension or a distinct UI action).
pub fn detect_format(content: &str) -> Option<ImportFormat> {
    let trimmed = content.trim_start();
    if trimmed.starts_with("curl ") || trimmed.starts_with("curl\t") {
        return Some(ImportFormat::Curl);
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if value.get("log").and_then(|l| l.get("entries")).is_some() {
            return Some(ImportFormat::Har);
        }
        if value.get("info").and_then(|i| i.get("schema")).is_some() || value.get("item").is_some() {
            return Some(ImportFormat::PostmanCollection);
        }
        if value.get("openapi").is_some() || value.get("swagger").is_some() {
            return Some(ImportFormat::OpenApi);
        }
    }
    if let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(trimmed) {
        if value.get("openapi").is_some() || value.get("swagger").is_some() {
            return Some(ImportFormat::OpenApi);
        }
    }
    None
}
