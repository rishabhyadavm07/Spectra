use spectra_core::export::{self, ExportFormat};
use spectra_core::{AppContext, ApiError, ApiResult};

fn parse_format(format: &str) -> ApiResult<ExportFormat> {
    match format {
        "curl" => Ok(ExportFormat::Curl),
        "postman" => Ok(ExportFormat::PostmanCollection),
        "openapi" => Ok(ExportFormat::OpenApi),
        other => Err(ApiError::Validation(format!("unknown export format: {other}"))),
    }
}

/// Exports an entire workspace's folder/request tree as Postman or OpenAPI.
/// cURL has no notion of a folder tree, so it's not a valid format here —
/// use `export_request` for a single request instead.
pub async fn export_workspace(ctx: &AppContext, workspace_id: String, format: String) -> ApiResult<String> {
    let format = parse_format(&format)?;
    let workspace = ctx.storage.get_workspace(&workspace_id).await?;
    let folders = ctx.storage.list_folders(&workspace_id).await?;
    let requests = ctx.storage.list_requests(&workspace_id).await?;
    let tree = export::build_tree(&folders, &requests, None);

    match format {
        ExportFormat::PostmanCollection => Ok(export::postman::export(&workspace.name, &tree)),
        ExportFormat::OpenApi => Ok(export::openapi::export(&workspace.name, &tree)),
        ExportFormat::Curl => {
            Err(ApiError::Validation("cURL export applies to a single request — use export_request".into()))
        }
    }
}

/// Exports a single request. Only cURL is meaningful for one request in
/// isolation; use `export_workspace` for Postman/OpenAPI collection export.
pub async fn export_request(ctx: &AppContext, request_id: String, format: String) -> ApiResult<String> {
    let format = parse_format(&format)?;
    let req = ctx.storage.find_request(&request_id).await?;

    match format {
        ExportFormat::Curl => Ok(export::curl::export(&req)),
        ExportFormat::PostmanCollection | ExportFormat::OpenApi => {
            Err(ApiError::Validation(format!(
                "{format:?} export applies to a whole workspace — use export_workspace"
            )))
        }
    }
}
