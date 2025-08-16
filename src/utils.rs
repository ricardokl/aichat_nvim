use nvim_oxi::print;

/// Utility functions for common Neovim operations

/// Shows an info notification to the user
///
/// # Arguments
/// * `msg` - The message to display
pub fn info(msg: &str) {
    print!("[INFO] {}", msg);
}

/// Shows an error notification to the user
///
/// # Arguments
/// * `msg` - The error message to display
pub fn error(msg: &str) {
    print!("[ERROR] {}", msg);
}

/// Shows a warning notification to the user
///
/// # Arguments
/// * `msg` - The warning message to display
#[allow(dead_code)]
pub fn warn(msg: &str) {
    print!("[WARN] {}", msg);
}

/// Shows a debug notification to the user
///
/// # Arguments
/// * `msg` - The debug message to display
#[allow(dead_code)]
pub fn debug(msg: &str) {
    print!("[DEBUG] {}", msg);
}

/// Shows a trace notification to the user
///
/// # Arguments
/// * `msg` - The trace message to display
#[allow(dead_code)]
pub fn trace(msg: &str) {
    print!("[TRACE] {}", msg);
}
