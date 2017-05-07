//! Error module.

use std::path::PathBuf;

use STATE_FILE;

error_chain!{
    foreign_links {
        Io(::std::io::Error) #[allow(missing_docs)];
        Toml(::toml::de::Error) #[allow(missing_docs)];
        Log(::log4rs::Error) #[allow(missing_docs)];
        LogSet(::log::SetLoggerError) #[allow(missing_docs)];
        LogConfig(::log4rs::config::Errors) #[allow(missing_docs)];
        FromUTF8(::std::string::FromUtf8Error) #[allow(missing_docs)];
    }

    errors {
        /// Invalid GPS status.
        GPSInvalidStatus(s: String) {
            description("invalid GPS status")
            display("invalid GPS status: '{}'", s)
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

        /// Error initializing the data/ filesystem.
        DataFSInit {
            description("the camera was already recording")
            display("the camera was already recording")
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

        /// Invalid last state.
        InvalidState(state: String) {
            description("the last state is invalid")
            display("the last state '{}' is invalid", state)
        }

        /// Camera was already recording
        CameraAlreadyRecording {
            description("the camera was already recording")
            display("the camera was already recording")
        }

        /// Camera was already recording
        CameraFileExists(file: PathBuf) {
            description("the output file for the camera already exists")
            display("the output file {} for the camera already exists", file.display())
        }
    }
}
