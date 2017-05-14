//! GPS module.
//!
//! *In development…*

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, UTC};

use error::*;

/// GPS information structure.
#[derive(Debug)]
struct GPS {
    /// Time of the current fix.
    fix_time: DateTime<UTC>,
    /// GPS fix status.
    status: GPSStatus,
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

impl GPS {
    /// Gets the time of the current fix.
    pub fn fix_time(&self) -> DateTime<UTC> {
        self.fix_time
    }

    /// Gets the GPS fix status.
    pub fn status(&self) -> GPSStatus {
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

/// GPS fix status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GPSStatus {
    /// GPS fix active.
    Active,
    /// GPS fix not valid.
    Void,
}

impl FromStr for GPSStatus {
    type Err = Error;
    fn from_str(s: &str) -> Result<GPSStatus> {
        match s {
            "A" => Ok(GPSStatus::Active),
            "V" => Ok(GPSStatus::Void),
            _ => Err(ErrorKind::GPSInvalidStatus(s.to_owned()).into()),
        }
    }
}

impl fmt::Display for GPSStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            GPSStatus::Active => "A",
            GPSStatus::Void => "V",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gps_status_from_str() {
        assert_eq!("A".parse::<GPSStatus>().unwrap(), GPSStatus::Active);
        assert_eq!("V".parse::<GPSStatus>().unwrap(), GPSStatus::Void);

        // Check errors.
        assert!("".parse::<GPSStatus>().is_err());
        assert!("sadfsa".parse::<GPSStatus>().is_err());
        assert!("a".parse::<GPSStatus>().is_err());
        assert!("Ab".parse::<GPSStatus>().is_err());
    }

    #[test]
    fn gps_status_display() {
        assert_eq!(format!("{}", GPSStatus::Active), "A");
        assert_eq!(format!("{}", GPSStatus::Void), "V");
    }
}
