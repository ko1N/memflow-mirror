use std::ffi::c_void;
use std::time::Instant;

use winapi::um::libloaderapi::GetModuleHandleA;

mod dxgi;
use dxgi::{DXGIManager, BGRA8};

// TODO: marker
static mut RESOLUTION: Option<(usize, usize)> = None;
static mut CURRENT_FRAME: Option<Vec<BGRA8>> = None;

fn main() {
    let module_base = unsafe { GetModuleHandleA(std::ptr::null_mut()) } as u64;
    println!("module_base: 0x{:x}", module_base);

    // offsets
    {
        unsafe {
            let resolution_ref = &RESOLUTION;
            let resolution_addr = resolution_ref as *const _ as *const c_void as u64;
            println!("resolution offset: 0x{:x}", resolution_addr - module_base);
        }
        unsafe {
            let frame_ref = &CURRENT_FRAME;
            let frame_addr = frame_ref as *const _ as *const c_void as u64;
            println!("frame offset: 0x{:x}", frame_addr - module_base);
        }
    }

    let mut dxgi = DXGIManager::new(1000).expect("unable to create dxgi manager");
    let geometry = dxgi.geometry();
    println!("geometry: {:?}", geometry);
    unsafe {
        RESOLUTION = Some(geometry);
        CURRENT_FRAME = Some(vec![
            BGRA8 {
                b: 0,
                g: 0,
                r: 0,
                a: 0
            };
            geometry.0 * geometry.1
        ]);
    }

    let start = Instant::now();
    let mut frames = 0;
    loop {
        if let Ok(mut frame) = dxgi.capture_frame() {
            // frame captured
            //println!("px(0,0) = {:?}", frame.0[0]);
            for pixel in frame.0.iter_mut() {
                // TODO: hacky conversion to RGBA8
                let r = pixel.r;
                let b = pixel.b;
                pixel.b = r;
                pixel.r = b;
            }

            unsafe {
                if let Some(current_frame) = &mut CURRENT_FRAME {
                    current_frame.copy_from_slice(frame.0.as_slice());
                }
            }

            frames += 1;
        }

        if (frames % 1000) == 0 {
            let elapsed = start.elapsed().as_millis() as f64;
            if elapsed > 0.0 {
                println!("{} fps", (f64::from(frames)) / elapsed * 1000.0);
            }
        }

        //std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
