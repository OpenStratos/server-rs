#![doc(html_logo_url = "https://openstratos.org/wp-content/uploads/2017/05/OpenStratos-768x226.png",
       html_favicon_url = "https://openstratos.org/wp-content/uploads/2015/10/OpenStratos-mark.png",
html_root_url = "https://openstratos.github.io/server-rs/")]

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

// #![forbid(deprecated, overflowing_literals, stable_features, trivial_casts,
// unconditional_recursion,
//     plugin_as_library, unused_allocation, trivial_numeric_casts, unused_features, while_truem,
//     unused_parens, unused_comparisons, unused_extern_crates, unused_import_braces,
// unused_results,
//     improper_ctypes, non_shorthand_field_patterns, private_no_mangle_fns,
// private_no_mangle_statics,
//     filter_map, used_underscore_binding, option_map_unwrap_or, option_map_unwrap_or_else,
//     mutex_integer, mut_mut, mem_forget, print_stdout)]
// #![deny(unused_qualifications, unused, unused_attributes)]
#![warn(missing_docs, variant_size_differences, enum_glob_use, if_not_else,
    invalid_upcast_comparisons, items_after_statements, nonminimal_bool, pub_enum_variant_names,
    shadow_reuse, shadow_same, shadow_unrelated, similar_names, single_match_else, string_add,
    string_add_assign, unicode_not_nfc, unseparated_literal_suffix, use_debug,
    wrong_pub_self_convention, option_unwrap_used, result_unwrap_used,
    missing_docs_in_private_items)]
// Allowing these at least for now.
#![allow(unknown_lints, stutter, integer_arithmetic, cast_possible_truncation, cast_possible_wrap,
         indexing_slicing, cast_precision_loss, cast_sign_loss, cyclomatic_complexity)]

#![allow(unused)]

// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate lazy_static;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;
#[macro_use]
extern crate log;
extern crate log4rs;
extern crate colored;
extern crate chrono;
extern crate libc;
extern crate sysfs_gpio;
extern crate tokio_serial;

/// Configuration file.
pub const CONFIG_FILE: &str = "config.toml";
/// Last state file, in the `data` directory.
pub const STATE_FILE: &str = "last_state";

pub mod error;
pub mod config;
pub mod logic;
#[cfg(feature = "gps")]
pub mod gps;
#[cfg(feature = "raspicam")]
pub mod raspicam;
#[cfg(feature = "fona")]
pub mod fona;
#[cfg(feature = "telemetry")]
pub mod telemetry;

use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use error::*;
pub use config::CONFIG;
use logic::{MainLogic, State, OpenStratos};

/// The main logic of the program.
pub fn run() -> Result<()> {
    initialize_data_filesystem().chain_err(
        || ErrorKind::DataFSInit,
    )?;

    if let Some(state) = State::get_last().chain_err(|| ErrorKind::LastStateRead)? {
        unimplemented!()
    } else {
        logic::init().chain_err(|| ErrorKind::Init)?.main_logic()
    }
}

/// Initializes the data file system for videos and images.
pub fn initialize_data_filesystem() -> Result<()> {
    let video_path = CONFIG.data_dir().join("video");
    fs::create_dir_all(&video_path).chain_err(|| {
        ErrorKind::DirectoryCreation(video_path)
    })?;

    let img_path = CONFIG.data_dir().join("img");
    fs::create_dir_all(&img_path).chain_err(|| {
        ErrorKind::DirectoryCreation(img_path)
    })?;

    Ok(())
}

/// Generates a stack trace string of an error.
#[allow(use_debug)]
pub fn generate_error_string<S: AsRef<str>>(error: &Error, main_error: S) -> String {
    let mut result = format!("{}:\n{}\n", main_error.as_ref(), error);

    for e in error.iter().skip(1) {
        result.push_str(&format!("\tcaused by: {}\n", e));
    }

    // The backtrace is not always generated.
    if let Some(backtrace) = error.backtrace() {
        result.push_str(&format!("\tbacktrace: {:?}\n", backtrace));
    }

    result
}

