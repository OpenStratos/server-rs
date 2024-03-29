//! Configuration module.
//!
//! One of the main features of OpenStratos is that it's almost 100% configurable. Apart from the
//! features above, the package contains a `config.toml` file, written in
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
//! You can also check the [`Config`](struct.Config.html) structure for further implementation
//! details.

#![allow(missing_debug_implementations)]

use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
    result::Result,
    u8,
};

// Only required for raspicam
#[cfg(feature = "raspicam")]
use std::{ffi::OsStr, i8, u16};

// Only required for GPS, FONA or telemetry
#[cfg(any(feature = "gps", feature = "fona"))]
use std::fmt;

use anyhow::{Context, Error};
use colored::Colorize;
use once_cell::sync::Lazy;
use serde::Deserialize;
use toml;

// Only required for GPS, FONA or telemetry
#[cfg(any(feature = "gps", feature = "fona"))]
use serde::de::{self, Deserializer, Visitor};

// Only required for GPS or FONA
#[cfg(any(feature = "gps", feature = "fona"))]
use sysfs_gpio::Pin;

use crate::{error, generate_error_string, CONFIG_FILE};

/// Configuration object.
pub static CONFIG: Lazy<Config> = Lazy::new(|| match Config::from_file(CONFIG_FILE) {
    Err(e) => {
        panic!(
            "{}",
            generate_error_string(&e, "error loading configuration").red()
        );
    }
    Ok(c) => c,
});

/// Configuration object.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Wether the application should run in debug mode or not.
    debug: Option<bool>,
    /// The data directory.
    data_dir: PathBuf,
    /// Flight configuration.
    flight: Flight,
    /// Battery configuration.
    #[cfg(feature = "fona")]
    battery: Battery,
    /// Video configuration.
    #[cfg(feature = "raspicam")]
    video: Video,
    /// Picture configuration.
    #[cfg(feature = "raspicam")]
    picture: Picture,
    /// GPS configuration.
    #[cfg(feature = "gps")]
    gps: Gps,
    /// FONA module configuration.
    #[cfg(feature = "fona")]
    fona: Fona,
    ///Telemetry configuration.
    #[cfg(feature = "telemetry")]
    telemetry: Telemetry,
}

impl Config {
    /// Creates a new configuration object from a path.
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Config, Error> {
        let file = File::open(path.as_ref()).context(error::Config::Open {
            path: path.as_ref().to_owned(),
        })?;
        let mut reader = BufReader::new(file);
        let mut contents = String::new();

        let _ = reader
            .read_to_string(&mut contents)
            .context(error::Config::Read {
                path: path.as_ref().to_owned(),
            })?;

        let config: Config = toml::from_str(&contents).context(error::Config::InvalidToml {
            path: path.as_ref().to_owned(),
        })?;

        if let (false, errors) = config.verify() {
            Err(error::Config::Invalid { errors }.into())
        } else {
            Ok(config)
        }
    }

