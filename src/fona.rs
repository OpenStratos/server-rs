//! Adafruit FONA GSM module.

#![allow(missing_debug_implementations)]

// TODO: timeouts

use std::{fmt, io::Write, sync::Mutex, thread, time::Duration};

use failure::{Error, Fail, ResultExt};
use tokio_serial::{Serial, SerialPortSettings};

use config::CONFIG;
use error;
use generate_error_string;

lazy_static! {
    /// The FONA module control structure.
    pub static ref FONA: Mutex<Fona> = Mutex::new(Fona { serial: None });
}

/// Minimum battery voltage for the FONA battery.
///
/// In a 1S LiPo battery, it should be 3.7 volts.
pub const BAT_FONA_MIN_V: f32 = 3.7;

/// Maximum battery voltage for the FONA battery.
///
/// In a 1S LiPo battery, it should be 4.2 volts.
pub const BAT_FONA_MAX_V: f32 = 4.2;

/// Adafruit FONA control structure.
pub struct Fona {
    serial: Option<Serial>,
}

impl fmt::Debug for Fona {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use tokio_serial::SerialPort;

        write!(
            f,
            "Fona {{ serial: {:?} }}",
            if let Some(ref serial) = &self.serial {
                serial.port_name()
            } else {
                None
            }
        )
    }
}

impl Fona {
    /// Initializes the Adafruit FONA module.
    pub fn initialize(&mut self) -> Result<(), Error> {
        if self.is_on()? {
            info!("FONA module is on, rebooting for stability.");
            self.turn_off()?;
            info!("Module is off, sleeping 3 seconds before turning it on…");
            thread::sleep(Duration::from_secs(3));
        }

        self.turn_on()?;
        if !self.is_on()? {
            error!("The module is still off. Finishing initialization.");
            bail!(error::Fona::PowerOn);
        }

        info!("Starting serial connection.");
        let mut settings = SerialPortSettings::default();
        settings.baud_rate = CONFIG.fona().baud_rate();

        let serial = Serial::from_path(CONFIG.fona().uart(), &settings)?;
        // serial.set_timeout(Duration::from_secs(5));
        self.serial = Some(serial);
        info!("Serial connection started.");

        info!("Checking OK initialization (3 times).");
        for _ in 0..2 {
            if self.send_command_read("AT")? != "OK" {
                info!("Not initialized.");
            }
            thread::sleep(Duration::from_millis(100));
        }

        if self.send_command_read("AT")? != "OK" {
            error!("Initialization error.");
            Err(error::Fona::Init.into())
        } else {
            thread::sleep(Duration::from_millis(100));
            info!("Initialization OK.");

            // Turn off echo.
            let _ = self.send_command_read("ATE0")?;
            thread::sleep(Duration::from_millis(100));

            if self.send_command_read("ATE0")? == "OK" {
                Ok(())
            } else {
                Err(error::Fona::EchoOff.into())
            }
        }
    }

    /// Checks if the FONA module is on.
    pub fn is_on(&self) -> Result<bool, Error> {
        Ok(CONFIG.fona().status_gpio().get_value()? == 1)
    }

