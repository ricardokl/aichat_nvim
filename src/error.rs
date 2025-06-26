use nvim_oxi::{api, Error as NvimOxiError};
use std::process::ExitStatus;
use thiserror::Error;

/// Main error type for the aichat_nvim plugin
#[derive(Error, Debug)]
pub enum AichatError {
    /// Neovim API related errors
    #[error("Neovim API error: {0}")]
    NvimApi(#[from] NvimOxiError),

    /// Process execution errors
    #[error("Failed to execute aichat command: {0}")]
    ProcessExecution(#[from] std::io::Error),

    /// Command execution failed with non-zero exit status
    #[error("Aichat command failed with exit status: {status}. stderr: {stderr}")]
    CommandFailed { status: ExitStatus, stderr: String },

    /// Configuration related errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Invalid option type provided
    #[error("Invalid option type: {0}")]
    InvalidOptionType(String),

    /// Required value is missing
    #[error("Missing required value: {0}")]
    MissingValue(String),

    /// No code block found in output
    #[error("No code block found in aichat output")]
    NoCodeBlock,

    /// No lines found in buffer
    #[error("No lines found in the current buffer selection")]
    NoLines,

    /// UTF-8 conversion error
    #[error("UTF-8 conversion error: {0}")]
    Utf8Conversion(#[from] std::string::FromUtf8Error),

    /// String conversion error from UTF-8 lossy
    #[error("String conversion error: {0}")]
    StringConversion(String),

    /// Generic application error
    #[error("Application error: {0}")]
    Application(String),
}

impl AichatError {
    /// Creates a configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Creates an invalid option type error
    pub fn invalid_option_type(option_type: impl Into<String>) -> Self {
        Self::InvalidOptionType(option_type.into())
    }

    /// Creates a missing value error
    pub fn missing_value(description: impl Into<String>) -> Self {
        Self::MissingValue(description.into())
    }

    /// Creates an application error
    pub fn application(msg: impl Into<String>) -> Self {
        Self::Application(msg.into())
    }

    /// Creates a command failed error from process output
    pub fn command_failed(status: ExitStatus, stderr: Vec<u8>) -> Self {
        let stderr_str = String::from_utf8_lossy(&stderr).to_string();
        Self::CommandFailed {
            status,
            stderr: stderr_str,
        }
    }

    /// Creates a string conversion error
    pub fn string_conversion(msg: impl Into<String>) -> Self {
        Self::StringConversion(msg.into())
    }
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, AichatError>;

/// Converts AichatError to nvim_oxi::Error for compatibility with nvim-oxi functions
impl From<AichatError> for NvimOxiError {
    fn from(err: AichatError) -> Self {
        match err {
            AichatError::NvimApi(nvim_err) => nvim_err,
            other => NvimOxiError::Api(api::Error::Other(other.to_string().into())),
        }
    }
}

/// Utility function to notify user about errors
/// This should be called at the boundary where errors are finally handled
pub fn notify_error(err: &AichatError) {
    let _ = crate::utils::error(&err.to_string());
}

// /// Utility function to convert Result<T, AichatError> to nvim_oxi::Result<T>
// /// and notify the user about the error
// pub fn handle_error<T>(result: Result<T>) -> nvim_oxi::Result<T> {
//     match result {
//         Ok(value) => Ok(value),
//         Err(err) => {
//             notify_error(&err);
//             Err(err.into())
//         }
//     }
// }

// /// Utility function to convert Result<T, AichatError> to nvim_oxi::Result<()>
// /// This is useful for functions that don't need to return a value but need to handle errors
// pub fn handle_error_unit(result: Result<()>) -> nvim_oxi::Result<()> {
//     handle_error(result)
// }

