use crate::error::{AichatError, Result};
use crate::ui;
use nvim_oxi::conversion::{Error as ConversionError, FromObject};
use nvim_oxi::serde::Deserializer;
use nvim_oxi::{
    api::{
        self,
        opts::{OptionOpts, OptionScope::Local, SetKeymapOpts},
    },
    lua, Object,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct AichatConfig {
    pub mode_flag: Mode,
    pub mode_arg: Box<str>,
    pub rag: Option<Box<str>>,
    pub session: Option<Box<str>>,
}

impl Default for AichatConfig {
    fn default() -> Self {
        Self {
            mode_flag: Mode::Role,
            mode_arg: Box::from("sambanova1filecoder"),
            rag: None,
            session: None,
        }
    }
}

impl Clone for AichatConfig {
    fn clone(&self) -> Self {
        Self {
            mode_flag: self.mode_flag,
            mode_arg: self.mode_arg.clone(),
            rag: self.rag.clone(),
            session: self.session.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum Mode {
    Role,
    Agent,
    Macro,
}

impl FromObject for AichatConfig {
    fn from_object(obj: Object) -> std::result::Result<Self, ConversionError> {
        Self::deserialize(Deserializer::new(obj)).map_err(Into::into)
    }
}

impl lua::Poppable for AichatConfig {
    unsafe fn pop(lstate: *mut lua::ffi::State) -> std::result::Result<Self, lua::Error> {
        let obj = Object::pop(lstate)?;
        Self::from_object(obj).map_err(lua::Error::pop_error_from_err::<Self, _>)
    }
}

// Global static to store the config
static CONFIG: Lazy<RwLock<AichatConfig>> = Lazy::new(|| RwLock::new(AichatConfig::default()));

/// Gets a read-only reference to the global configuration
pub fn get_config() -> std::sync::RwLockReadGuard<'static, AichatConfig> {
    CONFIG.read().unwrap_or_else(|e| e.into_inner())
}

/// Gets a mutable reference to the global configuration
pub fn get_config_mut() -> std::sync::RwLockWriteGuard<'static, AichatConfig> {
    CONFIG.write().unwrap_or_else(|e| e.into_inner())
}

/// Fetches available options from the aichat CLI tool
fn fetch_aichat_options(option_type: &str) -> Result<Vec<String>> {
    use std::process::Command;

    // Map option type to the appropriate CLI flag
    let flag = match option_type {
        "roles" => "--list-roles",
        "agents" => "--list-agents",
        "macros" => "--list-macros",
        "sessions" => "--list-sessions",
        "rags" => "--list-rags",
        _ => {
            return Err(AichatError::invalid_option_type(option_type));
        }
    };

    // Execute the aichat command with the appropriate flag
    let output = Command::new("aichat").arg(flag).output()?;

    if !output.status.success() {
        return Err(AichatError::command_failed(output.status, output.stderr));
    }

    // Parse the output into lines
    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut options: Vec<String> = output_str
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Only add unset option for sessions and rags
    if option_type == "sessions" || option_type == "rags" {
        options.push("(unset)".into());
    }

    Ok(options)
}

/// Shows the main configuration menu for aichat
pub fn show_config_menu() -> nvim_oxi::Result<()> {
    let menu_items = vec![
        "Set Role".to_string(),
        "Set Agent".to_string(),
        "Set Macro".to_string(),
        "Set Session".to_string(),
        "Set RAG".to_string(),
    ];

    let opts = ui::SelectOpts {
        prompt: Some("Aichat Configuration".to_string()),
        kind: None,
    };

    ui::vim_ui_select(menu_items, Some(opts), |selection, _index| {
        if let Some(selection) = selection {
            let result = match selection.as_str() {
                "Set Role" => handle_config_selection("roles", Some(Mode::Role)),
                "Set Agent" => handle_config_selection("agents", Some(Mode::Agent)),
                "Set Macro" => handle_config_selection("macros", Some(Mode::Macro)),
                "Set Session" => handle_config_selection("sessions", None),
                "Set RAG" => handle_config_selection("rags", None),
                _ => Ok(()),
            };

            if let Err(e) = result {
                crate::error::notify_error(&e);
            }
        }
    })
}

/// Handles the selection of a specific config option type
fn handle_config_selection(option_type: &str, mode: Option<Mode>) -> Result<()> {
    // Fetch options from aichat CLI
    match fetch_aichat_options(option_type) {
        Ok(options) => {
            // Clone option_type to own it inside the closure
            let option_type_owned: String = option_type.into();

            let opts = ui::SelectOpts {
                prompt: Some(format!("Select {}", option_type)),
                kind: None,
            };

            ui::vim_ui_select(options, Some(opts), move |selection, _index| {
                if let Some(selection) = selection {
                    let result = if selection == "(unset)" {
                        // Unset the config value
                        update_config(&option_type_owned, None, mode)
                    } else {
                        // Set the config value
                        update_config(&option_type_owned, Some(selection), mode)
                    };

                    if let Err(e) = result {
                        crate::error::notify_error(&e);
                    }
                }
            })?;

            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Updates the AichatConfig with the selected value
fn update_config(option_type: &str, value: Option<String>, mode: Option<Mode>) -> Result<()> {
    let mut config = get_config_mut();

    //Notify the user about the change
    let status = if let Some(val) = &value {
        format!("Set {} to: {}", option_type, val)
    } else {
        format!("Unset {}", option_type)
    };

    // Update the configuration based on the option type
    match option_type {
        "roles" | "agents" | "macros" => {
            let mode_val = mode.ok_or_else(|| {
                AichatError::missing_value("Mode must be specified for this option type")
            })?;

            let value_str = value.ok_or_else(|| {
                AichatError::missing_value("Mode argument must exist for this option type")
            })?;

            config.mode_flag = mode_val;
            config.mode_arg = value_str.into_boxed_str();
        }
        "sessions" => {
            config.session = value.map(|s| s.into_boxed_str());
        }
        "rags" => {
            config.rag = value.map(|s| s.into_boxed_str());
        }
        _ => {
            return Err(AichatError::invalid_option_type(option_type));
        }
    }

    //Notify the user about the successful update
    crate::utils::info(&status);

    Ok(())
}

/// Shows the current aichat configuration in a floating window
pub fn show_current_config() -> nvim_oxi::Result<()> {
    // Get the current configuration
    let config = get_config();

    // Create a buffer for the window
    let mut buffer = api::create_buf(false, true)?;

    // Prepare the content lines
    let mut lines = Vec::new();
    lines.push("Current Aichat Configuration:".into());
    lines.push("".into());

    // Add mode configuration
    let mode_str = match config.mode_flag {
        Mode::Role => "Role",
        Mode::Agent => "Agent",
        Mode::Macro => "Macro",
    };
    lines.push(format!("Mode: {} - {}", mode_str, config.mode_arg));

    // Add RAG configuration
    if let Some(rag) = &config.rag {
        lines.push(format!("RAG: {}", rag));
    } else {
        lines.push("RAG: Not set".into());
    }

    // Add session configuration
    if let Some(session) = &config.session {
        lines.push(format!("Session: {}", session));
    } else {
        lines.push("Session: Not set".into());
    }

    // Calculate window dimensions
    let width = 50;
    let height = lines.len() as u32;

    // Set buffer lines
    buffer.set_lines(0..0, false, lines)?;

    // Make buffer read-only
    let opts = OptionOpts::builder().scope(Local).buffer(&buffer).build();
    api::set_option_value("modifiable", false, &opts)?;
    api::set_option_value("buftype", "nofile", &opts)?;

    // Get editor dimensions
    let current_window = api::get_current_win();
    let width_editor = current_window.get_width()? as u32;
    let height_editor = current_window.get_height()? as u32;

    // Calculate center position
    let row = (height_editor - height) / 2;
    let col = (width_editor - width) / 2;

    // Create window configuration
    let win_config = api::types::WindowConfig::builder()
        .relative(api::types::WindowRelativeTo::Editor)
        .width(width)
        .height(height)
        .row(row)
        .col(col)
        .style(api::types::WindowStyle::Minimal)
        .border(api::types::WindowBorder::Rounded)
        .title(api::types::WindowTitle::SimpleString(
            "Aichat Configuration".into(),
        ))
        .title_pos(api::types::WindowTitlePosition::Center)
        .build();

    // Open the window
    let window = api::open_win(&buffer, true, &win_config)?;

    // Set window options
    api::set_option_value(
        "cursorline",
        false,
        &OptionOpts::builder().scope(Local).win(&window).build(),
    )?;

    // Add a keymap to close the window with any key
    buffer.set_keymap(
        api::types::Mode::Normal,
        "<Esc>",
        ":q<CR>",
        &SetKeymapOpts::builder().noremap(true).silent(true).build(),
    )?;

    buffer.set_keymap(
        api::types::Mode::Normal,
        "q",
        ":q<CR>",
        &SetKeymapOpts::builder().noremap(true).silent(true).build(),
    )?;

    Ok(())
}
