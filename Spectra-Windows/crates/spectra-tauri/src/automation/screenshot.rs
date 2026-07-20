//! OS-level window screenshot capture.
//!
//! On macOS/Linux this uses the `xcap` crate (in-process, gives us
//! `Window::pid()` so we can match "the window belonging to this exact
//! process" directly instead of matching by window title).
//!
//! On Windows, `xcap::Window::all()` called from *within* the process whose
//! window we're looking for silently excludes that process's own window from
//! the enumeration (confirmed via debug logging: an external probe process
//! sees the window fine via the same API; the window's own process does
//! not) — so pid-matching against `Window::all()` can never succeed for
//! self-capture there. Instead we go straight to the Tauri window's HWND and
//! capture it directly via `PrintWindow`, sidestepping enumeration entirely.

#[cfg(not(target_os = "windows"))]
use xcap::Window;

/// Finds this process's own GUI window and saves a screenshot of it to
/// `save_path`. Returns the (possibly relative, passed-through) save path on
/// success, or a plain-English error string on failure — this is surfaced up
/// through the automation IPC response and ultimately to the MCP caller, so
/// it should be readable without cross-referencing this source file.
#[cfg(not(target_os = "windows"))]
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

    save_image(image, save_path)
}

#[cfg(not(target_os = "windows"))]
fn save_image(image: image::RgbaImage, save_path: &str) -> Result<String, String> {
    if let Some(parent) = std::path::Path::new(save_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("failed to create directory for save_path: {e}"))?;
        }
    }

    image.save(save_path).map_err(|e| format!("failed to save screenshot to {save_path}: {e}"))?;

    Ok(save_path.to_string())
}

/// Windows path: capture this process's main window directly by HWND via
/// `PrintWindow`, rather than enumerating all windows and matching by pid
/// (see module doc for why that doesn't work here).
#[cfg(target_os = "windows")]
pub fn capture_hwnd(hwnd: windows::Win32::Foundation::HWND, save_path: &str) -> Result<String, String> {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetDC, GetObjectW, ReleaseDC,
        SelectObject, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
    };
    use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }.map_err(|e| format!("failed to get window rect: {e}"))?;
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width <= 0 || height <= 0 {
        return Err("Spectra window has zero size (is it minimized?)".to_string());
    }

    unsafe {
        let screen_dc = GetDC(None);
        let mem_dc = CreateCompatibleDC(Some(screen_dc));
        let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
        let old_obj = SelectObject(mem_dc, HGDIOBJ(bitmap.0));

        // PW_RENDERFULLCONTENT is required for WebView2-hosted content
        // (DirectComposition-rendered windows don't show up with plain BitBlt).
        let printed = PrintWindow(hwnd, mem_dc, PRINT_WINDOW_FLAGS(2));

        let result = if !printed.as_bool() {
            Err("PrintWindow failed".to_string())
        } else {
            let mut bmp = BITMAP::default();
            GetObjectW(HGDIOBJ(bitmap.0), std::mem::size_of::<BITMAP>() as i32, Some(&mut bmp as *mut _ as *mut _));

            let mut info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height, // negative = top-down DIB
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0 as u32,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut buf = vec![0u8; (width * height * 4) as usize];
            let scanlines = GetDIBits(
                mem_dc,
                bitmap,
                0,
                height as u32,
                Some(buf.as_mut_ptr() as *mut _),
                &mut info,
                DIB_RGB_COLORS,
            );

            if scanlines == 0 {
                Err("GetDIBits returned no data".to_string())
            } else {
                // BGRA -> RGBA
                for px in buf.chunks_exact_mut(4) {
                    px.swap(0, 2);
                }
                image::RgbaImage::from_raw(width as u32, height as u32, buf)
                    .ok_or_else(|| "failed to construct image from captured pixels".to_string())
            }
        };

        let _ = SelectObject(mem_dc, old_obj);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(mem_dc);
        ReleaseDC(None, screen_dc);

        let image = result?;

        if let Some(parent) = std::path::Path::new(save_path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("failed to create directory for save_path: {e}"))?;
            }
        }
        image
            .save(save_path)
            .map_err(|e| format!("failed to save screenshot to {save_path}: {e}"))?;

        Ok(save_path.to_string())
    }
}
