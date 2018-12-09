//! Waiting launch logic.

use failure::Error;

use super::{GoingUp, OpenStratos, StateMachine, WaitingLaunch};

impl StateMachine for OpenStratos<WaitingLaunch> {
    type Next = OpenStratos<GoingUp>;

    fn execute(self) -> Result<Self::Next, Error> {
        unimplemented!()
    }
}
