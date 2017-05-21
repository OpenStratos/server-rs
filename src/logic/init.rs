//! Initialization logic.

use std::io;

use libc::c_ulong;

use super::*;
use error::*;

impl StateMachine for OpenStratos<Init> {
    #[cfg(feature = "gps")]
    type Next = OpenStratos<AcquiringFix>;

    #[cfg(not(feature = "gps"))]
    type Next = OpenStratos<EternalLoop>;

    fn execute(self) -> Result<Self::Next> {

        let disk_space = get_available_disk_space()?;
        info!("Available disk space: {:.2} GiB",
              disk_space as f32 / 1024_f32 / 1024_f32 / 1024_f32);
        if disk_space < CONFIG.flight().length() as u64 * 60 * CONFIG.video().bitrate() as u64 / 8 {
            error!("Not enough disk space.");
            power_off();
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
