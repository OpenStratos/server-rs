//! Initialization logic.

use super::*;

impl StateMachine for OpenStratos<Init> {
    type Next = OpenStratos<AcquiringFix>;

    fn execute(self) -> Result<Self::Next> {
        unimplemented!()
    }
}
