//! Initialization logic.

use std::{io, thread, process};
use std::time::Duration;

use libc::c_ulong;

use super::*;
use error::*;
#[cfg(feature = "gps")]
use gps::GPS_DATA;

impl StateMachine for OpenStratos<Init> {
    #[cfg(feature = "gps")]
    type Next = OpenStratos<AcquiringFix>;

    #[cfg(not(feature = "gps"))]
    type Next = OpenStratos<EternalLoop>;

    #[allow(block_in_if_condition_expr)]
    fn execute(self) -> Result<Self::Next> {

        let disk_space = get_available_disk_space()?;
        info!("Available disk space: {:.2} GiB",
              disk_space as f32 / 1024_f32 / 1024_f32 / 1024_f32);

        #[cfg(feature = "raspicam")]
        info!("Disk space enough for about {} minutes of fullHD video.",
              CONFIG.video().bitrate() / (8 * 60));

        if {
               #[cfg(feature = "raspicam")]
               {
                   // 1.2 times the length of the light, just in case.
                   disk_space <
                   CONFIG.flight().length() as u64 * 6 * 60 * CONFIG.video().bitrate() as u64 /
                   (8 * 5)
               }
               #[cfg(not(feature = "raspicam"))]
               {
                   disk_space < 2 * 1024 * 1024 * 1024 // 2 GiB
               }
           } {
            error!("Not enough disk space.");
            #[cfg(not(feature = "no_power_off"))]
            power_off();
            #[cfg(feature = "no_power_off")]
            process::exit(1);
        }

        #[cfg(feature = "gps")]
        {
            info!("Initializing GPS…");
            match GPS_DATA.lock() {
                Ok(mut gps) => gps.initialize().chain_err(|| ErrorKind::GPSInit)?,
                Err(poisoned) => {
                    error!("The GPS_DATA mutex was poisoned.");
                    poisoned
                        .into_inner()
                        .initialize()
                        .chain_err(|| ErrorKind::GPSInit)?
                }
            }
            info!("GPS initialized.");
        }

        #[cfg(feature = "fona")]
        {
            info!("Initializing Adafruit FONA GSM module…");
            // TODO
            // match FONA.lock() {
            //     Ok(mut fona) => fona.initialize().chain_err(|| ErrorKind::FONAInit)?,
            //     Err(poisoned) => {
            //         error!("The FONA mutex was poisoned.");
            //         poisoned
            //             .into_inner()
            //             .initialize()
            //             .chain_err(|| ErrorKind::FONAInit)?
            //     }
            // }
            info!("Adafruit FONA GSM module initialized.");

            info!("Checking batteries…");
            // TODO check batteries
            // double main_battery, gsm_battery;
            // if ( ! GSM::get_instance().get_battery_status(main_battery, gsm_battery) &&
            // 	 ! GSM::get_instance().get_battery_status(main_battery, gsm_battery))
            // {
            // 	logger.log("Error checking batteries.");
            //
            // 	logger.log("Turning GSM off...");
            // 	if (GSM::get_instance().turn_off())
            // 		logger.log("GSM off.");
            // 	else
            // 		logger.log("Error turning GSM off.");
            //
            // 	logger.log("Turning GPS off...");
            // 	if (GPS::get_instance().turn_off())
            // 		logger.log("GPS off.");
            // 	else
            // 		logger.log("Error turning GPS off.");
            //
            // 	#ifndef NO_POWER_OFF
            // 		sync();
            // 		reboot(RB_POWER_OFF);
            // 	#else
            // 		exit(1);
            // 	#endif
            // }
            //
            // logger.log("Batteries checked => Main battery: "+
            //(main_battery > -1 ? to_string(main_battery*100)+"%" : "disconnected") +
            // 	" - GSM battery: "+ to_string(gsm_battery*100) +"%");
            //
            // if ((main_battery < MIN_MAIN_BAT  && main_battery > -1) || gsm_battery < MIN_GSM_BAT)
            // {
            // 	logger.log("Error: Not enough battery.");
            //
            // 	logger.log("Turning GSM off...");
            // 	if (GSM::get_instance().turn_off())
            // 		logger.log("GSM off.");
            // 	else
            // 		logger.log("Error turning GSM off.");
            //
            // 	logger.log("Turning GPS off...");
            // 	if (GPS::get_instance().turn_off())
            // 		logger.log("GPS off.");
            // 	else
            // 		logger.log("Error turning GPS off.");
            //
            // 	#ifndef NO_POWER_OFF
            // 		sync();
            // 		reboot(RB_POWER_OFF);
            // 	#else
            // 		exit(1);
            // 	#endif
            // }

            info!("Waiting for GSM connectivity…");
            while {
                      // TODO
                      // match GSM.lock() {
                      //     Ok(gsm) => gsm.has_connectivity()?,
                      //     Err(poisoned) => {
                      //         error!("The GSM mutex was poisoned.");
                      //         poisoned.into_inner().has_connectivity()?
                      //     }
                      // }
                      false
                  } {
                thread::sleep(Duration::from_secs(1));
            }
            info!("GSM connected.");
        }

        #[cfg(feature = "raspicam")]
        {
            use raspicam::CAMERA;

            info!("Testing camera recording…");
            info!("Recording 10 seconds as test…");
            match CAMERA.lock() {
                Ok(mut cam) => {
                    cam.record(Duration::from_secs(10), "test.h264")
                        .chain_err(|| ErrorKind::CameraTest)?
                }
                Err(poisoned) => {
                    error!("The CAMERA mutex was poisoned.");
                    poisoned
                        .into_inner()
                        .record(Duration::from_secs(10), "test.h264")
                        .chain_err(|| ErrorKind::CameraTest)?
                }
            }
        }
        unimplemented!()
    }
}

/// Gets the available disk space for OpenStratos.
fn get_available_disk_space() -> Result<u64> {
    use libc;
    use std::ffi::{CString, OsStr};
    use std::os::unix::ffi::OsStrExt;
    use std::mem;

    let dir = CString::new(CONFIG.data_dir().as_os_str().as_bytes())?;

    let mut stats: libc::statvfs;
    let res = unsafe {
        stats = mem::uninitialized();
        libc::statvfs(dir.as_ptr(), &mut stats)
    };

    if res == 0 {
        Ok(stats.f_bsize as u64 * stats.f_bavail as u64)
    } else {
        Err(Error::from(io::Error::last_os_error()))
    }
}

/// Powers the system off.
///
/// It takes care of disk synchronization.
fn power_off() -> Result<()> {
    use libc::{reboot, RB_POWER_OFF};

    // TODO sync();

    if unsafe { reboot(RB_POWER_OFF) } == -1 {
        Err(Error::from(io::Error::last_os_error()))
    } else {
        Ok(())
    }
}
