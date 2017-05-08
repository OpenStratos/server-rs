//! Raspberry Pi camera module.

use std::{fmt, fs};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::process::{Command, Stdio};

use chrono::{DateTime, UTC};

use error::*;
use generate_error_string;
use config::CONFIG;
#[cfg(feature = "gps")]
use gps::GPSStatus;

lazy_static! {
    /// Shared static camera object.
    pub static ref CAMERA: Camera = Camera {
        video_dir: CONFIG.data_dir().join("video"),
        img_dir: CONFIG.data_dir().join("img"),
    };
}

/// Camera structure.
///
/// This structure controlls the use of the camera.
#[derive(Debug)]
pub struct Camera {
    video_dir: PathBuf,
    img_dir: PathBuf,
}

impl Camera {
    /// Starts recording video with the camera.
    ///
    /// If `Some(Duration)` is passed, the camera will stop recording after that time. This should
    /// be the default behaviour, and it will block for the given time. If no duration is given, it
    /// will record indefinitely, and the thread won't block. To stop the camera recording anytime,
    /// `Camera::stop_recording()` can be used.
    ///
    /// An optional second parameter can be provided to specify a file name for the video recording,
    /// useful in case of testing. If that file name is provided, or if the method is executed
    /// as a test, the file will be removed after the recording, except if the `mantain_test_video`
    /// feature is used. If a file name is provided, a time should be provided too, and it will
    /// throw a warning if not.
    ///
    /// **Panics** if the duration is less than 1 second.
    pub fn record<P: AsRef<Path>>(&self,
                                  time: Option<Duration>,
                                  file_name: Option<P>)
                                  -> Result<()> {
        if let Some(time) = time {
            info!("Recording video for {}.{} seconds.",
                  time.as_secs(),
                  time.subsec_nanos());
        } else {
            info!("Recording video indefinitely.");
            if file_name.is_some() {
                warn!("File name specified for testing purposes but trying to record indefinitely.")
            }
        }
        if self.is_recording()? {
            error!("The camera is already recording.");
            return Err(ErrorKind::CameraAlreadyRecording.into());
        }
        let file = self.video_dir
            .join(if cfg!(test) {
                      PathBuf::from("test.h264")
                  } else if let Some(path) = file_name {
                path.as_ref().to_path_buf()
            } else {
                PathBuf::from(&format!("video-{}.h264", fs::read_dir(&self.video_dir)?.count()))
            });
        if file.exists() {
            error!("Trying to write the video in {} but the file already exists.",
                   file.display());
            return Err(ErrorKind::CameraFileExists(file).into());
        }

        let mut command = Camera::generate_video_command(time, file);

        #[allow(use_debug)]
        {
            debug!("Recording command: {:?}", command);
        }
        info!("Starting video recording…");

        if time.is_some() {
            let output = command.output()?;
            if output.status.success() {
                info!("Video recording finished successfully.");
            } else {
                let stdout = String::from_utf8(output.stdout)?;
                let stderr = String::from_utf8(output.stderr)?;
                warn!("Video recording ended with an error.\nstdout: {}, stderr: {}",
                      stdout,
                      stderr);
            }
        } else {
            command.stdin(Stdio::null());
            command.stdout(Stdio::null());
            command.stderr(Stdio::null());
            let child = command.spawn()?;
            info!("Video recording started with PID {}.", child.id());
        }
        Ok(())
    }

    /// Generates the video command with the configured parameters.
    fn generate_video_command(time: Option<Duration>, file: PathBuf) -> Command {
        let mut command = Command::new("raspivid");
        command.arg("-n").arg("-o").arg(file);
        if let Some(time) = time {
            command
                .arg("-t")
                .arg(format!("{}",
                             time.as_secs() * 1_000 + time.subsec_nanos() as u64 / 1_000_000));
        }
        command
            .arg("-w")
            .arg(format!("{}", CONFIG.video().width()))
            .arg("-h")
            .arg(format!("{}", CONFIG.video().height()))
            .arg("-fps")
            .arg(format!("{}", CONFIG.video().fps()))
            .arg("-b")
            .arg(format!("{}", CONFIG.video().bitrate()));
        if let Some(rot) = CONFIG.camera_rotation() {
            command.arg("-rot").arg(format!("{}", rot));
        }
        if let Some(ex) = CONFIG.video().exposure() {
            command.arg("-ex").arg(ex);
        }
        if let Some(br) = CONFIG.video().brightness() {
            command.arg("-br").arg(format!("{}", br));
        }
        if let Some(co) = CONFIG.video().contrast() {
            command.arg("-co").arg(format!("{}", co));
        }
        if let Some(sh) = CONFIG.video().sharpness() {
            command.arg("-sh").arg(format!("{}", sh));
        }
        if let Some(sa) = CONFIG.video().saturation() {
            command.arg("-sa").arg(format!("{}", sa));
        }
        if let Some(iso) = CONFIG.video().iso() {
            command.arg("-ISO").arg(format!("{}", iso));
        }
        if CONFIG.video().stabilization() {
            command.arg("-vs");
        }
        if let Some(ev) = CONFIG.video().ev() {
            command.arg("-ev").arg(format!("{}", ev));
        }
        if let Some(awb) = CONFIG.video().white_balance() {
            command.arg("-awb").arg(awb);
        }

        command
    }

