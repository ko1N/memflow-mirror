use std::convert::TryInto;
use std::io::Cursor;

use clap::{crate_authors, crate_version, App, Arg};
use frame_counter::FrameCounter;
use log::{error, info, Level};

use memflow::prelude::v1::*;

pub mod window;
use window::Window;

use mirror_dto::{GlobalBuffer, TextureMode};

fn find_marker(module_buf: &[u8]) -> Option<usize> {
    use ::regex::bytes::*;

    // 0D 0E 0A 0D 0B 0A 0B 0E ? ? ? ? 0 0 0 0
    // since the global buffer contains 2 resolution values (width and height) right after the marker
    // and the resolution is definatly smaller than u32::MAX we can narrow down the search
    // by adding those trailing 0's to the scan
    let re = Regex::new("(?-u)\\x0D\\x0E\\x0A\\x0D\\x0B\\x0A\\x0B\\x0E(?s:.)(?s:.)(?s:.)(?s:.)\\x00\\x00\\x00\\x00(?s:.)(?s:.)(?s:.)(?s:.)\\x00\\x00\\x00\\x00")
        .expect("malformed marker signature");
    let buf_offs = re.find_iter(module_buf).next()?.start();

    Some(buf_offs as usize)
}

fn main() {
    let matches = App::new("memflow-mirror")
        .version(crate_version!())
        .author(crate_authors!())
        .arg(Arg::new("verbose").help("Sets the level of verbosity. For example -vv will set the logging level to 'Warn'.").short('v').multiple_occurrences(true))
        .arg(
            Arg::new("connector")
            .help("The name of the memflow connector that you want to use.")
                .long("connector")
                .short('c')
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("connector-args")
            .help("Additional arguments supplied to the connector.")
                .long("connector-args")
                .short('a')
                .takes_value(true),
        )
        .arg(
            Arg::new("process")
            .help("The name of the memflow-guest process running in the vm.")
                .long("process")
                .short('p')
                .takes_value(true)
                .default_value("mirror-guest.exe"),
        )
        .arg(
            Arg::new("obs")
            .help("Enables capturing via obs.")
                .long("obs")
                .short('o')
        )
        .arg(
            Arg::new("vsync")
            .help("Enabled vertical synchronization (vsync) in the renderer.")
                .long("vsync")
                .short('s')
        )
        .arg(
            Arg::new("stretch")
            .help("Always stretch the contents to the mirror window.")
                .long("stretch")
                .short('f')
        )
        .get_matches();

    let log_level = match matches.occurrences_of("verbose") {
        0 => Level::Error,
        1 => Level::Warn,
        2 => Level::Info,
        3 => Level::Debug,
        4 => Level::Trace,
        _ => Level::Trace,
    };
    simplelog::TermLogger::init(
        log_level.to_level_filter(),
        simplelog::Config::default(),
        simplelog::TerminalMode::Stdout,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();

    match thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Max) {
        Ok(_) => info!("Main thread's priority set to max"),
        Err(_) => error!("Failed setting main thread's priority"),
    };

    #[allow(unused)]
    let conn_name = matches
        .value_of("connector")
        .expect("no connector specified");
    let conn_args = matches
        .value_of("connector-args")
        .unwrap_or_default()
        .parse()
        .expect("unable to parse connector arguments");

    // build connector + osy
    #[cfg(feature = "memflow-static")]
    let os = {
        // load connector/os statically
        let connector =
            memflow_qemu::create_connector(&conn_args).expect("unable to create qemu connector");

        memflow_win32::prelude::Win32Kernel::builder(connector)
            .build_default_caches()
            .build()
            .expect("unable to instantiate win32 instance with qemu connector")
    };

    // load connector/os via inventory
    #[cfg(not(feature = "memflow-static"))]
    let inventory = Inventory::scan();
    #[cfg(not(feature = "memflow-static"))]
    let os = {
        inventory
            .builder()
            .connector(conn_name)
            .args(conn_args)
            .os("win32")
            .build()
            .expect("unable to instantiate connector / os")
    };

    // load process
    let proc_name = matches.value_of("process").expect("no process specified");

    let mut process = os
        .into_process_by_name(proc_name)
        .expect("unable to find dxgi-capture process");
    info!("found process: {:?}", process.info());

    let module_info = process
        .module_by_name(proc_name)
        .expect("unable to find dxgi-capture module in process");
    info!("found module: {:?}", module_info);

    // read entire module for sigscanning
    let module_buf = process
        .read_raw(module_info.base, module_info.size.try_into().unwrap())
        .data_part()
        .expect("unable to read module");

    let marker_offs = find_marker(&module_buf).expect("unable to find marker in binary");
    info!("marker found at {:x} + {:x}", module_info.base, marker_offs);
    let marker_addr = module_info.base + marker_offs;

    let mut global_buffer: GlobalBuffer = process
        .read(marker_addr)
        .expect("unable to read global buffer");
    info!(
        "found resolution: {}x{}",
        global_buffer.width, global_buffer.height
    );
    info!(
        "found address of frame0: {:x}",
        global_buffer.frame0.buffer.as_ptr() as umem
    );

    // pre-allocate buffer for the frame being renderes
    let mut render_buffer = vec![0u8; (global_buffer.width * global_buffer.height * 4) as usize];

    // create window
    let mut wnd = Window::new(matches.is_present("vsync"));

    // create frame texture
    let image = glium::texture::RawImage2d::from_raw_rgba_reversed(
        &render_buffer[..],
        (global_buffer.width as u32, global_buffer.height as u32),
    );
    let mut texture_mode = TextureMode::BGRA;
    let mut texture = glium::texture::SrgbTexture2d::new(&wnd.display, image).unwrap();

    // create cursor texture
    let cursor_image_png = image::load(
        Cursor::new(&include_bytes!("../resources/cursor.png")[..]),
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

    let mut frame_counter = FrameCounter::new(100f64);
    let mut update_counter = FrameCounter::new(100f64);

    let enable_obs = matches.is_present("obs");
    let stretch_to_window = matches.is_present("stretch");
    let mut previous_write_frame = 0;
    loop {
        frame_counter.tick();

        // check if a frame buffer is necessary
        process.read_into(marker_addr, &mut global_buffer).unwrap();
        if global_buffer.write_frame != previous_write_frame {
            update_counter.tick();

            // check if resolution has been changed
            if texture.width() != global_buffer.width as u32
                || texture.height() != global_buffer.height as u32
            {
                // limit to 16k resolution
                if global_buffer.width <= 15360 && global_buffer.height <= 8640 {
                    info!(
                        "changing resolution: to {}x{}",
                        global_buffer.width, global_buffer.height
                    );
                    render_buffer =
                        vec![0u8; (global_buffer.width * global_buffer.height * 4) as usize];
                    let new_image = glium::texture::RawImage2d::from_raw_rgba(
                        render_buffer.clone(),
                        (global_buffer.width as u32, global_buffer.height as u32),
                    );
                    texture = glium::texture::SrgbTexture2d::new(&wnd.display, new_image).unwrap();
                }
            }

            // update frame_buffer
            let read_frame = if global_buffer.write_frame == 0 {
                &global_buffer.frame1
            } else {
                &global_buffer.frame0
            };
            process
                .read_into(
                    (read_frame.buffer.as_ptr() as umem).into(),
                    &mut render_buffer[..],
                )
                .ok();
            global_buffer.config.obs = enable_obs; // TODO: more configuration options
            global_buffer.read_frame = global_buffer.write_frame;
            process.write(marker_addr, &global_buffer).ok();

            // update image
            let new_image = glium::texture::RawImage2d::from_raw_rgba(
                render_buffer.clone(),
                (global_buffer.width as u32, global_buffer.height as u32),
            );

            // update texture
            texture_mode = unsafe { std::mem::transmute(read_frame.texture_mode) };
            texture.write(
                glium::Rect {
                    left: 0,
                    bottom: 0,
                    width: global_buffer.width as u32,
                    height: global_buffer.height as u32,
                },
                new_image,
            );

            previous_write_frame = global_buffer.write_frame;
        }

        let mut frame = wnd.frame();

        // compute rendering position
        let window_size = frame.window.display.window().drawable_size();
        let window_aspect = window_size.0 as f32 / window_size.1 as f32;
        let capture_aspect = texture.width() as f32 / texture.height() as f32;
        let (x, y, w, h) = if !stretch_to_window {
            if window_aspect >= capture_aspect {
                let target_width = 2.0 * capture_aspect / window_aspect;
                (-1.0 + (2.0 - target_width) / 2.0, 1.0, target_width, -2.0)
            } else {
                let target_height = 2.0 * window_aspect / capture_aspect;
                (-1.0, 1.0 - (2.0 - target_height) / 2.0, 2.0, -target_height)
            }
        } else {
            (-1.0, 1.0, 2.0, -2.0)
        };

        // draw texture
        frame.draw_texture(x, y, w, h, &texture, texture_mode, false);
        let offset = 0; //1920;
                        // draw cursor
        if global_buffer.cursor.is_visible != 0 {
            let scale = (
                w / global_buffer.width as f32,
                -h / global_buffer.height as f32,
            );
            let dimensions = (
                scale.0 * cursor_dimensions.0 as f32,
                scale.1 * cursor_dimensions.1 as f32,
            );
            frame.draw_texture(
                x + scale.0 * (global_buffer.cursor.x - offset) as f32,
                y - scale.1 * global_buffer.cursor.y as f32 - dimensions.1,
                dimensions.0,
                dimensions.1,
                &cursor_texture,
                TextureMode::RGBA,
                true,
            );
        }

        // fps and ups counter
        {
            frame.draw_text(
                &format!("fps: {:.0}", frame_counter.avg_frame_rate()),
                [25.0, 35.0],
                [0.025, 0.025],
                [0.0, 1.0, 1.0, 1.0],
            );
            frame.draw_text(
                &format!("ups: {:.0}", update_counter.avg_frame_rate()),
                [25.0, 55.0],
                [0.025, 0.025],
                [0.0, 1.0, 1.0, 1.0],
            );
            frame.draw_text(
                &format!("texmode: {:?}", texture_mode),
                [25.0, 75.0],
                [0.025, 0.025],
                [0.0, 1.0, 1.0, 1.0],
            );
        }

        if !frame.end() {
            break;
        }
    }
}
