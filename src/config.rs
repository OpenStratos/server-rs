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

use std::{fmt, u8, i8, u16};
use std::result::Result as StdResult;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{Read, BufRead, BufReader};
use std::ffi::OsStr;

use toml;
use serde::de::{self, Deserialize, Deserializer, Visitor};

use error::*;
use CONFIG_FILE;
use print_system_failure;

lazy_static! {
    /// Configuration object.
    pub static ref CONFIG: Config = match Config::from_file(CONFIG_FILE) {
        Err(e) => {
            print_system_failure(&e, "Error loading configuration");
            panic!();
        },
        Ok(c) => c,
    };
}

/// Configuration object.
#[derive(Debug, Deserialize)]
pub struct Config {
    debug: Option<bool>,
    #[cfg(feature = "raspicam")]
    video: Video,
    #[cfg(feature = "raspicam")]
    picture: Picture,
    #[cfg(feature = "raspicam")]
    camera_rotation: Option<u16>,
    data_dir: PathBuf,
}

impl Config {
    /// Creates a new configuration object from a path.
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Config> {
        let file = File::open(path.as_ref())
            .chain_err(|| ErrorKind::ConfigOpen(path.as_ref().to_owned()))?;
        let mut reader = BufReader::new(file);
        let mut contents = String::new();

        reader
            .read_to_string(&mut contents)
            .chain_err(|| ErrorKind::ConfigRead(path.as_ref().to_owned()))?;

        let config: Config =
            toml::from_str(&contents)
                .chain_err(|| ErrorKind::ConfigInvalidToml(path.as_ref().to_owned()))?;

        if let (false, errors) = config.verify() {
            Err(ErrorKind::ConfigInvalid(errors).into())
        } else {
            Ok(config)
        }
    }