    /// Stops the video recording.
    ///
    /// It will return `Ok(_)` if the video stopped successfully. The boolean in the return type
    /// indicates if the camera was already recording previously.
    ///
    /// *In development…*
    pub fn stop_recording(&self) -> Result<bool> {
        unimplemented!()
    }

    /// Checks if the camera is currently recording video.
    pub fn is_recording(&self) -> Result<bool> {
        Ok(Command::new("pidof")
               .arg("-x")
               .arg("raspivid")
               .output()?
               .status
               .success())
    }

    /// Takes a picture with the camera.
    ///
    /// *In development…*
    pub fn take_picture(&self) -> Result<()> {
        unimplemented!()
    }
}

impl Drop for Camera {
    fn drop(&mut self) {
        info!("Shutting down…");
        match self.is_recording() {
            Ok(true) => {
                info!("The camera is recording video, stopping…");
                match self.stop_recording() {
                    Ok(p) => {
                        info!("Video recording stopped.");
                        if p {
                            warn!("Video recording had already stopped.");
                        }
                    }
                    Err(e) => {
                        error!("{}",
                               generate_error_string(&e, "Error stopping video recording"))
                    }
                }
            }
            Ok(false) => {}
            Err(e) => {
                error!("{}",
                       generate_error_string(&e, "Error checking if camera was recording video"))
            }
        }
        info!("Shut down finished");
    }
}

/// Structure representing EXIF data for a picture.
#[cfg(feature = "gps")]
#[derive(Debug)]
pub struct ExifData {
    gps_latitude: Option<(LatitudeRef, f32)>,
    gps_longitude: Option<(LongitudeRef, f32)>,
    gps_altitude: Option<f32>,
    // TODO gps_timestamp: Option<DateTime<UTC>>,
    gps_satellites: Option<u8>,
    gps_status: Option<GPSStatus>,
    gps_dop: Option<f32>,
    gps_speed: Option<f32>,
    gps_track: Option<f32>,
}

#[cfg(feature = "gps")]
impl ExifData {
    /// *In development…*
    fn new(wait: bool) -> Self {
        if wait {
            unimplemented!();
        }

        unimplemented!();
    }
}

#[cfg(feature = "gps")]
impl ToString for ExifData {
    fn to_string(&self) -> String {
        let mut exif = String::from(" -x GPSMeasureMode=3 -x GPS.GPSDifferential=0");

        if let Some((lat_ref, lat)) = self.gps_latitude {
            exif.push_str(&format!(" -x GPS.GPSLatitudeRef={} -x GPS.GPSLatitude={:.0}/1000000",
                                  lat_ref,
                                  lat * 1_000_000_f32));
        }
        if let Some((lon_ref, lon)) = self.gps_longitude {
            exif.push_str(&format!(" -x GPS.GPSLongitudeRef={} -x GPS.GPSLongitude={:.0}/1000000",
                                  lon_ref,
                                  lon * 1_000_000_f32));
        }
        if let Some(alt) = self.gps_altitude {
            // TODO configurable altitude ref.
            exif.push_str(&format!(" -x GPS.GPSAltitudeRef=0 -x GPS.GPSAltitude={:.0}/100",
                                  alt * 100_f32));
        }
        // TODO add GPS timestamp
        // if let Some(timestamp) = self.gps_timestamp {
        //     exif.push_str(&format!(" -x GPS.GPSAltitudeRef=0 -x GPS.GPSAltitude={:.0}/100",
        //                            alt * 100f32))
        // }
        if let Some(sat) = self.gps_satellites {
            exif.push_str(&format!(" -x GPS.GPSSatellites={}", sat));
        }
        if let Some(status) = self.gps_status {
            exif.push_str(&format!(" -x GPS.GPSStatuss={}", status));
        }
        if let Some(dop) = self.gps_dop {
            exif.push_str(&format!(" -x GPS.GPSDOP={:.0}/1000", dop * 1_000_f32));
        }
        if let Some(speed) = self.gps_speed {
            // TODO configurable speed ref.
            exif.push_str(&format!(" -x GPS.GPSSpeedRef=N -x GPS.GPSSpeed={}/1000",
                                  speed * 1_000_f32));
        }
        if let Some(track) = self.gps_track {
            // TODO configurable track ref.
            exif.push_str(&format!(" -x GPS.GPSTrackRef=T -x GPS.GPSTrack={}/1000",
                                  track * 1_000_f32));
        }

        exif
    }
}

