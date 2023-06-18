//! Waiting launch logic.

use anyhow::Error;

use super::{GoingUp, OpenStratos, StateMachine, WaitingLaunch};

impl StateMachine for OpenStratos<WaitingLaunch> {
    type Next = OpenStratos<GoingUp>;

    fn execute(self) -> Result<Self::Next, Error> {
        unimplemented!()
    }
}
