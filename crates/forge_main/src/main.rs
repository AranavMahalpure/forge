use anyhow::Result;
use chrono::Local;
use clap::Parser;
use forge_domain::{ChatRequest, ChatResponse, ModelId};
use forge_main::{StatusDisplay, StatusKind, UserInput, CONSOLE};
use forge_server::API;
use tokio_stream::StreamExt;

#[derive(Parser)]
struct Cli {
    exec: Option<String>,
    #[arg(long, default_value_t = false)]
    verbose: bool,
}

fn get_timestamp() -> String {
    Local::now().format("%H:%M:%S%.3f").to_string()
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let initial_content = if let Some(path) = cli.exec {
        let cwd = std::env::current_dir()?;
        let full_path = cwd.join(path);
        tokio::fs::read_to_string(&full_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", full_path.display(), e))?
            .trim()
            .to_string()
    } else {
        UserInput::prompt_initial()?
    };

    CONSOLE.writeln(initial_content.trim())?;
    let mut current_conversation_id = None;
    let api = API::init()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize API: {}", e))?;

    let mut content = initial_content;

    loop {
        let model = ModelId::from_env(api.env());
        let chat = ChatRequest {
            content: content.clone(),
            model,
            conversation_id: current_conversation_id,
        };

        match api.chat(chat).await {
            Ok(mut stream) => {
                while let Some(message) = stream.next().await {
                    match message {
                        Ok(message) => match message {
                            ChatResponse::Text(text) => {
                                CONSOLE.write(&text)?;
                            }
                            ChatResponse::ToolCallDetected(_) => {}
                            ChatResponse::ToolCallArgPart(arg) => {
                                if cli.verbose {
                                    CONSOLE.write(&arg)?;
                                }
                            }
                            ChatResponse::ToolCallStart(tool_call_full) => {
                                let tool_name = tool_call_full.name.as_str();
                                let status = StatusDisplay {
                                    kind: StatusKind::Execute,
                                    message: tool_name,
                                    timestamp: Some(get_timestamp()),
                                    error_details: None,
                                };
                                CONSOLE.newline()?;
                                CONSOLE.writeln(status.format())?;
                            }
                            ChatResponse::ToolCallEnd(tool_result) => {
                                if cli.verbose {
                                    CONSOLE.writeln(tool_result.to_string())?;
                                }
                                let tool_name = tool_result.name.as_str();
                                let status = if tool_result.is_error {
                                    StatusDisplay {
                                        kind: StatusKind::Failed,
                                        message: tool_name,
                                        timestamp: Some(get_timestamp()),
                                        error_details: Some("error"),
                                    }
                                } else {
                                    StatusDisplay {
                                        kind: StatusKind::Success,
                                        message: tool_name,
                                        timestamp: Some(get_timestamp()),
                                        error_details: None,
                                    }
                                };
                                CONSOLE.write(status.format())?;
                            }
                            ChatResponse::ConversationStarted(conversation_id) => {
                                current_conversation_id = Some(conversation_id);
                            }
                            ChatResponse::ModifyContext(_) => {}
                            ChatResponse::Complete => {}
                            ChatResponse::Error(err) => {
                                let status = StatusDisplay {
                                    kind: StatusKind::Failed,
                                    message: &err.to_string(),
                                    timestamp: Some(get_timestamp()),
                                    error_details: None,
                                };
                                CONSOLE.writeln(status.format())?;
                            }
                            ChatResponse::PartialTitle(_) => {}
                            ChatResponse::CompleteTitle(title) => {
                                let status = StatusDisplay {
                                    kind: StatusKind::Title,
                                    message: &title,
                                    timestamp: Some(get_timestamp()),
                                    error_details: None,
                                };
                                CONSOLE.writeln(status.format())?;
                            }
                            ChatResponse::FinishReason(_) => {}
                        },
                        Err(err) => {
                            let status = StatusDisplay {
                                kind: StatusKind::Failed,
                                message: &err.to_string(),
                                timestamp: Some(get_timestamp()),
                                error_details: None,
                            };
                            CONSOLE.writeln(status.format())?;
                        }
                    }
                }
            }
            Err(err) => {
                let status = StatusDisplay {
                    kind: StatusKind::Failed,
                    message: &err.to_string(),
                    timestamp: Some(get_timestamp()),
                    error_details: Some("Failed to establish chat stream"),
                };
                CONSOLE.writeln(status.format())?;
            }
        }

        match UserInput::prompt()? {
            UserInput::End => break,
            UserInput::New => {
                CONSOLE.writeln("Starting fresh conversation...")?;
                current_conversation_id = None;
                content = UserInput::prompt_initial()?;
            }
            UserInput::Message(msg) => {
                content = msg;
            }
        }
    }

    Ok(())
}