    /// Verify the correctness of the configuration, and return a list of errors if invalid.
    fn verify(&self) -> (bool, String) {
        let mut errors = String::new();
        let mut ok = true;

        #[cfg(feature = "raspicam")]
        {
            // Check for picture configuration errors.
            if self.picture.width > 3280 {
                ok = false;
                errors.push_str(&format!("picture width must be below or equal to 3280px, found \
                                          {}px\n",
                                         self.picture.width));
            }
            if self.picture.height > 2464 {
                ok = false;
                errors.push_str(
                    &format!("picture height must be below or equal to 2464px, found {}px\n",
                             self.picture.height));
            }

            if self.picture.quality > 100 {
                ok = false;
                errors.push_str(
                    &format!("picture quality must be a number between 0 and 100, found {}px\n",
                             self.picture.quality));
            }

            if let Some(b @ 101...u8::MAX) = self.picture.brightness {
                ok = false;
                errors.push_str(&format!("picture brightness must be between 0 and 100, found {}\n",
                                         b));
            }

            match self.picture.contrast {
                Some(c @ i8::MIN...-101) |
                Some(c @ 101...i8::MAX) => {
                    ok = false;
                    errors.push_str(&format!("picture contrast must be between -100 and 100, found \
                                              {}\n",
                                             c));
                }
                _ => {}
            }

            match self.picture.sharpness {
                Some(s @ i8::MIN...-101) |
                Some(s @ 101...i8::MAX) => {
                    ok = false;
                    errors.push_str(&format!("picture sharpness must be between -100 and 100, \
                                              found {}\n",
                                             s));
                }
                _ => {}
            }

            match self.picture.saturation {
                Some(s @ i8::MIN...-101) |
                Some(s @ 101...i8::MAX) => {
                    ok = false;
                    errors.push_str(&format!("picture saturation must be between -100 and 100, \
                                              found {}\n",
                                             s));
                }
                _ => {}
            }

            match self.picture.iso {
                Some(i @ 0...99) |
                Some(i @ 801...u16::MAX) => {
                    ok = false;
                    errors.push_str(&format!("picture ISO must be between 100 and 800, found {}\n",
                                             i));
                }
                _ => {}
            }

            match self.picture.ev {
                Some(e @ i8::MIN...-11) |
                Some(e @ 11...i8::MAX) => {
                    ok = false;
                    errors.push_str(&format!("picture EV compensation must be between -10 and 10, \
                                              found {}\n",
                                             e));
                }
                _ => {}
            }

            // Check for video configuration errors.
            if self.video.width > 2592 {
                ok = false;
                errors.push_str(&format!("video width must be below or equal to 2592px, found \
                                          {}px\n",
                                         self.video.width));
            }
            if self.video.height > 1944 {
                ok = false;
                errors.push_str(&format!("video height must be below or equal to 1944px, found \
                                          {}px\n",
                                         self.video.height));
            }
            if self.video.fps > 90 {
                ok = false;
                errors.push_str(
                    &format!("video framerate must be below or equal to 90fps, found {}fps\n",
                             self.video.fps));
            }

            // Video modes.
            match (self.video.width, self.video.height, self.video.fps) {
                (2592, 1944, 1...15) |
                (1920, 1080, 1...30) |
                (1296, 972, 1...42) |
                (1296, 730, 1...49) |
                (640, 480, 1...90) => {}
                (w, h, f) => {
                    ok = false;
                    errors.push_str(
                        &format!("video mode must be one of 2592×1944 1-15fps, 1920×1080 \
                                  1-30fps, 1296×972 1-42fps, 1296×730 1-49fps, 640×480 1-60fps, \
                                  found {}x{} {}fps\n",
                                 w, h, f));
                }
            }

            if let Some(r @ 360...u16::MAX) = self.camera_rotation {
                ok = false;
                errors.push_str(
                    &format!("camera rotation must be between 0 and 359 degrees, found {} \
                              degrees\n",
                             r));
            }

            if let Some(b @ 101...u8::MAX) = self.video.brightness {
                ok = false;
                errors.push_str(&format!("video brightness must be between 0 and 100, found {}\n",
                                         b));
            }

            match self.video.contrast {
                Some(c @ i8::MIN...-101) |
                Some(c @ 101...i8::MAX) => {
                    ok = false;
                    errors.push_str(&format!("video contrast must be between -100 and 100, found \
                                              {}\n",
                                             c));
                }
                _ => {}
            }

            match self.video.sharpness {
                Some(s @ i8::MIN...-101) |
                Some(s @ 101...i8::MAX) => {
                    ok = false;
                    errors.push_str(&format!("video sharpness must be between -100 and 100, found \
                                              {}\n",
                                             s));
                }
                _ => {}
            }

            match self.video.saturation {
                Some(s @ i8::MIN...-101) |
                Some(s @ 101...i8::MAX) => {
                    ok = false;
                    errors.push_str(&format!("video saturation must be between -100 and 100, found \
                                              {}\n",
                                             s));
                }
                _ => {}
            }

            match self.video.iso {
                Some(i @ 0...99) |
                Some(i @ 801...u16::MAX) => {
                    ok = false;
                    errors.push_str(&format!("video ISO must be between 100 and 800, found {}\n",
                                             i));
                }
                _ => {}
            }

            match self.video.ev {
                Some(e @ i8::MIN...-11) |
                Some(e @ 11...i8::MAX) => {
                    ok = false;
                    errors.push_str(&format!("video EV compensation must be between -10 and 10, \
                                              found {}\n",
                                             e));
                }
                _ => {}
            }
        }

        (ok, errors)
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

    /// Gets the configured data directory.
    pub fn data_dir(&self) -> &Path {
        self.data_dir.as_path()
    }
}

/// Video configuration structure.
#[cfg(feature = "raspicam")]
#[derive(Debug, Deserialize)]
pub struct Video {
    height: u16,
    width: u16,
    fps: u8,
    bitrate: u32,
    exposure: Option<Exposure>,
    brightness: Option<u8>,
    contrast: Option<i8>,
    sharpness: Option<i8>,
    saturation: Option<i8>,
    iso: Option<u16>,
    stabilization: Option<bool>,
    ev: Option<i8>,
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
#[derive(Debug, Deserialize)]
pub struct Picture {
    height: u16,
    width: u16,
    quality: u8,
    #[cfg(feature = "gps")]
    exif: Option<bool>,
    raw: Option<bool>,
    exposure: Option<Exposure>,
    brightness: Option<u8>,
    contrast: Option<i8>,
    sharpness: Option<i8>,
    saturation: Option<i8>,
    iso: Option<u16>,
    ev: Option<i8>,
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

#[cfg(test)]
mod tests {
    use super::*;

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
    }

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

        let config = Config {
            debug: None,
            picture,
            video: Video {
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
            },
            camera_rotation: Some(180),
            data_dir: PathBuf::from("data"),
        };
        let (verify, errors) = config.verify();

        assert_eq!(verify, false);
        assert_eq!(errors,
                   "picture width must be below or equal to 3280px, found 5246px\npicture height \
                    must be below or equal to 2464px, found 10345px\nvideo width must be below or \
                    equal to 2592px, found 5648px\nvideo height must be below or equal to 1944px, \
                    found 12546px\nvideo framerate must be below or equal to 90fps, found 92fps\n\
                    video mode must be one of 2592×1944 1-15fps, 1920×1080 1-30fps, 1296×972 \
                    1-42fps, 1296×730 1-49fps, 640×480 1-60fps, found 5648x12546 92fps\n");
    }

    #[test]
    fn config_static() {
        assert!(CONFIG.debug());
    }
}
