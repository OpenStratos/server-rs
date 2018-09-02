//! Acquiring fix logic.

use failure::Error;

use super::*;

impl StateMachine for OpenStratos<AcquiringFix> {
    type Next = OpenStratos<FixAcquired>;

    fn execute(self) -> Result<Self::Next, Error> {
        unimplemented!()
    }
}
