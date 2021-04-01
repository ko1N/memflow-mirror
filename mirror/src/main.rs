use std::io::Cursor;
use std::time::Instant;

use clap::{load_yaml, App};
use image;
use log::{info, Level};

use memflow::prelude::v1::*;

pub mod window;
use window::Window;

use mirror_dto::GlobalBufferRaw;

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

    let mut global_buffer: GlobalBufferRaw = process
        .virt_mem()
        .virt_read(marker_addr)
        .expect("unable to read global buffer");
    println!(
        "found resolution: {}x{}",
        global_buffer.width, global_buffer.height
    );
    println!("found frame_buffer addr: {:x}", global_buffer.frame_buffer);

    // pre-allocate frame_buffer
    let mut frame_buffer = vec![0u8; (global_buffer.width * global_buffer.height * 4) as usize];

    // create window
    let mut wnd = Window::new(matches.is_present("vsync"));

    // create frame texture
    let image = glium::texture::RawImage2d::from_raw_rgba_reversed(
        &frame_buffer[..],
        (global_buffer.width as u32, global_buffer.height as u32),
    );
    let mut texture = glium::texture::SrgbTexture2d::new(&wnd.display, image).unwrap();

    // create cursor texture
    let cursor_image_png = image::load(
        Cursor::new(&include_bytes!("resources/cursor.png")[..]),
        image::ImageFormat::Png,
    )
    .expect("unable to load cursor image")
    .to_rgba8();
    let cursor_dimensions = cursor_image_png.dimensions();
    let cursor_image = glium::texture::RawImage2d::from_raw_rgba_reversed(
        &cursor_image_png.into_raw(),
        cursor_dimensions,
    );
    let cursor_texture = glium::texture::SrgbTexture2d::new(&wnd.display, cursor_image).unwrap();

    let mut start = Instant::now();
    let mut frames = 0;
    let mut updates = 0;
    let mut previous_frame_counter = 0;
    loop {
        let virt_mem = process.virt_mem();

        // check if a frame buffer is necessary
        virt_mem
            .virt_read_into(marker_addr, &mut global_buffer)
            .unwrap();
        if global_buffer.frame_counter != previous_frame_counter {
            // check if resolution has been changed
            if texture.width() != global_buffer.width as u32
                || texture.height() != global_buffer.height as u32
            {
                println!(
                    "changing resolution: to {}x{}",
                    global_buffer.width, global_buffer.height
                );
                frame_buffer = vec![0u8; (global_buffer.width * global_buffer.height * 4) as usize];
                let new_image = glium::texture::RawImage2d::from_raw_rgba(
                    frame_buffer.clone(),
                    (global_buffer.width as u32, global_buffer.height as u32),
                );
                texture = glium::texture::SrgbTexture2d::new(&wnd.display, new_image).unwrap();
            }

            // update frame_buffer
            virt_mem
                .virt_read_into(global_buffer.frame_buffer.into(), &mut frame_buffer[..])
                .unwrap();
            global_buffer.frame_read_counter = global_buffer.frame_counter;
            virt_mem.virt_write(marker_addr, &global_buffer).unwrap();

            // update image
            let new_image = glium::texture::RawImage2d::from_raw_rgba(
                frame_buffer.clone(),
                (global_buffer.width as u32, global_buffer.height as u32),
            );

            // update texture
            texture.write(
                glium::Rect {
                    left: 0,
                    bottom: 0,
                    width: global_buffer.width as u32,
                    height: global_buffer.height as u32,
                },
                new_image,
            );

            updates += 1;
            previous_frame_counter = global_buffer.frame_counter;
        }

        let mut frame = wnd.frame();

        // draw texture
        frame.draw_texture(-1.0, 1.0, 2.0, -2.0, &texture, false);

        // draw cursor
        if global_buffer.cursor.is_visible != 0 {
            let scale = (
                2.0 / global_buffer.width as f32,
                2.0 / global_buffer.height as f32,
            );
            let dimensions = (
                scale.0 * cursor_dimensions.0 as f32,
                scale.1 * cursor_dimensions.1 as f32,
            );
            frame.draw_texture(
                -1.0 + scale.0 * global_buffer.cursor.x as f32,
                1.0 - scale.1 * global_buffer.cursor.y as f32 - dimensions.1,
                dimensions.0,
                dimensions.1,
                &cursor_texture,
                true,
            );
        }

        // fps and ups counter
        {
            let elapsed = start.elapsed().as_millis() as f64;
            frame.draw_text(
                &format!("fps: {:.0}", (f64::from(frames)) / elapsed * 1000.0),
                [25.0, 35.0],
                [0.025, 0.025],
                [1.0; 4],
            );
            frame.draw_text(
                &format!("ups: {:.0}", (f64::from(updates)) / elapsed * 1000.0),
                [25.0, 55.0],
                [0.025, 0.025],
                [1.0; 4],
            );

            // reset counters
            if elapsed >= 1000.0 {
                start = Instant::now();
                frames = 0;
                updates = 0;
            }
        }

        frames += 1;
        if !frame.end() {
            break;
        }
    }
}
