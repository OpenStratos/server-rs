//! Fix acquired logic.

use failure::Error;

use super::*;

impl StateMachine for OpenStratos<FixAcquired> {
    type Next = OpenStratos<WaitingLaunch>;

    fn execute(self) -> Result<Self::Next, Error> {
        unimplemented!()
    }
}
