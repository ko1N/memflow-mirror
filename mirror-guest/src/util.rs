use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::ptr;

use winapi::shared::windef::RECT;
use winapi::um::shellapi::{SHQueryUserNotificationState, QUNS_BUSY, QUNS_RUNNING_D3D_FULL_SCREEN};
use winapi::um::winuser::{
    GetDesktopWindow, GetForegroundWindow, GetShellWindow, GetWindowRect, GetWindowTextW,
};

/// Tries to find a fullscreen window.
/// On success this function returns the name of the window, otherwise None.
pub fn find_fullscreen_window() -> Option<String> {
    // windows 7, vista and above
    let mut pquns = 0;
    unsafe { SHQueryUserNotificationState(&mut pquns) };
    if pquns == QUNS_BUSY || pquns == QUNS_RUNNING_D3D_FULL_SCREEN {
        let hwnd = unsafe { GetForegroundWindow() };

        let name = vec![0u16; 1024];
        let ptr = name.as_ptr();
        let name_len = unsafe { GetWindowTextW(hwnd, ptr as *mut u16, 1024) };
        if name_len > 0 {
            // convert name to string
            let osstr = OsString::from_wide(&name[..name_len as usize]);
            osstr.into_string().ok()
        } else {
            // name could not be read
            None
        }
    } else {
        None
    }

    // below win7 (do we even support this?)
    /*
    let desktop_wnd = unsafe { GetDesktopWindow() };
    let shell_wnd = unsafe { GetShellWindow() };

    let wnd = unsafe { GetForegroundWindow() };
    if wnd != ptr::null_mut() {
        if wnd != desktop_wnd && wnd != shell_wnd {
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            unsafe { GetWindowRect(wnd, &mut rect) }; // TODO: error check
                                                      // GetWindowRect(GetDesktopWindow(), &rc);
        }
    }
    */
}
