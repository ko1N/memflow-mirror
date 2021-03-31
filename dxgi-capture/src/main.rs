use std::ffi::c_void;
use std::time::Instant;

use winapi::um::libloaderapi::GetModuleHandleA;

mod dxgi;
use dxgi::{DXGIManager, BGRA8};

mod cursor;

#[repr(C)]
pub struct GlobalFrameBuffer {
    _marker: [u8; 8],           // 0x0
    resolution: (usize, usize), // 0x8
    frame_counter: u32,         // 0x18
    frame_buffer: Vec<BGRA8>,   // 0x20
    cursor: cursor::Cursor,     // 0x28
}

static mut GLOBAL_FRAME_BUFFER: Option<GlobalFrameBuffer> = None;

fn main() {
    let module_base = unsafe { GetModuleHandleA(std::ptr::null_mut()) } as u64;
    println!("module_base: 0x{:x}", module_base);

    // offsets
    {
        unsafe {
            let marker_ref = &GLOBAL_FRAME_BUFFER;
            let marker_addr = marker_ref as *const _ as *const c_void as u64;
            println!("marker offset: 0x{:x}", marker_addr - module_base);
        }
    }

    let mut dxgi = DXGIManager::new(1000).expect("unable to create dxgi manager");
    let geometry = dxgi.geometry();
    println!("geometry: {:?}", geometry);
    unsafe {
        GLOBAL_FRAME_BUFFER = Some(GlobalFrameBuffer {
            _marker: [0xD, 0xE, 0xA, 0xD, 0xB, 0xA, 0xB, 0xE],
            resolution: geometry,
            frame_counter: 0,
            frame_buffer: vec![
                BGRA8 {
                    b: 0,
                    g: 0,
                    r: 0,
                    a: 0
                };
                geometry.0 * geometry.1
            ],
            cursor: cursor::Cursor::default(),
        })
    }

    let start = Instant::now();
    let mut frame_counter = 0u32;
    loop {
        // generate frame
        if let Ok(frame) = dxgi.capture_frame() {
            // frame captured, put into global buffer
            unsafe {
                if let Some(global_frame) = &mut GLOBAL_FRAME_BUFFER {
                    global_frame
                        .frame_buffer
                        .copy_from_slice(frame.0.as_slice());
                    global_frame.resolution = frame.1;
                    global_frame.frame_counter = frame_counter;
                }
            }

            frame_counter += 1;

            if (frame_counter % 1000) == 0 {
                let elapsed = start.elapsed().as_millis() as f64;
                if elapsed > 0.0 {
                    println!("{} fps", (f64::from(frame_counter)) / elapsed * 1000.0);
                }
            }
        }

        // update cursor
        if let Ok(cursor) = cursor::get_state() {
            unsafe {
                if let Some(global_frame) = &mut GLOBAL_FRAME_BUFFER {
                    global_frame.cursor = cursor;
                }
            }
        }
    }
}
