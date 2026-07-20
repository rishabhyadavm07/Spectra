use spectra_core::model::AppSettings;
use spectra_core::{AppContext, ApiResult};

pub async fn get_settings(ctx: &AppContext) -> ApiResult<AppSettings> {
    ctx.storage.get_settings().await
}

pub async fn save_settings(ctx: &AppContext, settings: AppSettings) -> ApiResult<()> {
    ctx.storage.save_settings(&settings).await
}
