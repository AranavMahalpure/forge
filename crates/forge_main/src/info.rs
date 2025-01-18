use std::io;

use colored::Colorize;
use forge_domain::{Environment, Usage};

use crate::CONSOLE;

pub fn display_info(env: &Environment, usage: &Usage) -> io::Result<()> {
    CONSOLE.newline()?;
    CONSOLE.writeln(format!("{} {}", "OS:".bold().bright_yellow(), env.os))?;
    CONSOLE.writeln(format!(
        "{} {}",
        "Working Directory:".bold().bright_yellow(),
        env.cwd
    ))?;
    CONSOLE.writeln(format!("{} {}", "Shell:".bold().bright_yellow(), env.shell))?;
    if let Some(home) = &env.home {
        CONSOLE.writeln(format!(
            "{} {}",
            "Home Directory:".bold().bright_yellow(),
            home
        ))?;
    }
    CONSOLE.writeln(format!(
        "{} {}",
        "File Count:".bold().bright_yellow(),
        env.files.len()
    ))?;
    CONSOLE.newline()?;
    CONSOLE.writeln(format!(
        "{} {}",
        "Primary Model:".bold().bright_yellow(),
        env.large_model_id
    ))?;
    CONSOLE.writeln(format!(
        "{} {}",
        "Secondary Model:".bold().bright_yellow(),
        env.small_model_id
    ))?;
    CONSOLE.newline()?;
    CONSOLE.writeln(format!(
        "{} {}",
        "Prompt:".bold().bright_yellow(),
        usage.prompt_tokens
    ))?;
    CONSOLE.writeln(format!(
        "{} {}",
        "Completion:".bold().bright_yellow(),
        usage.completion_tokens
    ))?;
    CONSOLE.writeln(format!(
        "{} {}",
        "Total:".bold().bright_yellow(),
        usage.total_tokens
    ))?;
    CONSOLE.newline()?;
    Ok(())
}
