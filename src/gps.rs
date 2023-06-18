//! GPS module.

#![allow(missing_debug_implementations)]

use crate::{config::CONFIG, error};
use anyhow::{Context, Error};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use std::{
    fmt,
    io::{self, Read, Write},
    str::FromStr,
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};
use sysfs_gpio::Direction;
use tracing::{info, warn};

/// GPS data for concurrent check.
pub static GPS: Lazy<Mutex<Gps>> = Lazy::new(|| Mutex::new(Gps::default()));

/// GPS information structure.
#[derive(Debug, Default)]
pub struct Gps {
    latest_data: Option<Frame>,
}

impl Gps {
    /// Initializes the GPS.
    pub fn initialize(&mut self) -> Result<(), Error> {
        info!("Initializing GPS\u{2026}");
        CONFIG
            .gps()
            .power_gpio()
            .set_direction(Direction::Out)
            .context(error::Gps::Init)?;

        if self.is_on().context(error::Gps::Init)? {
            info!("GPS is on, turning off for 2 seconds for stability");
            self.turn_off().context(error::Gps::Init)?;
            thread::sleep(Duration::from_secs(2))
        }

        info!("Turning GPS on\u{2026}");
        self.turn_on().context(error::Gps::Init)?;
        info!("GPS on.");

        info!("Starting serial connection\u{2026}");
        let mut serial = tokio_serial::new(
            CONFIG.gps().uart().to_string_lossy(),
            CONFIG.gps().baud_rate(),
        )
        .open()
        .context(error::Gps::Init)?;
        // serial.set_exclusive(false).context(error::Gps::Init)?;
        info!("Serial connection started.");

        info!("Sending configuration frames\u{2026}");
        let messages = [
            // Set refresh
            vec![
                0xB5, 0x62, 0x06, 0x08, 0x06, 0x00, 0x64, 0x00, 0x01, 0x00, 0x01, 0x00, 0x7A, 0x12,
            ],
            // Disable GSV:
            vec![0xB5, 0x62, 0x05, 0x01, 0x02, 0x00, 0x06, 0x01, 0x0F, 0x38],
            // Disable VTG:
            vec![
                0xB5, 0x62, 0x06, 0x01, 0x03, 0x00, 0xF0, 0x01, 0x00, 0xFB, 0x11,
            ],
            // Disable GLL:
            vec![
                0xB5, 0x62, 0x06, 0x01, 0x03, 0x00, 0xF0, 0x03, 0x00, 0xFD, 0x15,
            ],
            // Disable ZDA:
            vec![
                0xB5, 0x62, 0x06, 0x01, 0x03, 0x00, 0xF0, 0x05, 0x00, 0xFF, 0x19,
            ],
        ];

        for message in &messages {
            for _ in 0..100 {
                serial.write_all(message).context(error::Gps::Init)?;
                thread::sleep(Duration::from_millis(10));
            }
        }
        info!("Configuration frames sent");

        info!("Setting GPS to airborne (<1g) mode");
        if Gps::enter_airborne_1g_mode(&mut serial).is_ok() {
            info!("GPS entered airborne (<1g) mode successfully");
        } else {
            warn!("GPS failed to enter airborne (<1g) mode");
        }

        // TODO: select appropriate maximum length
        // let (_writer, reader) = LinesCodec::new_with_max_length(250)
        //     .framed(serial)
        //     .then(Self::parse_frame)
        //     .split();

        // let processor = reader
        //     .for_each(|frame| {
        //         GPS.lock().unwrap().latest_data = if frame.is_valid() { Some(frame) } else { None };
        //         Ok(())
        //     })
        //     .map_err(|e| warn!("Error processing frame: {}", e));
        // tokio::run(processor);

        Ok(())
    }

    /// Checks if the GPS is on.
    pub fn is_on(&self) -> Result<bool, Error> {
        Ok(CONFIG.gps().power_gpio().get_value()? == 1)
    }

    /// Turns the GPS on.
    pub fn turn_on(&self) -> Result<(), Error> {
        if self.is_on()? {
            warn!("Turning on the GPS but it was already on.");
        } else {
            CONFIG.gps().power_gpio().set_value(1)?
        }

        Ok(())
    }

