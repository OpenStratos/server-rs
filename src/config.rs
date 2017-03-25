//! Configuration module.

use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{Read, BufRead, BufReader};

use toml;

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
    picture_height: u16,
    picture_width: u16,
    exif: bool,
    video_height: u16,
    video_width: u16,
    video_fps: u8,
    data_dir: PathBuf,
}

impl Config {
    /// Creates a new configuration object from a path.
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Config> {
        let file = File::open(path.as_ref()).chain_err(|| {
                ErrorKind::ConfigOpen(path.as_ref().to_owned())
            })?;
        let mut reader = BufReader::new(file);
        let mut contents = String::new();

        reader.read_to_string(&mut contents)
            .chain_err(|| ErrorKind::ConfigRead(path.as_ref().to_owned()))?;

        let config: Config = toml::from_str(&contents).chain_err(|| {
                ErrorKind::ConfigInvalidToml(path.as_ref().to_owned())
            })?;

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
        if self.picture_width > 3280 {
            ok = false;
            errors.push_str(&format!("picture width must be below or equal to 3280px, found {}px\n",
                                     self.picture_width));
        }
        if self.picture_height > 2464 {
            ok = false;
            errors.push_str(
                &format!("picture height must be below or equal to 2464px, found {}px\n",
                         self.picture_height));
        }

        // Check for video configuration errors.
        if self.video_width > 2592 {
            ok = false;
            errors.push_str(&format!("video width must be below or equal to 2592px, found {}px\n",
                                     self.video_width));
        }
        if self.video_height > 1944 {
            ok = false;
            errors.push_str(&format!("video height must be below or equal to 1944px, found {}px\n",
                                     self.video_height));
        }
        if self.video_fps > 90 {
            ok = false;
            errors.push_str(
                &format!("video framerate must be below or equal to 90fps, found {}fps\n",
                         self.video_fps));
        }

        // Video modes.
        match (self.video_width, self.video_height, self.video_fps) {
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

        (ok, errors)
    }

    /// Gets wether OpenStratos should run in debug mode.
    pub fn debug(&self) -> bool {
        self.debug
    }

    /// Gets the configured picture height for the camera, in pixels.
    pub fn picture_height(&self) -> u16 {
        self.picture_height
    }

    /// Gets the configured picture width for the camera, in pixels.
    pub fn picture_width(&self) -> u16 {
        self.picture_width
    }

    /// Gets wether the camera should add available EXIF information to pictures.
    pub fn exif(&self) -> bool {
        self.exif
    }

    /// Gets the configured video height for the camera, in pixels.
    pub fn video_height(&self) -> u16 {
        self.video_height
    }

    /// Gets the configured video width for the camera, in pixels.
    pub fn video_width(&self) -> u16 {
        self.video_width
    }

    /// Gets the configured video framerate for the camera, in frames per second.
    pub fn video_fps(&self) -> u8 {
        self.video_fps
    }

    /// Gets the configured data directory.
    pub fn data_dir(&self) -> &Path {
        self.data_dir.as_path()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_config() {
        let config = Config::from_file("config.toml").unwrap();

        assert_eq!(config.debug(), true);
        assert_eq!(config.picture_height(), 2464);
        assert_eq!(config.picture_width(), 3280);
        assert_eq!(config.exif(), true);
        assert_eq!(config.video_height(), 1080);
        assert_eq!(config.video_width(), 1920);
        assert_eq!(config.video_fps(), 30);
    }

    #[test]
    fn config_error() {
        let config = Config {
            debug: false,
            picture_height: 10_345,
            picture_width: 5_246,
            exif: true,
            video_height: 12_546,
            video_width: 5_648,
            video_fps: 92,
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
