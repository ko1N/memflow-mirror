use std::time::Instant;

use dxgcap::{DXGIManager, BGRA8};

// TODO: marker
static mut CURRENT_FRAME: Option<Vec<BGRA8>> = None;

fn main() {
    let mut dxgi = DXGIManager::new(1000).expect("unable to create dxgi manager");
    println!("geometry: {:?}", dxgi.geometry());

    let start = Instant::now();
    let mut frames = 0;
    loop {
        if let Ok(frame) = dxgi.capture_frame() {
            // frame captured
            //println!("px(0,0) = {:?}", frame.0[0]);
            unsafe {
                CURRENT_FRAME = Some(frame.0);
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
