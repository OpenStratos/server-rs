//! GPS module.

use std::{fmt, thread};
use std::str::FromStr;
use std::sync::Mutex;
use std::time::Duration;

use chrono::{DateTime, UTC};
use sysfs_gpio::{Direction, Pin};

use error::*;
use config::CONFIG;

lazy_static! {
    /// GPS data for concurrent check.
    pub static ref GPS: Mutex<Gps> = Mutex::new(Gps {
        fix_time: UTC::now(),
        status: GPSStatus::Void,
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

impl Gps {
    /// Initializes the GPS.
    pub fn initialize(&mut self) -> Result<()> {
        CONFIG.gps().power_gpio().set_direction(Direction::Out)?;

        if self.is_on()? {
            info!("GPS is on, turning off for 2 seconds for stability.");
            self.turn_off()?;
            thread::sleep(Duration::from_secs(2));
        }

        info!("Turning GPS on…");
        self.turn_on()?;

        // TODO start serial and so on.
        unimplemented!()
    }

    /// Turns the GPS on.
    pub fn turn_on(&mut self) -> Result<()> {
        if self.is_on()? {
            warn!("Turning on GPS but GPS was already on.");
        } else {
            CONFIG.gps().power_gpio().set_value(1)?;
        }
        Ok(())
    }

    /// Turns the GPS off.
    pub fn turn_off(&mut self) -> Result<()> {
        if self.is_on()? {
            CONFIG.gps().power_gpio().set_value(0)?;
        } else {
            warn!("Turning off GPS but GPS was already off.");
        }
        Ok(())
    }

    /// Checks if the GPS is on.
    pub fn is_on(&self) -> Result<bool> {
        Ok(CONFIG.gps().power_gpio().get_value()? == 1)
    }

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

// TODO drop.

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
