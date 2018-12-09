//! Raspberry Pi camera module.

#![allow(missing_debug_implementations)]

use std::{
    fs, io, mem,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Mutex,
    thread,
    time::Duration,
};
// Only required for GPS
#[cfg(feature = "gps")]
use std::fmt;

use failure::{bail, Error};
use lazy_static::lazy_static;
use log::{debug, error, info, warn};

#[cfg(feature = "gps")]
use crate::gps::{FixStatus, GPS};
use crate::{config::CONFIG, error, generate_error_string};

/// Video directory inside data directory.
pub const VIDEO_DIR: &str = "video";
/// Image directory inside data directory.
pub const IMG_DIR: &str = "img";

lazy_static! {
    /// Shared static camera object.
    pub static ref CAMERA: Mutex<Camera> = Mutex::new(Camera {
        video_dir: CONFIG.data_dir().join(VIDEO_DIR),
        picture_dir: CONFIG.data_dir().join(IMG_DIR),
        process: None,
    });
}

/// Camera structure.
///
/// This structure controls the use of the camera.
#[derive(Debug)]
pub struct Camera {
    /// Directory to save video files.
    video_dir: PathBuf,
    /// Directory to save picture files.
    picture_dir: PathBuf,
    /// Video process handle.
    process: Option<Child>,
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
    /// as a test, the file will be removed after the recording, except if the `maintain_test_video`
    /// feature is used. If a file name is provided, a time should be provided too, and it will
    /// throw a warning if not.
    ///
    /// **Panics** if the duration is less than 1 second.
    pub fn record<T, P, FN>(&mut self, time: T, file_name: FN) -> Result<(), Error>
    where
        T: Into<Option<Duration>>,
        P: AsRef<Path>,
        FN: Into<Option<P>>,
    {
        let time = time.into();
        let file_name = file_name.into();

        if let Some(time) = time {
            info!(
                "Recording video for {}.{} seconds.",
                time.as_secs(),
                time.subsec_nanos()
            );
        } else {
            info!("Recording video indefinitely.");
            if file_name.is_some() {
                warn!("File name specified for testing purposes but trying to record indefinitely.")
            }
        }
        if self.is_recording() {
            error!("The camera is already recording.");
            bail!(error::Raspicam::AlreadyRecording);
        }
        let file = self.video_dir.join(if cfg!(test) {
            PathBuf::from("test.h264")
        } else if let Some(path) = file_name {
            path.as_ref().to_path_buf()
        } else {
            PathBuf::from(&format!(
                "video-{}.h264",
                fs::read_dir(&self.video_dir)?.count()
            ))
        });
        if file.exists() {
            error!(
                "Trying to write the video in {} but the file already exists.",
                file.display()
            );
            bail!(error::Raspicam::FileExists { file });
        }

        let mut command = Camera::generate_video_command(time, file);

        #[allow(clippy::use_debug)]
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
                warn!(
                    "Video recording ended with an error.\n\tstdout: {}\n\tstderr: {}",
                    stdout, stderr
                );
            }
        } else {
            let _ = command.stdin(Stdio::null());
            let _ = command.stdout(Stdio::null());
            let _ = command.stderr(Stdio::null());
            let child = command.spawn()?;
            info!("Video recording started with PID {}.", child.id());
            self.process = Some(child);
        }
        Ok(())
    }

    /// Generates the video command with the configured parameters.
    fn generate_video_command(time: Option<Duration>, file: PathBuf) -> Command {
        let mut command = Command::new("raspivid");
        let _ = command
            .arg("-n")
            .arg("-o")
            .arg(file)
            .arg("-w")
            .arg(format!("{}", CONFIG.video().width()))
            .arg("-h")
            .arg(format!("{}", CONFIG.video().height()))
            .arg("-fps")
            .arg(format!("{}", CONFIG.video().fps()))
            .arg("-b")
            .arg(format!("{}", CONFIG.video().bitrate()));
        if let Some(time) = time {
            let _ = command.arg("-t").arg(format!(
                "{}",
                time.as_secs() * 1_000 + u64::from(time.subsec_nanos()) / 1_000_000
            ));
        }
        if let Some(rot) = CONFIG.video().rotation() {
            let _ = command.arg("-rot").arg(format!("{}", rot));
        }
        if let Some(ex) = CONFIG.video().exposure() {
            let _ = command.arg("-ex").arg(ex);
        }
        if let Some(br) = CONFIG.video().brightness() {
            let _ = command.arg("-br").arg(format!("{}", br));
        }
        if let Some(co) = CONFIG.video().contrast() {
            let _ = command.arg("-co").arg(format!("{}", co));
        }
        if let Some(sh) = CONFIG.video().sharpness() {
            let _ = command.arg("-sh").arg(format!("{}", sh));
        }
        if let Some(sa) = CONFIG.video().saturation() {
            let _ = command.arg("-sa").arg(format!("{}", sa));
        }
        if let Some(iso) = CONFIG.video().iso() {
            let _ = command.arg("-ISO").arg(format!("{}", iso));
        }
        if CONFIG.video().stabilization() {
            let _ = command.arg("-vs");
        }
        if let Some(ev) = CONFIG.video().ev() {
            let _ = command.arg("-ev").arg(format!("{}", ev));
        }
        if let Some(awb) = CONFIG.video().white_balance() {
            let _ = command.arg("-awb").arg(awb);
        }

        command
    }

    /// Stops the video recording.
    pub fn stop_recording(&mut self) -> Result<(), io::Error> {
        info!("Stopping video recording…");
        if let Some(mut child) = mem::replace(&mut self.process, None) {
            match child.kill() {
                Ok(()) => {
                    info!("Video recording stopped correctly.");
                    thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    error!("Something had already stopped the video when trying to stop it.");
                    return Err(e);
                }
            }
        } else {
            warn!("There was no process to kill when trying to stop recording.");
            if Camera::is_really_recording()? {
                warn!(
                    "The raspivid process existed but it was not controlled by OpenStratos. \
                     Killing it…"
                );
                Camera::kill_process()?;
                info!("Forcefully killed the raspivid process");
            }
        }
        Ok(())
    }

    /// Checks if the camera is recording.
    pub fn is_recording(&self) -> bool {
        self.process.is_some()
    }

    /// Checks if there is a `raspivid` process currently recording video.
    fn is_really_recording() -> Result<bool, io::Error> {
        Ok(Command::new("pidof")
            .arg("-x")
            .arg("raspivid")
            .output()?
            .status
            .success())
    }

    /// Forcefully kills the `raspivid` process.
    fn kill_process() -> Result<(), io::Error> {
        match Command::new("pkill").arg("raspivid").output() {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Takes a picture with the camera.
    pub fn take_picture<P, FN>(&mut self, file_name: FN) -> Result<(), Error>
    where
        P: AsRef<Path>,
        FN: Into<Option<P>>,
    {
        let file_name = file_name.into();

        info!("Taking picture…");
        if self.is_recording() {
            warn!("The camera was recording video when trying to take the picture. Stopping…");
            self.stop_recording()?;
        }
        let file = self.picture_dir.join(if cfg!(test) {
            PathBuf::from("test.jpg")
        } else if let Some(path) = file_name {
            path.as_ref().to_path_buf()
        } else {
            PathBuf::from(&format!(
                "img-{}.jpg",
                fs::read_dir(&self.picture_dir)?.count()
            ))
        });
        if file.exists() {
            error!(
                "Trying to write the picture in {} but the file already exists.",
                file.display()
            );
            return Err(error::Raspicam::FileExists { file }.into());
        }

        let mut command = Camera::generate_picture_command(file);
        #[allow(clippy::use_debug)]
        {
            debug!("Picture command: {:?}", command);
        }
        info!("Taking picture…");

        let output = command.output()?;
        if output.status.success() {
            info!("Picture taken successfully.");
        } else {
            let stdout = String::from_utf8(output.stdout)?;
            let stderr = String::from_utf8(output.stderr)?;
            warn!(
                "Picture taking ended with an error.\n\tstdout: {}\n\tstderr: {}",
                stdout, stderr
            );
        }

        Ok(())
    }

    /// Generates the picture command with the configured parameters.
    fn generate_picture_command(file: PathBuf) -> Command {
        let mut command = Command::new("raspistill");
        let _ = command
            .arg("-n")
            .arg("-o")
            .arg(file)
            .arg("-t")
            .arg("0")
            .arg("-w")
            .arg(format!("{}", CONFIG.picture().width()))
            .arg("-h")
            .arg(format!("{}", CONFIG.picture().height()))
            .arg("-q")
            .arg(format!("{}", CONFIG.picture().quality()));
        if let Some(rot) = CONFIG.picture().rotation() {
            let _ = command.arg("-rot").arg(format!("{}", rot));
        }
        #[cfg(feature = "gps")]
        {
            if CONFIG.picture().exif() {
                let _ = command.arg("-x").arg(ExifData::new().to_string());
            }
        }
        if let Some(ex) = CONFIG.picture().exposure() {
            let _ = command.arg("-ex").arg(ex);
        }
        if let Some(br) = CONFIG.picture().brightness() {
            let _ = command.arg("-br").arg(format!("{}", br));
        }
        if let Some(co) = CONFIG.picture().contrast() {
            let _ = command.arg("-co").arg(format!("{}", co));
        }
        if let Some(sh) = CONFIG.picture().sharpness() {
            let _ = command.arg("-sh").arg(format!("{}", sh));
        }
        if let Some(sa) = CONFIG.picture().saturation() {
            let _ = command.arg("-sa").arg(format!("{}", sa));
        }
        if let Some(iso) = CONFIG.picture().iso() {
            let _ = command.arg("-ISO").arg(format!("{}", iso));
        }
        if let Some(ev) = CONFIG.picture().ev() {
            let _ = command.arg("-ev").arg(format!("{}", ev));
        }
        if let Some(awb) = CONFIG.picture().white_balance() {
            let _ = command.arg("-awb").arg(awb);
        }

        command
    }
}

impl Drop for Camera {
    fn drop(&mut self) {
        info!("Shutting down…");
        if self.is_recording() {
            info!("The camera is recording video, stopping…");
            match self.stop_recording() {
                Ok(()) => {
                    info!("Video recording stopped.");
                }
                Err(e) => error!(
                    "{}",
                    generate_error_string(&e.into(), "Error stopping video recording")
                ),
            }
        }
        info!("Shut down finished");
    }
}

/// Structure representing EXIF data for a picture.
#[cfg(feature = "gps")]
#[derive(Debug, Clone, Copy)]
pub struct ExifData {
    /// GPS latitude and reference.
    gps_latitude: (LatitudeRef, f32),
    /// GPS longitude and reference.
    gps_longitude: (LongitudeRef, f32),
    /// GPS altitude from sea level.
    gps_altitude: f32,
    // TODO gps_timestamp: DateTime<UTC>,
    /// Number of GPS satellites.
    gps_satellites: u8,
    /// GPS fix status.
    gps_status: FixStatus,
    /// GPS position dilution of precision.
    gps_dop: f32,
    /// GPS speed.
    gps_speed: f32,
    /// GPS course.
    gps_track: f32,
}

#[cfg(feature = "gps")]
impl ExifData {
    /// Creates new EXIF data from GPS.
    ///
    /// *In development…*
    fn new() -> Self {
        let gps = match GPS.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                error!("The GPS mutex was poisoned.");
                poisoned.into_inner()
            }
        };

        Self {
            gps_latitude: (LatitudeRef::from(gps.latitude()), gps.latitude()),
            gps_longitude: (LongitudeRef::from(gps.longitude()), gps.longitude()),
            gps_altitude: gps.altitude(),
            // TODO gps_timestamp: DateTime<UTC>,
            gps_satellites: gps.satellites(),
            gps_status: gps.status(),
            gps_dop: gps.pdop(),
            gps_speed: gps.speed(),
            gps_track: gps.course(),
        }
    }
}

