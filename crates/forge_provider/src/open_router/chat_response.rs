use std::collections::HashMap;

use forge_tool::ToolName;
use serde::{Deserialize, Serialize};

use crate::error::{Error, ProviderError};
use crate::model::{Response as ModelResponse, ToolUse};
use crate::UseId;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChatResponse {
    pub id: String,
    pub provider: Option<String>,
    pub model: String,
    pub choices: Vec<Choice>,
    pub created: u64,
    pub object: String,
    pub system_fingerprint: Option<String>,
    pub usage: Option<ResponseUsage>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ResponseUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum Choice {
    NonChat {
        finish_reason: Option<String>,
        text: String,
        error: Option<ErrorResponse>,
    },
    NonStreaming {
        logprobs: Option<serde_json::Value>,
        index: u32,
        finish_reason: Option<String>,
        message: ResponseMessage,
        error: Option<ErrorResponse>,
    },
    Streaming {
        finish_reason: Option<String>,
        delta: ResponseMessage,
        error: Option<ErrorResponse>,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ErrorResponse {
    pub code: u32,
    pub message: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ResponseMessage {
    pub content: Option<String>,
    pub role: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub refusal: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ToolCall {
    pub id: Option<UseId>,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FunctionCall {
    pub name: Option<ToolName>,
    pub arguments: String,
}

impl TryFrom<ChatResponse> for ModelResponse {
    type Error = Error;

    fn try_from(res: ChatResponse) -> Result<Self, Self::Error> {
        if let Some(choice) = res.choices.first() {
            // Check for error response first
            match choice {
                Choice::NonChat { error: Some(e), .. } |
                Choice::NonStreaming { error: Some(e), .. } |
                Choice::Streaming { error: Some(e), .. } => {
                    return Err(Error::Provider {
                        provider: "Open Router".to_string(),
                        error: ProviderError::UpstreamError(serde_json::json!({
                            "code": e.code,
                            "message": e.message,
                            "metadata": e.metadata
                        }))
                    });
                }
                _ => {}
            }

            let response = match choice {
                Choice::NonChat { text, .. } => ModelResponse::new(text.clone()),
                Choice::NonStreaming { message, .. } => {
                    let mut resp = ModelResponse::new(message.content.clone().unwrap_or_default());
                    if let Some(tool_calls) = &message.tool_calls {
                        for tool_call in tool_calls {
                            resp = resp.add_call(ToolUse {
                                tool_use_id: tool_call.id.clone(),
                                tool_name: tool_call.function.name.clone(),
                                input: serde_json::from_str(&tool_call.function.arguments)?,
                            });
                        }
                    }
                    resp
                }
                Choice::Streaming { delta, .. } => {
                    let mut resp = ModelResponse::new(delta.content.clone().unwrap_or_default());
                    if let Some(tool_calls) = &delta.tool_calls {
                        for tool_call in tool_calls {
                            resp = resp.add_call(ToolUse {
                                tool_use_id: tool_call.id.clone(),
                                tool_name: tool_call.function.name.clone(),
                                input: tool_call.function.arguments.clone(),
                            });
                        }
                    }
                    resp
                }
            };
            Ok(response)
        } else {
            Err(Error::empty_response("Open Router"))
        }
    }
}
