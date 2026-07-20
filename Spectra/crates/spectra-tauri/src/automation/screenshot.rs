//! OS-level window screenshot capture, using the `xcap` crate rather than
//! shelling out to macOS's `screencapture -l <window_id>` CLI. Both were
//! viable per HANDOFF.md (this project is macOS-only in practice, so
//! shelling out would have been acceptable), but `xcap` was chosen because:
//! - It's an in-process Rust API (no subprocess spawn/parse, no dependency on
//!   the `screencapture` binary's argument format staying stable).
//! - It gives us `Window::pid()`, so we can match "the window belonging to
//!   this exact process" directly instead of matching by window title
//!   (fragile — title includes the active tab name and changes constantly)
//!   or shelling out to `osascript`/`CGWindowListCopyWindowInfo` ourselves to
//!   resolve a title to a CGWindowID.
//! - It's cross-platform (Linux/Windows/macOS), so if this project ever
//!   stops being macOS-only in practice, this code doesn't need to change.

use xcap::Window;

/// Finds this process's own GUI window and saves a screenshot of it to
/// `save_path`. Returns the (possibly relative, passed-through) save path on
/// success, or a plain-English error string on failure — this is surfaced up
/// through the automation IPC response and ultimately to the MCP caller, so
/// it should be readable without cross-referencing this source file.
pub fn capture_own_window(save_path: &str) -> Result<String, String> {
    let pid = std::process::id();

    let windows = Window::all().map_err(|e| format!("failed to enumerate windows: {e}"))?;

    // Prefer a non-minimized window belonging to this process. Tauri's main
    // window is normally the only one, but this also guards against stray
    // helper windows (e.g. a devtools window) if one happens to be open.
    let window = windows
        .into_iter()
        .filter(|w| w.pid().map(|p| p == pid).unwrap_or(false))
        .filter(|w| !w.is_minimized().unwrap_or(false))
        .next()
        .ok_or_else(|| {
            "could not find the Spectra GUI window to screenshot (is it minimized, or did the window handle change?)"
                .to_string()
        })?;

    let image = window.capture_image().map_err(|e| format!("failed to capture window image: {e}"))?;

    if let Some(parent) = std::path::Path::new(save_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("failed to create directory for save_path: {e}"))?;
        }
    }

    image.save(save_path).map_err(|e| format!("failed to save screenshot to {save_path}: {e}"))?;

    Ok(save_path.to_string())
}
