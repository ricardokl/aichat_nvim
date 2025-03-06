use nvim_oxi::{
    api::{
        self,
        opts::SetKeymapOpts,
        types::{LogLevel, WindowConfig},
        Buffer,
    },
    Error, Result,
};
use std::sync::{Arc, Mutex};

/// UiSelect provides a floating window UI component for selecting from a list of items
/// This component creates a bordered window with selectable items and keyboard navigation
pub struct UiSelect {
    items: Vec<String>,
}

impl UiSelect {
    /// Creates a new UiSelect instance with the provided items
    ///
    /// # Arguments
    /// * `items` - Vector of strings to display as selectable options
    pub fn new(items: Vec<String>) -> Self {
        Self { items }
    }

    /// Displays the selection UI with the given title and calls the provided callback with the selection
    ///
    /// # Arguments
    /// * `title` - The title to display at the top of the selection window
    /// * `callback` - Function to call with the selected item (or None if cancelled)
    ///
    /// # Returns
    /// * `Result<()>` - Success or error from Neovim operations
    pub fn show_with_callback<F>(self, title: String, callback: F) -> Result<()>
    where
        F: FnOnce(Option<String>) + 'static + Send,
    {
        // Create a buffer for the window
        let mut buffer = api::create_buf(false, true)?;

        // Set buffer lines directly with the items to select from
        buffer.set_lines(0..1, false, self.items.clone())?;

        // Make buffer read-only to prevent editing the options
        buffer.set_option("modifiable", false)?;
        buffer.set_option("buftype", "nofile")?;

        // Calculate window dimensions based on content
        let width = self.items.iter().map(|text| text.len()).max().unwrap_or(20) as u32 + 2;
        let height = self.items.len() as u32;

        // Get the editor dimensions
        let current_window = api::get_current_win();
        let width_editor = current_window.get_width()? as u32;
        let height_editor = current_window.get_height()? as u32;

        // Calculate center position
        let row = (height_editor - height - 1) / 2;
        let col = (width_editor - width) / 2;

        // Create window configuration for the floating window
        let win_config = WindowConfig::builder()
            .relative(api::types::WindowRelativeTo::Editor)
            .width(width)
            .height(height + 1)
            .row(row)
            .col(col)
            .style(api::types::WindowStyle::Minimal)
            .border(api::types::WindowBorder::Rounded)
            .title(api::types::WindowTitle::SimpleString(title.into()))
            .title_pos(api::types::WindowTitlePosition::Center)
            .build();

        // Open the window with our buffer and configuration
        let mut window = api::open_win(&buffer, true, &win_config)?;

        // Set window options for better UX
        window.set_option("cursorline", true)?;
        window.set_option("wrap", false)?;

        // Setup keymappings with callback for interaction with the selection window
        setup_keymappings_with_callback(buffer, self.items.clone(), callback)?;

        Ok(())
    }
}

/// Sets up key mappings with callback for the selection buffer
///
/// # Arguments
/// * `buffer` - The buffer to set mappings for
/// * `items` - The list of selectable items
/// * `callback` - The callback to call with the selection
///
/// # Returns
/// * `Result<()>` - Success or error from Neovim operations
fn setup_keymappings_with_callback<F>(
    mut buffer: Buffer,
    items: Vec<String>,
    callback: F,
) -> Result<()>
where
    F: FnOnce(Option<String>) + 'static + Send,
{
    let callback = Arc::new(Mutex::new(Some(
        Box::new(callback) as Box<dyn FnOnce(Option<String>) + Send>
    )));

    // Clone for Enter key
    let enter_callback = callback.clone();
    let enter_items = items.clone();

    // Handle Enter key (selection) with callback
    buffer.set_keymap(
        api::types::Mode::Normal,
        "<CR>",
        "",
        &SetKeymapOpts::builder()
            .noremap(true)
            .silent(true)
            .callback(move |_| {
                // Get current line number (1-based in Vim)
                let win = api::get_current_win();
                let selected_item = if let Ok((row, _)) = win.get_cursor() {
                    let idx = row as usize - 1; // Convert to 0-based index
                    if idx < enter_items.len() {
                        Some(enter_items[idx].clone())
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Take the callback out of the Arc<Mutex> to call it
                let callback_fn = match enter_callback.lock() {
                    Ok(mut guard) => guard.take(),
                    Err(poisoned) => {
                        // Recover from poisoned mutex
                        api::notify(
                            "Recovering from poisoned mutex",
                            LogLevel::Warn,
                            &Default::default(),
                        )
                        .ok();
                        poisoned.into_inner().take()
                    }
                };

                // Execute the callback with panic protection
                if let Some(cb) = callback_fn {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        cb(selected_item);
                    }));

                    if let Err(e) = result {
                        // Convert panic payload to string for error message
                        let panic_msg = if let Some(s) = e.downcast_ref::<&str>() {
                            (*s).to_string()
                        } else if let Some(s) = e.downcast_ref::<String>() {
                            s.clone()
                        } else {
                            "Unknown panic".to_string()
                        };

                        api::notify(
                            &format!("Panic in selection callback: {}", panic_msg),
                            LogLevel::Error,
                            &Default::default(),
                        )
                        .ok();
                    }
                }

                api::command("q!").ok();
                Ok::<(), Error>(())
            })
            .build(),
    )?;

    // Handle Escape key with similar panic protection
    buffer.set_keymap(
        api::types::Mode::Normal,
        "<Esc>",
        ":q<CR>",
        &SetKeymapOpts::builder().noremap(true).silent(true).build(),
    )?;

    Ok(())
}
