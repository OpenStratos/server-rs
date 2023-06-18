//! Landed logic.

use anyhow::Error;

use super::{Landed, OpenStratos, ShutDown, StateMachine};

impl StateMachine for OpenStratos<Landed> {
    type Next = OpenStratos<ShutDown>;

    fn execute(self) -> Result<Self::Next, Error> {
        unimplemented!()
    }
}
