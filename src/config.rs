//! Configuration module.

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
            print_system_failure(e, "Error loading configuration");
            panic!();
        },
        Ok(c) => c,
    };
}

/// Configuration object.
#[derive(Debug, Deserialize)]
pub struct Config {
    debug: bool,
    video: Video,
    picture: Picture,
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

        // Check for picture configuration errors.
        if self.picture.width > 3280 {
            ok = false;
            errors.push_str(&format!("picture width must be below or equal to 3280px, found {}px\n",
                                     self.picture.width));
        }
        if self.picture.height > 2464 {
            ok = false;
            errors.push_str(
                &format!("picture height must be below or equal to 2464px, found {}px\n",
                         self.picture.height));
        }

        // Check for video configuration errors.
        if self.video.width > 2592 {
            ok = false;
            errors.push_str(&format!("video width must be below or equal to 2592px, found {}px\n",
                                     self.video.width));
        }
        if self.video.height > 1944 {
            ok = false;
            errors.push_str(&format!("video height must be below or equal to 1944px, found {}px\n",
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
                    &format!("video mode must be one of 2592×1944 1-15fps, 1920×1080 1-30fps, \
                              1296×972 1-42fps, 1296×730 1-49fps, 640×480 1-60fps, found {}x{} \
                              {}fps\n",
                             w, h, f));
            }
        }

        if let Some(r @ 360...u16::MAX) = self.camera_rotation {
            ok = false;
            errors.push_str(
                &format!("camera rotation must be between 0 and 359 degrees, found {} degrees\n",
                         r));
        }

        if let Some(b @ 101...u8::MAX) = self.video.brightness {
            ok = false;
            errors.push_str(&format!("video brightness must be between 0 and 100, found {}\n", b));
        }

        match self.video.contrast {
            Some(c @ i8::MIN...-101) |
            Some(c @ 101...i8::MAX) => {
                ok = false;
                errors.push_str(&format!("video contrast must be between -100 and 100, found {}\n",
                                         c));
            }
            _ => {}
        }

        match self.video.sharpness {
            Some(s @ i8::MIN...-101) |
            Some(s @ 101...i8::MAX) => {
                ok = false;
                errors.push_str(&format!("video sharpness must be between -100 and 100, found {}\n",
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
                errors.push_str(&format!("video ISO must be between 100 and 800, found {}\n", i));
            }
            _ => {}
        }

        match self.video.ev {
            Some(e @ i8::MIN...-11) |
            Some(e @ 11...i8::MAX) => {
                ok = false;
                errors.push_str(&format!("video EV compensation must be between -10 and 10, found \
                                          {}\n",
                                         e));
            }
            _ => {}
        }

        (ok, errors)
    }

    /// Gets wether OpenStratos should run in debug mode.
    pub fn debug(&self) -> bool {
        self.debug
    }

    /// Gets the configuration for video.
    pub fn video(&self) -> &Video {
        &self.video
    }

    /// Gets the configuration for pictures.
    pub fn picture(&self) -> &Picture {
        &self.picture
    }

    /// Gets the configured camera rotation.
    pub fn camera_rotation(&self) -> Option<u16> {
        self.camera_rotation
    }

    /// Gets the configured data directory.
    pub fn data_dir(&self) -> &Path {
        self.data_dir.as_path()
    }
}

/// Video configuration structure.
#[derive(Debug, Deserialize)]
pub struct Video {
    height: u16,
    width: u16,
    fps: u8,
    bitrate: u32,
    exposure: Exposure,
    brightness: Option<u8>,
    contrast: Option<i8>,
    sharpness: Option<i8>,
    saturation: Option<i8>,
    iso: Option<u16>,
    stabilization: Option<bool>,
    ev: Option<i8>,
}

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
    pub fn exposure(&self) -> Exposure {
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
}

/// Picture configuration structure.
#[derive(Debug, Deserialize)]
pub struct Picture {
    height: u16,
    width: u16,
    exif: Option<bool>,
}

impl Picture {
    /// Gets the configured picture height for the camera, in pixels.
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Gets the configured picture width for the camera, in pixels.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Gets wether the camera should add available EXIF information to pictures.
    pub fn exif(&self) -> bool {
        self.exif == Some(true)
    }
}

/// Exposure setting.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_config() {
        let config = Config::from_file("config.toml").unwrap();

        assert_eq!(config.debug(), true);
        assert_eq!(config.picture().height(), 2464);
        assert_eq!(config.picture().width(), 3280);
        assert_eq!(config.picture().exif(), true);
        assert_eq!(config.video().height(), 1080);
        assert_eq!(config.video().width(), 1920);
        assert_eq!(config.video().fps(), 30);
    }

    #[test]
    fn config_error() {
        let config = Config {
            debug: false,
            picture: Picture {
                height: 10_345,
                width: 5_246,
                exif: Some(true),
            },
            video: Video {
                height: 12_546,
                width: 5_648,
                fps: 92,
                bitrate: 20000000,
                exposure: Exposure::AntiShake,
                brightness: Some(50),
                contrast: Some(50),
                sharpness: None,
                saturation: None,
                iso: None,
                stabilization: Some(true),
                ev: None,
            },
            camera_rotation: Some(180),
            data_dir: PathBuf::from("data"),
        };
        let (verify, errors) = config.verify();

        assert_eq!(verify, false);
        assert_eq!(errors,
                   "picture width must be below or equal to 3280px, found 5246px\npicture \
                            height must be below or equal to 2464px, found 10345px\nvideo width \
                            must be below or equal to 2592px, found 5648px\nvideo height must be \
                            below or equal to 1944px, found 12546px\nvideo framerate must be below \
                            or equal to 90fps, found 92fps\nvideo mode must be one of 2592×1944 \
                            1-15fps, 1920×1080 1-30fps, 1296×972 1-42fps, 1296×730 1-49fps, \
                            640×480 1-60fps, found 5648x12546 92fps\n");
    }

    #[test]
    fn config_static() {
        assert!(CONFIG.debug());
    }
}