#[cfg(feature = "gps")]
impl ToString for ExifData {
    fn to_string(&self) -> String {
        let mut exif = String::from(" -x GPSMeasureMode=3 -x GPS.GPSDifferential=0");

        let (lat_ref, lat) = self.gps_latitude;
        exif.push_str(&format!(
            " -x GPS.GPSLatitudeRef={} -x GPS.GPSLatitude={:.0}/1000000",
            lat_ref,
            lat * 1_000_000_f32
        ));

        let (lon_ref, lon) = self.gps_longitude;
        exif.push_str(&format!(
            " -x GPS.GPSLongitudeRef={} -x GPS.GPSLongitude={:.0}/1000000",
            lon_ref,
            lon * 1_000_000_f32
        ));

        // TODO configurable altitude ref.
        exif.push_str(&format!(
            " -x GPS.GPSAltitudeRef=0 -x GPS.GPSAltitude={:.0}/100",
            self.gps_altitude * 100_f32
        ));

        // TODO add GPS timestamp
        // exif.push_str(&format!(" -x GPS.GPSAltitudeRef=0 -x GPS.GPSAltitude={:.0}/100",
        //                        self.gps_timestamp * 100f32));

        exif.push_str(&format!(" -x GPS.GPSSatellites={}", self.gps_satellites));
        exif.push_str(&format!(" -x GPS.GPSStatus={}", self.gps_status));
        exif.push_str(&format!(
            " -x GPS.GPSDOP={:.0}/1000",
            self.gps_dop * 1_000_f32
        ));

        // TODO configurable speed ref.
        exif.push_str(&format!(
            " -x GPS.GPSSpeedRef=N -x GPS.GPSSpeed={}/1000",
            self.gps_speed * 1_000_f32
        ));

        // TODO configurable track ref.
        exif.push_str(&format!(
            " -x GPS.GPSTrackRef=T -x GPS.GPSTrack={}/1000",
            self.gps_track * 1_000_f32
        ));

        exif
    }
}

