//! Local IPC automation channel between the running Spectra GUI process and
//! external clients (currently: `spectra-mcp`'s `automation_screenshot_request`
//! tool). This is intentionally GUI-only glue, not a `spectra-api` command:
//! it doesn't touch domain state at all (it drives the *already-running React
//! UI* of this exact process — opening a tab, waiting for a render, taking an
//! OS-level screenshot of this process's own window), so there's no
//! "business logic" here for spectra-api to own. See HANDOFF.md's
//! architecture rule for why this note exists.
//!
//! Transport: a Unix domain socket at `~/.spectra/automation.sock`, chosen
//! over a TCP port to avoid any port-conflict risk on a machine running
//! multiple local dev tools — this project is macOS-only in practice (per
//! HANDOFF.md), so a Unix socket has no portability downside here. Protocol
//! is newline-delimited JSON, one request object in, one response object out,
//! then the connection is closed (no persistent session).
//!
//! Flow for a single screenshot request:
//! 1. A client connects to the socket and sends `{"request_id", "save_path",
//!    "environment_id"}` as one JSON line.
//! 2. This module emits a `automation://prepare-request` event to the
//!    frontend with that payload.
//! 3. The frontend (see `src/automation.ts`) opens/switches to the tab for
//!    that request via the app's normal tab-opening logic, sends it if there
//!    is no response yet, waits for the response to render, then calls the
//!    `automation_tab_ready` command back on this side.
//! 4. That command signals a `tokio::sync::Notify` keyed by request_id that
//!    this module is waiting on (with a timeout, in case the frontend never
//!    reports ready — e.g. the request errors in a way the frontend doesn't
//!    treat as "settled").
//! 5. Once signaled (or timed out), this module screenshots this process's
//!    own window (via `xcap`, matched by OS pid rather than title so it's
//!    robust to window-title changes) and saves it to `save_path`.
//! 6. The result (success + saved path, or an error message) is written back
//!    as one JSON line and the socket connection closes.

mod screenshot;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, Notify};

/// How long to wait for the frontend to report the tab/response is ready to
/// screenshot before giving up. Generous because response times (and slow
/// OAuth interactive flows, if the request needs one) vary; this is meant to
/// catch "the frontend never calls back" rather than police fast-happy-path
/// latency.
const READY_TIMEOUT: Duration = Duration::from_secs(30);

/// Payload sent from an IPC client (spectra-mcp) to request a screenshot,
/// and the same shape re-emitted to the frontend as the `automation://prepare-request` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotRequest {
    pub request_id: String,
    pub save_path: String,
    #[serde(default)]
    pub environment_id: Option<String>,
    #[serde(default)]
    pub force_send: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotResponse {
    pub success: bool,
    pub saved_path: Option<String>,
    pub error: Option<String>,
}