    /// Verify the correctness of the configuration, and return a list of errors if invalid.
    #[allow(clippy::too_many_lines)]
    fn verify(&self) -> (bool, String) {
        // Only required for Raspicam
        #[cfg(feature = "raspicam")]
        let mut errors = String::new();
        #[cfg(feature = "raspicam")]
        let mut ok = true;

        #[cfg(feature = "raspicam")]
        {
            // Check for picture configuration errors.
            if self.picture.width > 3280 {
                ok = false;
                errors.push_str(&format!(
                    "picture width must be below or equal to 3280px, found {}px\n",
                    self.picture.width
                ));
            }
            if self.picture.height > 2464 {
                ok = false;
                errors.push_str(&format!(
                    "picture height must be below or equal to 2464px, found {}px\n",
                    self.picture.height
                ));
            }

            if let Some(rotation) = self.picture.rotation {
                if rotation > 359 {
                    ok = false;
                    errors.push_str(&format!(
                        "camera rotation must be between 0 and 359 degrees, found {rotation} \
                         degrees\n"
                    ));
                }
            }

            if self.picture.quality > 100 {
                ok = false;
                errors.push_str(&format!(
                    "picture quality must be a number between 0 and 100, found {}px\n",
                    self.picture.quality
                ));
            }

            if let Some(brightness) = self.picture.brightness {
                if brightness > 100 {
                    ok = false;
                    errors.push_str(&format!(
                        "picture brightness must be between 0 and 100, found {brightness}\n",
                    ));
                }
            }

            if let Some(contrast) = self.picture.contrast {
                if !(-100..=100).contains(&contrast) {
                    ok = false;
                    errors.push_str(&format!(
                        "picture contrast must be between -100 and 100, found {contrast}\n",
                    ));
                }
            }

            if let Some(sharpness) = self.picture.sharpness {
                if !(-100..=100).contains(&sharpness) {
                    ok = false;
                    errors.push_str(&format!(
                        "picture sharpness must be between -100 and 100, found {sharpness}\n",
                    ));
                }
            }
            if let Some(saturation) = self.picture.saturation {
                if !(-100..=100).contains(&saturation) {
                    ok = false;
                    errors.push_str(&format!(
                        "picture saturation must be between -100 and 100, found {saturation}\n",
                    ));
                }
            }

            if let Some(iso) = self.picture.iso {
                if !(100..=800).contains(&iso) {
                    ok = false;
                    errors.push_str(&format!(
                        "picture ISO must be between 100 and 800, found {iso}\n",
                    ));
                }
            }

            if let Some(ev) = self.picture.ev {
                if !(-10..=10).contains(&ev) {
                    ok = false;
                    errors.push_str(&format!(
                        "picture EV compensation must be between -10 and 10, found {ev}\n",
                    ));
                }
            }

            // Check for video configuration errors.
            if self.video.width > 2592 {
                ok = false;
                errors.push_str(&format!(
                    "video width must be below or equal to 2592px, found {}px\n",
                    self.video.width
                ));
            }
            if self.video.height > 1944 {
                ok = false;
                errors.push_str(&format!(
                    "video height must be below or equal to 1944px, found {}px\n",
                    self.video.height
                ));
            }

            if let Some(rotation) = self.video.rotation {
                if rotation > 359 {
                    ok = false;
                    errors.push_str(&format!(
                        "camera rotation must be between 0 and 359 degrees, found {rotation} \
                         degrees\n",
                    ));
                }
            }

            if self.video.fps > 90 {
                ok = false;
                errors.push_str(&format!(
                    "video framerate must be below or equal to 90fps, found {}fps\n",
                    self.video.fps
                ));
            }

            if let Some(brightness) = self.video.brightness {
                if brightness > 100 {
                    ok = false;
                    errors.push_str(&format!(
                        "video brightness must be between 0 and 100, found {brightness}\n",
                    ));
                }
            }

            if let Some(contrast) = self.video.contrast {
                if !(-100..=100).contains(&contrast) {
                    ok = false;
                    errors.push_str(&format!(
                        "video contrast must be between -100 and 100, found {contrast}\n",
                    ));
                }
            }

            if let Some(sharpness) = self.video.sharpness {
                if !(-100..=100).contains(&sharpness) {
                    ok = false;
                    errors.push_str(&format!(
                        "video sharpness must be between -100 and 100, found {sharpness}\n",
                    ));
                }
            }

            if let Some(saturation) = self.video.saturation {
                if !(-100..=100).contains(&saturation) {
                    ok = false;
                    errors.push_str(&format!(
                        "video saturation must be between -100 and 100, found {saturation}\n",
                    ));
                }
            }

            if let Some(iso) = self.video.iso {
                if !(100..=800).contains(&iso) {
                    ok = false;
                    errors.push_str(&format!(
                        "video ISO must be between 100 and 800, found {iso}\n",
                    ));
                }
            }

            if let Some(ev) = self.video.ev {
                if !(-10..=10).contains(&ev) {
                    ok = false;
                    errors.push_str(&format!(
                        "video EV compensation must be between -10 and 10, found {ev}\n",
                    ));
                }
            }

            // Video modes.
            match (self.video.width, self.video.height, self.video.fps) {
                (2592, 1944, 1..=15)
                | (1920, 1080, 1..=30)
                | (1296, 972, 1..=42)
                | (1296, 730, 1..=49)
                | (640, 480, 1..=90) => {}
                (w, h, f) => {
                    ok = false;
                    errors.push_str(&format!(
                        "video mode must be one of 2592\u{d7}1944 1-15fps, 1920\u{d7}1080 \
                         1-30fps, 1296\u{d7}972 1-42fps, 1296\u{d7}730 1-49fps, 640\u{d7}480 \
                         1-90fps, found {w}x{h} {f}fps\n",
                    ));
                }
            }
        }

        // TODO check GPS configuration

        // Only required for Raspicam
        #[cfg(feature = "raspicam")]
        {
            (ok, errors)
        }

        #[cfg(not(feature = "raspicam"))]
        {
            (true, String::new())
        }
    }

