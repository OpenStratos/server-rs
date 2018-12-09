//! Going down logic.

use failure::Error;

use super::{GoingDown, Landed, OpenStratos, StateMachine};

impl StateMachine for OpenStratos<GoingDown> {
    type Next = OpenStratos<Landed>;

    fn execute(self) -> Result<Self::Next, Error> {
        unimplemented!()
    }
}
