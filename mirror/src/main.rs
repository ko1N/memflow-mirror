use std::time::Instant;

use clap::{load_yaml, App};
use log::{info, Level};

use memflow::prelude::v1::*;

pub mod window;
use window::Window;

pub use mirror_dto::GlobalBuffer;

fn find_marker(module_buf: &[u8]) -> Option<usize> {
    use ::regex::bytes::*;

    // 0D 0E 0A 0D 0B 0A 0B 0E
    let re = Regex::new("(?-u)\\x0D\\x0E\\x0A\\x0D\\x0B\\x0A\\x0B\\x0E")
        .expect("malformed marker signature");
    let buf_offs = re
        .find_iter(&module_buf[..])
        .nth(1)? // TODO: fixme
        .start();

    Some(buf_offs as usize)
}

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

    // load process
    let proc_name = matches.value_of("process").expect("no process specified");

    let mut process = os
        .into_process_by_name(&proc_name)
        .expect("unable to find dxgi-capture process");
    info!("found process: {:?}", process.info());

    let module_info = process
        .module_by_name(&proc_name)
        .expect("unable to find dxgi-capture module in process");
    info!("found module: {:?}", module_info);

    // read entire module for sigscanning
    let module_buf = process
        .virt_mem()
        .virt_read_raw(module_info.base, module_info.size)
        .data_part()
        .expect("unable to read module");

    let marker_offs = find_marker(&module_buf).expect("unable to find marker in binary");
    info!("marker found at {:x} + {:x}", module_info.base, marker_offs);
    let marker_addr = module_info.base + marker_offs;

    let width: usize = process.virt_mem().virt_read(marker_addr + 0x8).unwrap();
    let height: usize = process
        .virt_mem()
        .virt_read(marker_addr + 0x8 + 0x8)
        .unwrap();
    println!("found resolution: {}x{}", width, height);

    let frame_addr = process
        .virt_mem()
        .virt_read_addr64(marker_addr + 0x20)
        .unwrap();
    println!("found frame_addr: {:x}", frame_addr);

    // pre-allocate frame_buffer
    let mut frame_buffer = vec![0u8; (width * height * 4) as usize]; // BGRA8

    // create window
    let mut wnd = Window::new(matches.is_present("vsync"));

    let start = Instant::now();
    let mut frames = 0;

    // create texture
    let image = glium::texture::RawImage2d::from_raw_rgba_reversed(
        &frame_buffer[..],
        (width as u32, height as u32),
    );
    let texture = glium::texture::SrgbTexture2d::new(&wnd.display, image).unwrap();

    let mut previous_frame_count = 0;
    loop {
        let mut frame = wnd.frame();

        loop {
            let frame_count: u32 = process.virt_mem().virt_read(marker_addr + 0x18).unwrap();
            if frame_count != previous_frame_count {
                previous_frame_count = frame_count;
                break;
            }
        }

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
                info!("{} fps", (f64::from(frames)) / elapsed * 1000.0);
            }
        }
    }
}