    /// Gets wether OpenStratos should run in debug mode.
    #[must_use]
    pub fn debug(&self) -> bool {
        self.debug == Some(true)
    }

    /// Gets the flight information.
    #[must_use]
    pub fn flight(&self) -> Flight {
        self.flight
    }

    /// Gets battery configuration
    #[cfg(feature = "fona")]
    #[must_use]
    pub fn battery(&self) -> Battery {
        self.battery
    }

    /// Gets the configuration for video.
    #[cfg(feature = "raspicam")]
    #[must_use]
    pub fn video(&self) -> &Video {
        &self.video
    }

    /// Gets the configuration for pictures.
    #[cfg(feature = "raspicam")]
    #[must_use]
    pub fn picture(&self) -> &Picture {
        &self.picture
    }

    /// Gets the GPS configuration.
    #[cfg(feature = "gps")]
    #[must_use]
    pub fn gps(&self) -> &Gps {
        &self.gps
    }

    /// Gets the FONA module configuration.
    #[cfg(feature = "fona")]
    #[must_use]
    pub fn fona(&self) -> &Fona {
        &self.fona
    }

    /// Gets the telemetry configuration.
    #[cfg(feature = "telemetry")]
    #[must_use]
    pub fn telemetry(&self) -> &Telemetry {
        &self.telemetry
    }

    /// Gets the configured data directory.
    #[must_use]
    pub fn data_dir(&self) -> &Path {
        self.data_dir.as_path()
    }
}

/// Flight configuration structure.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Flight {
    /// Approximate expected flight length, in minutes.
    length: u32,
    /// Approximate expected maximum height, in meters.
    expected_max_height: u32,
}

impl Flight {
    /// Gets the approximate expected flight length, in minutes.
    #[must_use]
    pub fn length(self) -> u32 {
        self.length
    }

    /// Gets the approximate expected maximum height, in meters.
    #[must_use]
    pub fn expected_max_height(self) -> u32 {
        self.expected_max_height
    }
}

/// Battery configuration structure.
#[cfg(feature = "fona")]
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Battery {
    /// Minimum voltage for the main battery when empty, at 0%, in volts (`V`).
    main_min: f32,
    /// Maximum voltage for the main battery when full, at 100%, in volts (`V`).
    main_max: f32,
    /// Minimum voltage for the FONA battery when empty, at 0%, in volts (`V`).
    fona_min: f32,
    /// Maximum voltage for the FONA battery when full, at 100%, in volts (`V`).
    fona_max: f32,
    /// Minimum admissible percentage for main battery for the launch.
    main_min_percent: f32,
    /// Minimum admissible percentage for FONA battery for the launch.
    fona_min_percent: f32,
}

#[cfg(feature = "fona")]
impl Battery {
    /// Gets the minimum voltage for the main battery when empty, at 0%, in volts (`V`).
    #[must_use]
    pub fn main_min(self) -> f32 {
        self.main_min
    }

    /// Gets the maximum voltage for the main battery when full, at 100%, in volts (`V`).
    #[must_use]
    pub fn main_max(self) -> f32 {
        self.main_max
    }

    /// Gets the minimum voltage for the FONA battery when empty, at 0%, in volts (`V`).
    #[must_use]
    pub fn fona_min(self) -> f32 {
        self.fona_min
    }

    /// Gets the maximum voltage for the FONA battery when full, at 0%, in volts (`V`).
    #[must_use]
    pub fn fona_max(self) -> f32 {
        self.fona_max
    }

    /// Gets the minimum admissible percentage for main battery for the launch.
    #[must_use]
    pub fn main_min_percent(self) -> f32 {
        self.main_min_percent
    }

    /// Gets the minimum admissible percentage for FONA battery for the launch.
    #[must_use]
    pub fn fona_min_percent(self) -> f32 {
        self.fona_min_percent
    }
}

