use nvim_oxi::Result;
use nvim_oxi::{
    api::{
        self,
        opts::{OptionOpts, OptionScope::Local, SetKeymapOpts},
        types::{Mode::Normal as N, WindowConfig},
        Window,
    },
    Array, Dictionary, Function, Object,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

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
    let window = api::open_win(buffer, true, win_config)?;

    // Configure window options
    let opts = OptionOpts::builder().scope(Local).win(&window).build();
    api::set_option_value("cursorline", true, &opts)?;
    api::set_option_value("wrap", false, &opts)?;

    // Wrap window in Rc<RefCell<Option<Window>>> for the callbacks
    Ok(RefCell::new(Some(window)).into())
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

impl From<Vec<String>> for UiSelect {
    fn from(items: Vec<String>) -> Self {
        Self {
            items: items.into_iter().map(String::into_boxed_str).collect(),
        }
    }
}

impl UiSelect {
    /// Creates a new UiSelect instance with the provided items
    ///
    /// # Arguments
    /// * `items` - Any collection that can be converted into UiSelect
    pub fn new<T>(items: T) -> Self
    where
        T: Into<Self>,
    {
        items.into()
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
        let opts = OptionOpts::builder().scope(Local).buffer(&buffer).build();
        api::set_option_value("modifiable", false, &opts)?;
        api::set_option_value("buftype", "nofile", &opts)?;

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
        let win_config = match self.create_window_config(title) {
            Ok(config) => config,
            Err(e) => {
                api::err_writeln(&format!("Failed to create window config: {e}"));
                return Err(e);
            }
        };

        // Create and configure the buffer
        let mut buffer = match self.create_configured_buffer() {
            Ok(buffer) => buffer,
            Err(e) => {
                api::err_writeln(&format!("Failed to create buffer: {e}"));
                return Err(e);
            }
        };

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
pub fn show_input_prompt(prompt: &str) -> Result<Option<Box<str>>> {
    let input: String = api::call_function("input", (prompt,))?;
    Ok(if input.is_empty() {
        None
    } else {
        Some(input.into())
    })
}

/// Options for vim.ui.select() wrapper
pub struct SelectOpts {
    pub prompt: Option<String>,
    pub kind: Option<String>,
}

impl Default for SelectOpts {
    fn default() -> Self {
        Self {
            prompt: Some("Select one of:".to_string()),
            kind: None,
        }
    }
}

/// Wrapper around vim.ui.select() that provides a Rust-friendly interface
///
/// This function calls Neovim's built-in vim.ui.select() which respects user's
/// UI configuration (telescope, fzf, etc.) rather than using our custom floating window.
///
/// # Arguments
/// * `items` - Vector of items to select from
/// * `opts` - Optional configuration (prompt, kind)
/// * `callback` - Function to call with the selected item and its 1-based index
///
/// # Returns
/// * `Result<()>` - Success or error from Neovim operations
pub fn vim_ui_select<F>(items: Vec<String>, opts: Option<SelectOpts>, callback: F) -> Result<()>
where
    F: Fn(Option<String>, Option<usize>) + 'static + Send,
{
    if items.is_empty() {
        callback(None, None);
        return Ok(());
    }

    let opts = opts.unwrap_or_default();

    // Convert items to Lua array - need to build it manually
    let mut items_array = Array::new();
    for item in items.iter() {
        items_array.push(Object::from(item.clone()));
    }

    // Create options dictionary
    let mut opts_dict = Dictionary::new();
    if let Some(prompt) = opts.prompt {
        opts_dict.insert("prompt", Object::from(prompt));
    }
    if let Some(kind) = opts.kind {
        opts_dict.insert("kind", Object::from(kind));
    }

    // Wrap callback in Arc to share it
    let callback = Arc::new(callback);

    // Create a Lua callback function that will call our Rust callback
    // We need to store the callback in a way that Lua can call it
    let callback_wrapper = move |args: nvim_oxi::Array| -> nvim_oxi::Result<()> {
        // vim.ui.select callback receives (choice, idx)
        let nil_obj = Object::nil();
        let choice = args.get(0).unwrap_or(&nil_obj);
        let idx = args.get(1).unwrap_or(&nil_obj);

        let selected_item = if choice.is_nil() {
            None
        } else {
            // Try to convert Object to String using nvim-oxi conversion
            use nvim_oxi::conversion::FromObject;
            String::from_object(choice.clone()).ok()
        };

        let selected_index = if idx.is_nil() {
            None
        } else {
            // Try to convert Object to integer using nvim-oxi conversion
            use nvim_oxi::conversion::FromObject;
            i64::from_object(idx.clone()).ok().map(|i| i as usize)
        };

        callback(selected_item, selected_index);
        Ok(())
    };

    // Convert the Rust callback to a Lua function
    let lua_callback: Function<nvim_oxi::Array, ()> = Function::from_fn(callback_wrapper);

    // Call vim.ui.select with the prepared arguments
    api::call_function::<_, ()>(
        "vim.ui.select",
        (
            items_array,
            Object::from(opts_dict),
            Object::from(lua_callback),
        ),
    )?;

    Ok(())
}
