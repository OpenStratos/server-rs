//! Going down logic.

use failure::Error;

use super::*;

impl StateMachine for OpenStratos<GoingDown> {
    type Next = OpenStratos<Landed>;

    fn execute(self) -> Result<Self::Next, Error> {
        unimplemented!()
    }
}