/// Video configuration structure.
#[cfg(feature = "raspicam")]
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Video {
    /// Height of the video, in px.
    height: u16,
    /// Width of the video, in px.
    width: u16,
    /// Rotation of the camera, in degrees (°).
    rotation: Option<u16>,
    /// Frames per second (FPS) for the video.
    fps: u8,
    /// Bit rate for the video, in bps (bits per second).
    bitrate: u32,
    /// Exposure configuration.
    exposure: Option<Exposure>,
    /// Brightness correction.
    brightness: Option<u8>,
    /// Contrast correction.
    contrast: Option<i8>,
    /// Sharpness configuration.
    sharpness: Option<i8>,
    /// Saturation configuration.
    saturation: Option<i8>,
    /// ISO for the image.
    iso: Option<u16>,
    /// Video stabilization.
    stabilization: Option<bool>,
    /// EV correction configuration.
    ev: Option<i8>,
    /// White balance configuration.
    white_balance: Option<WhiteBalance>,
}

#[cfg(feature = "raspicam")]
impl Video {
    /// Gets the configured video height for the camera, in pixels.
    #[must_use]
    pub fn height(self) -> u16 {
        self.height
    }

    /// Gets the configured video width for the camera, in pixels.
    #[must_use]
    pub fn width(self) -> u16 {
        self.width
    }

    /// Gets the configured picture rotation for the camera, in degrees (°).
    #[must_use]
    pub fn rotation(self) -> Option<u16> {
        self.rotation
    }

    /// Gets the configured video framerate for the camera, in frames per second.
    #[must_use]
    pub fn fps(self) -> u8 {
        self.fps
    }

    /// Gets the configured bitrate for videos.
    #[must_use]
    pub fn bitrate(self) -> u32 {
        self.bitrate
    }

    /// Gets the configured exposure for videos.
    #[must_use]
    pub fn exposure(self) -> Option<Exposure> {
        self.exposure
    }

    /// Gets the configured brightness for videos.
    #[must_use]
    pub fn brightness(self) -> Option<u8> {
        self.brightness
    }

    /// Gets the configured contrast for videos.
    #[must_use]
    pub fn contrast(self) -> Option<i8> {
        self.contrast
    }

    /// Gets the configured sharpness for videos.
    #[must_use]
    pub fn sharpness(self) -> Option<i8> {
        self.sharpness
    }

    /// Gets the configured saturation for videos.
    #[must_use]
    pub fn saturation(self) -> Option<i8> {
        self.saturation
    }

    /// Gets the configured ISO for videos.
    #[must_use]
    pub fn iso(self) -> Option<u16> {
        self.iso
    }

    /// Gets if video stabilization needs to be turned on.
    #[must_use]
    pub fn stabilization(self) -> bool {
        self.stabilization == Some(true)
    }

    /// Gets the configured EV compensation for videos.
    #[must_use]
    pub fn ev(self) -> Option<i8> {
        self.ev
    }

    /// Gets the configured automatic white balance for videos.
    #[must_use]
    pub fn white_balance(self) -> Option<WhiteBalance> {
        self.white_balance
    }
}

/// Picture configuration structure.
#[cfg(feature = "raspicam")]
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Picture {
    /// Height of the picture, in px.
    height: u16,
    /// Width of the picture, in px.
    width: u16,
    /// Rotation of the camera, in degrees (°).
    rotation: Option<u16>,
    /// Quality of the picture, in px.
    quality: u8,
    /// Wether to add EXIF data to pictures or not.
    #[cfg(feature = "gps")]
    exif: Option<bool>,
    /// Wether to save the raw sensor data as JPG metadata.
    raw: Option<bool>,
    /// Exposure configuration.
    exposure: Option<Exposure>,
    /// Brightness correction.
    brightness: Option<u8>,
    /// Contrast correction.
    contrast: Option<i8>,
    /// Sharpness configuration.
    sharpness: Option<i8>,
    /// Saturation configuration.
    saturation: Option<i8>,
    /// ISO for the image.
    iso: Option<u16>,
    /// EV correction configuration.
    ev: Option<i8>,
    /// White balance configuration.
    white_balance: Option<WhiteBalance>,
    /// Interval between pictures during flight.
    interval: u32,
    /// Repeat each picture after these seconds (for issues with probe movement).
    repeat: Option<u32>,
    /// Timeout for first picture after launch, in seconds.
    first_timeout: u32,
}

#[cfg(feature = "raspicam")]
impl Picture {
    /// Gets the configured picture height for the camera, in pixels.
    #[must_use]
    pub fn height(self) -> u16 {
        self.height
    }

    /// Gets the configured picture width for the camera, in pixels.
    #[must_use]
    pub fn width(self) -> u16 {
        self.width
    }

