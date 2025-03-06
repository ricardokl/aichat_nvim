use nvim_oxi::{Dictionary, Function, Object, Result};

mod config;
mod ui;

#[nvim_oxi::plugin]
fn aichat_nvim() -> Result<Dictionary> {
    let show_config = Function::from_fn(|()| {
        let _ = config::show_aichat_config();
    });

    let show_config_menu = Function::from_fn(|()| {
        let _ = config::show_config_menu();
    });

    Ok(Dictionary::from_iter([
        ("config", Object::from(show_config)),
        ("config_menu", Object::from(show_config_menu)),
    ]))
}
