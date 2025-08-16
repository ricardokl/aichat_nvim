use nvim_oxi::{self as nvim, Result};
use std::thread;

/// Runs a blocking function on a background thread and schedules a callback
/// on the main Neovim thread to handle the result.
///
/// # Arguments
///
/// * `work` - The closure to run on the background thread. It must be `Send`
///   and `'static`. It should return a `Result<T>`.
///
/// * `callback` - The closure to run on the main thread with the result of
///   `work`. It must also be `Send` and `'static`.
pub fn run_async<F, C, T>(work: F, callback: C)
where
    F: FnOnce() -> Result<T> + Send + 'static,
    C: FnOnce(Result<T>) -> Result<()> + Send + 'static,
    T: Send + 'static,
{
    thread::spawn(move || {
        let result = work();
        // This schedules the callback to be executed on the main Neovim thread.
        // The `|_|` is because `nvim::schedule` expects a closure that takes `()`
        // as an argument.
        nvim::schedule(move |_| {
            // We ignore the result of the callback. If it fails, the error
            // should be handled inside the callback itself (e.g., by
            // notifying the user).
            let _ = callback(result);
        });
    });
}