    /// Gets the configured picture rotation for the camera, in degrees (°).
    #[must_use]
    pub fn rotation(self) -> Option<u16> {
        self.rotation
    }

    /// Gets the configured picture quality for the camera.
    #[must_use]
    pub fn quality(self) -> u8 {
        self.quality
    }

    /// Gets wether the camera should add available EXIF information to pictures.
    #[cfg(feature = "gps")]
    #[must_use]
    pub fn exif(self) -> bool {
        self.exif == Some(true)
    }

    /// Gets wether the camera should add raw sensor data to pictures as JPEG metadata.
    #[must_use]
    pub fn raw(self) -> bool {
        self.raw == Some(true)
    }

    /// Gets the configured exposure for pictures.
    #[must_use]
    pub fn exposure(self) -> Option<Exposure> {
        self.exposure
    }

    /// Gets the configured brightness for pictures.
    #[must_use]
    pub fn brightness(self) -> Option<u8> {
        self.brightness
    }

    /// Gets the configured contrast for pictures.
    #[must_use]
    pub fn contrast(self) -> Option<i8> {
        self.contrast
    }

    /// Gets the configured sharpness for pictures.
    #[must_use]
    pub fn sharpness(self) -> Option<i8> {
        self.sharpness
    }

    /// Gets the configured saturation for pictures.
    #[must_use]
    pub fn saturation(self) -> Option<i8> {
        self.saturation
    }

    /// Gets the configured ISO for pictures.
    #[must_use]
    pub fn iso(self) -> Option<u16> {
        self.iso
    }

    /// Gets the configured EV compensation for pictures.
    #[must_use]
    pub fn ev(self) -> Option<i8> {
        self.ev
    }

    /// Gets the configured automatic white balance for pictures.
    #[must_use]
    pub fn white_balance(self) -> Option<WhiteBalance> {
        self.white_balance
    }

    /// Gets the interval between pictures during flight.
    #[must_use]
    pub fn interval(self) -> u32 {
        self.interval
    }

    /// Gets the timeout for repeated picture.
    ///
    /// Repeat each picture after these seconds (for issues with probe movement).
    #[must_use]
    pub fn repeat(self) -> Option<u32> {
        self.repeat
    }

    /// Gets the timeout for first picture after launch, in seconds.
    #[must_use]
    pub fn first_timeout(self) -> u32 {
        self.first_timeout
    }
}

/// Exposure setting.
#[cfg(feature = "raspicam")]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
pub enum Exposure {
    /// Turns off exposure control.
    Off,
    /// Use automatic exposure mode.
    Auto,
    /// Select setting for night shooting.
    Night,
    /// Night preview mode.
    NightPreview,
    /// Select setting for back-lit subject.
    BackLight,
    /// Spot light mode.
    SpotLight,
    /// Select setting for sports (fast shutter etc.).
    Sports,
    /// Select setting optimized for snowy scenery.
    Snow,
    /// Select setting optimized for beach.
    Beach,
    /// Select setting for long exposures.
    VeryLong,
    /// Constrain fps to a fixed value.
    FixedFps,
    /// Anti-shake mode.
    AntiShake,
    /// Select setting optimized for fireworks.
    Fireworks,
}

#[cfg(feature = "raspicam")]
impl AsRef<OsStr> for Exposure {
    fn as_ref(&self) -> &OsStr {
        OsStr::new(match *self {
            Exposure::Off => "off",
            Exposure::Auto => "auto",
            Exposure::Night => "night",
            Exposure::NightPreview => "nightpreview",
            Exposure::BackLight => "backlight",
            Exposure::SpotLight => "spotlight",
            Exposure::Sports => "sports",
            Exposure::Snow => "snow",
            Exposure::Beach => "beach",
            Exposure::VeryLong => "verylong",
            Exposure::FixedFps => "fixedfps",
            Exposure::AntiShake => "antishake",
            Exposure::Fireworks => "fireworks",
        })
    }
}

/// White balance setting.
#[cfg(feature = "raspicam")]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
pub enum WhiteBalance {
    /// Turn off white balance calculation.
    Off,
    /// Automatic mode (default).
    Auto,
    /// Sunny mode.
    Sun,
    /// Cloudy mode.
    CloudShade,
    /// Tungsten lighting mode.
    Tungsten,
    /// Fluorescent lighting mode.
    Fluorescent,
    /// Incandescent lighting mode.
    Incandescent,
    /// Flash mode.
    Flash,
    /// Horizon mode.
    Horizon,
}

