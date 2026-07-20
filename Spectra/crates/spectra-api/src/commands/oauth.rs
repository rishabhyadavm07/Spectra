use crate::commands::request::resolve_effective_auth;
use spectra_core::model::{AuthConfig, NamedOAuthToken, OAuth2Grant, OAuthStatus, PendingUserAction};
use spectra_core::{oauth2, oauth_flow, AppContext, ApiError, ApiResult};

pub async fn start_oauth_flow(ctx: &AppContext, request_id: String, token_name: Option<String>) -> ApiResult<PendingUserAction> {
    let req = ctx.storage.find_request(&request_id).await?;
    let grant = match resolve_effective_auth(ctx, &req).await? {
        AuthConfig::OAuth2 { grant } => grant,
        _ => return Err(ApiError::Validation("request is not configured for OAuth2".into())),
    };
    match &grant {
        OAuth2Grant::ClientCredentials { .. } | OAuth2Grant::Password { .. } | OAuth2Grant::RefreshToken { .. } => {
            return Err(ApiError::Unsupported(
                "this grant type does not require an interactive flow; call send_request directly".into(),
            ));
        }
        _ => {}
    }
    oauth_flow::start_flow(ctx.http.clone(), ctx.oauth_store.clone(), request_id, grant, token_name).await
}

pub async fn finish_oauth_flow(ctx: &AppContext, url: String) -> ApiResult<()> {
    oauth_flow::finish_flow_from_url(&ctx.http, &ctx.oauth_store, &url).await
}

pub async fn get_oauth_status(ctx: &AppContext, request_id: String) -> ApiResult<OAuthStatus> {
    Ok(ctx.oauth_store.get(&request_id).await)
}

pub async fn cancel_oauth_flow(ctx: &AppContext, request_id: String) -> ApiResult<()> {
    ctx.oauth_store.cancel_task(&request_id).await;
    ctx.oauth_store.set(&request_id, OAuthStatus::NotStarted).await;
    Ok(())
}

/// Fetches a fresh token on demand for a non-interactive grant (Client
/// Credentials/Password/Refresh Token) and saves it under `name` — the
/// "Get New Access Token" button in Postman's OAuth2 panel. Interactive
/// grants use start_oauth_flow instead, which already saves under a
/// generated name once the loopback/device flow completes.
pub async fn fetch_oauth_token(ctx: &AppContext, request_id: String, name: Option<String>) -> ApiResult<NamedOAuthToken> {
    let req = ctx.storage.find_request(&request_id).await?;
    let grant = match resolve_effective_auth(ctx, &req).await? {
        AuthConfig::OAuth2 { grant } => grant,
        _ => return Err(ApiError::Validation("request is not configured for OAuth2".into())),
    };
    let token = oauth2::fetch_token_noninteractive(&ctx.http, &grant)
        .await?
        .ok_or_else(|| ApiError::Unsupported("this grant type requires an interactive flow; call start_oauth_flow".into()))?;
    ctx.oauth_store.save_token(&request_id, name, token).await;
    ctx.oauth_store
        .list_tokens(&request_id)
        .await
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::IoError("token was saved but could not be read back".into()))
}

pub async fn refresh_oauth_token(ctx: &AppContext, request_id: String) -> ApiResult<NamedOAuthToken> {
    let req = ctx.storage.find_request(&request_id).await?;
    let grant = match resolve_effective_auth(ctx, &req).await? {
        AuthConfig::OAuth2 { grant } => grant,
        _ => return Err(ApiError::Validation("request is not configured for OAuth2".into())),
    };

    let refresh_token = ctx.oauth_store.expired_refresh_token(&request_id).await
        .ok_or_else(|| ApiError::Auth("No refresh token available to refresh".into()))?;

    let refreshed = oauth2::refresh_grant_token(&ctx.http, &grant, &refresh_token)
        .await?
        .ok_or_else(|| ApiError::Auth("Could not refresh token".into()))?;

    // Let's just save the refreshed token as a new default token, or we could pass the old name.
    // Since save_token just takes Option<String>, None creates a timestamped name.
    let current_name: Option<String> = None;

    ctx.oauth_store.save_token(&request_id, current_name, refreshed).await;
    
    ctx.oauth_store
        .list_tokens(&request_id)
        .await
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::IoError("token was saved but could not be read back".into()))
}

pub async fn list_oauth_tokens(ctx: &AppContext, request_id: String) -> ApiResult<Vec<NamedOAuthToken>> {
    Ok(ctx.oauth_store.list_tokens(&request_id).await)
}

pub async fn select_oauth_token(ctx: &AppContext, request_id: String, name: String) -> ApiResult<()> {
    ctx.oauth_store.select_token(&request_id, &name).await
}

pub async fn delete_oauth_token(ctx: &AppContext, request_id: String, name: String) -> ApiResult<()> {
    ctx.oauth_store.delete_token(&request_id, &name).await;
    Ok(())
}
