use nvim_oxi::{
    api::{self, opts::SetKeymapOpts, types::WindowConfig, Error, Window},
    Result,
};
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

        // Set buffer lines directly with the items to select from
        buffer.set_lines(0..1, false, self.items.clone())?;

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
    pub fn show_with_callback<F>(self, title: &str, callback: F) -> Result<()>
    where
        F: FnOnce(String) + 'static + Send,
    {
        // Get window configuration
        let win_config = self.create_window_config(title)?;

        // Create and configure the buffer
        let mut buffer = self.create_configured_buffer()?;

        // Open and configure the window, already wrapped in Rc<RefCell<Option<Window>>>
        let window_rc = open_configured_window(&buffer, &win_config)?;

        let items = self.items.clone();
        let w1 = window_rc.clone();
        let callback_rc = Rc::new(RefCell::new(Some(callback)));
        buffer.set_keymap(
            api::types::Mode::Normal,
            "<CR>",
            "",
            &SetKeymapOpts::builder()
                .noremap(true)
                .silent(true)
                .callback(move |_| {
                    if let Some(win) = w1.borrow_mut().take() {
                        let row = win.get_cursor()?.0;
                        let line = items
                            .get(row - 1)
                            .ok_or(Error::Other("No lines found".into()))?;
                        let _ = win.close(false)?;
                        if let Some(call) = callback_rc.borrow_mut().take() {
                            call(line.to_owned());
                        };
                        Ok(())
                    } else {
                        Err(Error::Other("No window found".into()))
                    }
                })
                .build(),
        )?;

        let w2 = window_rc.clone();
        buffer.set_keymap(
            api::types::Mode::Normal,
            "<ESC>",
            "",
            &SetKeymapOpts::builder()
                .noremap(true)
                .silent(true)
                .callback(move |_| {
                    if let Some(win) = w2.borrow_mut().take() {
                        win.close(false)
                    } else {
                        Err(Error::Other("No window found".into()))
                    }
                })
                .build(),
        )?;
        Ok(())
    }
}
