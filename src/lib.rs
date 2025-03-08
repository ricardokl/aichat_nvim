use nvim_oxi::{Dictionary, Function, Object, Result};
use nvim_oxi::api::types::LogLevel;
use nvim_oxi::api;

mod config;
mod ui;
mod job_runner;

#[nvim_oxi::plugin]
fn aichat_nvim() -> Result<Dictionary> {
    let show_config_menu = Function::from_fn(|()| {
        let _ = config::show_config_menu();
    });

    let show_current_config = Function::from_fn(|()| {
        let _ = config::show_current_config();
    });

    Ok(Dictionary::from_iter([
        ("config_menu", Object::from(show_config_menu)),
        ("current_config", Object::from(show_current_config)),
        ("run_aichat", Object::from(Function::from_fn(|input: String| {
            let config = match config::get_current_config() {
                Ok(config) => config,
                Err(e) => {
                    api::notify(
                        &format!("Failed to get config: {}", e),
                        LogLevel::Error,
                        &Default::default(),
                    ).ok();
                    return Err(e);
                }
            };
            
            match job_runner::run_aichat_command(&config, &input) {
                Ok(output) => Ok(output),
                Err(e) => {
                    api::notify(
                        &format!("Failed to run aichat: {}", e),
                        LogLevel::Error,
                        &Default::default(),
                    ).ok();
                    Err(e)
                }
            }
        }))),
    ]))
}
