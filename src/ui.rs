use nvim_oxi::{
    api::{self, opts::SetKeymapOpts, types::WindowConfig, Buffer},
    Result,
};

pub struct UiSelect {
    items: Vec<String>, // Single string per item
}

impl UiSelect {
    pub fn new(items: Vec<String>) -> Self {
        Self { items }
    }

    pub fn show(self, title: String) -> Result<Option<String>> {
        // Create a buffer for the window
        let mut buffer = api::create_buf(false, true)?;

        // Set buffer lines directly
        buffer.set_lines(0..1, false, self.items.clone())?;

        // Make buffer read-only
        buffer.set_option("modifiable", false)?;
        buffer.set_option("buftype", "nofile")?;

        // Calculate window dimensions
        let width = self.items.iter().map(|text| text.len()).max().unwrap_or(20) as u32 + 2;
        let height = std::cmp::min(self.items.len(), 10) as u32;

        // Create window configuration
        let win_config = WindowConfig::builder()
            .relative(api::types::WindowRelativeTo::Editor)
            .width(width)
            .height(height)
            .row(3)
            .col(3)
            .style(api::types::WindowStyle::Minimal)
            .border(api::types::WindowBorder::Rounded)
            .title(api::types::WindowTitle::SimpleString(title.into()))
            .title_pos(api::types::WindowTitlePosition::Center)
            .build();

        // Open the window
        let mut window = api::open_win(&buffer, true, &win_config)?;

        // Set window options
        window.set_option("cursorline", true)?;
        window.set_option("wrap", false)?;

        // Setup keymappings that will store selection in z register
        setup_keymappings(buffer)?;

        // Clear register z before opening the window
        api::set_reg("z", "", api::types::RegType::Characterwise)?;

        // Wait for the window to close
        let window_id = window.get_number()?;
        while api::win_is_valid(window_id) {
            api::command("redraw")?;
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        // Check if anything was selected (stored in register z)
        let selection = api::get_reg("z")?;
        if selection.is_empty() {
            Ok(None)
        } else {
            // Find the item that matches the selection
            let selected_text = selection.trim();
            Ok(self
                .items
                .iter()
                .find(|&item| item == selected_text)
                .cloned())
        }
    }
}

fn setup_keymappings(mut buffer: Buffer) -> Result<()> {
    // Handle Enter key (selection)
    buffer.set_keymap(
        api::types::Mode::Normal,
        "<CR>",
        "0\"zY:q!<CR>",
        &SetKeymapOpts::builder().noremap(true).silent(true).build(),
    )?;

    // Handle Escape key (cancel)
    buffer.set_keymap(
        api::types::Mode::Normal,
        "<Esc>",
        ":q!<CR>",
        &SetKeymapOpts::builder().noremap(true).silent(true).build(),
    )?;
    Ok(())
}

// Public API
pub fn ui_select(title: &str, items: Vec<String>) -> Result<Option<String>> {
    let ui = UiSelect::new(items);
    ui.show(title.into())
}
