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
        check_disk_space()?;

        #[cfg(feature = "gps")]
        {
            if let Err(e) = initialize_gps() {
                // TODO: shut down GPS.
                return Err(e);
            }
        }

        #[cfg(feature = "fona")]
        {
            if let Err(e) = initialize_fona() {
                // TODO: shut down GPS (if feature enabled) and FONA.
                return Err(e);
            }
        }

        #[cfg(feature = "raspicam")]
        {
            if let Err(e) = test_raspicam() {
                // TODO: shut down GPS (if feature enabled) and FONA (if feature enabled).
                return Err(e);
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

/// Checks if the available disk space is enough.
fn check_disk_space() -> Result<(), Error> {
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

    // 1.2 times the length of the flight, just in case.
    #[cfg(feature = "raspicam")]
    let enough_space = disk_space
        > u64::from(CONFIG.flight().length()) * 6 * 60 * u64::from(CONFIG.video().bitrate())
            / (8 * 5);

    #[cfg(not(feature = "raspicam"))]
    let enough_space = disk_space > 2 * 1024 * 1024 * 1024; // 2 GiB

    if !enough_space {
        error!("Not enough disk space.");
        #[cfg(not(feature = "no_power_off"))]
        power_off()?;
        #[cfg(feature = "no_power_off")]
        process::exit(1);
    }

    Ok(())
}

/// Initializes the GPS module.
#[cfg(feature = "gps")]
fn initialize_gps() -> Result<(), Error> {
    info!("Initializing GPS…");
    match GPS.lock() {
        Ok(mut gps) => gps.initialize().context(error::Init::GpsInit)?,
        Err(poisoned) => {
            error!("The GPS mutex was poisoned.");
            poisoned
                .into_inner()
                .initialize()
                .context(error::Init::GpsInit)?
        }
    }
    info!("GPS initialized.");
    Ok(())
}

/// Initializes the FONA module.
#[cfg(feature = "fona")]
fn initialize_fona() -> Result<(), Error> {
    info!("Initializing Adafruit FONA GSM module…");
    match FONA.lock() {
        Ok(mut fona) => fona.initialize().context(error::Init::FonaInit)?,
        Err(poisoned) => {
            error!("The FONA mutex was poisoned.");
            poisoned
                .into_inner()
                .initialize()
                .context(error::Init::FonaInit)?
        }
    }
    info!("Adafruit FONA GSM module initialized.");

    check_batteries()?;

    info!("Waiting for GSM connectivity…");
    while {
        match FONA.lock() {
            Ok(mut fona) => !fona
                .has_connectivity()
                .context(error::Init::CheckGsmConnectivity)?,
            Err(poisoned) => {
                error!("The FONA mutex was poisoned.");
                !poisoned
                    .into_inner()
                    .has_connectivity()
                    .context(error::Init::CheckGsmConnectivity)?
            }
        }
    } {
        thread::sleep(Duration::from_secs(1));
    }
    info!("GSM connected.");

    Ok(())
}

/// Checks the batteries of the probe using the FONA's built-in ADC.
#[cfg(feature = "fona")]
fn check_batteries() -> Result<(), Error> {
    info!("Checking batteries…");

    let fona_bat_percent = match FONA.lock() {
        Ok(mut fona) => fona.battery_percent().context(error::Init::CheckBatteries)?,
        Err(poisoned) => {
            error!("The FONA mutex was poisoned.");
            poisoned
                .into_inner()
                .battery_percent()
                .context(error::Init::CheckBatteries)?
        }
    };
    let adc_voltage = match FONA.lock() {
        Ok(mut fona) => fona.adc_voltage().context(error::Init::CheckBatteries)?,
        Err(poisoned) => {
            error!("The FONA mutex was poisoned.");
            poisoned
                .into_inner()
                .adc_voltage()
                .context(error::Init::CheckBatteries)?
        }
    };
    let main_bat_percent = (adc_voltage - CONFIG.battery().main_min())
        / (CONFIG.battery().main_max() - CONFIG.battery().main_min());

    info!(
        "Batteries checked => Main battery: {} - GSM battery: {}",
        if main_bat_percent > -1_f32 {
            format!("{}%", main_bat_percent * 100_f32)
        } else {
            "disconnected".to_owned()
        },
        if fona_bat_percent > -1_f32 {
            format!("{}%", fona_bat_percent * 100_f32)
        } else {
            "disconnected".to_owned()
        }
    );

    if (main_bat_percent < CONFIG.battery().main_min_percent() && main_bat_percent > -1_f32)
        || fona_bat_percent < CONFIG.battery().fona_min_percent()
    {
        error!("Not enough battery.");
        Err(error::Init::NotEnoughBattery.into())
    } else {
        Ok(())
    }
}

/// Performs a test in the Raspicam module.
#[cfg(feature = "raspicam")]
fn test_raspicam() -> Result<(), Error> {
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
    Ok(())
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
        Ok(stats.f_bsize * stats.f_bavail)
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