/// A request to just open/switch/send a request in the GUI without taking a
/// screenshot — the `automation_focus_request` MCP tool's payload. Useful on
/// its own (e.g. to visually confirm a request in a screen-sharing session)
/// and as a lighter-weight way to verify the open/send/render pipeline
/// independent of screenshot capture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusRequest {
    pub request_id: String,
    #[serde(default)]
    pub environment_id: Option<String>,
    #[serde(default)]
    pub force_send: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetUiLineNumbersRequest {
    pub show: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendAndScreenshotRequest {
    pub request_id: String,
    pub save_path: String,
    #[serde(default)]
    pub environment_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponseRequest {
    pub request_id: String,
    pub query: String,
    pub save_path: String,
    #[serde(default)]
    pub environment_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusResponse {
    pub success: bool,
    /// What the frontend reports about the tab it opened — see `TabReadyReport`.
    pub report: Option<TabReadyReport>,
    pub error: Option<String>,
}

/// Reported by the frontend when it calls `automation_tab_ready` back, so the
/// Rust side (and, through it, the MCP caller) can tell the difference
/// between "opened and rendered a real response," "opened but the request
/// itself errored," and "never actually found/opened the request" — instead
/// of only knowing that *some* callback happened, which was the previous
/// blind-trust behavior this session's debugging showed isn't good enough.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabReadyReport {
    pub request_id: String,
    pub workspace_id: Option<String>,
    pub name: Option<String>,
    pub url: Option<String>,
    /// True once a response or a send-error was observed in the tab; false
    /// if the frontend gave up (e.g. couldn't resolve the request at all).
    pub rendered: bool,
    pub send_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendAndScreenshotResponse {
    pub success: bool,
    pub saved_path: Option<String>,
    pub report: Option<TabReadyReport>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchReadyReport {
    pub request_id: String,
    pub match_count: usize,
    pub first_match_line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponseResult {
    pub saved_path: String,
    pub match_count: usize,
    pub first_match_line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponseIpcResponse {
    pub success: bool,
    pub result: Option<SearchResponseResult>,
    pub error: Option<String>,
}

/// Shared state, held in Tauri's `.manage()`, tracking in-flight
/// "waiting for the frontend to say it's ready to be screenshotted" requests
/// keyed by request_id. A `Notify` rather than a oneshot channel because the
/// registration (from the IPC handler) and the notification (from the
/// `automation_tab_ready` command) can't easily share ownership of a
/// single-use sender across that boundary as cleanly as a shared `Notify`.
#[derive(Default)]
pub struct AutomationState {
    waiters: Mutex<HashMap<String, Arc<Notify>>>,
    /// The most recent `TabReadyReport` the frontend has sent for a given
    /// request_id, so the waiting IPC handler can read back *what* happened
    /// (not just *that* something happened) once `signal_ready` wakes it.
    reports: Mutex<HashMap<String, TabReadyReport>>,
    search_reports: Mutex<HashMap<String, SearchReadyReport>>,
}

impl AutomationState {
    async fn register_waiter(&self, request_id: &str) -> Arc<Notify> {
        let notify = Arc::new(Notify::new());
        self.waiters.lock().await.insert(request_id.to_string(), notify.clone());
        notify
    }

    async fn clear_waiter(&self, request_id: &str) {
        self.waiters.lock().await.remove(request_id);
    }

    async fn take_report(&self, request_id: &str) -> Option<TabReadyReport> {
        self.reports.lock().await.remove(request_id)
    }
    
    async fn take_search_report(&self, request_id: &str) -> Option<SearchReadyReport> {
        self.search_reports.lock().await.remove(request_id)
    }

    /// Called by the `automation_tab_ready` Tauri command once the frontend
    /// has opened the tab, sent the request if needed, and rendered the
    /// response (or given up and reported why).
    pub async fn signal_ready(&self, report: TabReadyReport) {
        let request_id = report.request_id.clone();
        self.reports.lock().await.insert(request_id.clone(), report);
        if let Some(notify) = self.waiters.lock().await.get(&request_id) {
            notify.notify_waiters();
        }
    }
    
    pub async fn signal_search_ready(&self, report: SearchReadyReport) {
        let request_id = report.request_id.clone();
        self.search_reports.lock().await.insert(request_id.clone(), report);
        if let Some(notify) = self.waiters.lock().await.get(&request_id) {
            notify.notify_waiters();
        }
    }
}

/// Path to the automation socket: `~/.spectra/automation.sock`, alongside the
/// `~/.spectra/workspaces` data directory `lib.rs` already uses.
fn socket_addr() -> &'static str {
    "127.0.0.1:41243"
}

/// Starts the IPC server as a background Tokio task. Fire-and-forget from
/// `lib.rs`'s `run()` — errors (e.g. failing to bind) are logged, not
/// propagated, since automation is an optional capability and shouldn't be
/// able to prevent the GUI itself from starting.
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = serve(app).await {
            eprintln!("[automation] IPC server failed to start: {e}");
        }
    });
}

async fn serve(app: AppHandle) -> std::io::Result<()> {
    let addr = socket_addr();
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = handle_connection(app, stream).await {
                eprintln!("[automation] connection error: {e}");
            }
        });
    }
}

