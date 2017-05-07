//! GPS module.

use std::fmt;
use std::str::FromStr;

use error::*;

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
        write!(f,
               "{}",
               match *self {
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
