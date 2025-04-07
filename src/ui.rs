use nvim_oxi::api::{
    self,
    opts::SetKeymapOpts,
    types::{Mode::Normal as N, WindowConfig},
    Window,
};
use nvim_oxi::Result;
use std::{cell::RefCell, rc::Rc};

/// Opens and configures a window with the given buffer and configuration
///
/// # Arguments
/// * `buffer` - Buffer to display in the window
/// * `win_config` - Window configuration
///
/// # Returns
/// * `Result<Rc<RefCell<Option<Window>>>>` - Configured window wrapped for callbacks
fn open_configured_window(
    buffer: &api::Buffer,
    win_config: &WindowConfig,
) -> Result<Rc<RefCell<Option<Window>>>> {
    let mut window = api::open_win(buffer, true, win_config)?;

    // Configure window options
    window.set_option("cursorline", true)?;
    window.set_option("wrap", false)?;

    // Wrap window in Rc<RefCell<Option<Window>>> for the callbacks
    Ok(Rc::new(RefCell::new(Some(window))))
}

/// Helper function to set a keymap with common options
///
/// # Arguments
/// * `buffer` - Buffer to set the keymap on
/// * `key` - Key to map
/// * `callback` - Callback function to execute when the key is pressed
///
/// # Returns
/// * `Result<()>` - Success or error from Neovim operations
fn set_normal_keymap<F>(buffer: &mut api::Buffer, key: &str, callback: F) -> Result<()>
where
    F: FnMut(()) + 'static,
{
    buffer.set_keymap(
        N,
        key,
        "",
        &SetKeymapOpts::builder()
            .noremap(true)
            .silent(true)
            .callback(callback)
            .build(),
    )?;
    Ok(())
}

/// UiSelect provides a floating window UI component for selecting from a list of items
/// This component creates a bordered window with selectable items and keyboard navigation
pub struct UiSelect {
    items: Vec<Box<str>>,
}

impl From<Vec<&str>> for UiSelect {
    fn from(items: Vec<&str>) -> Self {
        Self {
            items: items.into_iter().map(Box::from).collect(),
        }
    }
}

impl UiSelect {
    /// Creates a new UiSelect instance with the provided items
    ///
    /// # Arguments
    /// * `items` - Vector of strings to display as selectable options
    pub fn new(items: Vec<String>) -> Self {
        Self {
            items: items.into_iter().map(String::into_boxed_str).collect(),
        }
    }

    /// Creates window configuration for the selection UI
    ///
    /// # Arguments
    /// * `title` - The title to display at the top of the selection window
    ///
    /// # Returns
    /// * `Result<WindowConfig>` - Window configuration
    fn create_window_config(&self, title: &str) -> Result<WindowConfig> {
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

        Ok(win_config)
    }

    /// Creates and configures a buffer for the selection UI
    ///
    /// # Returns
    /// * `Result<api::Buffer>` - Configured buffer
    fn create_configured_buffer(&self) -> Result<api::Buffer> {
        // Create a buffer for the window
        let mut buffer = api::create_buf(false, true)?;

        // Convert Box<str> to String for the API call
        let items_strings: Vec<_> = self.items.iter().map(Box::to_string).collect();

        // Set buffer lines directly with the items to select from
        buffer.set_lines(0..1, false, items_strings)?;

        // Make buffer read-only to prevent editing the options
        buffer.set_option("modifiable", false)?;
        buffer.set_option("buftype", "nofile")?;

        Ok(buffer)
    }

    /// Displays the selection UI with the given title and calls the provided callback with the selection
    ///
    /// # Arguments
    /// * `title` - The title to display at the top of the selection window
    /// * `callback` - Function to call with the selected item (or None if cancelled)
    ///
    /// # Returns
    /// * `Result<()>` - Success or error from Neovim operations
    pub fn show_with_callback<F, E>(self, title: &str, mut callback: F) -> Result<()>
    where
        F: FnMut(String) -> std::result::Result<(), E> + 'static + Send,
        E: Into<nvim_oxi::Error> + 'static,
    {
        // Get window configuration
        let win_config = self.create_window_config(title)?;

        // Create and configure the buffer
        let mut buffer = self.create_configured_buffer()?;

        // Open and configure the window, already wrapped in Rc<RefCell<Option<Window>>>
        let window_rc = open_configured_window(&buffer, &win_config)?;

        let items = self.items.clone();
        let w1 = window_rc.clone();

        // Set Enter key mapping
        set_normal_keymap(&mut buffer, "<CR>", move |_| {
            if let Some(win) = w1.borrow_mut().take() {
                match win.get_cursor() {
                    Ok(cursor) => {
                        let row = cursor.0;
                        match items.get(row - 1) {
                            Some(line) => {
                                if let Err(e) = win.close(false) {
                                    api::err_writeln(&format!("Failed to close window: {e}"));
                                }
                                if let Err(e) = callback(line.to_string()) {
                                    api::err_writeln(&format!("Callback error: {}", e.into()));
                                }
                            }
                            None => {
                                api::err_writeln("No lines found");
                            }
                        }
                    }
                    Err(e) => {
                        api::err_writeln(&format!("Failed to get cursor: {e}"));
                    }
                }
            } else {
                api::err_writeln("No window found");
            }
        })?;

        let w2 = window_rc.clone();

        // Set Escape key mapping
        set_normal_keymap(&mut buffer, "<ESC>", move |_| {
            if let Some(win) = w2.borrow_mut().take() {
                if let Err(e) = win.close(false) {
                    api::err_writeln(&format!("Failed to close window: {e}"));
                }
            } else {
                api::err_writeln("No window found");
            }
        })?;

        Ok(())
    }
}

/// Displays an input prompt and returns user input, or None if cancelled
///
/// # Arguments
/// * `prompt` - The prompt to display before the input field
///
/// # Returns
/// * `Result<Option<String>>` - User input or None if cancelled
pub fn show_input_prompt(prompt: &str) -> Result<Option<String>> {
    let input: String = api::call_function("input", (prompt,))?;
    Ok(if input.is_empty() { None } else { Some(input) })
}