/// Wire envelope distinguishing the two request kinds a client can send on
/// the automation socket. Tagged on `kind` so `spectra-mcp`'s two client
/// functions (screenshot vs. focus-only) can share one socket/protocol.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind")]
enum IpcRequest {
    #[serde(rename = "screenshot")]
    Screenshot(ScreenshotRequest),
    #[serde(rename = "focus")]
    Focus(FocusRequest),
    #[serde(rename = "set_ui_line_numbers")]
    SetUiLineNumbers(SetUiLineNumbersRequest),
    #[serde(rename = "send_and_screenshot")]
    SendAndScreenshot(SendAndScreenshotRequest),
    #[serde(rename = "search_response")]
    SearchResponse(SearchResponseRequest),
}

async fn handle_connection(app: AppHandle, stream: TcpStream) -> std::io::Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let out = match serde_json::from_str::<IpcRequest>(line.trim()) {
        Ok(IpcRequest::Screenshot(req)) => {
            let response = handle_screenshot_request(&app, req).await;
            serde_json::to_string(&response)
        }
        Ok(IpcRequest::Focus(req)) => {
            let response = handle_focus_request(&app, req).await;
            serde_json::to_string(&response)
        }
        Ok(IpcRequest::SetUiLineNumbers(req)) => {
            // Tell the frontend to toggle line numbers globally via an event
            let _ = app.emit("automation://set-line-numbers", req);
            Ok(String::from("{}")) // empty JSON object to ACK
        }
        Ok(IpcRequest::SendAndScreenshot(req)) => {
            let response = handle_send_and_screenshot_request(&app, req).await;
            serde_json::to_string(&response)
        }
        Ok(IpcRequest::SearchResponse(req)) => {
            let response = handle_search_response_request(&app, req).await;
            serde_json::to_string(&response)
        }
        Err(e) => {
            let response = ScreenshotResponse {
                success: false,
                saved_path: None,
                error: Some(format!("invalid request: {e}")),
            };
            serde_json::to_string(&response)
        }
    };

    let mut out = out.unwrap_or_else(|e| {
        format!(r#"{{"success":false,"saved_path":null,"error":"failed to serialize response: {e}"}}"#)
    });
    out.push('\n');
    write_half.write_all(out.as_bytes()).await?;
    write_half.flush().await?;
    Ok(())
}

/// Shared by both request kinds: register a waiter, emit the prepare event,
/// wait for the frontend's readiness callback (with the same timeout), and
/// return the `TabReadyReport` it sent back (or `Err` on timeout/emit failure).
async fn wait_for_frontend(
    app: &AppHandle,
    request_id: &str,
    environment_id: Option<String>,
    force_send: bool,
) -> Result<TabReadyReport, String> {
    let state = app.state::<AutomationState>();
    let notify = state.register_waiter(request_id).await;

    let payload = ScreenshotRequest {
        request_id: request_id.to_string(),
        save_path: String::new(),
        environment_id,
        force_send,
    };
    if let Err(e) = app.emit("automation://prepare-request", &payload) {
        state.clear_waiter(request_id).await;
        return Err(format!("failed to notify frontend: {e}"));
    }

    let ready = tokio::time::timeout(READY_TIMEOUT, notify.notified()).await;
    state.clear_waiter(request_id).await;

    if ready.is_err() {
        return Err(format!(
            "timed out after {}s waiting for the GUI to open/send request {} and render its response",
            READY_TIMEOUT.as_secs(),
            request_id
        ));
    }

    state
        .take_report(request_id)
        .await
        .ok_or_else(|| "frontend signaled ready but sent no status report".to_string())
}

async fn handle_focus_request(app: &AppHandle, req: FocusRequest) -> FocusResponse {
    match wait_for_frontend(app, &req.request_id, req.environment_id, req.force_send).await {
        Ok(report) => FocusResponse { success: report.rendered, report: Some(report), error: None },
        Err(e) => FocusResponse { success: false, report: None, error: Some(e) },
    }
}

