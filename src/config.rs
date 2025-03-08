use crate::ui;
use nvim_oxi::api::types::LogLevel;
use nvim_oxi::conversion::{Error as ConversionError, FromObject, ToObject};
use nvim_oxi::serde::{Deserializer, Serializer};
use nvim_oxi::{api, lua, Error, Object};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Serialize, Deserialize)]
pub struct AichatConfig {
    pub mode_flag: Option<Mode>,
    pub mode_arg: Option<Box<str>>,
    pub rag: Option<Box<str>>,
    pub session: Option<Box<str>>,
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
    fn from_object(obj: Object) -> Result<Self, ConversionError> {
        Self::deserialize(Deserializer::new(obj)).map_err(Into::into)
    }
}

impl ToObject for AichatConfig {
    fn to_object(self) -> Result<Object, ConversionError> {
        self.serialize(Serializer::new()).map_err(Into::into)
    }
}

impl lua::Poppable for AichatConfig {
    unsafe fn pop(lstate: *mut lua::ffi::State) -> Result<Self, lua::Error> {
        let obj = Object::pop(lstate)?;
        Self::from_object(obj).map_err(lua::Error::pop_error_from_err::<Self, _>)
    }
}

impl lua::Pushable for AichatConfig {
    unsafe fn push(self, lstate: *mut lua::ffi::State) -> Result<std::ffi::c_int, lua::Error> {
        self.to_object()
            .map_err(lua::Error::push_error_from_err::<Self, _>)?
            .push(lstate)
    }
}

// Global static to store the config
static CONFIG: Lazy<Mutex<AichatConfig>> = Lazy::new(|| {
    Mutex::new(AichatConfig {
        mode_flag: None,
        mode_arg: None,
        rag: None,
        session: None,
    })
});

/// Gets a reference to the global configuration
///
/// Returns a guard that will automatically unlock the mutex when dropped
pub fn get_config() -> std::sync::MutexGuard<'static, AichatConfig> {
    match CONFIG.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            // Recover from poisoned state
            poisoned.into_inner()
        }
    }
}