    /// Turns on the FONA module.
    pub fn turn_on(&mut self) -> Result<(), Error> {
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
    pub fn turn_off(&mut self) -> Result<(), Error> {
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
    pub fn send_sms<M>(&mut self, message: M) -> Result<(), Error>
    where
        M: AsRef<str>,
    {
        let character_count = message.as_ref().chars().count();
        info!(
            "Sending SMS: `{}` ({} characters) to number {}",
            message.as_ref(),
            character_count,
            CONFIG.fona().sms_phone().as_str(),
        );

        #[cfg(not(feature = "no_sms"))]
        {
            if character_count > 160 {
                return Err(error::Fona::LongSms.into());
            }

            if self.send_command_read("AT+CMGF=1")? != "OK" {
                error!("Error sending SMS on `AT+CMGF=1` command.");
                return Err(error::Fona::SmsAtCmgf.into());
            }

            let cmgs_command = format!(r#"AT+CMGS="{}""#, CONFIG.fona().sms_phone().as_str());
            if self.send_command_read_limit(&cmgs_command, 2)? != "> " {
                error!("Error sending SMS on `{}` command.", cmgs_command);
                return Err(error::Fona::SmsAtCmgs.into());
            }

            if let Some(ref mut serial) = self.serial {
                debug!("Sending message…");

                // Write message
                serial
                    .write_all(message.as_ref().as_bytes())
                    .context(error::Fona::Command)?;

                debug!("Sent: `{}`", message.as_ref());

                // Write Ctrl+Z
                serial.write_all(&[0x1A]).context(error::Fona::Command)?;

                debug!("Sent Ctrl+Z");
            } else {
                error!(
                    "No serial when trying to send message `{}`",
                    message.as_ref()
                );
                return Err(error::Fona::NoSerial.into());
            }

            let new_line = self.read_line()?;
            if !new_line.is_empty() {
                warn!(
                    "There was some non-flushed output after sending the messsage: `{}`",
                    new_line
                );
            }

            let response = self.read_line()?;
            if !response.starts_with("+CMGS: ") {
                error!(
                    "Error reading +CMGS response to the message, read `{}`",
                    response
                );
                return Err(error::Fona::SmsCmgs.into());
            }

            let new_line = self.read_line()?;
            if !new_line.is_empty() {
                warn!(
                    "There was some non-flushed output after sending the messsage: `{}`",
                    new_line
                );
            }

            // TODO read the number of characters and check it.

            let ok = self.read_line()?;
            if ok != "OK" {
                error!("No OK received after sending SMS, received: `{}`", ok);
                return Err(error::Fona::SmsOk.into());
            }

            info!("SMS Sent.");
            Ok(())
        }

        #[cfg(feature = "no_sms")]
        {
            thread::sleep(Duration::from_secs(5));
            Ok(())
        }
    }

    /// Gets the current location using GPRS.
    pub fn location(&mut self) -> Result<Location, Error> {
        if self.send_command_read("AT+CMGF=1")? != "OK" {
            error!("Error getting location on `AT+CMGF=1` response.");
            return Err(error::Fona::LocAtCmgf.into());
        }

        if self.send_command_read("AT+CGATT=1")? != "OK" {
            error!("Error getting location on `AT+CGATT=1` response.");

            if self.send_command_read("AT+SAPBR=0,1")? != "OK" {
                error!("Error turning GPRS down.");

                return Err(error::Fona::LocAtGprsDown
                    .context(error::Fona::LocAtCgatt)
                    .into());
            } else {
                info!("GPRS off.");
                return Err(error::Fona::LocAtCgatt.into());
            }
        }

        if self.send_command_read(r#"AT+SAPBR=3,1,"CONTYPE","GPRS""#)? != "OK" {
            error!(r#"Error getting location on `AT+SAPBR=3,1,"CONTYPE","GPRS"` response."#);

            if self.send_command_read("AT+SAPBR=0,1")? != "OK" {
                error!("Error turning GPRS down.");

                return Err(error::Fona::LocAtGprsDown
                    .context(error::Fona::LocAtSapbrContype)
                    .into());
            } else {
                info!("GPRS off.");
                return Err(error::Fona::LocAtSapbrContype.into());
            }
        }

        let apn_message = format!(
            r#"AT+SAPBR=3,1,"APN","{}""#,
            CONFIG.fona().location_service()
        );
        if self.send_command_read(&apn_message)? != "OK" {
            error!("Error getting location on `{}` response.", apn_message);

            if self.send_command_read("AT+SAPBR=0,1")? != "OK" {
                error!("Error turning GPRS down.");

                return Err(error::Fona::LocAtGprsDown
                    .context(error::Fona::LocAtSapbrApn)
                    .into());
            } else {
                info!("GPRS off.");
                return Err(error::Fona::LocAtSapbrApn.into());
            }
        }

        if self.send_command_read("AT+SAPBR=1,1")? != "OK" {
            error!("Error getting location on `AT+SAPBR=1,1` response.");

            if self.send_command_read("AT+SAPBR=0,1")? != "OK" {
                error!("Error turning GPRS down.");

                return Err(error::Fona::LocAtGprsDown
                    .context(error::Fona::LocAtSapbr)
                    .into());
            } else {
                info!("GPRS off.");
                return Err(error::Fona::LocAtSapbr.into());
            }
        }

        let location_response = self.send_command_read("AT+CIPGSMLOC=1,1")?;
        let mut location_response_iter = location_response.split(',');
        // TODO: response could not be valid
        let longitude = location_response_iter
            .nth(1)
            .ok_or(error::Fona::LocLon)?
            .parse::<f32>()
            .context(error::Fona::LocLon)?;
        let latitude = location_response_iter
            .next()
            .ok_or(error::Fona::LocLat)?
            .parse::<f32>()
            .context(error::Fona::LocLat)?;

        if self.read_line()? != "OK" {
            error!("Error getting location on `AT+CIPGSMLOC=1,1` response.");

            if self.send_command_read("AT+SAPBR=0,1")? != "OK" {
                error!("Error turning GPRS down.");

                return Err(error::Fona::LocAtGprsDown
                    .context(error::Fona::LocAtCipgsmloc)
                    .into());
            } else {
                info!("GPRS off.");
                return Err(error::Fona::LocAtCipgsmloc.into());
            }
        }

        if self.send_command_read("AT+SAPBR=0,1")? != "OK" {
            error!("Error turning GPRS down.");

            return Err(error::Fona::LocAtGprsDown.into());
        } else {
            info!("GPRS off.");
        }

        Ok(Location {
            latitude,
            longitude,
        })
    }

    /// Checks the FONA battery level, in percentage.
    pub fn battery_percent(&mut self) -> Result<f32, Error> {
        let bat_voltage = self.battery_voltage()?;
        Ok((bat_voltage / 1000.0 - BAT_FONA_MIN_V) / (BAT_FONA_MAX_V - BAT_FONA_MIN_V))
    }

    /// Checks the FONA battery level, in voltage.
    pub fn battery_voltage(&mut self) -> Result<f32, Error> {
        let response = self.send_command_read("AT+CBC")?;
        let mut tokens = response.split(',');

        // TODO: check beginning of real response.
        if tokens.next() == Some("+CBC:") {
            match tokens.next() {
                Some(val) => {
                    Ok(val.parse::<f32>().context(error::Fona::CBCInvalidResponse)? / 1_000_f32)
                }
                None => Err(error::Fona::CBCInvalidResponse.into()),
            }
        } else {
            Err(error::Fona::CBCInvalidResponse.into())
        }
    }

    /// Checks the ADC (Analog-Digital converter) voltage of the FONA.
    pub fn adc_voltage(&mut self) -> Result<f32, Error> {
        let response = self.send_command_read("AT+CADC?")?;
        let mut tokens = response.split(',');

        if tokens.next() == Some("+CADC=1") {
            match tokens.next() {
                Some(val) => Ok(val
                    .parse::<f32>()
                    .context(error::Fona::CADCInvalidResponse)?
                    / 1_000_f32),
                None => Err(error::Fona::CADCInvalidResponse.into()),
            }
        } else {
            Err(error::Fona::CADCInvalidResponse.into())
        }
    }

    /// Checks if the FONA module has GSM connectivity.
    pub fn has_connectivity(&mut self) -> Result<bool, Error> {
        let response = self.send_command_read("AT+CREG?")?;
        Ok(response == "+CREG: 0,1" || response == "+CREG: 0,5")
    }

    /// Sends a command to the FONA module and reads the response.
    fn send_command_read<C>(&mut self, command: C) -> Result<String, Error>
    where
        C: AsRef<[u8]>,
    {
        self.send_command(command.as_ref())?;
        self.read_line()
    }

    /// Sends a command and reads a limited amount of characters.
    fn send_command_read_limit<C>(&mut self, command: C, count: usize) -> Result<String, Error>
    where
        C: AsRef<[u8]>,
    {
        use std::io::Read;

        self.send_command(command)?;

        if let Some(ref mut serial) = self.serial {
            let mut response = Vec::with_capacity(count);
            for res in serial.bytes() {
                response.push(res?);

                // We read enough bytes.
                if response.len() == count {
                    let res = String::from_utf8(response)?;
                    debug!(
                        "Received: `{}`",
                        res.replace('\r', "\\r").replace('\n', "\\n")
                    );
                    return Ok(res);
                }
            }

            Err(error::Fona::SerialEnd.into())
        } else {
            error!("No serial when trying to read response");
            Err(error::Fona::NoSerial.into())
        }
    }

    /// Sends a command to the FONA serial.
    fn send_command<C>(&mut self, command: C) -> Result<(), Error>
    where
        C: AsRef<[u8]>,
    {
        use std::borrow::Cow;

        if let Some(ref mut serial) = self.serial {
            debug!(
                "Sent command: `{}\\r\\n`", // TODO do we need the CRLF when sending Ctrl+Z?
                if command.as_ref() == [0x1A] {
                    Cow::from("Ctrl+Z")
                } else {
                    String::from_utf8_lossy(command.as_ref())
                }
            );

            serial
                .write_all(command.as_ref())
                .context(error::Fona::Command)?;

            // TODO do we need the CRLF when sending Ctrl+Z?
            serial.write_all(b"\r\n").context(error::Fona::Command)?;
        } else {
            error!(
                "No serial when trying to send command `{}`",
                String::from_utf8_lossy(command.as_ref())
            );
            return Err(error::Fona::NoSerial.into());
        }

        if !self
            .read_line()
            .context(error::Fona::SendCommandCrlf)?
            .is_empty()
        {
            Err(error::Fona::SendCommandCrlf.into())
        } else {
            Ok(())
        }
    }

    /// Reads a line from the serial.
    fn read_line(&mut self) -> Result<String, Error> {
        use std::io::{ErrorKind, Read};

        if let Some(ref mut serial) = self.serial {
            let mut response = Vec::new();
            for res in serial.bytes() {
                match res {
                    Ok(b'\r') => {}
                    Ok(b'\n') => {
                        let res = String::from_utf8(response)?;
                        debug!("Received: `{}\r\n`", res);
                        return Ok(res);
                    }
                    Ok(b) => {
                        response.push(b);
                    }
                    Err(e) => {
                        return Err(match e.kind() {
                            ErrorKind::TimedOut => {
                                let partial = String::from_utf8(response)?;
                                debug!("Received (partial): `{}`", partial);
                                error::Fona::PartialResponse { response: partial }.into()
                            }
                            _ => e.into(),
                        });
                    }
                }
            }

            Err(error::Fona::SerialEnd.into())
        } else {
            error!("No serial when trying to read response");
            Err(error::Fona::NoSerial.into())
        }
    }
}

impl Drop for Fona {
    fn drop(&mut self) {
        match self.is_on() {
            Ok(true) => {
                info!("Turning FONA off…");
                if let Err(e) = self.turn_off() {
                    error!("{}", generate_error_string(&e, "Error turning FONA off"));
                }
                info!("FONA off.");
            }
            Ok(false) => {}
            Err(e) => {
                error!(
                    "{}",
                    generate_error_string(
                        &e,
                        "Could not check if FONA was on when dropping the FONA object",
                    )
                );
            }
        }
    }
}

/// Structure representing the location of the probe as obtained by the FONA module.
#[derive(Debug, Clone, Copy)]
pub struct Location {
    /// Latitude of the location, in degrees (°).
    latitude: f32,
    /// Longitude of the location, in degrees (°).
    longitude: f32,
}

impl Location {
    /// Gets the latitude of the location, in degrees (°).
    pub fn latitude(self) -> f32 {
        self.latitude
    }

    /// Gets the longitude of the location, in degrees (°).
    pub fn longitude(self) -> f32 {
        self.longitude
    }
}

#[cfg(test)]
mod tests {
    use super::FONA;

    /// Tests FONA initialization.
    #[test]
    #[ignore]
    fn it_initialize() {
        FONA.lock().unwrap().initialize().unwrap();
    }

    #[test]
    #[ignore]
    fn it_send_sms() {
        FONA.lock().unwrap().initialize().unwrap();
        FONA.lock()
            .unwrap()
            .send_sms("OpenStratos test SMS")
            .unwrap();
    }
}
