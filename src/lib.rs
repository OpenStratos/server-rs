#![doc(
    html_logo_url = "https://openstratos.org/wp-content/uploads/2017/05/OpenStratos-768x226.png",
    html_favicon_url = "https://openstratos.org/wp-content/uploads/2015/10/OpenStratos-mark.png",
    html_root_url = "https://openstratos.github.io/server-rs/"
)]

//! OpenStratos balloon software.
//!
//! This crate provides the functionality required to control a stratospheric balloon. It provides
//! several modules that can be enabled using cargo features, and it can be extended by adding more
//! modules.
//!
//! ## Example:
//!
//! If you for example want to use the GPS and the GSM, but no real-time telemetry or Raspberry Pi
//! camera, it's as simple as compiling the crate as follows:
//!
//! ```text
//! cargo build --no-default-features --features="gps fona"
//! ```
//!
//! Here, the `--no-default-features` is required since by default, GPS, GSM (Adafruit FONA),
//! Raspberry Pi camera and real-time transparent serial telemetry will be activated.
//!
//! ## Configuration
//!
//! OpenStratos is highly configurable. Please refer to the [`config`](config/index.html) module for
//! further information.
//!
//! ## Launcher
//!
//! The project has a launcher in `src/main.rs` and can be launched by running `cargo run`. More
//! information can be found in the [`launcher`](../launcher/index.html) crate.
//!
//! ## Simulation mode
//!
//! *In development…*

#![deny(clippy::all)]
#![forbid(anonymous_parameters)]
#![warn(clippy::pedantic)]
#![deny(
    variant_size_differences,
    unused_results,
    unused_qualifications,
    unused_import_braces,
    unsafe_code,
    trivial_numeric_casts,
    trivial_casts,
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    box_pointers,
    unused_extern_crates
)]
// Removing some warnings
#![allow(unsafe_code, box_pointers, clippy::use_self)]

/// Configuration file.
pub const CONFIG_FILE: &str = "config.toml";
/// Last state file, in the `data` directory.
pub const STATE_FILE: &str = "last_state";

pub mod config;
pub mod error;
#[cfg(feature = "fona")]
pub mod fona;
#[cfg(feature = "gps")]
pub mod gps;
pub mod logic;
#[cfg(feature = "raspicam")]
pub mod raspicam;
#[cfg(feature = "telemetry")]
pub mod telemetry;

use std::fs;

use failure::{Error, ResultExt};

pub use crate::config::CONFIG;
use crate::logic::{MainLogic, State};

/// The main logic of the program.
pub fn run() -> Result<(), Error> {
    initialize_data_filesystem().context(error::Fs::DataInit)?;

    if let Some(_state) = State::get_last().context(error::LastState::Read)? {
        // TODO recover from last state and continue
        unimplemented!()
    } else {
        logic::init().context(error::Logic::Init)?.main_logic()
    }
}

/// Initializes the data file system for videos and images.
pub fn initialize_data_filesystem() -> Result<(), Error> {
    let video_path = CONFIG.data_dir().join("video");
    fs::create_dir_all(&video_path).context(error::Fs::DirectoryCreation { path: video_path })?;

    let img_path = CONFIG.data_dir().join("img");
    fs::create_dir_all(&img_path).context(error::Fs::DirectoryCreation { path: img_path })?;

    Ok(())
}

/// Generates a stack trace string of an error.
#[allow(clippy::use_debug)]
pub fn generate_error_string<S>(error: &Error, main_error: S) -> String
where
    S: AsRef<str>,
{
    let mut result = format!("{}:\n{}\n", main_error.as_ref(), error);

    for e in error.iter_causes() {
        result.push_str(&format!("\tcaused by: {}\n", e));
    }

    // TODO: print only on debug mode
    result.push_str(&format!("\tbacktrace: {:?}\n", error.backtrace()));

    result
}

