//! Acquiring fix logic.

use super::*;

impl StateMachine for OpenStratos<AcquiringFix> {
    type Next = OpenStratos<FixAcquired>;

    fn execute(self) -> Result<Self::Next> {
        unimplemented!()
    }
}
