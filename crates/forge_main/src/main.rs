use core::panic;

use clap::Parser;
use colored::Colorize;
use forge_domain::ModelId;
use forge_server::{ChatRequest, ChatResponse, Result, API};
use tokio_stream::StreamExt;

#[derive(Parser)]
struct Cli {
    path: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(path) = cli.path {
        let api = API::init().await?;

        let cwd = std::env::current_dir()?;
        let full_path = cwd.join(path);
        let mut content = tokio::fs::read_to_string(full_path).await?;

        println!("{}", content.trim());
        let mut current_conversation_id = None;
        loop {
            let model = ModelId::from_env(api.env());
            let chat = ChatRequest {
                content: content.clone(),
                model,
                conversation_id: current_conversation_id,
            };

            let mut stream = api.chat(chat).await?;
            while let Some(message) = stream.next().await {
                match message.unwrap() {
                    ChatResponse::Text(text) => {
                        print!("{}", text);
                    }
                    ChatResponse::ToolCallDetected(_) => {}
                    ChatResponse::ToolCallStart(tool_call_full) => {
                        println!(
                            "\n{} {}",
                            "Tool use detected:".green(),
                            tool_call_full.name.as_str()
                        );
                    }
                    ChatResponse::ToolCallEnd(tool_result) => {
                        println!("{}", tool_result.content);
                    }
                    ChatResponse::ConversationStarted(id) => {
                        current_conversation_id = Some(id);
                    }
                    ChatResponse::ModifyContext(_) => {}
                    ChatResponse::Complete => {}
                    ChatResponse::Error(err) => {
                        panic!("{:?}", err);
                    }
                }
            }

            println!();
            content = inquire::Text::new("")
                .with_help_message("type '/done' to end this conversation.")
                .prompt()
                .unwrap();
            if content.trim() == "/done" {
                break;
            }
        }

        Ok(())
    } else {
        Ok(API::init().await?.launch().await?)
    }
}
