//! Sharing an exported file: put it on the clipboard (as a CF_HDROP file drop,
//! so Ctrl+V pastes the file into Explorer/Slack/Discord), or reveal it in the
//! OS file browser. Windows-only for now; the commands stub out elsewhere.

/// Puts `path` on the clipboard as a single-item CF_HDROP file drop.
#[cfg(windows)]
pub fn copy_file_to_clipboard(path: &str) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, GMEM_ZEROINIT,
    };
    use windows::Win32::System::Ole::CF_HDROP;
    use windows::Win32::UI::Shell::DROPFILES;

    if !std::path::Path::new(path).exists() {
        return Err("file does not exist".into());
    }
    // Wide, double-null-terminated single-item file list (one NUL ends the path,
    // a second ends the list).
    let wide: Vec<u16> = std::path::Path::new(path)
        .as_os_str()
        .encode_wide()
        .chain([0u16, 0u16])
        .collect();
    let header = std::mem::size_of::<DROPFILES>();
    let total = header + wide.len() * std::mem::size_of::<u16>();

    unsafe {
        // GMEM_ZEROINIT so DROPFILES.pt / fNC start zeroed; we only set pFiles +
        // fWide below.
        let hglobal: HGLOBAL = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, total)
            .map_err(|e| format!("GlobalAlloc: {e}"))?;
        let ptr = GlobalLock(hglobal);
        if ptr.is_null() {
            return Err("GlobalLock failed".into());
        }
        let df = ptr as *mut DROPFILES;
        (*df).pFiles = header as u32;
        (*df).fWide = true.into();
        let dst = (ptr as *mut u8).add(header) as *mut u16;
        std::ptr::copy_nonoverlapping(wide.as_ptr(), dst, wide.len());
        let _ = GlobalUnlock(hglobal);

        OpenClipboard(Some(HWND(std::ptr::null_mut())))
            .map_err(|e| format!("OpenClipboard: {e}"))?;
        if let Err(e) = EmptyClipboard() {
            let _ = CloseClipboard();
            return Err(format!("EmptyClipboard: {e}"));
        }
        // On success the system takes ownership of the block; on failure we still
        // close the clipboard (the block leaks, which Windows reclaims on exit).
        let set = SetClipboardData(CF_HDROP.0 as u32, Some(HANDLE(hglobal.0)));
        let _ = CloseClipboard();
        set.map(|_| ()).map_err(|e| format!("SetClipboardData: {e}"))
    }
}

#[cfg(not(windows))]
pub fn copy_file_to_clipboard(_path: &str) -> Result<(), String> {
    Err("unsupported".into())
}

/// Opens the OS file browser with `path` pre-selected.
#[cfg(windows)]
pub fn reveal_in_explorer(path: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    if !std::path::Path::new(path).exists() {
        return Err("file does not exist".into());
    }
    // raw_arg avoids Rust's quoting so explorer's finicky `/select,"path"` parses
    // correctly even when the path contains spaces.
    std::process::Command::new("explorer.exe")
        .raw_arg(format!("/select,\"{path}\""))
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(windows))]
pub fn reveal_in_explorer(_path: &str) -> Result<(), String> {
    Err("unsupported".into())
}

/// Opens an https URL in the default browser. The caller passes only URLs from
/// a fixed allowlist (see `external_page_url` in lib.rs) — the webview can never
/// reach this with an arbitrary URL or path.
#[cfg(windows)]
pub fn open_external(url: &str) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let verb: Vec<u16> = std::ffi::OsStr::new("open").encode_wide().chain([0]).collect();
    let file: Vec<u16> = std::ffi::OsStr::new(url).encode_wide().chain([0]).collect();
    // ShellExecuteW returns an HINSTANCE > 32 on success (legacy Win32 convention).
    let h = unsafe {
        ShellExecuteW(
            Some(HWND(std::ptr::null_mut())),
            PCWSTR(verb.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if h.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("ShellExecuteW failed ({})", h.0 as isize))
    }
}

#[cfg(not(windows))]
pub fn open_external(_url: &str) -> Result<(), String> {
    Err("unsupported".into())
}