#[cfg(feature = "raspicam")]
impl AsRef<OsStr> for WhiteBalance {
    fn as_ref(&self) -> &OsStr {
        OsStr::new(match *self {
            WhiteBalance::Off => "off",
            WhiteBalance::Auto => "auto",
            WhiteBalance::Sun => "sun",
            WhiteBalance::CloudShade => "cloudshade",
            WhiteBalance::Tungsten => "tungsten",
            WhiteBalance::Fluorescent => "fluorescent",
            WhiteBalance::Incandescent => "incandescent",
            WhiteBalance::Flash => "flash",
            WhiteBalance::Horizon => "horizon",
        })
    }
}

/// GPS configuration structure.
#[cfg(feature = "gps")]
#[derive(Debug, Deserialize)]
pub struct Gps {
    /// UART serial console path.
    uart: PathBuf,
    /// Serial console baud rate.
    baud_rate: u32,
    /// Power GPIO pin.
    #[serde(deserialize_with = "deserialize_pin")]
    power_gpio: Pin,
}

#[cfg(feature = "gps")]
impl Gps {
    /// Gets the UART serial console path.
    #[must_use]
    pub fn uart(&self) -> &Path {
        &self.uart
    }

    /// Gets the serial console baud rate.
    #[must_use]
    pub fn baud_rate(&self) -> u32 {
        self.baud_rate
    }

    /// Gets the power GPIO pin.
    #[must_use]
    pub fn power_gpio(&self) -> Pin {
        self.power_gpio
    }
}

/// Fona configuration structure
#[cfg(feature = "fona")]
#[derive(Debug, Deserialize)]
pub struct Fona {
    /// UART serial console path.
    uart: PathBuf,
    /// Serial console baud rate.
    baud_rate: u32,
    /// Power control GPIO pin.
    #[serde(deserialize_with = "deserialize_pin")]
    power_gpio: Pin,
    /// Status GPIO pin.
    #[serde(deserialize_with = "deserialize_pin")]
    status_gpio: Pin,
    /// SMS receiver phone number.
    sms_phone: PhoneNumber,
    /// Operator GSM location service domain.
    location_service: String,
}

#[cfg(feature = "fona")]
impl Fona {
    /// Gets the UART serial console path.
    #[must_use]
    pub fn uart(&self) -> &Path {
        &self.uart
    }

    /// Gets the serial console baud rate.
    #[must_use]
    pub fn baud_rate(&self) -> u32 {
        self.baud_rate
    }

    /// Gets the power GPIO pin.
    #[must_use]
    pub fn power_gpio(&self) -> Pin {
        self.power_gpio
    }

    /// Gets the status GPIO pin.
    #[must_use]
    pub fn status_gpio(&self) -> Pin {
        self.status_gpio
    }

    /// Gets the phone number for SMSs.
    #[must_use]
    pub fn sms_phone(&self) -> &PhoneNumber {
        &self.sms_phone
    }

    /// Gets the location service for GSM location retrieval.
    #[must_use]
    pub fn location_service(&self) -> &str {
        &self.location_service
    }
}

/// Phone number representation.
#[cfg(feature = "fona")]
#[derive(Debug)]
pub struct PhoneNumber(String);

#[cfg(feature = "fona")]
impl PhoneNumber {
    /// Gets the phone number as a string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(feature = "fona")]
impl<'de> Deserialize<'de> for PhoneNumber {
    fn deserialize<D>(deserializer: D) -> Result<PhoneNumber, D::Error>
    where
        D: Deserializer<'de>,
    {
        // TODO: better parsing and checking

        /// Visitor for phone numbers.
        struct PhoneNumberVisitor;
        impl<'dev> Visitor<'dev> for PhoneNumberVisitor {
            type Value = PhoneNumber;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid phone number")
            }

            fn visit_str<E>(self, value: &str) -> Result<PhoneNumber, E>
            where
                E: de::Error,
            {
                Ok(PhoneNumber(value.to_owned()))
            }
        }

        deserializer.deserialize_str(PhoneNumberVisitor)
    }
}

/// Telemetry configuration structure.
#[cfg(feature = "telemetry")]
#[derive(Debug, Deserialize)]
pub struct Telemetry {
    /// UART serial console path.
    uart: PathBuf,
    /// Serial console baud rate.
    baud_rate: u32,
}

