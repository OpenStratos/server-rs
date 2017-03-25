//! Going up logic.

use super::*;

impl StateMachine for OpenStratos<GoingUp> {
    type Next = OpenStratos<GoingDown>;

    fn execute(self) -> Result<Self::Next> {
        unimplemented!()
    }
}