/// Fetches available options from the aichat CLI tool
fn fetch_aichat_options(option_type: &str) -> nvim_oxi::Result<Vec<String>> {
    use std::process::Command;

    // Map option type to the appropriate CLI flag
    let flag = match option_type {
        "roles" => "--list-roles",
        "agents" => "--list-agents",
        "macros" => "--list-macros",
        "sessions" => "--list-sessions",
        "rags" => "--list-rags",
        _ => {
            let error_msg = "Invalid option type";
            api::notify(error_msg, LogLevel::Error, &Default::default()).ok();
            return Err(Error::Api(api::Error::Other(error_msg.into())));
        }
    };

    // Execute the aichat command with the appropriate flag
    let output = match Command::new("aichat").arg(flag).output() {
        Ok(output) => output,
        Err(e) => {
            let error_msg = format!("Failed to execute aichat: {}", e);
            api::notify(&error_msg, LogLevel::Error, &Default::default()).ok();
            return Err(Error::Api(api::Error::Other(error_msg.into())));
        }
    };

    if !output.status.success() {
        let error_msg = format!(
            "aichat command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        api::notify(&error_msg, LogLevel::Error, &Default::default()).ok();
        return Err(Error::Api(api::Error::Other(error_msg.into())));
    }

    // Parse the output into lines
    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut options: Vec<String> = output_str
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Add an option to unset this config value
    options.push("(unset)".into());

    Ok(options)
}

/// Shows the main configuration menu for aichat
pub fn show_config_menu() -> nvim_oxi::Result<()> {
    let menu_items = vec![
        "Set Role",
        "Set Agent",
        "Set Macro",
        "Set Session",
        "Set RAG",
    ];

    let ui: ui::UiSelect = menu_items.into();
    ui.show_with_callback("Aichat Configuration", |selection| {
        match selection.as_str() {
            "Set Role" => handle_config_selection("roles", Some(Mode::Role)),
            "Set Agent" => handle_config_selection("agents", Some(Mode::Agent)),
            "Set Macro" => handle_config_selection("macros", Some(Mode::Macro)),
            "Set Session" => handle_config_selection("sessions", None),
            "Set RAG" => handle_config_selection("rags", None),
            _ => Ok(()),
        }
        .unwrap_or_else(|e| {
            api::notify(
                &format!("Error: {}", e),
                LogLevel::Error,
                &Default::default(),
            )
            .ok();
        });
    })?;

    Ok(())
}

/// Handles the selection of a specific config option type
fn handle_config_selection(option_type: &str, mode: Option<Mode>) -> nvim_oxi::Result<()> {
    // Fetch options from aichat CLI
    match fetch_aichat_options(option_type) {
        Ok(options) => {
            let ui = crate::ui::UiSelect::new(options);

            // Clone option_type to own it inside the closure
            let option_type_owned: String = option_type.into();

            ui.show_with_callback(
                format!("Select {}", option_type).as_str(),
                move |selection| {
                    let update_result = if selection == "(unset)" {
                        // Unset the config value
                        update_config(&option_type_owned, None, mode)
                    } else {
                        // Set the config value
                        update_config(&option_type_owned, Some(selection), mode)
                    };

                    // Handle any errors from update_config
                    if let Err(e) = update_result {
                        api::notify(
                            &format!("Failed to update config: {}", e),
                            LogLevel::Error,
                            &Default::default(),
                        )
                        .ok();
                    }
                },
            )?;

            Ok(())
        }
        Err(e) => {
            api::notify(
                &format!("Failed to fetch {} options: {}", option_type, e),
                LogLevel::Error,
                &Default::default(),
            )?;
            Err(e)
        }
    }
}

/// Updates the AichatConfig with the selected value
fn update_config(
    option_type: &str,
    value: Option<String>,
    mode: Option<Mode>,
) -> nvim_oxi::Result<()> {
    let mut config = get_config();

    //Notify the user about the change
    let status = if let Some(val) = &value {
        format!("Set {} to: {}", option_type, val)
    } else {
        format!("Unset {}", option_type)
    };

    // Update the configuration based on the option type
    match option_type {
        "roles" | "agents" | "macros" => {
            if let Some(mode_val) = mode {
                config.mode_flag = if value.is_some() {
                    Some(mode_val)
                } else {
                    None
                };
                config.mode_arg = value.map(|s| s.into_boxed_str());
            }
        }
        "sessions" => {
            config.session = value.map(|s| s.into_boxed_str());
        }
        "rags" => {
            config.rag = value.map(|s| s.into_boxed_str());
        }
        _ => {
            return Err(nvim_oxi::Error::Api(api::Error::Other(format!(
                "Invalid option type: {}",
                option_type
            ))));
        }
    }

    //Notify the user about the successful update
    api::notify(&status, LogLevel::Info, &Default::default())?;

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
    if let Some(mode) = config.mode_flag {
        let mode_str = match mode {
            Mode::Role => "Role",
            Mode::Agent => "Agent",
            Mode::Macro => "Macro",
        };

        if let Some(arg) = &config.mode_arg {
            lines.push(format!("Mode: {} - {}", mode_str, arg));
        } else {
            lines.push(format!("Mode: {}", mode_str));
        }
    } else {
        lines.push("Mode: Not set".into());
    }

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
    buffer.set_option("modifiable", false)?;
    buffer.set_option("buftype", "nofile")?;

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
    let mut window = api::open_win(&buffer, true, &win_config)?;

    // Set window options
    window.set_option("cursorline", false)?;

    // Add a keymap to close the window with any key
    buffer.set_keymap(
        api::types::Mode::Normal,
        "<Esc>",
        ":q<CR>",
        &api::opts::SetKeymapOpts::builder()
            .noremap(true)
            .silent(true)
            .build(),
    )?;

    buffer.set_keymap(
        api::types::Mode::Normal,
        "q",
        ":q<CR>",
        &api::opts::SetKeymapOpts::builder()
            .noremap(true)
            .silent(true)
            .build(),
    )?;

    Ok(())
}
