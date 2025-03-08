use nvim_oxi::{Dictionary, Function, Object, Result};

mod config;
mod ui;

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
    ]))
}