#[cfg(feature = "telemetry")]
impl Telemetry {
    /// Gets the UART serial console path.
    #[must_use]
    pub fn uart(&self) -> &Path {
        &self.uart
    }

    /// Gets the serial console baud rate.
    #[must_use]
    pub fn baud_rate(&self) -> u32 {
        self.baud_rate
    }
}

/// Deserializes a Raspberry Pi pin number into a `Pin` structure.
///
/// Note: it will make sure it deserializes a Pin between 2 and 28 (pin numbers for Raspberry Pi).
#[cfg(any(feature = "gps", feature = "fona"))]
fn deserialize_pin<'de, D>(deserializer: D) -> Result<Pin, D::Error>
where
    D: Deserializer<'de>,
{
    /// Visitor for u32 (comparing to usize).
    struct PinVisitor;
    impl<'dev> Visitor<'dev> for PinVisitor {
        type Value = Pin;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an integer between 2 and 28")
        }

        #[allow(clippy::absurd_extreme_comparisons)]
        fn visit_i64<E>(self, value: i64) -> Result<Pin, E>
        where
            E: de::Error,
        {
            if (2..=28).contains(&value) {
                #[allow(clippy::cast_sign_loss)]
                {
                    Ok(Pin::new(value as u64))
                }
            } else {
                Err(E::custom(format!("pin out of range: {value}")))
            }
        }
    }

    deserializer.deserialize_u8(PinVisitor)
}

#[cfg(test)]
mod tests {
    #[cfg(all(feature = "gps", feature = "raspicam"))]
    use super::Gps;
    #[cfg(all(feature = "raspicam", feature = "telemetry"))]
    use super::Telemetry;
    #[cfg(all(feature = "raspicam", feature = "fona"))]
    use super::{Battery, Fona, PhoneNumber};
    use super::{Config, CONFIG};
    #[cfg(feature = "raspicam")]
    use super::{Exposure, Flight, Picture, Video, WhiteBalance};

    #[cfg(all(feature = "raspicam", any(feature = "gps", feature = "fona")))]
    use sysfs_gpio::Pin;

    #[cfg(feature = "gps")]
    use std::path::Path;
    #[cfg(feature = "raspicam")]
    use std::path::PathBuf;

    /// Loads the default configuration and checks it.
    #[test]
    fn load_config() {
        let config = Config::from_file("config.toml").unwrap();

        assert!(config.debug());
        #[cfg(feature = "raspicam")]
        {
            assert_eq!(config.picture().height(), 2464);
            assert_eq!(config.picture().width(), 3280);
            #[cfg(feature = "gps")]
            {
                assert!(config.picture().exif());
            }
            assert_eq!(config.video().height(), 1080);
            assert_eq!(config.video().width(), 1920);
            assert_eq!(config.video().fps(), 30);
        }

        #[cfg(feature = "gps")]
        {
            assert_eq!(config.gps().uart(), Path::new("/dev/ttyAMA0"));
            assert_eq!(config.gps().baud_rate(), 9_600);
            assert_eq!(config.gps().power_gpio().get_pin(), 3);
        }
    }

