//! Balloon software launcher.

#[macro_use]
extern crate log;
extern crate os_balloon;

use os_balloon::*;

/// program entry point.
fn main() {
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