/// Initializes all loggers.
pub fn init_loggers() -> Result<log4rs::Handle, Error> {
    use chrono::Utc;
    use log::LevelFilter;
    use log4rs::{
        append::{console::ConsoleAppender, file::FileAppender},
        config::{Appender, Config, Logger, Root},
        encode::pattern::PatternEncoder,
    };
    // Only required for GPS, FONA or telemetry
    #[cfg(any(feature = "gps", feature = "fona", feature = "telemetry"))]
    use log::Record;
    #[cfg(any(feature = "gps", feature = "fona", feature = "telemetry"))]
    use log4rs::filter::{threshold::ThresholdFilter, Filter, Response};

    /// Filter that filters all but debug records.
    #[cfg(any(feature = "gps", feature = "fona", feature = "telemetry"))]
    #[derive(Debug, Clone, Copy)]
    struct DebugFilter;
    #[cfg(any(feature = "gps", feature = "fona", feature = "telemetry"))]
    impl Filter for DebugFilter {
        fn filter(&self, record: &Record) -> Response {
            if record.level() == LevelFilter::Debug {
                Response::Neutral
            } else {
                Response::Reject
            }
        }
    }

    /// Filter that filters all but trace records.
    #[cfg(any(feature = "gps", feature = "fona", feature = "telemetry"))]
    #[derive(Debug, Clone, Copy)]
    struct TraceFilter;
    #[cfg(any(feature = "gps", feature = "fona", feature = "telemetry"))]
    impl Filter for TraceFilter {
        fn filter(&self, record: &Record) -> Response {
            if record.level() == LevelFilter::Trace {
                Response::Neutral
            } else {
                Response::Reject
            }
        }
    }

    let now = Utc::now().format("%Y-%m-%d-%H-%M-%S");
    let pattern_naive = "[{d(%Y-%m-%d %H:%M:%S %Z)(utc)}][{l}] - {m}{n}";

    // Only required for GPS, FONA or telemetry
    #[cfg(any(feature = "gps", feature = "fona", feature = "telemetry"))]
    let pattern_exact = "[{d(%Y-%m-%d %H:%M:%S%.3f %Z)(utc)}][{l}] - {m}{n}";

    let stdout = ConsoleAppender::builder().build();
    let main = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern_naive)))
        .build(format!("data/logs/main-{}.log", now))
        .context(error::Log::Appender { name: "main" })?;
    let system = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern_naive)))
        .build(format!("data/logs/system-{}.log", now))
        .context(error::Log::Appender { name: "system" })?;

    // Only required for GPS, FONA or telemetry
    #[cfg(any(feature = "gps", feature = "fona", feature = "telemetry"))]
    let log_level = if CONFIG.debug() {
        LevelFilter::Trace
    } else {
        LevelFilter::Debug
    };

    let config = Config::builder()
        // Appenders
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("main", Box::new(main)))
        .appender(Appender::builder().build("system", Box::new(system)))
        // Loggers
        .logger(
            Logger::builder()
                .appender("system")
                .additive(false)
                .build("system", LevelFilter::Info),
        );

    #[cfg(feature = "raspicam")]
    let config = {
        let camera = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(pattern_naive)))
            .build(format!("data/logs/camera-{}.log", now))
            .context(error::Log::Appender { name: "camera" })?;

        config
            .appender(Appender::builder().build("camera", Box::new(camera)))
            .logger(
                Logger::builder()
                    .appender("camera")
                    .additive(false)
                    .build("os_balloon::camera", LevelFilter::Info),
            )
    };

    #[cfg(feature = "gps")]
    let config = {
        let gps = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(pattern_naive)))
            .build(format!("data/logs/gps-{}.log", now))
            .context(error::Log::Appender {
                name: "os_balloon::gps",
            })?;
        let gps_frames = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(pattern_exact)))
            .build(format!("data/logs/gps_frames-{}.log", now))
            .context(error::Log::Appender {
                name: "os_balloon::gps_frames",
            })?;
        let gps_logger = {
            let mut builder = Logger::builder()
                .appender("gps")
                .appender("gps_frames")
                .additive(false);
            if CONFIG.debug() {
                builder = builder.appender("gps_serial");
            }
            builder.build("os_balloon::gps", log_level)
        };

        let config = config
            .appender(
                Appender::builder()
                    .filter(Box::new(ThresholdFilter::new(LevelFilter::Info)))
                    .build("gps", Box::new(gps)),
            )
            .appender(
                Appender::builder()
                    .filter(Box::new(DebugFilter))
                    .build("gps_frames", Box::new(gps_frames)),
            )
            .logger(gps_logger);

        if CONFIG.debug() {
            let gps_serial = FileAppender::builder()
                .encoder(Box::new(PatternEncoder::new(pattern_exact)))
                .build(format!("data/logs/gps_serial-{}.log", now))
                .context(error::Log::Appender { name: "gps_serial" })?;
            config.appender(
                Appender::builder()
                    .filter(Box::new(TraceFilter))
                    .build("gps_serial", Box::new(gps_serial)),
            )
        } else {
            config
        }
    };

    #[cfg(feature = "fona")]
    let config = {
        let fona = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(pattern_naive)))
            .build(format!("data/logs/fona-{}.log", now))
            .context(error::Log::Appender {
                name: "os_balloon::fona",
            })?;
        let fona_frames = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(pattern_exact)))
            .build(format!("data/logs/fona_frames-{}.log", now))
            .context(error::Log::Appender {
                name: "os_balloon::fona_frames",
            })?;
        let fona_logger = {
            let mut builder = Logger::builder()
                .appender("fona")
                .appender("fona_frames")
                .additive(false);
            if CONFIG.debug() {
                builder = builder.appender("fona_serial");
            }
            builder.build("os_balloon::fona", log_level)
        };
        let config = config
            .appender(
                Appender::builder()
                    .filter(Box::new(ThresholdFilter::new(LevelFilter::Info)))
                    .build("fona", Box::new(fona)),
            )
            .appender(
                Appender::builder()
                    .filter(Box::new(DebugFilter))
                    .build("fona_frames", Box::new(fona_frames)),
            )
            .logger(fona_logger);

        if CONFIG.debug() {
            let gsm_serial = FileAppender::builder()
                .encoder(Box::new(PatternEncoder::new(pattern_exact)))
                .build(format!("data/logs/fona_serial-{}.log", now))
                .context(error::Log::Appender {
                    name: "fona_serial",
                })?;
            config.appender(
                Appender::builder()
                    .filter(Box::new(TraceFilter))
                    .build("fona_serial", Box::new(gsm_serial)),
            )
        } else {
            config
        }
    };

    #[cfg(feature = "telemetry")]
    let config = {
        let telemetry = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(pattern_exact)))
            .build(format!("data/logs/telemetry-{}.log", now))
            .context(error::Log::Appender { name: "telemetry" })?;
        let telemetry_frames = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(pattern_exact)))
            .build(format!("data/logs/telemetry_frames-{}.log", now))
            .context(error::Log::Appender {
                name: "telemetry_frames",
            })?;

        let telemetry_logger = {
            let mut builder = Logger::builder()
                .appender("telemetry")
                .appender("telemetry_frames")
                .additive(false);
            if CONFIG.debug() {
                builder = builder.appender("telemetry_serial");
            }
            builder.build("os_balloon::telemetry", log_level)
        };

        let config = config
            .appender(
                Appender::builder()
                    .filter(Box::new(ThresholdFilter::new(LevelFilter::Info)))
                    .build("telemetry", Box::new(telemetry)),
            )
            .appender(
                Appender::builder()
                    .filter(Box::new(DebugFilter))
                    .build("telemetry_frames", Box::new(telemetry_frames)),
            )
            .logger(telemetry_logger);

        if CONFIG.debug() {
            let telemetry_serial = FileAppender::builder()
                .encoder(Box::new(PatternEncoder::new(pattern_exact)))
                .build(format!("data/logs/telemetry_serial-{}.log", now))
                .context(error::Log::Appender {
                    name: "telemetry_serial",
                })?;

            config.appender(
                Appender::builder()
                    .filter(Box::new(TraceFilter))
                    .build("telemetry_serial", Box::new(telemetry_serial)),
            )
        } else {
            config
        }
    };

    let config = config
        .build(
            Root::builder()
                .appender("stdout")
                .appender("main")
                .build(LevelFilter::Info),
        )
        .context(error::Log::Build)?;

    Ok(log4rs::init_config(config)?)
}
