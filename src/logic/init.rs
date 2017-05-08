//! Initialization logic.

use super::*;

impl StateMachine for OpenStratos<Init> {
    #[cfg(feature = "gps")]
    type Next = OpenStratos<AcquiringFix>;

    #[cfg(not(feature = "gps"))]
    type Next = OpenStratos<EternalLoop>;

    fn execute(self) -> Result<Self::Next> {
        unimplemented!()
    }
}
