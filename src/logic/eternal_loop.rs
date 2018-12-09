//! Eternal loop, if GPS is not enabled.

use failure::Error;

use super::{EternalLoop, ShutDown, OpenStratos, StateMachine};

impl StateMachine for OpenStratos<EternalLoop> {
    type Next = OpenStratos<ShutDown>;

    fn execute(self) -> Result<Self::Next, Error> {
        unimplemented!()
    }
}
