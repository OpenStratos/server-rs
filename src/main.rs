#![doc(html_logo_url = "https://openstratos.org/wp-content/uploads/2017/05/OpenStratos-768x226.png",
       html_favicon_url = "https://openstratos.org/wp-content/uploads/2015/10/OpenStratos-mark.png",
html_root_url = "https://openstratos.github.io/server-rs/")]

//! Balloon software launcher.
//!
//! This crate contains the main initialization logic for the balloon software. For further
//! development documentation, please refer to the [`os_balloon`](../os_balloon/index.html) crate.
//! OpenStratos is designed to work in a Raspberry Pi model A+, B+, Zero, v2 B or v3 B, but it could
//! work in other Linux-powered devices too with proper configuration.
//!
//! ## Running OpenStratos
//!
//! Running OpenStratos is as simple as installing the latest Rust stable build (preferrably using
//! [rustup.rs](https://rustup.rs/)) and then running `cargo run` in the crate directory. Remember
//! to use `cargo run --release` to compile the software with all optimizations enabled.
//!
//! ## Features
//!
//! It is possible that the setup you want for OpenStratos is different from the default one.
//! Luckily, OpenStratos supports
//! [Cargo features](http://doc.crates.io/manifest.html#the-features-section) and can be configured
//! and extended easily.
//!
//! Current features are:
//!
//! * **GPS** (`--features="gps"`): Enables GPS monitoring. You will need to provide a serial port
//! and a disable/enable GPIO pin if you want OpenStratos to control the GPS on/off behaviour. The
//! tested and recommended GPS module is the
//! [uBLOX MAX-M8Q](https://store.uputronics.com/index.php?route=product/product&product_id=84)
//! model. If you wish to use a separate GPS device, or directly not use GPS, you can opt-out to
//! this feature. **Important**: OpenStratos won't have information about location above 1.2km
//! altitude (below that point only if GSM is active). This means that it will not be able to send
//! SMS / telemetry about position and that pictures won't have EXIF information about localization.
//! * **Raspberry Pi camera** (`--features="raspicam"`): Enables the Raspberry Pi camera module.
//! OpenStratos is prepared to work with the Raspberry Pi V2 camera module, but it will work with
//! the V1 version too (you will have to make sure you provide a proper configuration for picture
//! size, as you won't have 8MP available and OpenStratos checks configuration against those 8MP).
//! The software will take pictures at the configured intervals, and add EXIF GPS information if
//! available. It will also record video with the specified configuration. You can opt-out to this
//! feature and use your own external camera, or remove imaging directly. You could also use other
//! kind of camera if using a device different than the Raspberry Pi. If you wish to use a different
//! camera and control it using OpenStratos, you should extend the software adding a driver for that
//! device.
//! * **Adafruit FONA** (`--features="fona"`): Enables Adafruit's FONA GSM module. OpenStratos is
//! prepared to work with Adafruit's FONA GSM module. It will send SMSs in different stages of the
//! flight (check below for more information about SMSs). It will also use GSM localization if no
//! GPS is found, or if the GPS fails (safe mode operation). Furthermore, if configured properly,
//! it will check battery levels of both FONA's battery and the main battery using FONA's on-chip
//! ADC (analog-digital-converter). You can opt-out to this feature, and information will be sent
//! using telemetry module. Nevertheless, it's a good idea to have GSM in case live telemetry fails
//! or a LOS (loss of signal) happens.
//! * **Live transparent serial telemetry** (`--features="telemetry"`): Enables real time telemetry.
//! OpenStratos is capable to send telemetry via a serial device. This can be used with an XBee
//! module in transparent mode, for example, to receive the information via serial in the client
//! (in a laptop computer in the ground, for example).
//! * *TODO: commands?*
//!
//! All these features are enabled by default. You can opt-out to all of them passing the
//! `--no-default-features` flag to Cargo when building / running the software, and enable each of
//! them separatelly. To enable more than one feature, add space between them in the `--features`
//! option. E.g. `--no-default-features --features="gps telemetry"`.
//!
//! ## Configuration
//!
//! OpenStratos is highly configurable. Please refer to the
//! [`config`](../os_balloon/config/index.html) module for further information.
//!
//! ## Main logic
//!
//! OpenStratos logic is divided in states, implemented as a type-level state machine. For
//! implementation details, please refer to the [`logic`](../os_balloon/logic/index.html) module.
//!
//! When first powered up, if no state file exists (the software has never run before), the on-board
//! computer will start in initialization mode. Here,it will check for enough available disk space,
//! if the camera is working properly (in case the corresponding feature is enabled), will
//! initialize / reboot GPS and GSM (if they are enabled) and check if the probe has enough battery
//! for the flight (in case the ADC in the Adafruit FONA is configured for that). Once all tests
//! pass, it will start the picture and battery threads, that will take pictures once in a while and
//! log the battery usage respectively. Picture thread will only be started in the case that camera
//! feature is enabled, and battery thread will only log FONA battery if the ADC is not connected to
//! the main battery. FONA feature is required for this. One of the first steps of the
//! initialization, even before running the tests will be to start the system thread. This thread
//! will log information about CPU and RAM usage, along with CPU/GPU temperature.
//!
//! Once the initialization is complete, and if the GPS is enabled, OpenStratos will wait for a GPS
//! fix. Once the GPS fix is acquired, it will first wait 10 seconds for the GPS fix to stabilize,
//! and will then start the camera video recording. Once the camera starts recording properly, it
//! will send an SMS with information about the initialization:
//!
//! ```text
//! Init: OK.
//! Alt: 256 m
//! Lat: 3.2759
//! Lon: 40.1578
//! PDOP: 3.24
//! Sat: 7
//! Fix: OK
//! Main bat: 92%
//! GSM bat: 93%
//! Waiting launch.
//! ```
//!
//! The information in this SMS is the altitude, the latitude, the longitude, the position degree of
//! precission (PDOP), the number of GPS satellites connected, the GPS fix status and the battery
//! capacities. Of course, this content will vary if no GPS is provided. It will try to send it a
//! second time if it fails the first one. Once the SMS is sent, OpenStratos asumes that the balloon
//! could be launched, so it will not stop until landing or critical failure.
//!
//! The on-board computer will now wait for the launch. It will try to get a reasonable precission
//! in altitude to record the launch altitude (to check it later). It will then wait until launch.
//! It will try to detect a rapid ascent, or as a backup, if the current altitude is much higher
//! than the launch altitude (100m with good precission, more if the precission is bad). This will
//! only work if GPS is enabled. If not, it will simply record, OpenStratos will have no way of
//! knowing its state. You will need to provide your own tracking mechanism.

#[macro_use]
extern crate log;
extern crate os_balloon;

use os_balloon::*;

/// Program entry point.
///
/// This function will initialize configuration, initialize loggers and start the main logic of the
/// balloon software by running [`os_balloon::run()`](../os_balloon/fn.run.html). It will then
/// handle possible errors and try to recover from them.
pub fn main() {
    if CONFIG.debug() {
        println!("Debug mode active");
    }
    if let Err(e) = init_loggers() {
        print_system_failure(&e, "Error initializing loggers");
        panic!();
    }
    info!("OpenStratos {} starting", env!("CARGO_PKG_VERSION"));

    if let Err(e) = run() {
        print_system_failure(&e, "Error running OpenStratos");
        panic!(); // TODO safe mode / recovery mode / restart...
    }
}
