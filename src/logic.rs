//! Logic module.

#[cfg(feature = "gps")]
mod acquiring_fix;
#[cfg(not(feature = "gps"))]
mod eternal_loop;
#[cfg(feature = "gps")]
mod fix_acquired;
#[cfg(feature = "gps")]
mod going_down;
#[cfg(feature = "gps")]
mod going_up;
mod init;
#[cfg(feature = "gps")]
mod landed;
mod safe_mode;
mod shut_down;
#[cfg(feature = "gps")]
mod waiting_launch;

use std::{
    fmt,
    fs::{File, OpenOptions},
    io::{Read, Write},
    str::FromStr,
    sync::Mutex,
};

use failure::{Error, ResultExt};
use lazy_static::lazy_static;
use log::error;

use crate::{config::CONFIG, error, STATE_FILE};

lazy_static! {
    static ref CURRENT_STATE: Mutex<State> = Mutex::new(State::Init);
}

/// Trait representing a state machine.
pub trait StateMachine {
    /// The logic to run after the current state.
    type Next: MainLogic;

    /// Executes this state and returns the next one.
    fn execute(self) -> Result<Self::Next, Error>;
}

/// Trait to get the current state in the `State` enum for the current state in the state machine.
pub trait GetState {
    /// Gets the state enumeration variant for the current state.
    fn get_state(&self) -> State;
}

/// Trait implementing the main logic of the program.
#[allow(clippy::module_name_repetitions)]
pub trait MainLogic: GetState {
    /// Performs the main logic of the state.
    fn main_logic(self) -> Result<(), Error>;
}

impl<S> MainLogic for S
where
    S: StateMachine + GetState,
{
    fn main_logic(self) -> Result<(), Error> {
        let new_state = self.execute()?;
        {
            let mut current_state = match CURRENT_STATE.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    error!("The CURRENT_STATE mutex was poisoned.");
                    poisoned.into_inner()
                }
            };
            *current_state = new_state.get_state();
        }

        save_current_state()?;

        new_state.main_logic()
    }
}

/// Saves the current state into the state file.
fn save_current_state() -> Result<(), Error> {
    let path = CONFIG.data_dir().join(STATE_FILE);
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .context(error::LastState::FileOpen)?;
    {
        let current_state = match CURRENT_STATE.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                error!("The CURRENT_STATE mutex was poisoned.");
                poisoned.into_inner()
            }
        };
        file.write_all(current_state.as_str().as_bytes())
            .context(error::LastState::FileWrite)?;
    }
    Ok(())
}

/// Main OpenStratos state machine
#[derive(Debug, Clone, Copy)]
pub struct OpenStratos<S: GetState + fmt::Debug + Clone + Copy> {
    /// State of the logic item, only for compile time checks, no actual memory layout.
    state: S,
}

impl<S> GetState for OpenStratos<S>
where
    S: GetState + fmt::Debug + Clone + Copy,
{
    fn get_state(&self) -> State {
        self.state.get_state()
    }
}

/// Initializes a new state machine.
pub fn init() -> Result<OpenStratos<Init>, Error> {
    save_current_state()?;
    Ok(OpenStratos { state: Init })
}

/// States of the onboard computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Initialization.
    Init,
    /// Acquiring GPS fix.
    #[cfg(feature = "gps")]
    AcquiringFix,
    /// GPS fix has been acquired.
    #[cfg(feature = "gps")]
    FixAcquired,
    /// Waiting for balloon launch.
    #[cfg(feature = "gps")]
    WaitingLaunch,
    /// Going up.
    #[cfg(feature = "gps")]
    GoingUp,
    /// Going down.
    #[cfg(feature = "gps")]
    GoingDown,
    /// Probe landed.
    #[cfg(feature = "gps")]
    Landed,
    /// Shutting computer down.
    ShutDown,
    /// Safe mode operation.
    SafeMode,
    /// Eternal loop, without GPS.
    #[cfg(not(feature = "gps"))]
    EternalLoop,
}

