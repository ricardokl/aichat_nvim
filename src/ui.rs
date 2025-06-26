use nvim_oxi::Result;
use nvim_oxi::{
    api::{self},
    Array, Dictionary, Function, Object,
};
use std::sync::Arc;

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
#[derive(Debug, Clone)]
pub struct SelectOpts {
    pub prompt: Option<String>,
    pub kind: Option<String>,
}

impl SelectOpts {
    /// Create new SelectOpts with a prompt
    pub fn with_prompt(prompt: impl Into<String>) -> Self {
        Self {
            prompt: Some(prompt.into()),
            kind: None,
        }
    }

    /// Create new SelectOpts with both prompt and kind
    pub fn new(prompt: impl Into<String>, kind: Option<impl Into<String>>) -> Self {
        Self {
            prompt: Some(prompt.into()),
            kind: kind.map(|k| k.into()),
        }
    }
}

impl Default for SelectOpts {
    fn default() -> Self {
        Self {
            prompt: Some("Select one of:".into()),
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
/// * `items` - Vector of items to select from (accepts both String and &str)
/// * `opts` - Optional configuration (prompt, kind)
/// * `callback` - Function to call with the selected item and its 1-based index
///
/// # Returns
/// * `Result<()>` - Success or error from Neovim operations
///
/// # Examples
/// ```rust
/// // Using &str (no .to_string() needed!)
/// let items = vec!["Option 1", "Option 2", "Option 3"];
/// let opts = SelectOpts::with_prompt("Choose an option");
/// vim_ui_select(items, Some(opts), |selection, _index| {
///     // Handle selection
/// })?;
///
/// // Using String (still works)
/// let items = vec!["Option 1".to_string(), "Option 2".to_string()];
/// vim_ui_select(items, None, |selection, _index| {
///     // Handle selection
/// })?;
///
/// // Using slice with convenience function
/// let items = ["A", "B", "C"];
/// vim_ui_select_slice(&items, None, |selection, _index| {
///     // Handle selection
/// })?;
/// ```
pub fn vim_ui_select<T, F>(items: Vec<T>, opts: Option<SelectOpts>, callback: F) -> Result<()>
where
    T: AsRef<str> + Clone + Send + 'static,
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
        items_array.push(Object::from(item.as_ref()));
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

/// Convenience function for vim_ui_select that accepts a slice of string-like items
///
/// # Arguments
/// * `items` - Slice of items to select from (accepts both &[String] and &[&str])
/// * `opts` - Optional configuration (prompt, kind)
/// * `callback` - Function to call with the selected item and its 1-based index
///
/// # Returns
/// * `Result<()>` - Success or error from Neovim operations
pub fn vim_ui_select_slice<T, F>(items: &[T], opts: Option<SelectOpts>, callback: F) -> Result<()>
where
    T: AsRef<str> + Clone + Send + 'static,
    F: Fn(Option<String>, Option<usize>) + 'static + Send,
{
    vim_ui_select(items.to_vec(), opts, callback)
}