/// Initializes all loggers.
pub fn init_loggers() -> Result<log4rs::Handle> {
    use log::LogLevelFilter;
    use log4rs::append::console::ConsoleAppender;
    use log4rs::append::file::FileAppender;
    use log4rs::filter::threshold::ThresholdFilter;
    use log4rs::encode::pattern::PatternEncoder;
    use log4rs::config::{Appender, Config, Logger, Root};
    use chrono::UTC;

    let now = UTC::now().format("%Y-%m-%d-%H-%M-%S");
    let pattern_exact = "[{d(%Y-%m-%d %H:%M:%S%.3f %Z)(utc)}][{l}] - {m}{n}";
    let pattern_naive = "[{d(%Y-%m-%d %H:%M:%S %Z)(utc)}][{l}] - {m}{n}";

    let stdout = ConsoleAppender::builder().build();
    let main = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern_naive)))
        .build(format!("data/logs/main-{}.log", now))
        .chain_err(|| ErrorKind::LogAppender("main"))?;
    let system = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern_naive)))
        .build(format!("data/logs/system-{}.log", now))
        .chain_err(|| ErrorKind::LogAppender("system"))?;

    let log_level = if CONFIG.debug() {
        LogLevelFilter::Trace
    } else {
        LogLevelFilter::Debug
    };

    let config = Config::builder()
            // Appenders
            .appender(Appender::builder().build("stdout", Box::new(stdout)))
            .appender(Appender::builder().build("main", Box::new(main)))
            .appender(Appender::builder().build("system", Box::new(system)))
            // Loggers
            .logger(Logger::builder()
                        .appender("system")
                        .additive(false)
                        .build("system", LogLevelFilter::Info));

    #[cfg(feature = "raspicam")]
    let config = {
        let camera = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(pattern_naive)))
            .build(format!("data/logs/camera-{}.log", now))
            .chain_err(|| ErrorKind::LogAppender("camera"))?;

        config
            .appender(Appender::builder().build("camera", Box::new(camera)))
            .logger(Logger::builder().appender("camera").additive(false).build(
                "camera",
                LogLevelFilter::Info,
            ))
    };

    #[cfg(feature = "gps")]
    let config = {
        let gps = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(pattern_naive)))
            .build(format!("data/logs/gps-{}.log", now))
            .chain_err(|| ErrorKind::LogAppender("gps"))?;
        let gps_frames = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(pattern_exact)))
            .build(format!("data/logs/gps_frames-{}.log", now))
            .chain_err(|| ErrorKind::LogAppender("gps_frames"))?;
        let gps_logger = {
            let mut builder = Logger::builder()
                .appender("gps")
                .appender("gps_frames")
                .additive(false);
            if CONFIG.debug() {
                builder = builder.appender("gps_serial");
            }
            builder.build("gps", log_level)
        };

        let config = config
            .appender(
                Appender::builder()
                    .filter(Box::new(ThresholdFilter::new(LogLevelFilter::Info)))
                    .build("gps", Box::new(gps)),
            )
            .appender(
                Appender::builder()
                    .filter(Box::new(ThresholdFilter::new(LogLevelFilter::Debug)))
                    .build("gps_frames", Box::new(gps_frames)),
            )
            .logger(gps_logger);

        if CONFIG.debug() {
            let gps_serial = FileAppender::builder()
                .encoder(Box::new(PatternEncoder::new(pattern_exact)))
                .build(format!("data/logs/gps_serial-{}.log", now))
                .chain_err(|| ErrorKind::LogAppender("gps_serial"))?;
            config.appender(Appender::builder().build(
                "gps_serial",
                Box::new(gps_serial),
            ))
        } else {
            config
        }
    };

    #[cfg(feature = "fona")]
    let config = {
        let fona = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(pattern_naive)))
            .build(format!("data/logs/fona-{}.log", now))
            .chain_err(|| ErrorKind::LogAppender("fona"))?;
        let fona_frames = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(pattern_exact)))
            .build(format!("data/logs/fona_frames-{}.log", now))
            .chain_err(|| ErrorKind::LogAppender("fona_frames"))?;
        let fona_logger = {
            let mut builder = Logger::builder()
                .appender("fona")
                .appender("fona_frames")
                .additive(false);
            if CONFIG.debug() {
                builder = builder.appender("fona_serial");
            }
            builder.build("fona", log_level)
        };
        let config = config
            .appender(
                Appender::builder()
                    .filter(Box::new(ThresholdFilter::new(LogLevelFilter::Info)))
                    .build("fona", Box::new(fona)),
            )
            .appender(
                Appender::builder()
                    .filter(Box::new(ThresholdFilter::new(LogLevelFilter::Debug)))
                    .build("fona_frames", Box::new(fona_frames)),
            )
            .logger(fona_logger);

        if CONFIG.debug() {
            let gsm_serial = FileAppender::builder()
                .encoder(Box::new(PatternEncoder::new(pattern_exact)))
                .build(format!("data/logs/fona_serial-{}.log", now))
                .chain_err(|| ErrorKind::LogAppender("fona_serial"))?;
            config.appender(Appender::builder().build(
                "fona_serial",
                Box::new(gsm_serial),
            ))
        } else {
            config
        }
    };

    #[cfg(feature = "telemetry")]
    let config = {
        let telemetry = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(pattern_exact)))
            .build(format!("data/logs/telemetry-{}.log", now))
            .chain_err(|| ErrorKind::LogAppender("telemetry"))?;
        let telemetry_frames = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(pattern_exact)))
            .build(format!("data/logs/telemetry_frames-{}.log", now))
            .chain_err(|| ErrorKind::LogAppender("telemetry_frames"))?;

        let telemetry_logger = {
            let mut builder = Logger::builder()
                .appender("telemetry")
                .appender("telemetry_frames")
                .additive(false);
            if CONFIG.debug() {
                builder = builder.appender("telemetry_serial");
            }
            builder.build("telemetry", log_level)
        };

        let config = config
            .appender(
                Appender::builder()
                    .filter(Box::new(ThresholdFilter::new(LogLevelFilter::Info)))
                    .build("telemetry", Box::new(telemetry)),
            )
            .appender(
                Appender::builder()
                    .filter(Box::new(ThresholdFilter::new(LogLevelFilter::Debug)))
                    .build("telemetry_frames", Box::new(telemetry_frames)),
            )
            .logger(telemetry_logger);

        if CONFIG.debug() {
            let telemetry_serial = FileAppender::builder()
                .encoder(Box::new(PatternEncoder::new(pattern_exact)))
                .build(format!("data/logs/telemetry_serial-{}.log", now))
                .chain_err(|| ErrorKind::LogAppender("telemetry_serial"))?;

            config.appender(Appender::builder().build(
                "telemetry_serial",
                Box::new(telemetry_serial),
            ))
        } else {
            config
        }
    };

    let config = config
        .build(Root::builder().appender("stdout").appender("main").build(
            LogLevelFilter::Info,
        ))
        .chain_err(|| ErrorKind::LogBuild)?;

    Ok(log4rs::init_config(config)?)
}
