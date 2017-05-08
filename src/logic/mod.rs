//! Logic module.

mod init;
mod acquiring_fix;
mod fix_acquired;
mod waiting_launch;
mod going_up;
mod going_down;
mod landed;
mod shut_down;

use std::str::FromStr;

use error::*;
use STATE_FILE;
use config::CONFIG;

/// Trait representing a state machine.
pub trait StateMachine {
    /// Type of the next state.
    type Next: MainLogic;

    /// Executes this state and returns the next one.
    fn execute(self) -> Result<Self::Next>;
}

/// Trait implementing the main logic of the program.
pub trait MainLogic {
    /// Performs the main logic of the state.
    fn main_logic(self) -> Result<()>;
}

impl<S> MainLogic for S
    where S: StateMachine
{
    fn main_logic(self) -> Result<()> {
        self.execute()?.main_logic()
    }
}

/// Main OpenStratos state machine
pub struct OpenStratos<S> {
    state: S,
}

/// Initializes a new state machine.
pub fn init() -> OpenStratos<Init> {
    OpenStratos { state: Init }
}

/// States of the onboard computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Initialization.
    Init,
    /// Acquiring GPS fix.
    AcquiringFix,
    /// GPS fix has been acquired.
    FixAcquired,
    /// Waiting for balloon launch.
    WaitingLaunch,
    /// Going up.
    GoingUp,
    /// Going down.
    GoingDown,
    /// Probe landed.
    Landed,
    /// Shutting computer down.
    ShutDown,
    /// Safe mode operation.
    SafeMode,
}

impl State {
    /// Gets the last state of the application if there is one.
    pub fn get_last() -> Result<Option<State>> {
        use std::path::Path;
        use std::fs::File;
        use std::io::Read;

        let path = CONFIG.data_dir().join(STATE_FILE);
        if !path.exists() {
            return Ok(None);
        }
        let mut file = File::open(path).chain_err(|| ErrorKind::LastStateFileOpen)?;
        let mut state = String::new();
        file.read_to_string(&mut state)
            .chain_err(|| ErrorKind::LastStateFileRead)?;

        if state.is_empty() {
            Ok(None)
        } else {
            Ok(Some(state.parse()?))
        }
    }
}

impl FromStr for State {
    type Err = Error;

    fn from_str(s: &str) -> Result<State> {
        match s {
            "INITIALIZING" => Ok(State::Init),
            "ACQUIRING_FIX" => Ok(State::AcquiringFix),
            "FIX_ACQUIRED" => Ok(State::FixAcquired),
            "WAITING_LAUNCH" => Ok(State::WaitingLaunch),
            "GOING_UP" => Ok(State::GoingUp),
            "GOING_DOWN" => Ok(State::GoingDown),
            "LANDED" => Ok(State::Landed),
            "SHUT_DOWN" => Ok(State::ShutDown),
            "SAFE_MODE" => Ok(State::SafeMode),
            _ => Err(ErrorKind::InvalidState(s.to_owned()).into()),
        }
    }
}

/// Initialization state.
pub struct Init;

/// Acquiring fix state.
pub struct AcquiringFix;

/// Fix acquired state.
pub struct FixAcquired;

/// Waiting launch state.
pub struct WaitingLaunch;

/// Going up state.
pub struct GoingUp;

/// Going down state.
pub struct GoingDown;

/// Landed state.
pub struct Landed;

/// Shut down state.
pub struct ShutDown;

/// Safe mode state.
pub struct SafeMode;

/// Struct to allow type checking shut down.
pub struct Void;