impl State {
    /// Gets the last state of the application if there is one.
    pub fn get_last() -> Result<Option<Self>, Error> {
        let path = CONFIG.data_dir().join(STATE_FILE);
        if !path.exists() {
            return Ok(None);
        }
        let mut file = File::open(path).context(error::LastState::FileOpen)?;
        let mut state = String::new();
        let _ = file
            .read_to_string(&mut state)
            .context(error::LastState::FileRead)?;

        if state.is_empty() {
            Ok(None)
        } else {
            Ok(Some(state.parse()?))
        }
    }

    /// Gets the state as a string to be stored in the `LAST_STATE` file.
    pub fn as_str(&self) -> &str {
        match *self {
            State::Init => "INITIALIZING",
            #[cfg(feature = "gps")]
            State::AcquiringFix => "ACQUIRING_FIX",
            #[cfg(feature = "gps")]
            State::FixAcquired => "FIX_ACQUIRED",
            #[cfg(feature = "gps")]
            State::WaitingLaunch => "WAITING_LAUNCH",
            #[cfg(feature = "gps")]
            State::GoingUp => "GOING_UP",
            #[cfg(feature = "gps")]
            State::GoingDown => "GOING_DOWN",
            #[cfg(feature = "gps")]
            State::Landed => "LANDED",
            State::ShutDown => "SHUT_DOWN",
            State::SafeMode => "SAFE_MODE",
            #[cfg(not(feature = "gps"))]
            State::EternalLoop => "ETERNAL_LOOP",
        }
    }
}

impl FromStr for State {
    type Err = error::LastState;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "INITIALIZING" => Ok(State::Init),
            #[cfg(feature = "gps")]
            "ACQUIRING_FIX" => Ok(State::AcquiringFix),
            #[cfg(feature = "gps")]
            "FIX_ACQUIRED" => Ok(State::FixAcquired),
            #[cfg(feature = "gps")]
            "WAITING_LAUNCH" => Ok(State::WaitingLaunch),
            #[cfg(feature = "gps")]
            "GOING_UP" => Ok(State::GoingUp),
            #[cfg(feature = "gps")]
            "GOING_DOWN" => Ok(State::GoingDown),
            #[cfg(feature = "gps")]
            "LANDED" => Ok(State::Landed),
            "SHUT_DOWN" => Ok(State::ShutDown),
            "SAFE_MODE" => Ok(State::SafeMode),
            #[cfg(not(feature = "gps"))]
            "ETERNAL_LOOP" => Ok(State::EternalLoop),
            _ => Err(error::LastState::Invalid {
                state: s.to_owned(),
            }),
        }
    }
}

/// Initialization state.
#[derive(Debug, Clone, Copy)]
pub struct Init;

impl GetState for Init {
    fn get_state(&self) -> State {
        State::Init
    }
}

/// Acquiring fix state.
#[cfg(feature = "gps")]
#[derive(Debug, Clone, Copy)]
pub struct AcquiringFix;

#[cfg(feature = "gps")]
impl GetState for AcquiringFix {
    fn get_state(&self) -> State {
        State::AcquiringFix
    }
}

/// Fix acquired state.
#[cfg(feature = "gps")]
#[derive(Debug, Clone, Copy)]
pub struct FixAcquired;

#[cfg(feature = "gps")]
impl GetState for FixAcquired {
    fn get_state(&self) -> State {
        State::FixAcquired
    }
}

/// Waiting launch state.
#[cfg(feature = "gps")]
#[derive(Debug, Clone, Copy)]
pub struct WaitingLaunch;

#[cfg(feature = "gps")]
impl GetState for WaitingLaunch {
    fn get_state(&self) -> State {
        State::WaitingLaunch
    }
}

/// Going up state.
#[cfg(feature = "gps")]
#[derive(Debug, Clone, Copy)]
pub struct GoingUp;

#[cfg(feature = "gps")]
impl GetState for GoingUp {
    fn get_state(&self) -> State {
        State::GoingUp
    }
}

