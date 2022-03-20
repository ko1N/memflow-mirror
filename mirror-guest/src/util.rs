use std::ffi::CString;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::ptr;
use std::slice;

use log::{error, info, warn};

use winapi::shared::windef::RECT;
use winapi::um::libloaderapi::{GetModuleHandleA, GetProcAddress};
use winapi::um::processthreadsapi::GetCurrentProcess;
use winapi::um::processthreadsapi::SetPriorityClass;
use winapi::um::shellapi::{SHQueryUserNotificationState, QUNS_BUSY, QUNS_RUNNING_D3D_FULL_SCREEN};
use winapi::um::winbase::REALTIME_PRIORITY_CLASS;
use winapi::um::winnt::HANDLE;
use winapi::um::winuser::{
    GetDesktopWindow, GetForegroundWindow, GetShellWindow, GetWindowRect, GetWindowTextW,
};

pub fn raise_gpu_priority() {
    {
        let gdi32str = CString::new("gdi32").unwrap();
        let gdi32 = unsafe { GetModuleHandleA(gdi32str.as_ptr()) };
        if gdi32.is_null() {
            error!("Failed to set priority: unable to find gdi32.dll");
            return;
        }

        let dkmtschedulestr = CString::new("D3DKMTSetProcessSchedulingPriorityClass").unwrap();
        let funcptr = unsafe { GetProcAddress(gdi32, dkmtschedulestr.as_ptr()) };
        if funcptr.is_null() {
            error!(
                "Failed to set priority: unable to find D3DKMTSetProcessSchedulingPriorityClass"
            );
            return;
        }

        let func: unsafe extern "C" fn(HANDLE, i32) -> i32 =
            unsafe { std::mem::transmute(funcptr) };
        let result = unsafe {
            func(
                GetCurrentProcess() as _,
                5, /* D3DKMT_SCHEDULINGPRIORITYCLASS_REALTIME */
            )
        };
        if result < 0 {
            error!("Failed to set gpu priority. This usually indicates the process does not run as the system user.");
        }
    }
}

pub fn raise_process_priority() {
    if unsafe { SetPriorityClass(GetCurrentProcess(), REALTIME_PRIORITY_CLASS) } != 0 {
        info!("Process priority set to HIGH");
    } else {
        error!("Unable to set process priority. Maybe restart with admin rights?");
    }

    match thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Max) {
        Ok(_) => info!("Main thread's priority set to max"),
        Err(_) => error!("Failed setting main thread's priority"),
    };
}

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
