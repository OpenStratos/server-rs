//! Camera module.

use std::{fmt, fs};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::process::Command;

use chrono::{DateTime, UTC};

use error::*;
use generate_error_string;
use config::CONFIG;
use gps::GPSStatus;

lazy_static! {
    /// Camera controller.
    pub static ref CAMERA: Camera = Camera {
        video_dir: CONFIG.data_dir().join("video"),
        img_dir: CONFIG.data_dir().join("img"),
    };
}

#[derive(Debug)]
pub struct Camera {
    video_dir: PathBuf,
    img_dir: PathBuf,
}

impl Camera {
    /// Starts recording video with the camera.
    ///
    /// The only parameter for this function is a duration parameter. If `Some(Duration)` is passed,
    /// the camera will stop recording after that time.
    ///
    /// **Panics** if the duration is less than 1 second.
    pub fn record(&self, time: Option<Duration>) -> Result<()> {
        if let Some(time) = time {
            info!("Recording video for {}.{} seconds.",
                  time.as_secs(),
                  time.subsec_nanos());
        } else {
            info!("Recording video indefinitely.");
        }
        if self.is_recording()? {
            error!("The camera is already recording.");
            return Err(ErrorKind::CameraAlreadyRecording.into());
        }
        // string filename = time > 0 ? "data/video/test.h264" : "data/video/video-"+
        //     to_string(get_file_count("data/video/")) +".h264";
        // #ifdef OS_TESTING
        //     filename = "data/video/test.h264";
        // #endif
        let file = self.video_dir.join(if cfg!(test) || time.is_some() {
                                           "test.h264".to_owned()
                                       } else {
                                           format!("video-{}.h264",
                                                   fs::read_dir(&self.video_dir)?.count())
                                       });
        if Path::new(&file).exists() {
            error!("Trying to write the video in {} but the file already exists",
                   file.display());
        }

        let mut command = Command::new("raspivid");
        command.arg("-n").arg("-o").arg(file);
        if let Some(time) = time {
            command.arg("-t").arg(format!("{}",
                                          time.as_secs() * 1_000 +
                                          time.subsec_nanos() as u64 / 1_000_000));
        }
        command.arg("-w")
            .arg(format!("{}", CONFIG.video_width()))
            .arg("-h")
            .arg(format!("{}", CONFIG.video_height()));
        unimplemented!()
    }

    /// Stops the video recording.
    ///
    /// It will return `Ok(_)` if the video stopped successfully. The boolean in the return type
    /// indicates if the camera was already recording previously.
    pub fn stop_recording(&self) -> Result<bool> {
        unimplemented!()
    }

    /// Checks if the camera is currently recording video.
    pub fn is_recording(&self) -> Result<bool> {
        unimplemented!()
    }

    /// Takes a picture with the camera.
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
                               generate_error_string(e, "Error stopping video recording"))
                    }
                }
            }
            Ok(false) => {}
            Err(e) => {
                error!("{}",
                       generate_error_string(e, "Error checking if camera was recording video"))
            }
        }
        info!("Shut down finished");
    }
}

/// Structure representing EXIF data for a picture.
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

impl ExifData {
    fn new(wait: bool) -> Self {
        if wait {
            unimplemented!();
        }

        unimplemented!();
    }
}

impl ToString for ExifData {
    fn to_string(&self) -> String {
        let mut exif = String::from(" -x GPSMeasureMode=3 -x GPS.GPSDifferential=0");

        if let Some((lat_ref, lat)) = self.gps_latitude {
            exif.push_str(&format!(" -x GPS.GPSLatitudeRef={} -x GPS.GPSLatitude={:.0}/1000000",
                                   lat_ref,
                                   lat * 1_000_000f32));
        }
        if let Some((lon_ref, lon)) = self.gps_longitude {
            exif.push_str(&format!(" -x GPS.GPSLongitudeRef={} -x GPS.GPSLongitude={:.0}/1000000",
                                   lon_ref,
                                   lon * 1_000_000f32));
        }
        if let Some(alt) = self.gps_altitude {
            // TODO configurable altitude ref.
            exif.push_str(&format!(" -x GPS.GPSAltitudeRef=0 -x GPS.GPSAltitude={:.0}/100",
                                   alt * 100f32));
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
            exif.push_str(&format!(" -x GPS.GPSDOP={:.0}/1000", dop * 1_000f32));
        }
        if let Some(speed) = self.gps_speed {
            // TODO configurable speed ref.
            exif.push_str(&format!(" -x GPS.GPSSpeedRef=N -x GPS.GPSSpeed={}/1000",
                                   speed * 1_000f32));
        }
        if let Some(track) = self.gps_track {
            // TODO configurable track ref.
            exif.push_str(&format!(" -x GPS.GPSTrackRef=T -x GPS.GPSTrack={}/1000",
                                   track * 1_000f32));
        }

        exif
    }
}

/// Latitude reference for EXIF data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LatitudeRef {
    North,
    South,
}

impl From<f32> for LatitudeRef {
    fn from(lat: f32) -> Self {
        if lat > 0f32 {
            LatitudeRef::North
        } else {
            LatitudeRef::South
        }
    }
}

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LongitudeRef {
    East,
    West,
}

impl From<f32> for LongitudeRef {
    fn from(lon: f32) -> Self {
        if lon > 0f32 {
            LongitudeRef::East
        } else {
            LongitudeRef::West
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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

    #[test]
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
}
