use nvim_oxi::{
    api::{
        self,
        opts::CreateCommandOpts,
        types::{CommandArgs, CommandNArgs},
    },
    string, Dictionary, Function, Object, Result,
};

mod config;
mod job_runner;
mod ui;

fn aichat(args: CommandArgs) -> Result<()> {
    let line1 = args.line1;
    let line2 = args.line2;
    let buffer = api::get_current_buf();
    let ft = buffer
        .get_name()?
        .extension()
        .map(|x| x.to_string_lossy().to_string())
        .unwrap_or("".into());
    let line: Result<nvim_oxi::String> = buffer
        .get_lines(line1 - 1..line2, true)?
        .reduce(|acc: nvim_oxi::String, e: nvim_oxi::String| string!("{}\n{}", acc, e))
        .ok_or(api::Error::Other("No lines found".into()).into());
    let code = string!("```{}\n{}```", ft, line?.to_string_lossy());

    let _ = api::notify(
        code.to_str()
            .map_err(|_| api::Error::Other("Could not convert to str".into()))?,
        api::types::LogLevel::Info,
        &Default::default(),
    );
    Ok(())
}

fn show_input_demo() -> Result<()> {
    let input = ui::UiInput::new("Enter text:".to_string(), None);

    input.show_with_callback("Input Demo", |text| {
        let _ = api::notify(
            &format!("You entered: {}", text),
            api::types::LogLevel::Info,
            &Default::default(),
        );
    })?;

    Ok(())
}

#[nvim_oxi::plugin]
fn aichat_nvim() -> Result<Dictionary> {
    let show_config_menu: Function<(), Result<()>> =
        Function::from_fn(|()| -> Result<()> { config::show_config_menu() });

    let show_current_config: Function<(), Result<()>> =
        Function::from_fn(|()| -> Result<()> { config::show_current_config() });

    let show_input_demo: Function<(), Result<()>> =
        Function::from_fn(|()| -> Result<()> { show_input_demo() });

    let _ = api::create_user_command(
        "AichatRs",
        aichat,
        &CreateCommandOpts::builder()
            .range(api::types::CommandRange::WholeFile)
            .nargs(CommandNArgs::Zero)
            .desc("Run Aichat command")
            .build(),
    )?;

    Ok(Dictionary::from_iter([
        ("config_menu", Object::from(show_config_menu)),
        ("current_config", Object::from(show_current_config)),
        ("input_demo", Object::from(show_input_demo)),
    ]))
}
