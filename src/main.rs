#![allow(unused)]

// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;
extern crate toml;
#[macro_use]
extern crate log;
extern crate log4rs;
extern crate colored;
extern crate chrono;

const CONFIG_FILE: &'static str = "config.toml";
const STATE_FILE: &'static str = "last_state";

mod error;
mod config;
mod logic;
mod gps;
mod camera;

use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use error::*;
use config::CONFIG;
use logic::{MainLogic, State, OpenStratos};

/// program entry point.
fn main() {
    if CONFIG.debug() {
        println!("Debug mode active");
    }
    if let Err(e) = init_loggers() {
        print_system_failure(e, "Error initializing loggers");
        panic!();
    }
    info!("OpenStratos {} starting", env!("CARGO_PKG_VERSION"));

    if let Err(e) = run() {
        print_system_failure(e, "Error running OpenStratos");
        panic!(); // TODO safe mode / recovery mode / restart...
    }
}

/// The main logic of the program.
fn run() -> Result<()> {
    initialize_data_filesystem().chain_err(|| ErrorKind::DataFSInit)?;

    if let Some(state) = State::get_last().chain_err(|| ErrorKind::LastStateRead)? {
        unimplemented!()
    } else {
        logic::init().main_logic()
    }
}

/// Initializes the data filesystem for videos and images.
fn initialize_data_filesystem() -> Result<()> {
    let video_path = CONFIG.data_dir().join("video");
    fs::create_dir_all(&video_path).chain_err(|| ErrorKind::DirectoryCreation(video_path))?;

    let img_path = CONFIG.data_dir().join("img");
    fs::create_dir_all(&img_path).chain_err(|| ErrorKind::DirectoryCreation(img_path))?;

    Ok(())
}

/// Prints a stack trace of a complete system failure.
fn print_system_failure<S: AsRef<str>>(error: Error, main_error: S) {
    use colored::Colorize;
    print!("{}", generate_error_string(error, main_error).red());
}

/// Generates a stack trace string of an error.
fn generate_error_string<S: AsRef<str>>(error: Error, main_error: S) -> String {
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
fn init_loggers() -> Result<log4rs::Handle> {
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
        let mut builder =
            Logger::builder().appender("telemetry").appender("telemetry_frames").additive(false);
        if CONFIG.debug() {
            builder = builder.appender("telemetry_serial");
        }
        builder.build("telemetry", log_level)
    };
    let gps_logger = {
        let mut builder = Logger::builder().appender("gps").appender("gps_frames").additive(false);
        if CONFIG.debug() {
            builder = builder.appender("gps_serial");
        }
        builder.build("gps", log_level)
    };
    let gsm_logger = {
        let mut builder = Logger::builder().appender("gsm").appender("gsm_frames").additive(false);
        if CONFIG.debug() {
            builder = builder.appender("gsm_serial");
        }
        builder.build("gsm", log_level)
    };

    let config =
        Config::builder()
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

        config.appender(Appender::builder().build("telemetry_serial", Box::new(telemetry_serial)))
            .appender(Appender::builder().build("gps_serial", Box::new(gps_serial)))
            .appender(Appender::builder().build("gsm_serial", Box::new(gsm_serial)))
    } else {
        config
    };
    let config = config.build(Root::builder()
                                  .appender("stdout")
                                  .appender("main")
                                  .build(LogLevelFilter::Info))?;

    Ok(log4rs::init_config(config)?)
}
