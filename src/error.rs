//! Error module.

use std::path::PathBuf;

use STATE_FILE;

error_chain!{
    foreign_links {
        Io(::std::io::Error) #[doc = "Standard library I/O error."];
        Toml(::toml::de::Error) #[doc = "TOML deserializing error."];
        Log(::log4rs::Error) #[doc = "Log error."];
        LogSet(::log::SetLoggerError) #[doc = "Error setting up logger."];
        LogConfig(::log4rs::config::Errors) #[doc = "Logger configuration error."];
        FromUTF8(::std::string::FromUtf8Error) #[doc = "Error creating a String from UTF-8 data."];
        NulError(::std::ffi::NulError) #[doc = "A nul byte was found in the vector provided."];
        GPIO(::sysfs_gpio::Error) #[doc = "A GPIO library error."];
        SerialPort(::serialport::Error) #[doc = "Serial port error"];
    }

    errors {
        /// Invalid GPS status.
        #[cfg(feature = "gps")]
        GPSInvalidStatus(s: String) {
            description("invalid GPS status")
            display("invalid GPS status: '{}'", s)
        }

        /// GPS initialization error.
        #[cfg(feature = "gps")]
        GPSInit {
            description("GPS initialization error")
            display("an error occurred trying to initialize the GPS module")
        }

        /// GPS was already initialized when trying to initialize it.
        #[cfg(feature = "gps")]
        GpsAlreadyInitialized {
            description("GPS was already initialized")
            display("GPS was already initialized when trying to initialize it")
        }

        /// Error opening configuration file.
        ConfigOpen(path: PathBuf) {
            description("error opening config file")
            display("error opening the config file at {}", path.display())
        }

        /// Error reading configuration file.
        ConfigRead(path: PathBuf) {
            description("error reading config file")
            display("error reading the config file at {}", path.display())
        }

        /// Invalid TOML in configuration file.
        ConfigInvalidToml(path: PathBuf) {
            description("error reading config file")
            display("error reading the config file at {}", path.display())
        }

        /// Invalid configuration options.
        ConfigInvalid(errors: String) {
            description("invalid configuration")
            display("the configuration is invalid:\n{}", errors)
        }

        /// Error initializing the `data` filesystem.
        DataFSInit {
            description("the camera was already recording")
            display("the camera was already recording")
        }

        /// Error creating a log appender.
        LogAppender(appender_name: &'static str) {
            description("error creating log appender")
            display("error creating `{}` log appender", appender_name)
        }

        /// Error building the logger.
        LogBuild {
            display("error building the logger")
            description("error building the logger")
        }

        /// Error creating a directory.
        DirectoryCreation(path: PathBuf) {
            description("could not create directory")
            display("could not create directory '{}'", path.display())
        }

        /// Error reading last state.
        LastStateRead {
            description("error reading last state")
            display("error reading last state from {}", STATE_FILE)
        }

        /// Error opening last state file.
        LastStateFileOpen {
            description("error opening last state file")
            display("error opening last state file {}", STATE_FILE)
        }

        /// Error reading last state file.
        LastStateFileRead {
            description("error reading last state file")
            display("error reading last state file {}", STATE_FILE)
        }

        /// Error writing last state file.
        LastStateFileWrite {
            description("error writing last state file")
            display("error writing last state file {}", STATE_FILE)
        }

        /// Invalid last state.
        InvalidState(state: String) {
            description("the last state is invalid")
            display("the last state '{}' is invalid", state)
        }

        /// Initialization error.
        Init {
            description("initialization error")
            display("there was an error during the initialization")
        }

        /// FONA initialization error.
        #[cfg(feature = "fona")]
        FonaInit {
            description("FONA initialization error")
            display("FONA module initialization error")
        }

        /// Error turning FONA echo off.
        #[cfg(feature = "fona")]
        FonaNoEchoOff {
            description("error turning FONA echo off")
            display("there was an error turning FONA echo off")
        }

        /// FONA could not be turned on.
        #[cfg(feature = "fona")]
        FonaInitNoPowerOn {
            description("FONA could not be turned on")
            display("FONA did not turn on after doing the turn on process")
        }

        /// No FONA when trying to send command.
        #[cfg(feature = "fona")]
        FonaNoSerial {
            description("no open serial connection when sending command to FONA")
            display("there was no open serial connection when trying to send command to FONA")
        }

        /// FONA serial found EOF.
        #[cfg(feature = "fona")]
        FonaSerialEnd {
            description("FONA serial found EOF")
            display("EOF was found when reading the FONA serial")
        }

        /// SMS was too long to be sent.
        #[cfg(feature = "fona")]
        FonaLongSms {
            description("SMS was too long")
            display("SMS was longer than the 160 character limit")
        }

        /// Error sending SMS on 'AT+CMGD=1' response.
        #[cfg(feature = "fona")]
        FonaSmsAtCmgd {
            description("error sending SMS on `AT+CMGD=1` response")
            display("error sending SMS on `AT+CMGD=1` response")
        }

        /// FONA serial found EOF.
        #[cfg(feature = "fona")]
        FonaPartialResponse (res: String){
            description("FONA returned partial response")
            display("FONA returned a partial response: `{}`", res)
        }

        /// Error sending command to FONA.
        #[cfg(feature = "fona")]
        FonaCommand {
            description("error sending command to FONA")
            display("there was a I/O error when trying to send a command to the FONA module")
        }

        /// Camera was already recording.
        #[cfg(feature = "raspicam")]
        CameraAlreadyRecording {
            description("the camera was already recording")
            display("the camera was already recording")
        }

        /// Camera output file already exists.
        #[cfg(feature = "raspicam")]
        CameraFileExists(file: PathBuf) {
            description("the output file for the camera already exists")
            display("the output file {} for the camera already exists", file.display())
        }

        /// Camera testing error.
        #[cfg(feature = "raspicam")]
        CameraTest {
            description("camera test error")
            display("an error occurred when trying to test the camera")
        }

        /// Error removing camera test file.
        #[cfg(feature = "raspicam")]
        CameraTestRemove(test_file: PathBuf) {
            description("error removing camera test file")
            display("there was an error trying to remove the camera test file {}",
                    test_file.display())
        }
    }
}
