//! Thin client for the automation IPC socket exposed by a running
//! `spectra-tauri` GUI process (see `crates/spectra-tauri/src/automation/mod.rs`
//! for the server side and the full protocol description). This module holds
//! the actual connect/send/receive logic so the `#[tool]` method in
//! `server.rs` stays a one-line call, matching this crate's "thin wrapper"
//! convention.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// How long to wait for a response after the request is sent. Must exceed
/// the GUI-side `READY_TIMEOUT` (30s) so a legitimate "the GUI is still
/// waiting on a slow request" case surfaces the GUI's own timeout error
/// rather than this client giving up first.
const IPC_TIMEOUT: Duration = Duration::from_secs(35);

/// Wire envelope matching `spectra-tauri`'s `IpcRequest` enum (tagged on
/// `kind`) — lets both the screenshot and focus-only tools share one socket
/// and protocol.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
enum IpcRequest {
    #[serde(rename = "screenshot")]
    Screenshot { request_id: String, save_path: String, #[serde(skip_serializing_if = "Option::is_none")] environment_id: Option<String>, #[serde(default)] force_send: bool },
    #[serde(rename = "focus")]
    Focus { request_id: String, #[serde(skip_serializing_if = "Option::is_none")] environment_id: Option<String>, #[serde(default)] force_send: bool },
    #[serde(rename = "set_ui_line_numbers")]
    SetUiLineNumbers { show: bool },
    #[serde(rename = "send_and_screenshot")]
    SendAndScreenshot { request_id: String, save_path: String, #[serde(skip_serializing_if = "Option::is_none")] environment_id: Option<String> },
    #[serde(rename = "search_response")]
    SearchResponse { request_id: String, query: String, save_path: String, #[serde(skip_serializing_if = "Option::is_none")] environment_id: Option<String> },
}

#[derive(Debug, Clone, Deserialize)]
struct ScreenshotResponse {
    success: bool,
    saved_path: Option<String>,
    error: Option<String>,
}

/// Mirrors `spectra-tauri`'s `TabReadyReport` — what the frontend reported
/// about the request it opened/sent/rendered (or failed to).
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema, Serialize)]
pub struct TabReadyReport {
    pub request_id: String,
    pub workspace_id: Option<String>,
    pub name: Option<String>,
    pub url: Option<String>,
    pub rendered: bool,
    pub send_error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct FocusResponse {
    // `success` is part of the wire shape but not read here — whether the
    // request actually rendered is `report.rendered`, which the caller reads
    // directly off the returned `TabReadyReport`.
    #[allow(dead_code)]
    success: bool,
    report: Option<TabReadyReport>,
    error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SendAndScreenshotResponse {
    success: bool,
    saved_path: Option<String>,
    report: Option<TabReadyReport>,
    error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
pub struct SearchResponseResult {
    pub saved_path: String,
    pub match_count: usize,
    pub first_match_line: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchResponseIpcResponse {
    success: bool,
    result: Option<SearchResponseResult>,
    error: Option<String>,
}

fn socket_addr() -> &'static str {
    "127.0.0.1:41243"
}

/// Connects to the running GUI's automation socket, asks it to open/send
/// `request_id` and screenshot the resulting tab to `save_path`, and returns
/// the saved path on success. A connection failure is reported as a clear
/// "GUI isn't running" error rather than a raw OS error, since that's the
/// overwhelmingly likely cause (this socket only exists while spectra-tauri
/// is running — see the module doc above).
pub async fn request_screenshot(
    request_id: String,
    save_path: String,
    environment_id: Option<String>,
    force_send: bool,
) -> Result<String, String> {
    let addr = socket_addr();
    let mut stream = TcpStream::connect(addr).await.map_err(|_| {
        "Spectra GUI is not running (couldn't connect to 127.0.0.1:41243) — start it with `npm run tauri dev` first".to_string()
    })?;

    let req = IpcRequest::Screenshot { request_id, save_path, environment_id, force_send };
    let mut line = serde_json::to_string(&req).map_err(|e| format!("failed to encode request: {e}"))?;
    line.push('\n');

    stream.write_all(line.as_bytes()).await.map_err(|e| format!("failed to send request to GUI: {e}"))?;
    stream.flush().await.map_err(|e| format!("failed to send request to GUI: {e}"))?;

    let (read_half, _write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut response_line = String::new();

    let read_result = tokio::time::timeout(IPC_TIMEOUT, reader.read_line(&mut response_line)).await;
    match read_result {
        Err(_) => Err(format!("timed out after {}s waiting for the GUI to respond", IPC_TIMEOUT.as_secs())),
        Ok(Err(e)) => Err(format!("failed to read response from GUI: {e}")),
        Ok(Ok(0)) => Err("GUI closed the connection without responding".to_string()),
        Ok(Ok(_)) => {
            let resp: ScreenshotResponse = serde_json::from_str(response_line.trim())
                .map_err(|e| format!("failed to parse GUI response: {e}"))?;
            if resp.success {
                Ok(resp.saved_path.unwrap_or_default())
            } else {
                Err(resp.error.unwrap_or_else(|| "screenshot failed for an unknown reason".to_string()))
            }
        }
    }
}

/// Connects to the running GUI's automation socket and asks it to open/send
/// `request_id` (same pipeline as `request_screenshot`) but skips the
/// screenshot step, returning the frontend's status report directly. Useful
/// on its own — e.g. to navigate the GUI to a request during a screen-share,
/// or to verify the open/send/render pipeline independent of screenshot
/// capture — and was added specifically because debugging the screenshot
/// tool blind (no access to browser devtools from this environment) made
/// clear that a lighter-weight "did it actually render, and what does it
/// say" primitive was worth having on its own.
pub async fn request_focus(request_id: String, environment_id: Option<String>, force_send: bool) -> Result<TabReadyReport, String> {
    let addr = socket_addr();
    let mut stream = TcpStream::connect(addr).await.map_err(|_| {
        "Spectra GUI is not running (couldn't connect to 127.0.0.1:41243) — start it with `npm run tauri dev` first".to_string()
    })?;

    let req = IpcRequest::Focus { request_id, environment_id, force_send };
    let mut line = serde_json::to_string(&req).map_err(|e| format!("failed to encode request: {e}"))?;
    line.push('\n');

    stream.write_all(line.as_bytes()).await.map_err(|e| format!("failed to send request to GUI: {e}"))?;
    stream.flush().await.map_err(|e| format!("failed to send request to GUI: {e}"))?;

    let (read_half, _write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut response_line = String::new();

    let read_result = tokio::time::timeout(IPC_TIMEOUT, reader.read_line(&mut response_line)).await;
    match read_result {
        Err(_) => Err(format!("timed out after {}s waiting for the GUI to respond", IPC_TIMEOUT.as_secs())),
        Ok(Err(e)) => Err(format!("failed to read response from GUI: {e}")),
        Ok(Ok(0)) => Err("GUI closed the connection without responding".to_string()),
        Ok(Ok(_)) => {
            let resp: FocusResponse = serde_json::from_str(response_line.trim())
                .map_err(|e| format!("failed to parse GUI response: {e}"))?;
            match resp.report {
                Some(report) => Ok(report),
                None => Err(resp.error.unwrap_or_else(|| "focus failed for an unknown reason".to_string())),
            }
        }
    }
}

pub async fn set_ui_line_numbers(show: bool) -> Result<(), String> {
    let addr = socket_addr();
    let mut stream = TcpStream::connect(addr).await.map_err(|_| {
        "Spectra GUI is not running (couldn't connect to 127.0.0.1:41243) — start it with `npm run tauri dev` first".to_string()
    })?;

    let req = IpcRequest::SetUiLineNumbers { show };
    let mut line = serde_json::to_string(&req).map_err(|e| format!("failed to encode request: {e}"))?;
    line.push('\n');

    stream.write_all(line.as_bytes()).await.map_err(|e| format!("failed to send request to GUI: {e}"))?;
    stream.flush().await.map_err(|e| format!("failed to send request to GUI: {e}"))?;

    // Since we don't return any data, we just wait for the GUI to acknowledge by closing the stream or sending an empty object
    Ok(())
}

pub async fn request_send_and_screenshot(
    request_id: String,
    save_path: String,
    environment_id: Option<String>,
) -> Result<(String, TabReadyReport), String> {
    let addr = socket_addr();
    let mut stream = TcpStream::connect(addr).await.map_err(|_| {
        "Spectra GUI is not running (couldn't connect to 127.0.0.1:41243) — start it with `npm run tauri dev` first".to_string()
    })?;

    let req = IpcRequest::SendAndScreenshot { request_id, save_path, environment_id };
    let mut line = serde_json::to_string(&req).map_err(|e| format!("failed to encode request: {e}"))?;
    line.push('\n');

    stream.write_all(line.as_bytes()).await.map_err(|e| format!("failed to send request to GUI: {e}"))?;
    stream.flush().await.map_err(|e| format!("failed to send request to GUI: {e}"))?;

    let (read_half, _write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut response_line = String::new();

    let read_result = tokio::time::timeout(IPC_TIMEOUT, reader.read_line(&mut response_line)).await;
    match read_result {
        Err(_) => Err(format!("timed out after {}s waiting for the GUI to respond", IPC_TIMEOUT.as_secs())),
        Ok(Err(e)) => Err(format!("failed to read response from GUI: {e}")),
        Ok(Ok(0)) => Err("GUI closed the connection without responding".to_string()),
        Ok(Ok(_)) => {
            let resp: SendAndScreenshotResponse = serde_json::from_str(response_line.trim())
                .map_err(|e| format!("failed to parse GUI response: {e}"))?;
            if resp.success {
                let path = resp.saved_path.unwrap_or_default();
                let report = resp.report.unwrap_or_else(|| TabReadyReport {
                    request_id: "".to_string(),
                    workspace_id: None,
                    name: None,
                    url: None,
                    rendered: false,
                    send_error: None,
                });
                Ok((path, report))
            } else {
                Err(resp.error.unwrap_or_else(|| "send_and_screenshot failed for an unknown reason".to_string()))
            }
        }
    }
}

pub async fn request_search_response(
    request_id: String,
    query: String,
    save_path: String,
    environment_id: Option<String>,
) -> Result<SearchResponseResult, String> {
    let addr = socket_addr();
    let mut stream = TcpStream::connect(addr).await.map_err(|_| {
        "Spectra GUI is not running (couldn't connect to 127.0.0.1:41243) — start it with `npm run tauri dev` first".to_string()
    })?;

    let req = IpcRequest::SearchResponse { request_id, query, save_path, environment_id };
    let mut line = serde_json::to_string(&req).map_err(|e| format!("failed to encode request: {e}"))?;
    line.push('\n');

    stream.write_all(line.as_bytes()).await.map_err(|e| format!("failed to send request to GUI: {e}"))?;
    stream.flush().await.map_err(|e| format!("failed to send request to GUI: {e}"))?;

    let (read_half, _write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut response_line = String::new();

    let read_result = tokio::time::timeout(IPC_TIMEOUT, reader.read_line(&mut response_line)).await;
    match read_result {
        Err(_) => Err(format!("timed out after {}s waiting for the GUI to respond", IPC_TIMEOUT.as_secs())),
        Ok(Err(e)) => Err(format!("failed to read response from GUI: {e}")),
        Ok(Ok(0)) => Err("GUI closed the connection without responding".to_string()),
        Ok(Ok(_)) => {
            let resp: SearchResponseIpcResponse = serde_json::from_str(response_line.trim())
                .map_err(|e| format!("failed to parse GUI response: {e}"))?;
            if resp.success {
                Ok(resp.result.unwrap_or_else(|| SearchResponseResult {
                    saved_path: "".to_string(),
                    match_count: 0,
                    first_match_line: None,
                }))
            } else {
                Err(resp.error.unwrap_or_else(|| "search_response failed for an unknown reason".to_string()))
            }
        }
    }
}
