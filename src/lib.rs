use nvim_oxi::{
    api::{
        self,
        opts::CreateCommandOpts,
        types::{CommandArgs, CommandNArgs},
    },
    string, Result,
};

mod config;
mod error;
mod job_runner;
mod ui;
mod utils;

fn aichat(args: CommandArgs) -> Result<()> {
    let line1 = args.line1;
    let line2 = args.line2;
    let mut buffer = api::get_current_buf();
    let ft = buffer
        .get_name()?
        .extension()
        .map(|x| x.to_string_lossy().to_string())
        .unwrap_or("".into());
    let lines: Vec<nvim_oxi::String> = buffer.get_lines(line1 - 1..line2, true)?;
    let line = if lines.is_empty() {
        string!("")
    } else {
        lines
            .into_iter()
            .reduce(|acc, e| string!("{}\n{}", acc, e))
            .ok_or(api::Error::Other("No lines found".into()))?
    };
    let code = if line.is_empty() {
        String::new()
    } else {
        format!("```{}
{}```", ft, line.to_string())
    };

    // Create input prompt and handle response
    if let Some(user_text) = ui::show_input_prompt("Aichat Prompt >")? {
        utils::info("Sending to Aichat");

        let complete_prompt = format!("{}\n{}", user_text, code);
        let result = match job_runner::run_aichat_command(&config::get_config(), &complete_prompt) {
            Ok(result) => result,
            Err(err) => {
                error::notify_error(&err);
                return Err(err.into());
            }
        };

        let lines = result.split_terminator("\n");
        buffer.set_lines(line1 - 1..line2, true, lines)?;
        utils::info("Success");
    }

    Ok(())
}

#[nvim_oxi::plugin]
fn aichat_nvim() -> Result<()> {
    // Create command to run Aichat with the selected text
    let _ = api::create_user_command(
        "Aichat",
        aichat,
        &CreateCommandOpts::builder()
            .range(api::types::CommandRange::WholeFile)
            .nargs(CommandNArgs::Zero)
            .desc("Run Aichat command")
            .build(),
    )?;

    // Create command to set Aichat configuration
    let _ = api::create_user_command(
        "AichatSetConfig",
        |_| config::show_config_menu(),
        &CreateCommandOpts::builder()
            .nargs(CommandNArgs::Zero)
            .desc("Set the Config for Aichat")
            .build(),
    )?;

    // Create command to display current Aichat configuration
    let _ = api::create_user_command(
        "AichatShowConfig",
        |_| config::show_current_config(),
        &CreateCommandOpts::builder()
            .nargs(CommandNArgs::Zero)
            .desc("Show the Config for Aichat")
            .build(),
    )?;

    Ok(())
}