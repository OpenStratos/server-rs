//! Initialization logic.

use std::io;

// Only required for FONA or Raspicam
#[cfg(any(feature = "fona", feature = "raspicam"))]
use std::time::Duration;

// Only required for FONA
#[cfg(feature = "fona")]
use std::thread;

// Only required when no powering off
#[cfg(feature = "no_power_off")]
use std::process;

use super::*;
#[cfg(feature = "fona")]
use fona::FONA;
#[cfg(feature = "gps")]
use gps::GPS;
#[cfg(feature = "raspicam")]
use raspicam::VIDEO_DIR;

/// Test video file.
#[cfg(feature = "raspicam")]
pub const TEST_VIDEO_FILE: &str = "test.h264";

impl StateMachine for OpenStratos<Init> {
    #[cfg(feature = "gps")]
    type Next = OpenStratos<AcquiringFix>;

    #[cfg(not(feature = "gps"))]
    type Next = OpenStratos<EternalLoop>;

    #[cfg_attr(feature = "cargo-clippy", allow(block_in_if_condition_expr))]
    fn execute(self) -> Result<Self::Next, Error> {
        let disk_space = get_available_disk_space()?;
        info!(
            "Available disk space: {:.2} GiB",
            disk_space as f32 / 1024_f32 / 1024_f32 / 1024_f32
        );

        #[cfg(feature = "raspicam")]
        info!(
            "Disk space enough for about {} minutes of fullHD video.",
            CONFIG.video().bitrate() / (8 * 60)
        );

        if {
            #[cfg(feature = "raspicam")]
            {
                // 1.2 times the length of the flight, just in case.
                disk_space
                    < CONFIG.flight().length() as u64 * 6 * 60 * CONFIG.video().bitrate() as u64
                        / (8 * 5)
            }
            #[cfg(not(feature = "raspicam"))]
            {
                disk_space < 2 * 1024 * 1024 * 1024 // 2 GiB
            }
        } {
            error!("Not enough disk space.");
            #[cfg(not(feature = "no_power_off"))]
            power_off()?;
            #[cfg(feature = "no_power_off")]
            process::exit(1);
        }

        #[cfg(feature = "gps")]
        {
            info!("Initializing GPS…");
            match GPS.lock() {
                Ok(mut gps) => gps.initialize().context(error::Gps::Init)?,
                Err(poisoned) => {
                    error!("The GPS mutex was poisoned.");
                    poisoned
                        .into_inner()
                        .initialize()
                        .context(error::Gps::Init)?
                }
            }
            info!("GPS initialized.");
        }

        #[cfg(feature = "fona")]
        {
            info!("Initializing Adafruit FONA GSM module…");
            match FONA.lock() {
                Ok(mut fona) => fona.initialize().context(error::Fona::Init)?,
                Err(poisoned) => {
                    error!("The FONA mutex was poisoned.");
                    poisoned
                        .into_inner()
                        .initialize()
                        .context(error::Fona::Init)?
                }
            }
            info!("Adafruit FONA GSM module initialized.");

            info!("Checking batteries…");
            // TODO check batteries
            // double main_battery, gsm_battery;
            // if ( ! GSM::get_instance().get_battery_status(main_battery, gsm_battery) &&
            // 	 ! GSM::get_instance().get_battery_status(main_battery, gsm_battery))
            // {
            // 	error!("Error checking batteries.");
            //
            // 	logger.log("Turning GSM off...");
            // 	if (GSM::get_instance().turn_off())
            // 		logger.log("GSM off.");
            // 	else
            // 		logger.log("Error turning GSM off.");
            //
            // info!("Turning GPS off…");
            // match GPS.lock() {
            //     Ok(mut gps) => {
            //         if let Ok(_) = gps.turn_off() {
            //             info!("GPS off.");
            //         } else {
            //             error!("Could not turn GPS off.");
            //         }
            //     }
            //     Err(poisoned) => {
            //         error!("The GPS mutex was poisoned.");
            //         if let Ok(_) = poisoned.into_inner().turn_off() {
            //             info!("GPS off.");
            //         } else {
            //             error!("Could not turn GPS off.");
            //         }
            //     }
            // }
            //
            // #[cfg(not(feature = "no_power_off"))]
            // power_off();
            // #[cfg(feature = "no_power_off")]
            // process::exit(1);
            // }
            //
            // info!("Batteries checked => Main battery: "+
            //(main_battery > -1 ? to_string(main_battery*100)+"%" : "disconnected") +
            // 	" - GSM battery: "+ to_string(gsm_battery*100) +"%");
            //
            // if ((main_battery < MIN_MAIN_BAT  && main_battery > -1) || gsm_battery < MIN_GSM_BAT)
            // {
            // 	error!("Not enough battery.");
            //
            // 	logger.log("Turning GSM off...");
            // 	if (GSM::get_instance().turn_off())
            // 		logger.log("GSM off.");
            // 	else
            // 		logger.log("Error turning GSM off.");
            //
            // info!("Turning GPS off…");
            // match GPS.lock() {
            //     Ok(mut gps) => {
            //         if let Ok(_) = gps.turn_off() {
            //             info!("GPS off.");
            //         } else {
            //             error!("Could not turn GPS off.");
            //         }
            //     }
            //     Err(poisoned) => {
            //         error!("The GPS mutex was poisoned.");
            //         if let Ok(_) = poisoned.into_inner().turn_off() {
            //             info!("GPS off.");
            //         } else {
            //             error!("Could not turn GPS off.");
            //         }
            //     }
            // }
            //
            // #[cfg(not(feature = "no_power_off"))]
            // power_off();
            // #[cfg(feature = "no_power_off")]
            // process::exit(1);
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
            use std::fs::remove_file;

            use raspicam::CAMERA;

            info!("Testing camera recording…");
            info!("Recording 10 seconds as test…");
            match CAMERA.lock() {
                Ok(mut cam) => cam
                    .record(Duration::from_secs(10), TEST_VIDEO_FILE)
                    .context(error::Raspicam::Test)?,
                Err(poisoned) => {
                    error!("The CAMERA mutex was poisoned.");
                    poisoned
                        .into_inner()
                        .record(Duration::from_secs(10), TEST_VIDEO_FILE)
                        .context(error::Raspicam::Test)?
                }
            }

            let video_path = CONFIG.data_dir().join(VIDEO_DIR).join(TEST_VIDEO_FILE);
            if video_path.exists() {
                info!("Camera test OK.");
                info!("Removing test file…");
                remove_file(&video_path).context(error::Raspicam::TestRemove {
                    test_file: video_path.clone(),
                })?;
                info!("Test file removed.");
            } else {
                error!("Camera test file was not created.");
                // TODO
                // logger.log("Turning GSM off...");
                // if (GSM::get_instance().turn_off())
                // 	logger.log("GSM off.");
                // else
                // 	logger.log("Error turning GSM off.");
                //
                // logger.log("Turning GPS off...");
                // if (GPS::get_instance().turn_off())
                // 	logger.log("GPS off.");
                // else
                // 	logger.log("Error turning GPS off.");

                #[cfg(not(feature = "no_power_off"))]
                power_off()?;
                #[cfg(feature = "no_power_off")]
                process::exit(1);
            }
        }

        #[cfg(feature = "gps")]
        {
            Ok(OpenStratos {
                state: AcquiringFix,
            })
        }

        #[cfg(not(feature = "gps"))]
        {
            Ok(OpenStratos { state: EternalLoop })
        }
    }
}

/// Gets the available disk space for OpenStratos.
fn get_available_disk_space() -> Result<u64, Error> {
    use std::ffi::CString;
    use std::mem;
    use std::os::unix::ffi::OsStrExt;

    use libc;

    let dir = CString::new(CONFIG.data_dir().as_os_str().as_bytes())?;

    let mut stats: libc::statvfs;
    // TODO: Why is it safe?
    let res = unsafe {
        stats = mem::uninitialized();
        libc::statvfs(dir.as_ptr(), &mut stats)
    };

    if res == 0 {
        Ok(u64::from(stats.f_bsize) * u64::from(stats.f_bavail))
    } else {
        Err(io::Error::last_os_error().into())
    }
}

/// Powers the system off.
///
/// It takes care of disk synchronization.
#[cfg(not(feature = "no_power_off"))]
fn power_off() -> Result<(), io::Error> {
    use libc::{reboot, sync, RB_POWER_OFF};

    // Safe because `sync()` is always successful.
    unsafe {
        sync();
    }

    // TODO: Why is it safe?
    if unsafe { reboot(RB_POWER_OFF) } == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
