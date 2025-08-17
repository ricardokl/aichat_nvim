use nvim_oxi::{
    api::{
        self,
        opts::CreateCommandOpts,
        types::{CommandArgs, CommandNArgs},
    },
    conversion::ToObject,
    string, Dictionary, Function, Result,
};

mod config;
mod error;
mod job_runner;
mod ui;
mod utils;

fn aichat(args: CommandArgs) -> error::Result<()> {
    let line1 = args.line1;
    let line2 = args.line2;
    let mut buffer = api::get_current_buf();
    let ft = buffer
        .get_name()?
        .extension()
        .map(|x| x.to_string_lossy().to_string())
        .unwrap_or("".into());
    let lines: Vec<nvim_oxi::String> = buffer.get_lines(line1 - 1..line2, true)?.collect();
    let line = if lines.is_empty() {
        string!("")
    } else {
        lines
            .into_iter()
            .reduce(|acc, e| string!("{}\n{}", acc, e))
            .ok_or(error::AichatError::NoLines)?
    };
    let code = if line.is_empty() {
        String::new()
    } else {
        format!("```{}
{}```", ft, line.to_string())
    };

    if let Some(user_text) = ui::show_input_prompt("Aichat Prompt >")? {
        utils::info("Sending to Aichat");

        let complete_prompt = format!("{}\n{}", user_text, code);
        let result =
            job_runner::run_aichat_command(&config::get_config(), &complete_prompt)?;

        let lines = result.split_terminator("\n");
        buffer.set_lines(line1 - 1..line2, true, lines)?;
        utils::info("Success");
    }

    Ok(())
}

fn setup(config: Option<config::AichatConfig>) -> Result<()> {
    if let Some(config) = config {
        let mut global_config = config::get_config_mut();
        *global_config = config;
    }
    Ok(())
}

#[nvim_oxi::plugin]
fn aichat_nvim() -> Result<Dictionary> {
    // Create command to run Aichat with the selected text
    let aichat_cmd = |args| error::handle_error_unit(aichat(args));
    let _ = api::create_user_command(
        "Aichat",
        aichat_cmd,
        &CreateCommandOpts::builder()
            .range(api::types::CommandRange::WholeFile)
            .nargs(CommandNArgs::Zero)
            .desc("Run Aichat command")
            .build(),
    )?;

    // Create command to set Aichat configuration
    let set_config_cmd = |_| error::handle_error_unit(config::show_config_menu());
    let _ = api::create_user_command(
        "AichatSetConfig",
        set_config_cmd,
        &CreateCommandOpts::builder()
            .nargs(CommandNArgs::Zero)
            .desc("Set the Config for Aichat")
            .build(),
    )?;

    // Create command to display current Aichat configuration
    let show_config_cmd = |_| error::handle_error_unit(config::show_current_config());
    let _ = api::create_user_command(
        "AichatShowConfig",
        show_config_cmd,
        &CreateCommandOpts::builder()
            .nargs(CommandNArgs::Zero)
            .desc("Show the Config for Aichat")
            .build(),
    )?;

    let setup_fn: Function<Option<config::AichatConfig>, ()> =
        Function::from_fn(|config: Option<config::AichatConfig>| -> Result<()> {
            if let Err(err) = setup(config) {
                api::err_writeln(&err.to_string());
                return Err(err.into());
            }
            Ok(())
        });

    let setup_fn = setup_fn.to_object()?;
    let exports = Dictionary::from_iter([("setup", setup_fn)]);

    Ok(exports)
}