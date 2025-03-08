use nvim_oxi::{
    api::{self, opts::SetKeymapOpts, types::WindowConfig, Error, Window},
    Result,
};
use std::{cell::RefCell, rc::Rc};

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
        //let mut window = api::open_win(&buffer, true, &win_config)?;

        // Create a buffer for the window
        let mut buffer = api::create_buf(false, true)?;

        // Set buffer lines directly with the items to select from
        buffer.set_lines(0..1, false, self.items.clone())?;

        // Make buffer read-only to prevent editing the options
        buffer.set_option("modifiable", false)?;
        buffer.set_option("buftype", "nofile")?;

        let window: Rc<RefCell<Option<Window>>> = Rc::new(RefCell::new(Some(api::open_win(
            &buffer,
            true,
            &win_config,
        )?)));

        // Wrap in brackets so that borrow ends
        {
            let mut w = window.borrow_mut();
            if let Some(ref mut win) = w.as_mut() {
                win.set_option("cursorline", true)?;
                win.set_option("wrap", false)?;
            }
        }

        // Set window options for better UX
        let w1 = window.clone();
        let callback_rc = Rc::new(RefCell::new(Some(callback)));
        buffer.set_keymap(
            api::types::Mode::Normal,
            "<CR>",
            "",
            &SetKeymapOpts::builder()
                .noremap(true)
                .silent(true)
                .callback(move |_| {
                    let _ = api::notify(
                        "Inside callback",
                        api::types::LogLevel::Info,
                        &Default::default(),
                    );
                    if let Some(win) = w1.borrow_mut().take() {
                        let _ = api::notify(
                            "Inside window IF statement",
                            api::types::LogLevel::Info,
                            &Default::default(),
                        );
                        let row = win.get_cursor()?.0;
                        let line = self
                            .items
                            .get(row - 1)
                            .ok_or(Error::Other("No lines found".into()))?;
                        //let line: nvim_oxi::String = buffer
                        //    .get_lines(row..row + 1, false)?
                        //    .next()
                        //    .ok_or(Error::Other("No lines found".into()))?;
                        //let trimmed = line.to_string_lossy().trim_end_matches("\n").to_string();
                        let _ = api::notify(
                            &format!("line before close: {}", &line),
                            api::types::LogLevel::Info,
                            &Default::default(),
                        );
                        // Close the current window first to prevent nested window issues
                        let _ = win.close(false);

                        let _ = api::notify(
                            &format!("line after close: {}", &line),
                            api::types::LogLevel::Info,
                            &Default::default(),
                        );
                        if let Some(call) = callback_rc.borrow_mut().take() {
                            call(Some(line.to_owned()));
                        }

                        Ok(())
                    } else {
                        Err(Error::Other("No window found".into()))
                    }
                })
                .build(),
        )?;

        let w2 = window.clone();
        buffer.set_keymap(
            api::types::Mode::Normal,
            "<CR>",
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

        // Setup keymappings with callback for interaction with the selection window
        //setup_keymappings_with_callback(buffer, window, callback)?;

        Ok(())
    }
}
