//! GPS module.

#![allow(missing_debug_implementations)]

use std::fmt;
use std::str::FromStr;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use failure::Error;

use error;

lazy_static! {
    /// GPS data for concurrent check.
    pub static ref GPS: Mutex<Gps> = Mutex::new(Gps {
        fix_time: Utc::now(),
        status: FixStatus::Void,
        satellites: 0,
        latitude: 0_f32,
        longitude: 0_f32,
        altitude: 0_f32,
        pdop: 100_f32,
        hdop: 100_f32,
        vdop: 100_f32,
        speed: 0_f32,
        course: 0_f32,
    });
}

/// GPS information structure.
#[derive(Debug)]
pub struct Gps {
    /// Time of the current fix.
    fix_time: DateTime<Utc>,
    /// GPS fix status.
    status: FixStatus,
    /// Number of satellites connected.
    satellites: u8,
    /// Latitude of the GPS antenna, in *°* (degrees).
    latitude: f32,
    /// Longitude of the GPS antenna, in *°* (degrees).
    longitude: f32,
    /// Altitude of the GPS antenna from sea level, in *m*.
    altitude: f32,
    /// Position dilution of precision (3D).
    pdop: f32,
    /// Horizontal dilution of precision (2D).
    hdop: f32,
    /// Vertical dilution of precision (1D).
    vdop: f32,
    /// Speed of the velocity vector, in *m/s*.
    speed: f32,
    /// Course of the velocity vector, in *°* (degrees).
    course: f32,
}

impl Gps {
    /// Initializes the GPS.
    pub fn initialize(&mut self) -> Result<(), Error> {
        info!("Initializing GPS…");

        // TODO turn GPS on, start serial and so on.
        unimplemented!()
    }

    /// Gets the time of the current fix.
    pub fn fix_time(&self) -> DateTime<Utc> {
        self.fix_time
    }

    /// Gets the GPS fix status.
    pub fn status(&self) -> FixStatus {
        self.status
    }

    /// Gets the number of satellites connected.
    pub fn satellites(&self) -> u8 {
        self.satellites
    }

    /// Gets the latitude of the GPS antenna, in *°* (degrees).
    pub fn latitude(&self) -> f32 {
        self.latitude
    }

    /// Gets the longitude of the GPS antenna, in *°* (degrees).
    pub fn longitude(&self) -> f32 {
        self.longitude
    }

    /// Gets the altitude of the GPS antenna from sea level, in *m*.
    pub fn altitude(&self) -> f32 {
        self.altitude
    }

    /// Gets the position dilution of precision (3D).
    pub fn pdop(&self) -> f32 {
        self.pdop
    }

    /// Gets the horizontal dilution of precision (2D).
    pub fn hdop(&self) -> f32 {
        self.hdop
    }

    /// Gets the vertical dilution of precision (1D).
    pub fn vdop(&self) -> f32 {
        self.vdop
    }

    /// Gets the speed of the velocity vector, in *m/s*.
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Gets the course of the velocity vector, in *°* (degrees).
    pub fn course(&self) -> f32 {
        self.course
    }
}

impl Drop for Gps {
    fn drop(&mut self) {
        // TODO stop serial, turn GPS off.
    }
}

/// GPS fix status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixStatus {
    /// GPS fix active.
    Active,
    /// GPS fix not valid.
    Void,
}

impl FromStr for FixStatus {
    type Err = error::Gps;

    fn from_str(s: &str) -> Result<FixStatus, Self::Err> {
        match s {
            "A" => Ok(FixStatus::Active),
            "V" => Ok(FixStatus::Void),
            _ => Err(error::Gps::InvalidStatus {
                status: s.to_owned(),
            }),
        }
    }
}

impl fmt::Display for FixStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                FixStatus::Active => "A",
                FixStatus::Void => "V",
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Checks the GPS status from string conversion.
    #[test]
    fn gps_status_from_str() {
        assert_eq!("A".parse::<FixStatus>().unwrap(), FixStatus::Active);
        assert_eq!("V".parse::<FixStatus>().unwrap(), FixStatus::Void);

        // Check errors.
        assert!("".parse::<FixStatus>().is_err());
        assert!("sadfsa".parse::<FixStatus>().is_err());
        assert!("a".parse::<FixStatus>().is_err());
        assert!("Ab".parse::<FixStatus>().is_err());
    }

    /// Checks the GPS status to string conversion.
    #[test]
    fn gps_status_display() {
        assert_eq!(format!("{}", FixStatus::Active), "A");
        assert_eq!(format!("{}", FixStatus::Void), "V");
    }

    /// Checks the GPS initialization.
    #[test]
    #[ignore]
    fn gps_initialize() {
        let mut gps = GPS.lock().unwrap();
        gps.initialize().unwrap();
    }
}
