use anyhow::Result;
use clap::Parser;
use forge_main::{banner, UI};

/// Command line arguments for the application
#[derive(Parser)]
struct Cli {
    /// Optional file path to execute commands from
    exec: Option<String>,
    /// Enable verbose output, showing additional tool information
    #[arg(long, default_value_t = false)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Display the banner in dimmed colors
    banner::display()?;

    // Initialize and run the UI
    let mut ui = UI::new(cli.verbose, cli.exec).await?;
    ui.run().await?;

    Ok(())
}
