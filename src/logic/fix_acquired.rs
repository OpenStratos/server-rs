//! Fix acquired logic.

use super::*;

impl StateMachine for OpenStratos<FixAcquired> {
    type Next = OpenStratos<WaitingLaunch>;

    fn execute(self) -> Result<Self::Next> {
        unimplemented!()
    }
}
