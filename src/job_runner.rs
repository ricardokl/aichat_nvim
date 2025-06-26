use crate::config::{AichatConfig, Mode};
use crate::error::{AichatError, Result};
use std::io::Write;
use std::process::{Command, Stdio};

/// Runs the aichat command with the current configuration and input text
pub fn run_aichat_command(config: &AichatConfig, input: &str) -> Result<String> {
    // Start building the command
    let mut cmd = Command::new("aichat");

    // Add mode flag and argument if set
    match config.mode_flag {
        Mode::Role => cmd.arg("--role").arg(config.mode_arg.as_ref()),
        Mode::Agent => cmd.arg("--agent").arg(config.mode_arg.as_ref()),
        Mode::Macro => cmd.arg("--macro").arg(config.mode_arg.as_ref()),
    };

    // Add RAG if set
    if let Some(rag) = &config.rag {
        cmd.arg("--rag").arg(rag.as_ref());
    }

    // Add session if set
    if let Some(session) = &config.session {
        cmd.arg("--session").arg(session.as_ref());
    }

    // Configure stdin, stdout, and stderr
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Write input to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes())?;
    }

    // Wait for the command to complete
    let output = child.wait_with_output()?;

    // Check if the command was successful
    if !output.status.success() {
        return Err(AichatError::command_failed(output.status, output.stderr, output.stdout));
    }

    // Get the output
    let output_str = String::from_utf8_lossy(&output.stdout).to_string();

    // Extract the first code block
    extract_first_code_block(&output_str).ok_or(AichatError::NoCodeBlock)
}

/// Extracts the first code block from the output
fn extract_first_code_block(text: &str) -> Option<String> {
    // Look for code blocks with triple backticks
    let mut in_code_block = false;
    let mut code_block = String::new();

    for line in text.lines() {
        if line.trim().starts_with("```") {
            if !in_code_block {
                // Start of code block
                in_code_block = true;
                // Skip the language identifier line
                continue;
            } else {
                // End of code block
                return Some(code_block);
            }
        }

        if in_code_block {
            code_block.push_str(line);
            code_block.push('\n');
        }
    }

    // If we found a code block but no closing backticks, return it anyway
    if !code_block.is_empty() {
        Some(code_block)
    } else {
        None
    }
}
