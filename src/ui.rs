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

    pub fn show(self, title: String) -> Result<()> {
        // Create a buffer for the window
        let mut buffer = api::create_buf(false, true)?;

        // Set buffer lines directly
        buffer.set_lines(0..1, false, self.items.clone())?;

        // Make buffer read-only
        buffer.set_option("modifiable", false)?;
        buffer.set_option("buftype", "nofile")?;

        // Calculate window dimensions
        let width = self.items.iter().map(|text| text.len()).max().unwrap_or(20) as u32 + 2;
        let height = self.items.len() as u32;

        // Create window configuration
        let win_config = WindowConfig::builder()
            .relative(api::types::WindowRelativeTo::Editor)
            .width(width)
            .height(height + 1) // Add 1 for the title
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

        // Create a variable to store the selection
        api::set_var("ui_select_result", "")?;

        // Setup keymappings that will store selection in a variable
        setup_keymappings(buffer)?;
        return Ok(());
    }
}

fn setup_keymappings(mut buffer: Buffer) -> Result<()> {
    // Handle Enter key (selection)
    buffer.set_keymap(
        api::types::Mode::Normal,
        "<CR>",
        ":let g:ui_select_result = getline('.')<CR>:q!<CR>",
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
pub fn ui_select(title: &str, items: Vec<String>) -> Result<()> {
    let ui = UiSelect::new(items);
    ui.show(title.into())
}