/// Going down state.
#[cfg(feature = "gps")]
#[derive(Debug, Clone, Copy)]
pub struct GoingDown;

#[cfg(feature = "gps")]
impl GetState for GoingDown {
    fn get_state(&self) -> State {
        State::GoingDown
    }
}

/// Landed state.
#[cfg(feature = "gps")]
#[derive(Debug, Clone, Copy)]
pub struct Landed;

#[cfg(feature = "gps")]
impl GetState for Landed {
    fn get_state(&self) -> State {
        State::Landed
    }
}

/// Shut down state.
#[derive(Debug, Clone, Copy)]
pub struct ShutDown;

impl GetState for ShutDown {
    fn get_state(&self) -> State {
        State::ShutDown
    }
}

/// Safe mode state.
#[derive(Debug, Clone, Copy)]
pub struct SafeMode;

impl GetState for SafeMode {
    fn get_state(&self) -> State {
        State::SafeMode
    }
}

/// Eternal loop state, if no GPS is enabled.
#[cfg(not(feature = "gps"))]
#[derive(Debug, Clone, Copy)]
pub struct EternalLoop;

#[cfg(not(feature = "gps"))]
impl GetState for EternalLoop {
    fn get_state(&self) -> State {
        State::EternalLoop
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "gps"))]
    use super::EternalLoop;
    #[cfg(feature = "gps")]
    use super::{AcquiringFix, FixAcquired, GoingDown, GoingUp, Landed, WaitingLaunch};
    use super::{GetState, Init, SafeMode, ShutDown, State};

    /// Tests if the `Init` state generates the correct `State` enumeration variant in
    /// `get_state()`.
    #[test]
    fn it_get_state_init() {
        let state = Init;
        assert_eq!(state.get_state(), State::Init);
    }

    /// Tests if the `State::Init` is parsed correctly from a string.
    #[test]
    fn it_from_str_init() {
        assert_eq!("INITIALIZING".parse::<State>().unwrap(), State::Init);
    }

    /// Tests that the `State::Init` is translated to *INITIALIZING* as a string.
    #[test]
    fn it_as_str_init() {
        assert_eq!("INITIALIZING", State::Init.as_str());
    }

    /// Tests if the `AcquiringFix` state generates the correct `State` enumeration variant in
    /// `get_state()`.
    #[test]
    #[cfg(feature = "gps")]
    fn it_get_state_acquiring_fix() {
        let state = AcquiringFix;
        assert_eq!(state.get_state(), State::AcquiringFix);
    }

    /// Tests if the `State::AcquiringFix` is parsed correctly from a string.
    #[test]
    #[cfg(feature = "gps")]
    fn it_from_str_acquiring_fix() {
        assert_eq!(
            "ACQUIRING_FIX".parse::<State>().unwrap(),
            State::AcquiringFix
        );
    }

    /// Tests that the `State::AcquiringFix` is not parsed properly if the GPS feature is off.
    #[test]
    #[should_panic]
    #[cfg(not(feature = "gps"))]
    fn it_from_str_acquiring_fix() {
        let _ = "ACQUIRING_FIX".parse::<State>().unwrap();
    }

    /// Tests that the `State::AcquiringFix` is translated to `ACQUIRING_FIX` as a string.
    #[test]
    #[cfg(feature = "gps")]
    fn it_as_str_acquiring_fix() {
        assert_eq!("ACQUIRING_FIX", State::AcquiringFix.as_str());
    }

    /// Tests if the `FixAcquired` state generates the correct `State` enumeration variant in
    /// `get_state()`.
    #[test]
    #[cfg(feature = "gps")]
    fn it_get_state_fix_acquired() {
        let state = FixAcquired;
        assert_eq!(state.get_state(), State::FixAcquired);
    }

    /// Tests if the `State::FixAcquired` is parsed correctly from a string.
    #[test]
    #[cfg(feature = "gps")]
    fn it_from_str_fix_acquired() {
        assert_eq!("FIX_ACQUIRED".parse::<State>().unwrap(), State::FixAcquired);
    }

    /// Tests that the `State::FixAcquired` is not parsed properly if the GPS feature is off.
    #[test]
    #[should_panic]
    #[cfg(not(feature = "gps"))]
    fn it_from_str_fix_acquired() {
        let _ = "FIX_ACQUIRED".parse::<State>().unwrap();
    }

    /// Tests that the `State::FixAcquired` is translated to `FIX_ACQUIRED` as a string.
    #[test]
    #[cfg(feature = "gps")]
    fn it_as_str_fix_acquired() {
        assert_eq!("FIX_ACQUIRED", State::FixAcquired.as_str());
    }

    /// Tests if the `WaitingLaunch` state generates the correct `State` enumeration variant in
    /// `get_state()`.
    #[test]
    #[cfg(feature = "gps")]
    fn it_get_state_waiting_launch() {
        let state = WaitingLaunch;
        assert_eq!(state.get_state(), State::WaitingLaunch);
    }

    /// Tests if the `State::WaitingLaunch` is parsed correctly from a string.
    #[test]
    #[cfg(feature = "gps")]
    fn it_from_str_waiting_launch() {
        assert_eq!(
            "WAITING_LAUNCH".parse::<State>().unwrap(),
            State::WaitingLaunch
        );
    }

    /// Tests that the `State::WaitingLaunch` is not parsed properly if the GPS feature is off.
    #[test]
    #[should_panic]
    #[cfg(not(feature = "gps"))]
    fn it_from_str_waiting_launch() {
        let _ = "WAITING_LAUNCH".parse::<State>().unwrap();
    }

    /// Tests that the `State::WaitingLaunch` is translated to `WAITING_LAUNCH` as a string.
    #[test]
    #[cfg(feature = "gps")]
    fn it_as_str_waiting_launch() {
        assert_eq!("WAITING_LAUNCH", State::WaitingLaunch.as_str());
    }

    /// Tests if the `GoingUp` state generates the correct `State` enumeration variant in
    /// `get_state()`.
    #[test]
    #[cfg(feature = "gps")]
    fn it_get_state_going_up() {
        let state = GoingUp;
        assert_eq!(state.get_state(), State::GoingUp);
    }

    /// Tests if the `State::GoingUp` is parsed correctly from a string.
    #[test]
    #[cfg(feature = "gps")]
    fn it_from_str_going_up() {
        assert_eq!("GOING_UP".parse::<State>().unwrap(), State::GoingUp);
    }

    /// Tests that the `State::GoingUp` is not parsed properly if the GPS feature is off.
    #[test]
    #[should_panic]
    #[cfg(not(feature = "gps"))]
    fn it_from_str_going_up() {
        let _ = "GOING_UP".parse::<State>().unwrap();
    }

    /// Tests that the `State::GoingUp` is translated to `GOING_UP` as a string.
    #[test]
    #[cfg(feature = "gps")]
    fn it_as_str_going_up() {
        assert_eq!("GOING_UP", State::GoingUp.as_str());
    }

    /// Tests if the `GoingDown` state generates the correct `State` enumeration variant in
    /// `get_state()`.
    #[test]
    #[cfg(feature = "gps")]
    fn it_get_state_going_down() {
        let state = GoingDown;
        assert_eq!(state.get_state(), State::GoingDown);
    }

    /// Tests if the `State::GoingDown` is parsed correctly from a string.
    #[test]
    #[cfg(feature = "gps")]
    fn it_from_str_going_down() {
        assert_eq!("GOING_DOWN".parse::<State>().unwrap(), State::GoingDown);
    }

    /// Tests that the `State::GoingDown` is not parsed properly if the GPS feature is off.
    #[test]
    #[should_panic]
    #[cfg(not(feature = "gps"))]
    fn it_from_str_going_down() {
        let _ = "GOING_DOWN".parse::<State>().unwrap();
    }

    /// Tests that the `State::GoingDown` is translated to `GOING_DOWN` as a string.
    #[test]
    #[cfg(feature = "gps")]
    fn it_as_str_going_down() {
        assert_eq!("GOING_DOWN", State::GoingDown.as_str());
    }

    /// Tests if the `Landed` state generates the correct `State` enumeration variant in
    /// `get_state()`.
    #[test]
    #[cfg(feature = "gps")]
    fn it_get_state_landed() {
        let state = Landed;
        assert_eq!(state.get_state(), State::Landed);
    }

    /// Tests if the `State::Landed` is parsed correctly from a string.
    #[test]
    #[cfg(feature = "gps")]
    fn it_from_str_landed() {
        assert_eq!("LANDED".parse::<State>().unwrap(), State::Landed);
    }

    /// Tests that the `State::Landed` is not parsed properly if the GPS feature is off.
    #[test]
    #[should_panic]
    #[cfg(not(feature = "gps"))]
    fn it_from_str_landed() {
        let _ = "LANDED".parse::<State>().unwrap();
    }

    /// Tests that the `State::Landed` is translated to *LANDED* as a string.
    #[test]
    #[cfg(feature = "gps")]
    fn it_as_str_landed() {
        assert_eq!("LANDED", State::Landed.as_str());
    }

    /// Tests if the `ShutDown` state generates the correct `State` enumeration variant in
    /// `get_state()`.
    #[test]
    fn it_get_state_shut_down() {
        let state = ShutDown;
        assert_eq!(state.get_state(), State::ShutDown);
    }

    /// Tests if the `State::ShutDown` is parsed correctly from a string.
    #[test]
    fn it_from_str_shut_down() {
        assert_eq!("SHUT_DOWN".parse::<State>().unwrap(), State::ShutDown);
    }

    /// Tests that the `State::ShutDown` is translated to `SHUT_DOWN` as a string.
    #[test]
    fn it_as_str_shut_down() {
        assert_eq!("SHUT_DOWN", State::ShutDown.as_str());
    }

    /// Tests if the `SafeMode` state generates the correct `State` enumeration variant in
    /// `get_state()`.
    #[test]
    fn it_get_state_safe_mode() {
        let state = SafeMode;
        assert_eq!(state.get_state(), State::SafeMode);
    }

    /// Tests if the `State::SafeMode` is parsed correctly from a string.
    #[test]
    fn it_from_str_safe_mode() {
        assert_eq!("SAFE_MODE".parse::<State>().unwrap(), State::SafeMode);
    }

    /// Tests that the `State::SafeMode` is translated to `SAFE_MODE` as a string.
    #[test]
    fn it_as_str_safe_mode() {
        assert_eq!("SAFE_MODE", State::SafeMode.as_str());
    }

    /// Tests if the `EternalLoop` state generates the correct `State` enumeration variant in
    /// `get_state()`.
    #[test]
    #[cfg(not(feature = "gps"))]
    fn it_get_state_eternal_loop() {
        let state = EternalLoop;
        assert_eq!(state.get_state(), State::EternalLoop);
    }

    /// Tests if the `State::EternalLoop` is parsed correctly from a string.
    #[test]
    #[cfg(not(feature = "gps"))]
    fn it_from_str_eternal_loop() {
        assert_eq!("ETERNAL_LOOP".parse::<State>().unwrap(), State::EternalLoop);
    }

    /// Tests that the `State::EternalLoop` is not parsed properly if the GPS feature is on.
    #[test]
    #[should_panic]
    #[cfg(feature = "gps")]
    fn it_from_str_eternal_loop() {
        let _ = "ETERNAL_LOOP".parse::<State>().unwrap();
    }

    /// Tests that the `State::EternalLoop` is translated to `ETERNAL_LOOP` as a string.
    #[test]
    #[cfg(not(feature = "gps"))]
    fn it_as_str_eternal_loop() {
        assert_eq!("ETERNAL_LOOP", State::EternalLoop.as_str());
    }
}
