//! Adafruit FONA GSM module.

use sysfs_gpio::Pin;

use error::*;
use config::CONFIG;

/// Adafruit FONA control structure.
pub struct Fona {
    // TODO
}

impl Fona {
    /// Initializes the Adafruit FONA module.
    pub fn initialize(&mut self) -> Result<()> {
        unimplemented!()
    }

    /// Checks if the FONA module is on.
    pub fn is_on(&self) -> Result<bool> {
        Ok(CONFIG.fona().status_gpio().get_value()? == 1)
    }

    /// Turns on the FONA module.
    pub fn turn_on(&mut self) -> Result<()> {
        unimplemented!()
    }

    /// Tuns off the FONA module.
    pub fn turn_off(&mut self) -> Result<()> {
        unimplemented!()
    }

    /// Sends an SMS with the given text to the given phone number.
    pub fn send_sms<M>(&mut self, message: M) -> Result<()>
        where M: AsRef<str>
    {
        unimplemented!();
    }

    /// Gets the current location using GPRS.
    pub fn get_location(&mut self) -> Result<Location> {
        unimplemented!()
    }

    /// Checks the FONA battery level, in percentage.
    pub fn battery_percent(&mut self) -> Result<f32> {
        unimplemented!()
    }

    /// Checks the FONA battery level, in voltage.
    pub fn battery_voltage(&mut self) -> Result<f32> {
        unimplemented!()
    }

    /// Checks the ADC (Analog-Digital converter) voltage of the FONA.
    pub fn adc_voltage(&mut self) -> Result<f32> {
        unimplemented!()
    }

    /// Checks if the FONA module has GSM connectivity.
    pub fn has_connectivity(&mut self) -> Result<bool> {
        unimplemented!()
    }

    /// Sends a command to the FONA module and reads the response.
    fn send_command_read<C>(&mut self, command: C) -> Result<String>
        where C: AsRef<str>
    {
        unimplemented!()
    }
}

// TODO drop.

/// Structure representing the location of the probe as obtained by the FONA module.
pub struct Location {
    /// Latitude of the location, in degrees (°).
    latitude: f32,
    /// Longitude of the location, in degrees (°).
    longitude: f32,
}

impl Location {
    /// Gets the latitude of the location, in degrees (°).
    pub fn latitude(&self) -> f32 {
        self.latitude
    }

    /// Gets the longitude of the location, in degrees (°).
    pub fn longitude(&self) -> f32 {
        self.longitude
    }
}
