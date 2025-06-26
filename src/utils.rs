use nvim_oxi::api::{self, types::LogLevel};

/// Utility functions for common Neovim operations

/// Shows an info notification to the user
///
/// # Arguments
/// * `msg` - The message to display
pub fn info(msg: &str) {
    let _ = api::notify(msg, LogLevel::Info, &Default::default());
}

/// Shows an error notification to the user
///
/// # Arguments
/// * `msg` - The error message to display
pub fn error(msg: &str) {
    let _ = api::notify(msg, LogLevel::Error, &Default::default());
}

/// Shows a warning notification to the user
///
/// # Arguments
/// * `msg` - The warning message to display
#[allow(dead_code)]
pub fn warn(msg: &str) {
    let _ = api::notify(msg, LogLevel::Warn, &Default::default());
}

/// Shows a debug notification to the user
///
/// # Arguments
/// * `msg` - The debug message to display
#[allow(dead_code)]
pub fn debug(msg: &str) {
    let _ = api::notify(msg, LogLevel::Debug, &Default::default());
}

/// Shows a trace notification to the user
///
/// # Arguments
/// * `msg` - The trace message to display
#[allow(dead_code)]
pub fn trace(msg: &str) {
    let _ = api::notify(msg, LogLevel::Trace, &Default::default());
}

