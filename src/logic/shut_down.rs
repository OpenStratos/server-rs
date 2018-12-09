//! Shut down logic.

use failure::Error;

use super::{MainLogic, OpenStratos, ShutDown};

impl MainLogic for OpenStratos<ShutDown> {
    fn main_logic(self) -> Result<(), Error> {
        unimplemented!()
    }
}
