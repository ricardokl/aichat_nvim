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
