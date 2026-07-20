mod automation_client;
mod server;

use rmcp::ServiceExt;
use rmcp::transport::stdio;
use spectra_core::secrets::KeychainSecretStore;
use spectra_core::storage::Storage;
use spectra_core::AppContext;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let data_dir = dirs_home().join(".spectra").join("workspaces");
    let storage = Storage::new(data_dir).await.expect("failed to init storage");
    let cookie_store = spectra_core::cookies::ClearableCookieStore::new();
    let http = reqwest::Client::builder().cookie_provider(Arc::new(cookie_store.clone())).build().unwrap();
    let ctx = AppContext::new(storage, cookie_store, http, Arc::new(KeychainSecretStore));

    let server = server::SpectraServer::new(ctx);
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

fn dirs_home() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}