/// Latitude reference for EXIF data.
#[cfg(feature = "gps")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LatitudeRef {
    /// North reference.
    North,
    /// South reference.
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
        write!(
            f,
            "{}",
            match *self {
                LatitudeRef::North => "N",
                LatitudeRef::South => "S",
            }
        )
    }
}

/// Latitude reference for EXIF data.
#[cfg(feature = "gps")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LongitudeRef {
    /// East reference.
    East,
    /// West reference.
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
        write!(
            f,
            "{}",
            match *self {
                LongitudeRef::East => "E",
                LongitudeRef::West => "W",
            }
        )
    }
}

/// Tests module.
#[cfg(test)]
mod tests {
    use super::{ExifData, FixStatus, LatitudeRef, LongitudeRef, CAMERA};

    /// Tests EXIF generation.
    #[test]
    #[cfg(feature = "gps")]
    fn exif_data_complete() {
        let data = ExifData {
            gps_latitude: (LatitudeRef::North, 23.44497),
            gps_longitude: (LongitudeRef::East, 100.05792),
            gps_altitude: 1500.34,
            // gps_timestamp:
            //     DateTime::<UTC>::from_utc(NaiveDateTime::from_timestamp(1490443906, 0), UTC),
            gps_satellites: 7,
            gps_status: FixStatus::Active,
            gps_dop: 3.21,
            gps_speed: 13.5,
            gps_track: 1.65,
        };

        assert_eq!(
            data.to_string(),
            " -x GPSMeasureMode=3 -x GPS.GPSDifferential=0 -x GPS.GPSLatitudeRef=N -x \
             GPS.GPSLatitude=23444970/1000000 -x GPS.GPSLongitudeRef=E -x \
             GPS.GPSLongitude=100057920/1000000 -x GPS.GPSAltitudeRef=0 -x \
             GPS.GPSAltitude=150034/100 -x GPS.GPSSatellites=7 -x GPS.GPSStatus=A -x \
             GPS.GPSDOP=3210/1000 -x GPS.GPSSpeedRef=N -x GPS.GPSSpeed=13500/1000 -x \
             GPS.GPSTrackRef=T -x GPS.GPSTrack=1650/1000"
        );
    }

    /// Tests that the camera is not already recording.
    #[test]
    fn is_recording() {
        assert!(!CAMERA.lock().unwrap().is_recording());
    }
}
