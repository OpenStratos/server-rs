//! Configuration module.
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
#[cfg(any(feature = "gps", feature = "fona", feature = "telemetry"))]
use std::fmt;

use colored::Colorize;
use failure::{Error, ResultExt};
use toml;

// Only required for GPS, FONA or telemetry
#[cfg(any(feature = "gps", feature = "fona", feature = "telemetry"))]
use serde::de::{self, Deserializer, Visitor};
#[cfg(any(feature = "gps", feature = "fona", feature = "telemetry"))]
use tokio_serial::BaudRate;

// Only required for FONA
#[cfg(feature = "fona")]
use serde::de::Deserialize;

// Only required for GPS or FONA
#[cfg(any(feature = "gps", feature = "fona"))]
use sysfs_gpio::Pin;

use error;
use generate_error_string;
use CONFIG_FILE;

lazy_static! {
    /// Configuration object.
    pub static ref CONFIG: Config = match Config::from_file(CONFIG_FILE) {
        Err(e) => {
            println!("{}", generate_error_string(&e, "error loading configuration").red());
            panic!();
        },
        Ok(c) => c,
    };
}

/// Configuration object.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Wether the application should run in debug mode or not.
    debug: Option<bool>,
    /// Video configuration.
    #[cfg(feature = "raspicam")]
    video: Video,
    /// Picture configuration.
    #[cfg(feature = "raspicam")]
    picture: Picture,
    /// The rotation of the camera.
    #[cfg(feature = "raspicam")]
    camera_rotation: Option<u16>,
    /// GPS configuration.
    #[cfg(feature = "gps")]
    gps: Gps,
    /// FONA module configuration.
    #[cfg(feature = "fona")]
    fona: Fona,
    ///Telemetry configuration.
    #[cfg(feature = "telemetry")]
    telemetry: Telemetry,
    /// Flight information.
    flight: Flight,
    /// The data directory.
    data_dir: PathBuf,
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

            if self.picture.quality > 100 {
                ok = false;
                errors.push_str(&format!(
                    "picture quality must be a number between 0 and 100, found {}px\n",
                    self.picture.quality
                ));
            }

            if let Some(b @ 101...u8::MAX) = self.picture.brightness {
                ok = false;
                errors.push_str(&format!(
                    "picture brightness must be between 0 and 100, found {}\n",
                    b
                ));
            }

            match self.picture.contrast {
                Some(c @ i8::MIN...-101) | Some(c @ 101...i8::MAX) => {
                    ok = false;
                    errors.push_str(&format!(
                        "picture contrast must be between -100 and 100, found {}\n",
                        c
                    ));
                }
                _ => {}
            }

            match self.picture.sharpness {
                Some(s @ i8::MIN...-101) | Some(s @ 101...i8::MAX) => {
                    ok = false;
                    errors.push_str(&format!(
                        "picture sharpness must be between -100 and 100, found {}\n",
                        s
                    ));
                }
                _ => {}
            }

            match self.picture.saturation {
                Some(s @ i8::MIN...-101) | Some(s @ 101...i8::MAX) => {
                    ok = false;
                    errors.push_str(&format!(
                        "picture saturation must be between -100 and 100, found {}\n",
                        s
                    ));
                }
                _ => {}
            }

            match self.picture.iso {
                Some(i @ 0...99) | Some(i @ 801...u16::MAX) => {
                    ok = false;
                    errors.push_str(&format!(
                        "picture ISO must be between 100 and 800, found {}\n",
                        i
                    ));
                }
                _ => {}
            }

            match self.picture.ev {
                Some(e @ i8::MIN...-11) | Some(e @ 11...i8::MAX) => {
                    ok = false;
                    errors.push_str(&format!(
                        "picture EV compensation must be between -10 and 10, found {}\n",
                        e
                    ));
                }
                _ => {}
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
            if self.video.fps > 90 {
                ok = false;
                errors.push_str(&format!(
                    "video framerate must be below or equal to 90fps, found {}fps\n",
                    self.video.fps
                ));
            }

            // Video modes.
            match (self.video.width, self.video.height, self.video.fps) {
                (2592, 1944, 1...15)
                | (1920, 1080, 1...30)
                | (1296, 972, 1...42)
                | (1296, 730, 1...49)
                | (640, 480, 1...90) => {}
                (w, h, f) => {
                    ok = false;
                    errors.push_str(&format!(
                        "video mode must be one of 2592×1944 1-15fps, 1920×1080 1-30fps, 1296×972 \
                         1-42fps, 1296×730 1-49fps, 640×480 1-60fps, found {}x{} {}fps\n",
                        w, h, f
                    ));
                }
            }

            if let Some(r @ 360...u16::MAX) = self.camera_rotation {
                ok = false;
                errors.push_str(&format!(
                    "camera rotation must be between 0 and 359 degrees, found {} degrees\n",
                    r
                ));
            }

            if let Some(b @ 101...u8::MAX) = self.video.brightness {
                ok = false;
                errors.push_str(&format!(
                    "video brightness must be between 0 and 100, found {}\n",
                    b
                ));
            }

            match self.video.contrast {
                Some(c @ i8::MIN...-101) | Some(c @ 101...i8::MAX) => {
                    ok = false;
                    errors.push_str(&format!(
                        "video contrast must be between -100 and 100, found {}\n",
                        c
                    ));
                }
                _ => {}
            }

            match self.video.sharpness {
                Some(s @ i8::MIN...-101) | Some(s @ 101...i8::MAX) => {
                    ok = false;
                    errors.push_str(&format!(
                        "video sharpness must be between -100 and 100, found \
                         {}\n",
                        s
                    ));
                }
                _ => {}
            }

            match self.video.saturation {
                Some(s @ i8::MIN...-101) | Some(s @ 101...i8::MAX) => {
                    ok = false;
                    errors.push_str(&format!(
                        "video saturation must be between -100 and 100, found {}\n",
                        s
                    ));
                }
                _ => {}
            }

            match self.video.iso {
                Some(i @ 0...99) | Some(i @ 801...u16::MAX) => {
                    ok = false;
                    errors.push_str(&format!(
                        "video ISO must be between 100 and 800, found {}\n",
                        i
                    ));
                }
                _ => {}
            }

            match self.video.ev {
                Some(e @ i8::MIN...-11) | Some(e @ 11...i8::MAX) => {
                    ok = false;
                    errors.push_str(&format!(
                        "video EV compensation must be between -10 and 10, found {}\n",
                        e
                    ));
                }
                _ => {}
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
    pub fn debug(&self) -> bool {
        self.debug == Some(true)
    }

    /// Gets the configuration for video.
    #[cfg(feature = "raspicam")]
    pub fn video(&self) -> &Video {
        &self.video
    }

    /// Gets the configuration for pictures.
    #[cfg(feature = "raspicam")]
    pub fn picture(&self) -> &Picture {
        &self.picture
    }

    /// Gets the configured camera rotation.
    #[cfg(feature = "raspicam")]
    pub fn camera_rotation(&self) -> Option<u16> {
        self.camera_rotation
    }

    /// Gets the GPS configuration.
    #[cfg(feature = "gps")]
    pub fn gps(&self) -> &Gps {
        &self.gps
    }

    /// Gets the FONA module configuration.
    #[cfg(feature = "fona")]
    pub fn fona(&self) -> &Fona {
        &self.fona
    }

    /// Gets the telemetry configuration.
    #[cfg(feature = "telemetry")]
    pub fn telemetry(&self) -> &Telemetry {
        &self.telemetry
    }

    /// Gets the flight information.
    pub fn flight(&self) -> &Flight {
        &self.flight
    }

    /// Gets the configured data directory.
    pub fn data_dir(&self) -> &Path {
        self.data_dir.as_path()
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
    /// Frames per second (FPS) for the video.
    fps: u8,
    /// Bit rate for the video, in bps (bits per second).
    bitrate: u32,
    /// Exposure configuration.
    exposure: Option<Exposure>,
    /// Brightnes correction.
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
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Gets the configured video width for the camera, in pixels.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Gets the configured video framerate for the camera, in frames per second.
    pub fn fps(&self) -> u8 {
        self.fps
    }

    /// Gets the configured bitrate for videos.
    pub fn bitrate(&self) -> u32 {
        self.bitrate
    }

    /// Gets the configured exposure for videos.
    pub fn exposure(&self) -> Option<Exposure> {
        self.exposure
    }

    /// Gets the configured brightness for videos.
    pub fn brightness(&self) -> Option<u8> {
        self.brightness
    }

    /// Gets the configured contrast for videos.
    pub fn contrast(&self) -> Option<i8> {
        self.contrast
    }

    /// Gets the configured sharpness for videos.
    pub fn sharpness(&self) -> Option<i8> {
        self.sharpness
    }

    /// Gets the configured saturation for videos.
    pub fn saturation(&self) -> Option<i8> {
        self.saturation
    }

    /// Gets the configured ISO for videos.
    pub fn iso(&self) -> Option<u16> {
        self.iso
    }

    /// Gets if video stabilization needs to be turned on.
    pub fn stabilization(&self) -> bool {
        self.stabilization == Some(true)
    }

    /// Gets the configured EV compensation for videos.
    pub fn ev(&self) -> Option<i8> {
        self.ev
    }

    /// Gets the configured automatic white balance for videos.
    pub fn white_balance(&self) -> Option<WhiteBalance> {
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
    /// Quality of the picture, in px.
    quality: u8,
    /// Wether to add EXIF data to pictures or not.
    #[cfg(feature = "gps")]
    exif: Option<bool>,
    /// Wether to save the raw sensor data as JPG metadata.
    raw: Option<bool>,
    /// Exposure configuration.
    exposure: Option<Exposure>,
    /// Brightnes correction.
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
}

#[cfg(feature = "raspicam")]
impl Picture {
    /// Gets the configured picture height for the camera, in pixels.
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Gets the configured picture width for the camera, in pixels.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Gets the configured picture quality for the camera, in pixels.
    pub fn quality(&self) -> u8 {
        self.quality
    }

    /// Gets wether the camera should add available EXIF information to pictures.
    #[cfg(feature = "gps")]
    pub fn exif(&self) -> bool {
        self.exif == Some(true)
    }

    /// Gets wether the camera should add raw sensor data to pictures as JPEG metadata.
    pub fn raw(&self) -> bool {
        self.raw == Some(true)
    }

    /// Gets the configured exposure for pictures.
    pub fn exposure(&self) -> Option<Exposure> {
        self.exposure
    }

    /// Gets the configured brightness for pictures.
    pub fn brightness(&self) -> Option<u8> {
        self.brightness
    }

    /// Gets the configured contrast for pictures.
    pub fn contrast(&self) -> Option<i8> {
        self.contrast
    }

    /// Gets the configured sharpness for pictures.
    pub fn sharpness(&self) -> Option<i8> {
        self.sharpness
    }

    /// Gets the configured saturation for pictures.
    pub fn saturation(&self) -> Option<i8> {
        self.saturation
    }

    /// Gets the configured ISO for pictures.
    pub fn iso(&self) -> Option<u16> {
        self.iso
    }

    /// Gets the configured EV compensation for pictures.
    pub fn ev(&self) -> Option<i8> {
        self.ev
    }

    /// Gets the configured automatic white balance for pictures.
    pub fn white_balance(&self) -> Option<WhiteBalance> {
        self.white_balance
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
    /// Antishake mode.
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

/// Exposure setting.
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
    ///
    /// **Note:** it will only accept baud rates from 1 to `u32::MAX` (`usize` in Raspberry Pi is
    /// `u32`).
    #[serde(deserialize_with = "deserialize_baudrate")]
    baud_rate: BaudRate,
    /// Power GPIO pin.
    #[serde(deserialize_with = "deserialize_pin")]
    power_gpio: Pin,
}

#[cfg(feature = "gps")]
impl Gps {
    /// Gets the UART serial console path.
    pub fn uart(&self) -> &Path {
        &self.uart
    }

    /// Gets the serial console baud rate.
    pub fn baud_rate(&self) -> BaudRate {
        self.baud_rate
    }

    /// Gets the power GPIO pin.
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
    #[serde(deserialize_with = "deserialize_baudrate")]
    baud_rate: BaudRate,
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
    pub fn uart(&self) -> &Path {
        &self.uart
    }

    /// Gets the serial console baud rate.
    pub fn baud_rate(&self) -> BaudRate {
        self.baud_rate
    }

    /// Gets the power GPIO pin.
    pub fn power_gpio(&self) -> Pin {
        self.power_gpio
    }

    /// Gets the status GPIO pin.
    pub fn status_gpio(&self) -> Pin {
        self.status_gpio
    }

    /// Gets the phone number for SMSs.
    pub fn sms_phone(&self) -> &PhoneNumber {
        &self.sms_phone
    }
}

/// Phone number representation.
#[cfg(feature = "fona")]
#[derive(Debug)]
pub struct PhoneNumber(String);

#[cfg(feature = "fona")]
impl PhoneNumber {
    /// Gets the phone number as a string.
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
    #[serde(deserialize_with = "deserialize_baudrate")]
    baud_rate: BaudRate,
}

#[cfg(feature = "telemetry")]
impl Telemetry {
    /// Gets the UART serial console path.
    pub fn uart(&self) -> &Path {
        &self.uart
    }

    /// Gets the serial console baud rate.
    pub fn baud_rate(&self) -> BaudRate {
        self.baud_rate
    }
}

/// Deserializes a Tokio serial baud rate.
#[cfg(any(feature = "gps", feature = "telemetry", feature = "fona"))]
fn deserialize_baudrate<'de, D>(deserializer: D) -> Result<BaudRate, D::Error>
where
    D: Deserializer<'de>,
{
    /// Visitor for baud rate.
    struct TokioBaudRateVisitor;
    impl<'dev> Visitor<'dev> for TokioBaudRateVisitor {
        type Value = BaudRate;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            use std::u32;
            write!(formatter, "an integer between 1 and {}", u32::MAX)
        }

        fn visit_i64<E>(self, value: i64) -> Result<BaudRate, E>
        where
            E: de::Error,
        {
            use std::u32;

            if value > 0 && i64::from(u32::MAX) >= value {
                Ok(BaudRate::from(value as u32))
            } else {
                Err(E::custom(format!("baud rate out of range: {}", value)))
            }
        }

        // TODO: create more visitors, to make it future proof with other deserializers.
    }

    deserializer.deserialize_u32(TokioBaudRateVisitor)
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

        #[cfg_attr(feature = "cargo-clippy", allow(absurd_extreme_comparisons))]
        fn visit_i64<E>(self, value: i64) -> Result<Pin, E>
        where
            E: de::Error,
        {
            if value >= 2 && value <= 28 {
                Ok(Pin::new(value as u64))
            } else {
                Err(E::custom(format!("pin out of range: {}", value)))
            }
        }
    }

    deserializer.deserialize_u8(PinVisitor)
}

/// Flight information structure.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Flight {
    /// Expected length of the flight, in minutes.
    length: u32,
}

impl Flight {
    /// Gets the expected length for the flight.
    pub fn length(self) -> u32 {
        self.length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Loads the default configuration and checks it.
    #[test]
    fn load_config() {
        let config = Config::from_file("config.toml").unwrap();

        assert_eq!(config.debug(), true);
        #[cfg(feature = "raspicam")]
        {
            assert_eq!(config.picture().height(), 2464);
            assert_eq!(config.picture().width(), 3280);
            #[cfg(feature = "gps")]
            {
                assert_eq!(config.picture().exif(), true);
            }
            assert_eq!(config.video().height(), 1080);
            assert_eq!(config.video().width(), 1920);
            assert_eq!(config.video().fps(), 30);
        }

        #[cfg(feature = "gps")]
        {
            assert_eq!(config.gps().uart(), Path::new("/dev/ttyAMA0"));
            assert_eq!(config.gps().baud_rate(), BaudRate::Baud9600);
            assert_eq!(config.gps().power_gpio().get_pin(), 3)
        }
    }

    /// Tests an invalid configuration, and the error output.
    #[test]
    #[cfg(feature = "raspicam")]
    fn config_error() {
        #[cfg(feature = "gps")]
        let picture = Picture {
            height: 10_345,
            width: 5_246,
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
        };

        #[cfg(not(feature = "gps"))]
        let picture = Picture {
            height: 10_345,
            width: 5_246,
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
        };

        let video = Video {
            height: 12_546,
            width: 5_648,
            fps: 92,
            bitrate: 20000000,
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
            baud_rate: BaudRate::Baud9600,
            power_gpio: Pin::new(7),
            status_gpio: Pin::new(21),
            sms_phone: PhoneNumber(String::new()),
            location_service: "gprs-service.com".to_owned(),
        };

        #[cfg(feature = "telemetry")]
        let telemetry = Telemetry {
            uart: PathBuf::from("/dev/ttyUSB0"),
            baud_rate: BaudRate::BaudOther(230400),
        };

        #[cfg(feature = "gps")]
        let gps = Gps {
            uart: PathBuf::from("/dev/ttyAMA0"),
            baud_rate: BaudRate::Baud9600,
            power_gpio: Pin::new(3),
        };

        #[cfg(all(feature = "gps", feature = "fona", feature = "telemetry"))]
        let config = Config {
            debug: None,
            picture,
            video,
            camera_rotation: Some(180),
            gps,
            fona,
            telemetry,
            flight: Flight { length: 300 },
            data_dir: PathBuf::from("data"),
        };

        #[cfg(all(feature = "gps", feature = "fona", not(feature = "telemetry")))]
        let config = Config {
            debug: None,
            picture,
            video,
            camera_rotation: Some(180),
            gps,
            fona,
            flight: Flight { length: 300 },
            data_dir: PathBuf::from("data"),
        };

        #[cfg(all(feature = "gps", not(feature = "fona"), feature = "telemetry"))]
        let config = Config {
            debug: None,
            picture,
            video,
            camera_rotation: Some(180),
            gps,
            telemetry,
            flight: Flight { length: 300 },
            data_dir: PathBuf::from("data"),
        };

        #[cfg(all(feature = "gps", not(feature = "fona"), not(feature = "telemetry")))]
        let config = Config {
            debug: None,
            picture,
            video,
            camera_rotation: Some(180),
            gps,
            flight: Flight { length: 300 },
            data_dir: PathBuf::from("data"),
        };

        #[cfg(all(not(feature = "gps"), feature = "fona", feature = "telemetry"))]
        let config = Config {
            debug: None,
            picture,
            video,
            fona,
            telemetry,
            camera_rotation: Some(180),
            flight: Flight { length: 300 },
            data_dir: PathBuf::from("data"),
        };

        #[cfg(all(not(feature = "gps"), feature = "fona", not(feature = "telemetry")))]
        let config = Config {
            debug: None,
            picture,
            video,
            fona,
            camera_rotation: Some(180),
            flight: Flight { length: 300 },
            data_dir: PathBuf::from("data"),
        };

        #[cfg(all(not(feature = "gps"), not(feature = "fona"), feature = "telemetry"))]
        let config = Config {
            debug: None,
            picture,
            video,
            telemetry,
            camera_rotation: Some(180),
            flight: Flight { length: 300 },
            data_dir: PathBuf::from("data"),
        };

        #[cfg(all(not(feature = "gps"), not(feature = "fona"), not(feature = "telemetry")))]
        let config = Config {
            debug: None,
            picture,
            video,
            camera_rotation: Some(180),
            flight: Flight { length: 300 },
            data_dir: PathBuf::from("data"),
        };

        let (verify, errors) = config.verify();

        assert_eq!(verify, false);
        assert_eq!(
            errors,
            "picture width must be below or equal to 3280px, found 5246px\npicture height \
             must be below or equal to 2464px, found 10345px\nvideo width must be below or \
             equal to 2592px, found 5648px\nvideo height must be below or equal to 1944px, \
             found 12546px\nvideo framerate must be below or equal to 90fps, found 92fps\n\
             video mode must be one of 2592×1944 1-15fps, 1920×1080 1-30fps, 1296×972 \
             1-42fps, 1296×730 1-49fps, 640×480 1-60fps, found 5648x12546 92fps\n"
        );
    }

    /// Tests the default configuration and its loading using the static `CONFIG` constant.
    #[test]
    fn config_static() {
        assert!(CONFIG.debug());
    }
}
