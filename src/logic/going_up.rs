//! Going up logic.

use failure::Error;

use super::{GoingDown, GoingUp, OpenStratos, StateMachine};

impl StateMachine for OpenStratos<GoingUp> {
    type Next = OpenStratos<GoingDown>;

    fn execute(self) -> Result<Self::Next, Error> {
        unimplemented!()
    }
}
