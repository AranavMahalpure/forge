use anyhow::Result;
use forge::UI;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize and run the UI
    let mut ui = UI::init().await?;
    ui.run().await?;

    Ok(())
}
