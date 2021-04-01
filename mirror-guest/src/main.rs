use std::ffi::c_void;
use std::slice;
use std::time::Instant;

use winapi::um::libloaderapi::GetModuleHandleA;

use dxgcap::DXGIManager;

mod cursor;

use mirror_dto::GlobalBuffer;

static mut GLOBAL_BUFFER: Option<GlobalBuffer> = None;

fn main() {
    let module_base = unsafe { GetModuleHandleA(std::ptr::null_mut()) } as u64;
    println!("module_base: 0x{:x}", module_base);

    // offsets
    {
        unsafe {
            let marker_ref = &GLOBAL_BUFFER;
            let marker_addr = marker_ref as *const _ as *const c_void as u64;
            println!("marker offset: 0x{:x}", marker_addr - module_base);
        }
    }

    let mut dxgi = DXGIManager::new(1000).expect("unable to create dxgi manager");
    let resolution = dxgi.geometry();
    println!("resolution: {:?}", resolution);
    unsafe {
        GLOBAL_BUFFER = Some(GlobalBuffer::new(resolution));
    }

    let start = Instant::now();
    let mut frame_counter = 0u32;
    loop {
        // wait for frame to be read
        loop {
            unsafe {
                if let Some(global_buffer) = &GLOBAL_BUFFER {
                    let frame_read_counter =
                        std::ptr::read_volatile(&global_buffer.frame_read_counter);
                    if frame_read_counter == global_buffer.frame_counter {
                        break;
                    }
                }
            }
        }

        // update frame
        if let Ok(frame) = dxgi.capture_frame() {
            // frame captured, put into global buffer
            unsafe {
                if let Some(global_buffer) = &mut GLOBAL_BUFFER {
                    // TODO: doesnt work for all resolutions?
                    if global_buffer.frame_buffer.len() != frame.0.len() * 4 {
                        println!("changing resolution: {:?}", frame.1);
                        global_buffer.width = frame.1 .0;
                        global_buffer.height = frame.1 .1;
                        global_buffer.frame_buffer = vec![0u8; frame.0.len() * 4];
                    }
                    global_buffer
                        .frame_buffer
                        .copy_from_slice(slice::from_raw_parts(
                            frame.0.as_ptr() as *const u8,
                            frame.0.len() * 4,
                        ));
                }
            }
        }

        // update cursor
        if let Ok(cursor) = cursor::get_state() {
            unsafe {
                if let Some(global_frame) = &mut GLOBAL_BUFFER {
                    global_frame.cursor = cursor;
                }
            }
        }

        // update frame counter
        frame_counter += 1;
        unsafe {
            if let Some(global_buffer) = &mut GLOBAL_BUFFER {
                global_buffer.frame_counter = frame_counter;
            }
        }

        if (frame_counter % 1000) == 0 {
            let elapsed = start.elapsed().as_millis() as f64;
            if elapsed > 0.0 {
                println!("{} fps", (f64::from(frame_counter)) / elapsed * 1000.0);
            }
        }
    }
}
