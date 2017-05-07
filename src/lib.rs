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
//! One of the main features of OpenStratos is that it's almost 100% configurable. Apart from the
//! features above, the package contains a `config.toml` file, writen in
//! [TOML](https://en.wikipedia.org/wiki/TOML) that enables the configuration of the setup without
//! requiring a recompilation of the software. Things like the picture/video options, alert phone
//! number, debug mode, pin numbers and many more can be modified with that file.
//!
//! Some documentation can be found in the file itself, thank to its comments, but main options are
//! explained here:
//!
//! * **Debug mode** (`debug = bool`): Turns the debug mode on or off, it's off by default. The
//! debug mode will print all serial communication in logs, and it will add more insightful logs,
//! that enable debugging system malfunction. This mode will consume more resources than
//! non-debugging mode, and it's not recommended for normal balloon operation. Also, debug logs will
//! be full of silly comments that might not provide anything useful in a real flight.
//! * **Camera rotation** (`camera_rotation = 0-359`): Sets the rotation of the camera for videos
//! and pictures, in degrees. This is useful if the probe, by design, requires the camera to be in a
//! non-vertical position.
//! * **Data directory** (`data_dir = "/path/to/data"`): Sets the path to the main data output
//! directory. Logs, images, videos and current state file will be stored in this path. Make sure
//! it's a reliable path between reboots.
//! * **Picture section** (`[picture]`): Sets the configuration for pictures. Dimensions, quality,
//! brightness, contrast, ISO, exposure and many more can be configured. Two configuration options
//! are a bit different from the rest actually. The `exif` parameter sets if GPS data should be
//! added to images, so that the final image has position metadata, for example. The `raw` option
//! controls if the raw sensor data should be added to images as JPEG metadata. This will add about
//! 8MiB of information to the images, at least.
//! * **Video section** (`[video]`): Sets the configuration for videos. Dimensions, frames per
//! second, bitrate, and many more, most of them also available for pictures.
//!
//! ## Launcher
//!
//! The project has a launcher in `src/main.rs` and can be launched by running `cargo run`.
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
    wrong_pub_self_convention)]
// Allowing these at least for now.
#![allow(missing_docs_in_private_items, unknown_lints, stutter, option_unwrap_used,
    result_unwrap_used, integer_arithmetic, cast_possible_truncation, cast_possible_wrap,
    indexing_slicing, cast_precision_loss, cast_sign_loss)]

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

const CONFIG_FILE: &str = "config.toml";
const STATE_FILE: &str = "last_state";

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
    initialize_data_filesystem()
        .chain_err(|| ErrorKind::DataFSInit)?;

    if let Some(state) = State::get_last().chain_err(|| ErrorKind::LastStateRead)? {
        unimplemented!()
    } else {
        logic::init().main_logic()
    }
}

/// Initializes the data file system for videos and images.
pub fn initialize_data_filesystem() -> Result<()> {
    let video_path = CONFIG.data_dir().join("video");
    fs::create_dir_all(&video_path)
        .chain_err(|| ErrorKind::DirectoryCreation(video_path))?;

    let img_path = CONFIG.data_dir().join("img");
    fs::create_dir_all(&img_path)
        .chain_err(|| ErrorKind::DirectoryCreation(img_path))?;

    Ok(())
}

/// Prints a stack trace of a complete system failure.
pub fn print_system_failure<S: AsRef<str>>(error: &Error, main_error: S) {
    use colored::Colorize;
    print!("{}", generate_error_string(error, main_error).red());
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
        .unwrap();
    let system = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern_naive)))
        .build(format!("data/logs/system-{}.log", now))
        .unwrap();
    let telemetry = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern_exact)))
        .build(format!("data/logs/telemetry-{}.log", now))
        .unwrap();
    let telemetry_frames = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern_exact)))
        .build(format!("data/logs/telemetry_frames-{}.log", now))
        .unwrap();
    let gps = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern_naive)))
        .build(format!("data/logs/gps-{}.log", now))
        .unwrap();
    let gps_frames = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern_exact)))
        .build(format!("data/logs/gps_frames-{}.log", now))
        .unwrap();
    let gsm = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern_naive)))
        .build(format!("data/logs/gsm-{}.log", now))
        .unwrap();
    let gsm_frames = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern_exact)))
        .build(format!("data/logs/gsm_frames-{}.log", now))
        .unwrap();
    let camera = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern_naive)))
        .build(format!("data/logs/camera-{}.log", now))
        .unwrap();

    let log_level = if CONFIG.debug() {
        LogLevelFilter::Trace
    } else {
        LogLevelFilter::Debug
    };
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
    let gsm_logger = {
        let mut builder = Logger::builder()
            .appender("gsm")
            .appender("gsm_frames")
            .additive(false);
        if CONFIG.debug() {
            builder = builder.appender("gsm_serial");
        }
        builder.build("gsm", log_level)
    };

    let config = Config::builder()
            // Appenders
            .appender(Appender::builder().build("stdout", Box::new(stdout)))
            .appender(Appender::builder().build("main", Box::new(main)))
            .appender(Appender::builder().build("system", Box::new(system)))
            .appender(Appender::builder()
                          .filter(Box::new(ThresholdFilter::new(LogLevelFilter::Info)))
                          .build("telemetry", Box::new(telemetry)))
            .appender(Appender::builder()
                          .filter(Box::new(ThresholdFilter::new(LogLevelFilter::Debug)))
                          .build("telemetry_frames", Box::new(telemetry_frames)))
            .appender(Appender::builder()
                          .filter(Box::new(ThresholdFilter::new(LogLevelFilter::Info)))
                          .build("gps", Box::new(gps)))
            .appender(Appender::builder()
                          .filter(Box::new(ThresholdFilter::new(LogLevelFilter::Debug)))
                          .build("gps_frames", Box::new(gps_frames)))
            .appender(Appender::builder()
                          .filter(Box::new(ThresholdFilter::new(LogLevelFilter::Info)))
                          .build("gsm", Box::new(gsm)))
            .appender(Appender::builder()
                          .filter(Box::new(ThresholdFilter::new(LogLevelFilter::Debug)))
                          .build("gsm_frames", Box::new(gsm_frames)))
            .appender(Appender::builder().build("camera", Box::new(camera)))
            // Loggers
            .logger(Logger::builder()
                        .appender("system")
                        .additive(false)
                        .build("system", LogLevelFilter::Info))
            .logger(telemetry_logger)
            .logger(gps_logger)
            .logger(gsm_logger)
            .logger(Logger::builder()
                        .appender("camera")
                        .additive(false)
                        .build("camera", LogLevelFilter::Info));
    let config = if CONFIG.debug() {
        let telemetry_serial = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(pattern_exact)))
            .build(format!("data/logs/telemetry_serial-{}.log", now))
            .unwrap();
        let gps_serial = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(pattern_exact)))
            .build(format!("data/logs/gps_serial-{}.log", now))
            .unwrap();
        let gsm_serial = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(pattern_exact)))
            .build(format!("data/logs/gsm_serial-{}.log", now))
            .unwrap();

        config
            .appender(Appender::builder().build("telemetry_serial", Box::new(telemetry_serial)))
            .appender(Appender::builder().build("gps_serial", Box::new(gps_serial)))
            .appender(Appender::builder().build("gsm_serial", Box::new(gsm_serial)))
    } else {
        config
    };
    let config = config
        .build(Root::builder()
                   .appender("stdout")
                   .appender("main")
                   .build(LogLevelFilter::Info))?;

    Ok(log4rs::init_config(config)?)
}
