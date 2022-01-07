#![windows_subsystem = "windows"]

use std::ffi::CString;
use std::mem::MaybeUninit;
use std::slice;

use log::{error, info, LevelFilter};

use trayicon::{Icon, MenuBuilder, MenuItem, TrayIconBuilder};
use winapi::um::libloaderapi::{GetModuleHandleA, GetProcAddress};
use winapi::um::processthreadsapi::GetCurrentProcess;
use winapi::um::winnt::HANDLE;
use winapi::um::winuser;
use std::sync::mpsc::channel;

mod dxgi;
use dxgi::DXGIManager;

mod cursor;

use mirror_dto::GlobalBuffer;

static mut GLOBAL_BUFFER: Option<GlobalBuffer> = None;

fn raise_gpu_priority() {
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
            error!("Failed to set gpu priority");
        }
    }
}

fn main() {
    let log_path = ::std::env::current_exe()
        .unwrap()
        .with_file_name("mirror-guest.log");

    // setup logging
    let log_filter = LevelFilter::Trace;
    match simple_logging::log_to_file(&log_path, log_filter) {
        Ok(_) => info!("logging initialized with level {:?}", log_filter),
        Err(err) => {
            panic!("unable to initialize logging: {}", err);
        }
    }

    log_panics::init();

    // create tray icon
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    enum Events {
        NextScreen,
        Exit,
    }
    let (send, recv) = std::sync::mpsc::channel::<Events>();
    let mut change_screen_menu = MenuBuilder::new().item("Next Screen", Events::NextScreen);
    let _tray_icon = TrayIconBuilder::new()
        .sender(send)
        .icon_from_buffer(include_bytes!("../resources/icon.ico"))
        .tooltip("memflow mirror guest agent")
        .menu(MenuBuilder::new().submenu("Change Screen", change_screen_menu).item("E&xit", Events::Exit))
        .build()
        .expect("unable to create tray icon");
    let mut screen_index = 0;
    let (tx_screen_num, rx_screen_num) = channel();
    let (tx_reset_screen_num, rx_reset_screen_num) = channel();

    std::thread::spawn(move || {
        recv.iter().for_each(|m| match m {
            Events::NextScreen => {
                let should_reset_idx = rx_reset_screen_num.try_recv().unwrap_or(false);
                if should_reset_idx {
                    screen_index = 0;
                }
                screen_index += 1;
                tx_screen_num.send(screen_index).expect("could not send on channel");
            }
            Events::Exit => {
                std::process::exit(0);
            }
        })
    });

    raise_gpu_priority();
    let mut dxgi = DXGIManager::new(1000).expect("unable to create dxgi manager");
    let mut resolution = dxgi.geometry();
    info!("resolution: {:?}", resolution);
    unsafe {
        GLOBAL_BUFFER = Some(GlobalBuffer::new(resolution, screen_index));
    }

    // main application loop
    let mut frame_counter = 0u32;
    let mut last_output = 0;
    let mut x_offset: i32 = 0;
    loop {
        // tray icon loop
        unsafe {
            let mut msg = MaybeUninit::uninit();
            let bret = winuser::PeekMessageA(msg.as_mut_ptr(), 0 as _, 0, 0, winuser::PM_REMOVE);
            if bret > 0 {
                winuser::TranslateMessage(msg.as_ptr());
                winuser::DispatchMessageA(msg.as_ptr());
            }
        }
        let m = rx_screen_num.try_recv().unwrap_or(last_output);
        if m != last_output {
            match dxgi.set_capture_source_index(m) {
                Ok(_) => {
                    last_output = m;
                    x_offset += dxgi.geometry().0 as i32;
                },
                Err(_) => {
                    last_output = 0;
                    x_offset = 0;
                    tx_reset_screen_num.send(true).expect("could not send reset signal");
                    dxgi.set_capture_source_index(last_output);
                }
            };
        }
        

        // check if the frame has been read and we need to generate a new one
        let update_frame = unsafe {
            if let Some(global_buffer) = &GLOBAL_BUFFER {
                let frame_read_counter = std::ptr::read_volatile(&global_buffer.frame_read_counter);
                frame_read_counter == global_buffer.frame_counter
            } else {
                false
            }
        };

        // update frame
        if update_frame {
            if let Ok(frame) = dxgi.capture_frame() {
                // frame captured, put into global buffer
                frame_counter += 1;

                unsafe {
                    if let Some(global_buffer) = &mut GLOBAL_BUFFER {
                        if global_buffer.frame_buffer.len() != frame.0.len() * 4 {
                            info!("changing resolution: {:?}", frame.1);

                            // update frame width and height
                            resolution.0 = frame.1 .0;
                            resolution.1 = frame.1 .1;
                            std::ptr::write_volatile(&mut global_buffer.width, resolution.0);
                            std::ptr::write_volatile(&mut global_buffer.height, resolution.1);

                            // re-allocate buffer
                            global_buffer.frame_buffer = vec![0u8; frame.0.len() * 4];
                        }

                        // TODO: store frame buffer copy to rewrite it as well down below
                        global_buffer
                            .frame_buffer
                            .copy_from_slice(slice::from_raw_parts(
                                frame.0.as_ptr() as *const u8,
                                frame.0.len() * 4,
                            ));
                    }
                }
            }
        }

        // write metadata + cursor state in any case to prevent swap-outs on inactivity
        if let Ok(cursor) = cursor::get_state(x_offset) {
            unsafe {
                if let Some(global_buffer) = &mut GLOBAL_BUFFER {
                    // forcefully update metadata to prevent swap-outs
                    std::ptr::write_volatile(
                        &mut global_frame.marker,
                        [0xD, 0xE, 0xA, 0xD, 0xB, 0xA, 0xB, 0xE],
                    );

                    std::ptr::write_volatile(&mut global_buffer.frame_counter, frame_counter);
                    std::ptr::write_volatile(&mut global_buffer.width, resolution.0);
                    std::ptr::write_volatile(&mut global_buffer.height, resolution.1);

                    // update cursor
                    std::ptr::write_volatile(&mut global_buffer.cursor, cursor);
                }
            }
        }
    }
}
