use nvim_oxi::{
    api::{self, opts::SetKeymapOpts, types::WindowConfig, Buffer},
    Error, Result,
};

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

    /// Displays the selection UI with the given title
    ///
    /// # Arguments
    /// * `title` - The title to display at the top of the selection window
    ///
    /// # Returns
    /// * `Result<()>` - Success or error from Neovim operations
    pub fn show(self, title: String) -> Result<()> {
        // Create a buffer for the window
        // Parameters: not listed (false), scratch buffer (true)
        let mut buffer = api::create_buf(false, true)?;

        // Set buffer lines directly with the items to select from
        // Replace lines 0..1 (empty buffer) with our items
        buffer.set_lines(0..1, false, self.items.clone())?;

        // Make buffer read-only to prevent editing the options
        buffer.set_option("modifiable", false)?;
        buffer.set_option("buftype", "nofile")?;

        // Calculate window dimensions based on content
        // Width is determined by the longest item plus padding
        let width = self.items.iter().map(|text| text.len()).max().unwrap_or(20) as u32 + 2;
        // Height matches the number of items
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
            .relative(api::types::WindowRelativeTo::Editor) // Position relative to editor
            .width(width)
            .height(height + 1) // Add 1 for the title
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

        // Create a variable to store the selection result
        // This will be populated when user makes a selection
        api::set_var("ui_select_result", "")?;

        // Setup keymappings for interaction with the selection window
        setup_keymappings_basic(buffer)?;
        return Ok(());
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

/// Sets up basic key mappings for the selection buffer (for show method)
///
/// # Arguments
/// * `buffer` - The buffer to set mappings for
///
/// # Returns
/// * `Result<()>` - Success or error from Neovim operations
fn setup_keymappings_basic(mut buffer: Buffer) -> Result<()> {
    // Handle Enter key (selection)
    // When pressed:
    // 1. Store current line text in g:ui_select_result
    // 2. Close the window
    buffer.set_keymap(
        api::types::Mode::Normal,
        "<CR>",
        ":let g:ui_select_result = getline('.')<CR>:q!<CR>",
        &SetKeymapOpts::builder().noremap(true).silent(true).build(),
    )?;

    // Handle Escape key (cancel)
    // When pressed: Close the window without making a selection
    buffer.set_keymap(
        api::types::Mode::Normal,
        "<Esc>",
        ":q!<CR>",
        &SetKeymapOpts::builder().noremap(true).silent(true).build(),
    )?;
    Ok(())
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
    // We need to wrap the callback in a Box and then in an Arc to share it between closures
    use std::sync::{Arc, Mutex};
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
                if let Ok((row, _)) = win.get_cursor() {
                    let idx = row as usize - 1; // Convert to 0-based index
                    if idx < enter_items.len() {
                        // Take the callback out of the Arc<Mutex> to call it
                        if let Ok(mut cb_guard) = enter_callback.lock() {
                            if let Some(cb) = cb_guard.take() {
                                // Close the window first to avoid UI issues
                                api::command("q!").ok();
                                // Call the callback with the selected item
                                cb(Some(enter_items[idx].clone()));
                                return Ok::<(), Error>(());
                            }
                        }
                    }
                }

                // Close the window if we couldn't call the callback
                api::command("q!").ok();
                Ok(())
            })
            .build(),
    )?;

    // Clone for Escape key
    let esc_callback = callback;

    // Handle Escape key (cancel) with callback
    buffer.set_keymap(
        api::types::Mode::Normal,
        "<Esc>",
        "",
        &SetKeymapOpts::builder()
            .noremap(true)
            .silent(true)
            .callback(move |_| {
                // Take the callback out of the Arc<Mutex> to call it
                if let Ok(mut cb_guard) = esc_callback.lock() {
                    if let Some(cb) = cb_guard.take() {
                        // Close the window first to avoid UI issues
                        api::command("q!").ok();
                        // Call the callback with None to indicate cancellation
                        cb(None);
                        return Ok::<(), Error>(());
                    }
                }

                // Close the window if we couldn't call the callback
                api::command("q!").ok();
                Ok(())
            })
            .build(),
    )?;

    Ok(())
}

/// Public API function to create and show a selection UI
///
/// # Arguments
/// * `title` - The title for the selection window
/// * `items` - Vector of strings to display as selectable options
///
/// # Returns
/// * `Result<()>` - Success or error from Neovim operations
pub fn ui_select(title: &str, items: Vec<String>) -> Result<()> {
    let ui = UiSelect::new(items);
    ui.show(title.into())
}
