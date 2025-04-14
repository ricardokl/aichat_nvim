use nvim_oxi::{
    api::{
        self,
        opts::CreateCommandOpts,
        types::{CommandArgs, CommandNArgs, LogLevel},
    },
    string, Result,
};

mod config;
mod job_runner;
mod ui;

fn aichat(args: CommandArgs) -> Result<()> {
    let line1 = args.line1;
    let line2 = args.line2;
    let mut buffer = api::get_current_buf();
    let ft = buffer
        .get_name()?
        .extension()
        .map(|x| x.to_string_lossy().to_string())
        .unwrap_or("".into());
    let line_result: Result<nvim_oxi::String> = buffer
        .get_lines(line1 - 1..line2, true)?
        .reduce(|acc: nvim_oxi::String, e: nvim_oxi::String| string!("{}\n{}", acc, e))
        .ok_or(api::Error::Other("No lines found".into()).into());
    let line = line_result?;
    let code: String;
    if line.is_empty() {
        code = String::new();
    } else {
        code = format!("```{}\n{}```", ft, line.to_string());
    }

    // Create input prompt and handle response
    if let Some(user_text) = ui::show_input_prompt("Aichat Prompt ❯")? {
        let _ = api::notify("Sending to Aichat", LogLevel::Info, &Default::default());

        let complete_prompt = format!("{}\n{}", user_text, code);
        let output = job_runner::run_aichat_command(&config::get_config(), &complete_prompt);

        let result = match output {
            Ok(result) => result,
            Err(err) => {
                let _ = api::notify(
                    &format!("Error running Aichat command: {}", err),
                    LogLevel::Error,
                    &Default::default(),
                );
                return Ok(());
            }
        };

        let lines = result.split_terminator("\n");
        match buffer.set_lines(line1 - 1..line2, true, lines) {
            Ok(_) => {
                let _ = api::notify("Success", LogLevel::Info, &Default::default());
            }
            Err(_) => {
                let _ = api::notify("Something went wrong", LogLevel::Error, &Default::default());
            }
        }
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
