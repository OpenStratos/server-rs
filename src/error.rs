//! Error module.

use std::fmt;
use std::path::PathBuf;

use STATE_FILE;

/// Errors that happened in a certain part of the logic.
#[derive(Debug, Clone, Copy, Fail)]
pub enum Logic {
    /// Initialization error.
    #[fail(display = "there was an error during the initialization")]
    Init,
}

/// GPS errors.
#[cfg(feature = "gps")]
#[derive(Debug, Clone, Fail)]
pub enum Gps {
    /// GPS initialization error.
    #[fail(display = "an error occurred trying to initialize the GPS module")]
    Init,
    /// The GPS was already initialized when trying to initialize it.
    #[fail(display = "the GPS was already initialized when OpenStratos tried to initialize it")]
    AlreadyInitialized,
    /// Invalid GPS status code.
    #[fail(display = "invalid GPS status: '{}'", status)]
    InvalidStatus {
        /// The invalid GPS status code that was received
        status: String,
    },
}

/// Configuration errors.
#[derive(Debug, Fail)]
pub enum Config {
    /// Error opening the configuration file.
    Open {
        /// The path of the configuration file.
        path: PathBuf,
    },
    /// Error reading the configuration file.
    Read {
        /// The path of the configuration file.
        path: PathBuf,
    },
    /// Invalid TOML found in the configuration file.
    InvalidToml {
        /// The path of the configuration file.
        path: PathBuf,
    },
    /// Invalid configuration options.
    Invalid {
        /// The list of errors in the configuration.
        errors: String,
    },
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Config::Open { path } => write!(
                f,
                "error opening the configuration file at '{}'",
                path.display()
            ),
            Config::Read { path } => write!(
                f,
                "error reading the configuration file at '{}'",
                path.display()
            ),
            Config::InvalidToml { path } => write!(
                f,
                "invalid TOML found in the configuration file at '{}'",
                path.display()
            ),
            Config::Invalid { errors } => write!(f, "the configuration is invalid:\n{}", errors),
        }
    }
}

/// Errors dealing with the file system.
#[derive(Debug, Fail)]
pub enum Fs {
    /// Error initializing the `data` filesystem.
    DataInit,
    /// Error creating a directory.
    DirectoryCreation {
        /// Path to the directory meant to be greated.
        path: PathBuf,
    },
}

impl fmt::Display for Fs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Fs::DataInit => write!(f, "error initializing the 'data' directory"),
            Fs::DirectoryCreation { path } => {
                write!(f, "could not create directory '{}'", path.display())
            }
        }
    }
}

/// Errors handling loggers and logs.
#[derive(Debug, Clone, Copy, Fail)]
pub enum Log {
    /// Error creating a log appender.
    #[fail(display = "error creating the `{}` appender", name)]
    Appender {
        /// The name of the log appender.
        name: &'static str,
    },
    /// Error building the logger.
    #[fail(display = "error building the logger")]
    Build,
}

/// Errors related to reading and modifying the last known state.
#[derive(Debug, Clone, Fail)]
pub enum LastState {
    /// Error opening the last state file.
    FileOpen,
    /// Error reading the last state file.
    FileRead,
    /// Error writing the last state file.
    FileWrite,
    /// Error reading the last state.
    Read,
    /// Invalid last state found.
    Invalid {
        /// The invalid state found.
        state: String,
    },
}

impl fmt::Display for LastState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LastState::FileOpen => {
                write!(f, "error opening the last state file at '{}'", STATE_FILE)
            }
            LastState::FileRead => {
                write!(f, "error reading the last state file at '{}'", STATE_FILE)
            }
            LastState::FileWrite => {
                write!(f, "error writing the last state file at '{}'", STATE_FILE)
            }
            LastState::Read => write!(f, "error reading the last state from '{}'", STATE_FILE),
            LastState::Invalid { state } => write!(f, "the last state '{}' is invalid", state),
        }
    }
}

/// Errors related to the use of the Adafruit FONA module.
#[cfg(feature = "fona")]
#[derive(Debug, Fail)]
pub enum Fona {
    /// Error initializing the FONA module.
    #[fail(display = "there was an error during the initialization of the FONA module")]
    Init,
    /// Error turning the FONA module on.
    #[fail(display = "the FONA module did not turn on")]
    PowerOn,
    /// Error turning the FONA module's "echo" functionality off.
    #[fail(display = "there was an error turning the FONA 'echo' off")]
    EchoOff,
    /// There was no open serial connection when trying to send a command to the FONA module.
    #[fail(
        display = "there was no open serial connection when trying to send a command to the \
                   FONA module"
    )]
    NoSerial,
    /// `EOF` was found in the FONA serial.
    #[fail(display = "EOF was found when reading the FONA serial")]
    SerialEnd,
    /// SMS was too long to be sent.
    #[fail(display = "SMS was longer than the 160 character limit")]
    LongSms,
    /// Error sending SMS on 'AT+CMGD=1' response.
    #[fail(display = "error sending SMS on `AT+CMGD=1` response")]
    SmsAtCmgd,
    /// Error reading +CMGS response.
    #[fail(display = "error reading +CMGS response after sending SMS")]
    SmsCmgs,
    /// No OK received after sending SMS.
    #[fail(display = "no OK received after sending SMS")]
    SmsOk,
    /// Error reading CRLF (*\r\n*) after sending command to FONA.
    #[fail(
        display = "an error occurred when trying to read CRLF (\\r\\n) after sending command to \
                   FONA"
    )]
    SendCommandCrlf,
    /// FONA serial found EOF.
    #[fail(display = "FONA returned a partial response: `{}`", response)]
    PartialResponse {
        /// Contents of the partial response.
        response: String,
    },
    /// Error sending command to FONA.
    #[fail(display = "there was a I/O error when trying to send a command to the FONA module")]
    Command,
    /// Invalid response to AT+CADC? (read ADC) command.
    #[fail(display = "FONA returned an invalid response to AT+CADC?")]
    CADCInvalidResponse,
}

/// Errors related to the Raspicam camera.
#[cfg(feature = "raspicam")]
#[derive(Debug, Fail)]
pub enum Raspicam {
    /// Camera was already recording.
    AlreadyRecording,
    /// Camera output file already exists.
    FileExists {
        /// File that wasn't supposed to exist.
        file: PathBuf,
    },
    /// Camera testing error.
    Test,
    /// Error removing camera test file.
    TestRemove {
        /// Output file for the test.
        test_file: PathBuf,
    },
}

#[cfg(feature = "raspicam")]
impl fmt::Display for Raspicam {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Raspicam::AlreadyRecording => write!(f, "the camera was already recording"),
            Raspicam::FileExists { file } => write!(
                f,
                "the output file {} for the camera already exists",
                file.display()
            ),
            Raspicam::Test => write!(f, "an error occurred when trying to test the camera"),
            Raspicam::TestRemove { test_file } => write!(
                f,
                "there was an error trying to remove the camera test file {}",
                test_file.display()
            ),
        }
    }
}
