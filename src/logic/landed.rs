//! Landed logic.

use failure::Error;

use super::*;

impl StateMachine for OpenStratos<Landed> {
    type Next = OpenStratos<ShutDown>;

    fn execute(self) -> Result<Self::Next, Error> {
        unimplemented!()
    }
}
