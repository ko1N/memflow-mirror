use ::clap::{crate_authors, crate_version, Arg, ArgAction, Command};
use ::log::{warn, Level};

use ::memflow::prelude::v1::Result;

mod app;
pub use app::MirrorApp;

mod capture;
pub use capture::{Capture, SequentialCapture, ThreadedCapture};

mod config;
use config::MirrorConfig;

fn main() -> Result<()> {
    let matches = Command::new("memflow-mirror")
        .version(crate_version!())
        .author(crate_authors!())
        .arg(Arg::new("verbose").short('v').action(ArgAction::Count))
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

    if thread_priority::ThreadPriority::Max
        .set_for_current()
        .is_err()
    {
        warn!("Unable to set main thread priority");
    }

    // TODO: configuration via ui
    // parse args
    /*
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


    // update capture configuration
    capture.set_obs_capture(config.obs_capture);
    */

    // start ui
    //tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "memflow mirror",
        native_options,
        Box::new(|cc| Box::new(MirrorApp::new(cc))),
    );

    Ok(())
}
