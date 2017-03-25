//! Going down logic.

use super::*;

impl StateMachine for OpenStratos<GoingDown> {
    type Next = OpenStratos<Landed>;

    fn execute(self) -> Result<Self::Next> {
        unimplemented!()
    }
}
