//! Eternal loop, if GPS is not enabled.

use anyhow::Error;

use super::{EternalLoop, OpenStratos, ShutDown, StateMachine};

impl StateMachine for OpenStratos<EternalLoop> {
    type Next = OpenStratos<ShutDown>;

    fn execute(self) -> Result<Self::Next, Error> {
        unimplemented!()
    }
}
