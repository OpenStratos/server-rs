//! Adafruit FONA GSM module.

use std::thread;
use std::sync::Mutex;
use std::time::Duration;

use sysfs_gpio::Pin;

use error::*;
use config::CONFIG;
use generate_error_string;

lazy_static! {
    /// The FONA module control structure.
    pub static ref FONA: Mutex<Fona> = Mutex::new(Fona {});
}

/// Adafruit FONA control structure.
pub struct Fona {
    // TODO
}

impl Fona {
    /// Initializes the Adafruit FONA module.
    pub fn initialize(&mut self) -> Result<()> {
        if self.is_on()? {
            info!("FONA module is on, rebooting for stability");
            self.turn_off()?;
            info!("Module is off, sleeping 3 seconds before turning it on…");
            thread::sleep(Duration::from_secs(3));
        }

        self.turn_on()?;
        if !self.is_on()? {
            error!("The module is still off. Finishing initialization.");
            return Err(Error::from(ErrorKind::FonaInitNoPowerOn));
        }

        unimplemented!()
    }

    /// Checks if the FONA module is on.
    pub fn is_on(&self) -> Result<bool> {
        Ok(CONFIG.fona().status_gpio().get_value()? == 1)
    }

    /// Turns on the FONA module.
    pub fn turn_on(&mut self) -> Result<()> {
        if self.is_on()? {
            warn!("Trying to turn FONA on but it was already on.");
            Ok(())
        } else {
            info!("Turning FONA on…");

            CONFIG.fona().power_gpio().set_value(0)?;
            thread::sleep(Duration::from_secs(2));
            CONFIG.fona().power_gpio().set_value(1)?;

            thread::sleep(Duration::from_secs(3));

            info!("FONA on.");

            Ok(())
        }
    }

    /// Tuns off the FONA module.
    pub fn turn_off(&mut self) -> Result<()> {
        if self.is_on()? {
            info!("Turning FONA off…");

            CONFIG.fona().power_gpio().set_value(0)?;
            thread::sleep(Duration::from_secs(2));
            CONFIG.fona().power_gpio().set_value(1)?;

            thread::sleep(Duration::from_secs(3));

            info!("FONA off.");

            Ok(())
        } else {
            warn!("Trying to turn FONA off but it was already off.");
            Ok(())
        }
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

impl Drop for Fona {
    fn drop(&mut self) {
        // TODO close serial.

        match self.is_on() {
            Ok(true) => {
                info!("Turning off FONA…");
                if let Err(e) = self.turn_off() {
                    error!("{}", generate_error_string(&e, "Error turning FONA off"));
                }
                info!("FONA off.");
            }
            Ok(false) => {}
            Err(e) => {
                error!("{}",
                       generate_error_string(&e,
                                             "Could not check if FONA was on when dropping the \
                                              FONA object"));
            }
        }
    }
}

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