async fn handle_screenshot_request(app: &AppHandle, req: ScreenshotRequest) -> ScreenshotResponse {
    let report = match wait_for_frontend(app, &req.request_id, req.environment_id, req.force_send).await {
        Ok(report) => report,
        Err(e) => return ScreenshotResponse { success: false, saved_path: None, error: Some(e) },
    };

    if !report.rendered {
        return ScreenshotResponse {
            success: false,
            saved_path: None,
            error: Some(format!(
                "request {} never rendered a response or error in the GUI — nothing to screenshot",
                req.request_id
            )),
        };
    }

    match capture_screenshot_with_focus(app, &req.save_path).await {
        Ok(path) => ScreenshotResponse { success: true, saved_path: Some(path), error: None },
        Err(e) => ScreenshotResponse { success: false, saved_path: None, error: Some(e) },
    }
}

async fn handle_send_and_screenshot_request(app: &AppHandle, req: SendAndScreenshotRequest) -> SendAndScreenshotResponse {
    let report = match wait_for_frontend(app, &req.request_id, req.environment_id, false).await {
        Ok(report) => report,
        Err(e) => return SendAndScreenshotResponse { success: false, saved_path: None, report: None, error: Some(e) },
    };

    if !report.rendered {
        return SendAndScreenshotResponse {
            success: false,
            saved_path: None,
            report: Some(report.clone()),
            error: Some(format!(
                "request {} never rendered a response or error in the GUI — nothing to screenshot",
                req.request_id
            )),
        };
    }

    match capture_screenshot_with_focus(app, &req.save_path).await {
        Ok(path) => SendAndScreenshotResponse { success: true, saved_path: Some(path), report: Some(report), error: None },
        Err(e) => SendAndScreenshotResponse { success: false, saved_path: None, report: Some(report), error: Some(e) },
    }
}

async fn handle_search_response_request(app: &AppHandle, req: SearchResponseRequest) -> SearchResponseIpcResponse {
    let state = app.state::<AutomationState>();
    let notify = state.register_waiter(&req.request_id).await;

    if let Err(e) = app.emit("automation://search-response", &req) {
        state.clear_waiter(&req.request_id).await;
        return SearchResponseIpcResponse { success: false, result: None, error: Some(format!("failed to notify frontend: {e}")) };
    }

    let ready = tokio::time::timeout(READY_TIMEOUT, notify.notified()).await;
    state.clear_waiter(&req.request_id).await;

    if ready.is_err() {
        return SearchResponseIpcResponse {
            success: false,
            result: None,
            error: Some(format!("timed out after {}s waiting for the GUI to search response", READY_TIMEOUT.as_secs())),
        };
    }

    let report = match state.take_search_report(&req.request_id).await {
        Some(r) => r,
        None => return SearchResponseIpcResponse { success: false, result: None, error: Some("frontend signaled ready but sent no search report".to_string()) },
    };

    // Even if match_count is 0, we can still take a screenshot
    match capture_screenshot_with_focus(app, &req.save_path).await {
        Ok(path) => SearchResponseIpcResponse { 
            success: true, 
            result: Some(SearchResponseResult {
                saved_path: path,
                match_count: report.match_count,
                first_match_line: report.first_match_line,
            }), 
            error: None 
        },
        Err(e) => SearchResponseIpcResponse { success: false, result: None, error: Some(e) },
    }
}

async fn capture_screenshot_with_focus(app: &AppHandle, save_path: &str) -> Result<String, String> {
    let window = app.get_webview_window("main");
    if let Some(window) = &window {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
    }

    #[cfg(target_os = "windows")]
    {
        let window = window.ok_or_else(|| "no main window found".to_string())?;
        let hwnd = window.hwnd().map_err(|e| format!("failed to get window handle: {e}"))?;
        screenshot::capture_hwnd(hwnd, save_path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        screenshot::capture_own_window(save_path)
    }
}
