mod automation;
mod commands;

use automation::AutomationState;
use spectra_core::cookies::ClearableCookieStore;
use spectra_core::secrets::KeychainSecretStore;
use spectra_core::storage::Storage;
use spectra_core::AppContext;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = dirs_home().join(".spectra").join("workspaces");
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    let storage = rt.block_on(async { Storage::new(data_dir).await.expect("failed to init storage") });
    let settings = rt.block_on(async { storage.get_settings().await.unwrap_or_default() });
    let cookie_store = ClearableCookieStore::new();
    let http = reqwest::Client::builder()
        .cookie_provider(Arc::new(cookie_store.clone()))
        .danger_accept_invalid_certs(!settings.ssl_verification)
        .timeout(std::time::Duration::from_millis(settings.request_timeout_ms))
        .build()
        .unwrap();
    let ctx = AppContext::new(storage, cookie_store, http, Arc::new(KeychainSecretStore));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .manage(ctx)
        .manage(AutomationState::default())
        .setup(|app| {
            // Starts the automation IPC socket only while the GUI is
            // actually running — see crate::automation's module doc.
            automation::start(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_workspaces,
            commands::create_workspace,
            commands::open_workspace,
            commands::list_requests,
            commands::open_request,
            commands::create_request,
            commands::delete_request,
            commands::set_method,
            commands::set_url,
            commands::set_name,
            commands::set_notes,
            commands::set_headers,
            commands::set_params,
            commands::set_body,
            commands::set_auth,
            commands::get_auth,
            commands::get_effective_auth,
            commands::clear_auth,
            commands::list_auth_types,
            commands::send_request,
            commands::preview_headers,
            commands::clear_cookies,
            commands::start_oauth_flow,
            commands::finish_oauth_flow,
            commands::get_oauth_status,
            commands::cancel_oauth_flow,
            commands::fetch_oauth_token,
            commands::refresh_oauth_token,
            commands::list_oauth_tokens,
            commands::select_oauth_token,
            commands::delete_oauth_token,
            commands::list_environments,
            commands::create_environment,
            commands::update_environment,
            commands::delete_environment,
            commands::check_secrets_health,
            commands::set_active_environment,
            commands::set_workspace_auth,
            commands::list_folders,
            commands::create_folder,
            commands::set_folder_auth,
            commands::rename_folder,
            commands::move_folder,
            commands::delete_folder,
            commands::move_request,
            commands::list_history,
            commands::delete_history_entry,
            commands::list_history_for_request,
            commands::replay_history_entry,
            commands::convert_history_to_request,
            commands::list_saved_responses,
            commands::save_response,
            commands::delete_saved_response,
            commands::list_saved_auths,
            commands::get_saved_auth,
            commands::save_saved_auth,
            commands::delete_saved_auth,
            commands::import_collection,
            commands::export_workspace,
            commands::export_request,
            commands::automation_tab_ready,
            commands::automation_search_ready,
            commands::get_settings,
            commands::save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn dirs_home() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}