/// Latitude reference for EXIF data.
#[cfg(feature = "gps")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LatitudeRef {
    North,
    South,
}

#[cfg(feature = "gps")]
impl From<f32> for LatitudeRef {
    fn from(lat: f32) -> Self {
        if lat > 0_f32 {
            LatitudeRef::North
        } else {
            LatitudeRef::South
        }
    }
}

#[cfg(feature = "gps")]
impl fmt::Display for LatitudeRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "{}",
               match *self {
                   LatitudeRef::North => "N",
                   LatitudeRef::South => "S",
               })
    }
}

/// Latitude reference for EXIF data.
#[cfg(feature = "gps")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LongitudeRef {
    East,
    West,
}

#[cfg(feature = "gps")]
impl From<f32> for LongitudeRef {
    fn from(lon: f32) -> Self {
        if lon > 0_f32 {
            LongitudeRef::East
        } else {
            LongitudeRef::West
        }
    }
}

#[cfg(feature = "gps")]
impl fmt::Display for LongitudeRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "{}",
               match *self {
                   LongitudeRef::East => "E",
                   LongitudeRef::West => "W",
               })
    }
}

/// Tests module.
#[cfg(test)]
mod tests {
    use super::*;

    /// Tests EXIF generation for a complete data structure.
    #[test]
    #[cfg(feature = "gps")]
    fn exif_data_complete() {
        let data = ExifData {
            gps_latitude: Some((LatitudeRef::North, 23.44497)),
            gps_longitude: Some((LongitudeRef::East, 100.05792)),
            gps_altitude: Some(1500.34),
            // gps_timestamp:
            //     DateTime::<UTC>::from_utc(NaiveDateTime::from_timestamp(1490443906, 0), UTC),
            gps_satellites: Some(7),
            gps_status: Some(GPSStatus::Active),
            gps_dop: Some(3.21),
            gps_speed: Some(13.5),
            gps_track: Some(1.65),
        };

        assert_eq!(data.to_string(),
                   " -x GPSMeasureMode=3 -x GPS.GPSDifferential=0 -x GPS.GPSLatitudeRef=N -x \
                    GPS.GPSLatitude=23444970/1000000 -x GPS.GPSLongitudeRef=E -x \
                    GPS.GPSLongitude=100057920/1000000 -x GPS.GPSAltitudeRef=0 -x \
                    GPS.GPSAltitude=150034/100 -x GPS.GPSSatellites=7 -x GPS.GPSStatuss=A -x \
                    GPS.GPSDOP=3210/1000 -x GPS.GPSSpeedRef=N -x GPS.GPSSpeed=13500/1000 -x \
                    GPS.GPSTrackRef=T -x GPS.GPSTrack=1650/1000");
    }

    /// Tests EXIF generation for an incomplete data structure.
    #[test]
    #[cfg(feature = "gps")]
    fn exif_data_incomplete() {
        let data = ExifData {
            gps_latitude: None,
            gps_longitude: Some((LongitudeRef::West, 100.05792)),
            gps_altitude: Some(1500.34),
            // gps_timestamp: None,
            gps_satellites: Some(7),
            gps_status: Some(GPSStatus::Void),
            gps_dop: Some(3.21),
            gps_speed: Some(13.5),
            gps_track: None,
        };

        assert_eq!(data.to_string(),
                   " -x GPSMeasureMode=3 -x GPS.GPSDifferential=0 -x \
                    GPS.GPSLongitudeRef=W -x GPS.GPSLongitude=100057920/1000000 -x \
                    GPS.GPSAltitudeRef=0 -x GPS.GPSAltitude=150034/100 -x GPS.GPSSatellites=7 -x \
                    GPS.GPSStatuss=V -x GPS.GPSDOP=3210/1000 -x GPS.GPSSpeedRef=N -x \
                    GPS.GPSSpeed=13500/1000");
    }

    /// Tests that the camera is not already recording.
    #[test]
    fn is_recording() {
        assert!(!CAMERA.is_recording().unwrap());
    }
}