    /// Tests an invalid configuration, and the error output.
    #[test]
    #[cfg(feature = "raspicam")]
    #[allow(clippy::too_many_lines)]
    fn config_error() {
        let flight = Flight {
            length: 300,
            expected_max_height: 35000,
        };

        #[cfg(feature = "gps")]
        let picture = Picture {
            height: 10_345,
            width: 5_246,
            rotation: Some(180),
            quality: 95,
            raw: Some(true),
            exif: Some(true),
            exposure: Some(Exposure::AntiShake),
            brightness: Some(50),
            contrast: Some(50),
            sharpness: None,
            saturation: None,
            iso: None,
            ev: None,
            white_balance: Some(WhiteBalance::Horizon),
            first_timeout: 120,
            interval: 300,
            repeat: Some(30),
        };

        #[cfg(not(feature = "gps"))]
        let picture = Picture {
            height: 10_345,
            width: 5_246,
            rotation: Some(180),
            quality: 95,
            raw: Some(true),
            exposure: Some(Exposure::AntiShake),
            brightness: Some(50),
            contrast: Some(50),
            sharpness: None,
            saturation: None,
            iso: None,
            ev: None,
            white_balance: Some(WhiteBalance::Horizon),
            first_timeout: 120,
            interval: 300,
            repeat: Some(30),
        };

        let video = Video {
            height: 12_546,
            width: 5_648,
            rotation: Some(180),
            fps: 92,
            bitrate: 20_000_000,
            exposure: Some(Exposure::AntiShake),
            brightness: Some(50),
            contrast: Some(50),
            sharpness: None,
            saturation: None,
            iso: None,
            stabilization: Some(true),
            ev: None,
            white_balance: Some(WhiteBalance::Horizon),
        };

        #[cfg(feature = "fona")]
        let fona = Fona {
            uart: PathBuf::from("/dev/ttyUSB0"),
            baud_rate: 9_600,
            power_gpio: Pin::new(7),
            status_gpio: Pin::new(21),
            sms_phone: PhoneNumber(String::new()),
            location_service: "gprs-service.com".to_owned(),
        };

        #[cfg(feature = "fona")]
        let battery = Battery {
            main_min: 1.952_777_7,
            main_max: 2.216_666_7,
            fona_min: 3.7,
            fona_max: 4.2,
            main_min_percent: 0.8,
            fona_min_percent: 0.75,
        };

        #[cfg(feature = "telemetry")]
        let telemetry = Telemetry {
            uart: PathBuf::from("/dev/ttyUSB0"),
            baud_rate: 230_400,
        };

        #[cfg(feature = "gps")]
        let gps = Gps {
            uart: PathBuf::from("/dev/ttyAMA0"),
            baud_rate: 9_600,
            power_gpio: Pin::new(3),
        };

        #[cfg(all(feature = "gps", feature = "fona", feature = "telemetry"))]
        let config = Config {
            debug: None,
            flight,
            battery,
            data_dir: PathBuf::from("data"),
            picture,
            video,
            gps,
            fona,
            telemetry,
        };

        #[cfg(all(feature = "gps", feature = "fona", not(feature = "telemetry")))]
        let config = Config {
            debug: None,
            flight,
            battery,
            data_dir: PathBuf::from("data"),
            picture,
            video,
            gps,
            fona,
        };

        #[cfg(all(feature = "gps", not(feature = "fona"), feature = "telemetry"))]
        let config = Config {
            debug: None,
            flight,
            data_dir: PathBuf::from("data"),
            picture,
            video,
            gps,
            telemetry,
        };

        #[cfg(all(feature = "gps", not(feature = "fona"), not(feature = "telemetry")))]
        let config = Config {
            debug: None,
            flight,
            data_dir: PathBuf::from("data"),
            picture,
            video,
            gps,
        };

        #[cfg(all(not(feature = "gps"), feature = "fona", feature = "telemetry"))]
        let config = Config {
            debug: None,
            flight,
            battery,
            data_dir: PathBuf::from("data"),
            picture,
            video,
            fona,
            telemetry,
        };

        #[cfg(all(not(feature = "gps"), feature = "fona", not(feature = "telemetry")))]
        let config = Config {
            debug: None,
            flight,
            battery,
            data_dir: PathBuf::from("data"),
            picture,
            video,
            fona,
        };

        #[cfg(all(not(feature = "gps"), not(feature = "fona"), feature = "telemetry"))]
        let config = Config {
            debug: None,
            flight,
            data_dir: PathBuf::from("data"),
            picture,
            video,
            telemetry,
        };

        #[cfg(all(
            not(feature = "gps"),
            not(feature = "fona"),
            not(feature = "telemetry")
        ))]
        let config = Config {
            debug: None,
            flight,
            data_dir: PathBuf::from("data"),
            picture,
            video,
        };

        let (verify, errors) = config.verify();

        assert!(!verify);
        assert_eq!(
            errors,
            "picture width must be below or equal to 3280px, found 5246px\npicture height \
             must be below or equal to 2464px, found 10345px\nvideo width must be below or \
             equal to 2592px, found 5648px\nvideo height must be below or equal to 1944px, \
             found 12546px\nvideo framerate must be below or equal to 90fps, found 92fps\n\
             video mode must be one of 2592\u{d7}1944 1-15fps, 1920\u{d7}1080 1-30fps, \
             1296\u{d7}972 1-42fps, 1296\u{d7}730 1-49fps, 640\u{d7}480 1-60fps, found 5648x12546 \
             92fps\n"
        );
    }

    /// Tests the default configuration and its loading using the static `CONFIG` constant.
    #[test]
    fn config_static() {
        assert!(CONFIG.debug());
    }
}
