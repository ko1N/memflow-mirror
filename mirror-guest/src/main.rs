//#![windows_subsystem = "windows"]

use std::mem::MaybeUninit;
use std::time::{Duration, Instant};

use log::{error, info, warn, LevelFilter};

use std::sync::mpsc::channel;
use trayicon::{MenuBuilder, TrayIconBuilder};
use winapi::um::winuser;

mod capture;
use capture::{Capture, CaptureMode};

mod cursor;

mod util;

use mirror_dto::GlobalBufferGuest;

static mut GLOBAL_BUFFER: Option<GlobalBufferGuest> = None;

fn main() {
    // TODO: runtime option in tray + config file
    let log_path = ::std::env::current_exe()
        .unwrap()
        .with_file_name("mirror-guest.log");

    // setup logging
    let log_filter = LevelFilter::Trace;
    simple_logging::log_to(std::io::stdout(), log_filter);

    log_panics::init();

    // create tray icon
    /*
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    enum Events {
        NextScreen,
        Exit,
    }
    let (send, recv) = std::sync::mpsc::channel::<Events>();
    let change_screen_menu = MenuBuilder::new().item("Next Screen", Events::NextScreen);
    let _tray_icon = TrayIconBuilder::new()
        .sender(send)
        .icon_from_buffer(include_bytes!("../resources/icon.ico"))
        .tooltip("memflow mirror guest agent")
        .menu(
            MenuBuilder::new()
                .submenu("Change Screen", change_screen_menu)
                .item("E&xit", Events::Exit),
        )
        .build()
        .expect("unable to create tray icon");
        */
    let mut screen_index = 0;
    let (tx_screen_num, rx_screen_num) = channel();
    /*
    let (tx_reset_screen_num, rx_reset_screen_num) = channel();

    std::thread::spawn(move || {
        recv.iter().for_each(|m| match m {
            Events::NextScreen => {
                let should_reset_idx = rx_reset_screen_num.try_recv().unwrap_or(false);
                if should_reset_idx {
                    screen_index = 0;
                }
                screen_index += 1;
                tx_screen_num
                    .send(screen_index)
                    .expect("could not send on channel");
            }
            Events::Exit => {
                std::process::exit(0);
            }
        })
    });
    */

    util::raise_gpu_priority();

    util::raise_process_priority();

    // we start out with dxgi capturing by default
    let mut capture = Capture::new().expect("unable to start capture");
    let mut resolution = capture.resolution();
    info!("resolution: {:?}", resolution);
    unsafe {
        GLOBAL_BUFFER = Some(GlobalBufferGuest::new(resolution, screen_index));
    }

    // main application loop
    let mut last_capture_mode_check = Instant::now();
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
        let current_screen_index: usize;
        /*
        if m != last_output {
            last_output = m;
            if m >= dxgi.get_screen_count() {
                last_output = 0;
                x_offset = 0;
                info!("resetting");
                current_screen_index = 0;
                tx_reset_screen_num
                    .send(true)
                    .expect("could not send reset signal");
                dxgi.set_capture_source_index(last_output).ok();
            } else {
                x_offset += dxgi.geometry().0 as i32;
                current_screen_index = m;
            }
            match dxgi.set_capture_source_index(current_screen_index) {
                Ok(_) => {
                    info!("changed screen successfully to {}", current_screen_index)
                }
                Err(_) => {
                    info!("Could not set defined source index to {}", m);
                }
            };
        }
        */

        // check if the frame has been read and we need to generate a new one
        unsafe {
            if let Some(global_buffer) = &mut GLOBAL_BUFFER {
                if last_capture_mode_check.elapsed() >= Duration::from_secs(1) {
                    // detect fullscreen window once per second
                    if global_buffer.config.obs {
                        if let Some(window_name) = util::find_fullscreen_window() {
                            if capture.mode() != CaptureMode::OBS(window_name.clone()) {
                                println!(
                                    "new fullscreen window detected, trying to switch to obs capture for: {}",
                                    &window_name
                                );
                                capture.set_mode(CaptureMode::OBS(window_name)).ok();
                            }
                        } else {
                            if global_buffer.config.dxgi && capture.mode() != CaptureMode::DXGI {
                                println!("fullscreen window closed, trying to switch to dxgi");
                                capture.set_mode(CaptureMode::DXGI).ok();
                            }
                        }
                    } else {
                        if global_buffer.config.dxgi && capture.mode() != CaptureMode::DXGI {
                            println!("fullscreen window closed, trying to switch to dxgi");
                            capture.set_mode(CaptureMode::DXGI).ok();
                        }
                    }

                    // TODO: update target list in config

                    // reset timer
                    last_capture_mode_check = Instant::now();
                }

                // generate new frame first then check if we can update it
                let captured_frame = capture.capture_frame();
                let update_frame = {
                    let frame_read_counter =
                        std::ptr::read_volatile(&global_buffer.frame_read_counter);
                    frame_read_counter == global_buffer.frame_counter
                };
                if captured_frame.is_ok() && update_frame {
                    let frame = captured_frame.unwrap();

                    // frame captured, put into global buffer
                    frame_counter += 1;

                    let frame_resolution = frame.resolution();
                    let frame_buffer_len = frame.buffer_len();

                    // forcefully update metadata to prevent swap-outs
                    std::ptr::write_volatile(
                        &mut global_buffer.marker,
                        [0xD, 0xE, 0xA, 0xD, 0xB, 0xA, 0xB, 0xE],
                    );

                    if global_buffer.frame_buffer.len() != frame_buffer_len {
                        info!("Changing resolution: {:?}", frame_resolution);

                        // update frame width and height & re-allocate buffer
                        resolution = frame_resolution;
                        global_buffer.frame_buffer = vec![0u8; frame_buffer_len].into();
                    }

                    std::ptr::write_volatile(&mut global_buffer.width, resolution.0);
                    std::ptr::write_volatile(&mut global_buffer.height, resolution.1);
                    std::ptr::write_volatile(
                        &mut global_buffer.frame_texmode,
                        frame.texture_mode() as u8,
                    );
                    frame.copy_frame(&mut global_buffer.frame_buffer);

                    if let Ok(cursor) = cursor::get_state(x_offset) {
                        std::ptr::write_volatile(&mut global_buffer.cursor, cursor);
                    }

                    // update frame counter
                    std::ptr::write_volatile(&mut global_buffer.frame_counter, frame_counter);
                } else {
                    // forcefully update metadata to prevent swap-outs
                    std::ptr::write_volatile(
                        &mut global_buffer.marker,
                        [0xD, 0xE, 0xA, 0xD, 0xB, 0xA, 0xB, 0xE],
                    );

                    std::ptr::write_volatile(&mut global_buffer.width, resolution.0);
                    std::ptr::write_volatile(&mut global_buffer.height, resolution.1);

                    if let Ok(cursor) = cursor::get_state(x_offset) {
                        std::ptr::write_volatile(&mut global_buffer.cursor, cursor);
                    }

                    std::ptr::write_volatile(&mut global_buffer.frame_counter, frame_counter);
                }
            }
        }
    }
}
