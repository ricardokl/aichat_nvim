use nvim_oxi::api::types::LogLevel;
use nvim_oxi::{api, Error, Result};
use once_cell::sync::Lazy;
use std::sync::Mutex;

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

#[derive(Clone, Copy)]
pub enum Mode {
    Role,
    Agent,
    Macro,
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
            let error_msg = "Invalid option type";
            api::notify(error_msg, LogLevel::Error, &Default::default()).ok();
            return Err(Error::Api(api::Error::Other(error_msg.to_string())));
        }
    };

    // Execute the aichat command with the appropriate flag
    let output = match Command::new("aichat").arg(flag).output() {
        Ok(output) => output,
        Err(e) => {
            let error_msg = format!("Failed to execute aichat: {}", e);
            api::notify(&error_msg, LogLevel::Error, &Default::default()).ok();
            return Err(Error::Api(api::Error::Other(error_msg.to_string())));
        }
    };

    if !output.status.success() {
        let error_msg = format!(
            "aichat command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        api::notify(&error_msg, LogLevel::Error, &Default::default()).ok();
        return Err(Error::Api(api::Error::Other(error_msg.to_string())));
    }

    // Parse the output into lines
    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut options: Vec<String> = output_str
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Add an option to unset this config value
    options.push("(unset)".to_string());

    Ok(options)
}

/// Shows the main configuration menu for aichat
pub fn show_config_menu() -> Result<()> {
    let menu_items = vec![
        "Set Role".to_string(),
        "Set Agent".to_string(),
        "Set Macro".to_string(),
        "Set Session".to_string(),
        "Set RAG".to_string(),
    ];

    let ui = crate::ui::UiSelect::new(menu_items);
    ui.show_with_callback("Aichat Configuration".to_string(), |selection| {
        if let Some(selection) = selection {
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
        }
    })?;

    Ok(())
}

/// Handles the selection of a specific config option type
fn handle_config_selection(option_type: &str, mode: Option<Mode>) -> Result<()> {
    // Fetch options from aichat CLI
    match fetch_aichat_options(option_type) {
        Ok(options) => {
            let ui = crate::ui::UiSelect::new(options);
            ui.show_with_callback(format!("Select {}", option_type), move |selection| {
                if let Some(selection) = selection {
                    if selection == "(unset)" {
                        // Unset the config value
                        update_config(option_type, None, mode);
                    } else {
                        // Set the config value
                        update_config(option_type, Some(selection), mode);
                    }
                }
            })?;

            Ok(())
        }
        Err(e) => {
            api::notify(
                &format!("Failed to fetch {} options: {}", option_type, e),
                LogLevel::Error,
                &Default::default(),
            )
            .ok();
            Err(e)
        }
    }
}

/// Updates the AichatConfig with the selected value
fn update_config(option_type: &str, value: Option<String>, mode: Option<Mode>) {
    let mut config = CONFIG.lock().unwrap();

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
        _ => {}
    }

    // Notify the user about the change
    let status = if let Some(val) = &value {
        format!("Set {} to: {}", option_type, val)
    } else {
        format!("Unset {}", option_type)
    };

    api::out_write(&format!("{}\n", status));
}

/// Public API function to show the aichat configuration menu
pub fn show_aichat_config() -> Result<()> {
    show_config_menu()
}

/// Get the current configuration
pub fn get_config() -> AichatConfig {
    CONFIG.lock().unwrap().clone()
}
