use std::time::Instant;

use clap::{load_yaml, App};
use log::{info, Level};

use memflow::prelude::v1::*;

pub mod window;
use window::Window;

fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from(yaml).get_matches();

    let level = match matches.occurrences_of("verbose") {
        0 => Level::Error,
        1 => Level::Warn,
        2 => Level::Info,
        3 => Level::Debug,
        4 => Level::Trace,
        _ => Level::Trace,
    };

    simple_logger::SimpleLogger::new()
        .with_level(level.to_level_filter())
        .init()
        .unwrap();

    let conn_name = matches
        .value_of("connector")
        .expect("no connector specified");
    let conn_args = Args::parse(matches.value_of("args").unwrap_or_default())
        .expect("unable to parse connector arguments");

    // build connector + os
    let inventory = Inventory::scan();
    let os = inventory
        .builder()
        .connector(conn_name)
        .args(conn_args)
        .os("win32")
        .build()
        .expect("unable to instantiate connector / os");

    let mut process = os
        .into_process_by_name("dxgi-capture.exe")
        .expect("unable to find dxgi-capture process");
    info!("found process: {:?}", process.info());

    let module_info = process
        .module_by_name("dxgi-capture.exe")
        .expect("unable to find dxgi-capture module in process");
    info!("found module: {:?}", module_info);

    let offset_resolution = 0x31100;
    let offset_frame = 0x31118;

    let width: u64 = process
        .virt_mem()
        .virt_read(module_info.base + offset_resolution + 0x8)
        .unwrap();
    let height: u64 = process
        .virt_mem()
        .virt_read(module_info.base + offset_resolution + 2 * 0x8)
        .unwrap();
    println!("detected resolution: {}x{}", width, height);

    let frame_addr = process
        .virt_mem()
        .virt_read_addr64(module_info.base + offset_frame)
        .unwrap();
    println!("detected frame addr: {:x}", frame_addr);

    let mut frame_buffer = vec![0u8; (width * height * 4) as usize]; // BGRA8

    // create window
    let mut wnd = Window::new();

    let start = Instant::now();
    let mut frames = 0;

    // create texture
    let image = glium::texture::RawImage2d::from_raw_rgba_reversed(
        &frame_buffer[..],
        (width as u32, height as u32),
    );
    let texture = glium::texture::SrgbTexture2d::new(&wnd.display, image).unwrap();

    loop {
        let mut frame = wnd.frame();

        process
            .virt_mem()
            .virt_read_into(frame_addr, &mut frame_buffer[..])
            .unwrap();

        /*
        frame.draw_text(
            &format!("fps: {:.0}", frame_counter.avg_frame_rate()),
            [25.0, 35.0],
            [0.025, 0.025],
            [1.0; 4],
        );
        */

        let new_image = glium::texture::RawImage2d::from_raw_rgba(
            frame_buffer.clone(),
            (width as u32, height as u32),
        );
        texture.write(
            glium::Rect {
                left: 0,
                bottom: 0,
                width: width as u32,
                height: height as u32,
            },
            new_image,
        );
        frame.draw_texture(&texture);

        if !frame.end() {
            break;
        }

        frames += 1;
        if (frames % 100) == 0 {
            let elapsed = start.elapsed().as_millis() as f64;
            if elapsed > 0.0 {
                println!("{} fps", (f64::from(frames)) / elapsed * 1000.0);
            }
        }
    }
}