    /// Turns the GPS off.
    pub fn turn_off(&self) -> Result<(), Error> {
        if self.is_on()? {
            CONFIG.gps().power_gpio().set_value(0)?
        } else {
            warn!("Turning off the GPS but it was already off.");
        }

        Ok(())
    }

    /// Enters airborne (<1g) GPS mode.
    fn enter_airborne_1g_mode<S>(serial: &mut S) -> Result<(), Error>
    where
        S: Write + Read,
    {
        let msg = [
            // Header, class, ID, Length
            0xB5, 0x62, 0x06, 0x24, 0x24, 0x00,
            // Payload:
            // Mask, Dynmodel, FixType
            0xFF, 0xFF, 0x06, 0x03, 0x00, 0x00, 0x00, 0x00, 0x10, 0x27, 0x00, 0x00, 0x05, 0x00,
            0xFA, 0x00, 0xFA, 0x00, 0x64, 0x00, 0x2C, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Checksum
            0x16, 0xDC,
        ];
        let mut ack = [
            0xB5, 0x62, 0x05, 0x01, 0x02, 0x00, msg[2], msg[3], 0x00, 0x00,
        ];
        // Compute checksum
        for i in 2..8 {
            ack[8] += ack[i];
            ack[9] += ack[8];
        }

        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(6) {
            // Send the message
            serial.flush()?;
            serial.write_all(&[0xFF])?;
            thread::sleep(Duration::from_millis(500));
            serial.write_all(&msg)?;

            // Wait for the ACK
            let mut checked_bytes = 0;
            let ack_start = Instant::now();
            while ack_start.elapsed() < Duration::from_secs(3) {
                if checked_bytes == 10 {
                    return Ok(());
                }

                let mut byte = [0];
                serial.read_exact(&mut byte)?; // FIXME: could this block?
                if byte[0] == ack[checked_bytes] {
                    checked_bytes += 1;
                } else {
                    checked_bytes = 0;
                }
            }
        }

        unimplemented!() // TODO: return error
    }

    /// Gets the latest GPS data.
    pub fn latest_data(&self) -> Option<Frame> {
        self.latest_data
    }

    /// Parses a GPS frame.
    fn parse_frame(line: Result<String, io::Error>) -> Result<Frame, Error> {
        let _line_str = line?; //                 if (bytes_ordered > 9)
                               // 		{
                               // 			return true;
                               // 		}

        // 		gettimeofday(&time_now, NULL);
        // 		ms_now = (long)((time_now.tv_sec)*1000 + (time_now.tv_usec)/1000);

        // 		if (this->serial->available())
        // 		{
        // 			byte = this->serial->read_byte();
        // 			if (byte == ack_packet[bytes_ordered])
        // 			{
        // 				bytes_ordered++;
        // 			}
        // 			else
        // 			{
        // 				bytes_ordered = 0;
        // 			}
        // }
        unimplemented!()
    }
}

impl Drop for Gps {
    fn drop(&mut self) {
        // TODO stop serial, turn GPS off.
    }
}

/// This structure represents a GPS frame.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
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

impl Frame {
    /// Gets the time of the current fix.
    pub fn fix_time(&self) -> DateTime<Utc> {
        self.fix_time
    }

    /// Gets the GPS fix status.
    pub fn status(&self) -> FixStatus {
        self.status
    }

    /// Checks if the frame is from a valid fix.
    pub fn is_valid(&self) -> bool {
        self.status == FixStatus::Active
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
    use super::{FixStatus, GPS};

    /// Checks the GPS status from string conversion.
    #[test]
    fn gps_status_from_str() {
        assert_eq!("A".parse::<FixStatus>().unwrap(), FixStatus::Active);
        assert_eq!("V".parse::<FixStatus>().unwrap(), FixStatus::Void);

        // Check errors.
        assert!("".parse::<FixStatus>().is_err());
        assert!("invalid".parse::<FixStatus>().is_err());
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
