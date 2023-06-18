//! Acquiring fix logic.

use anyhow::Error;

use super::{AcquiringFix, FixAcquired, OpenStratos, StateMachine};

impl StateMachine for OpenStratos<AcquiringFix> {
    type Next = OpenStratos<FixAcquired>;

    fn execute(self) -> Result<Self::Next, Error> {
        unimplemented!()
    }
}
