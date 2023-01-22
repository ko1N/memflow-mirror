use ::std::{thread, time::Duration};

use ::clap::{crate_authors, crate_version, Arg, ArgAction, Command};
use ::log::Level;

use ::memflow::prelude::v1::*;
use ::mirror::prelude::v1::*;

fn main() -> Result<()> {
    let matches = Command::new("memflow-mirror-example")
        .version(crate_version!())
        .author(crate_authors!())
        .arg(Arg::new("verbose").short('v').action(ArgAction::Count))
        .arg(
            Arg::new("connector")
                .long("connector")
                .short('c')
                .action(ArgAction::Append)
                .required(false),
        )
        .arg(
            Arg::new("os")
                .long("os")
                .short('o')
                .action(ArgAction::Append)
                .required(true),
        )
        .get_matches();

    let log_level = match matches.get_count("verbose") {
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

    // parse args
    let conn_iter = matches
        .indices_of("connector")
        .zip(matches.get_many::<String>("connector"))
        .map(|(a, b)| a.zip(b.map(String::as_str)))
        .into_iter()
        .flatten();

    let os_iter = matches
        .indices_of("os")
        .zip(matches.get_many::<String>("os"))
        .map(|(a, b)| a.zip(b.map(String::as_str)))
        .into_iter()
        .flatten();

    let chain = OsChain::new(conn_iter, os_iter)?;

    // create memflow inventory + os
    let inventory = Inventory::scan();
    let os = inventory.builder().os_chain(chain).build()?;

    // initialize capture
    let mut capture = SequentialCapture::new(os);
    capture.set_obs_capture(true);

    let mut frame_counter = 0;
    loop {
        // update internal state, then read frame_counter and image_data
        capture.update();

        // only update frame_texture on demand
        let current_frame_counter = capture.frame_counter();
        if current_frame_counter != frame_counter {
            // grab image data
            let frame = capture.image_data();

            // update frame_counter
            frame_counter = current_frame_counter;

            // output stats
            println!(
                "frame {} read: size={}x{}",
                frame_counter,
                frame.width(),
                frame.height()
            );
        }

        thread::sleep(Duration::from_millis(10));
    }
}
