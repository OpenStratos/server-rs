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

use anyhow::{Context, Error};

pub use crate::config::CONFIG;
use crate::logic::{MainLogic, State};
use std::fs;

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

    while let Some(error) = error.source() {
        result.push_str(&format!("\tcaused by: {}\n", error));
    }

    result
}
