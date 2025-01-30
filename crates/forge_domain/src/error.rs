use thiserror::Error;

// NOTE: Deriving From for error is a really bad idea. This is because you end
// up converting errors incorrectly without much context. For eg: You don't want
// all serde error to be treated as the same. Instead we want to know exactly
// where that serde failure happened and for what kind of value.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Missing tool name")]
    ToolCallMissingName,

    #[error("Invalid tool call arguments: {0}")]
    ToolCallArgument(serde_json::Error),

    #[error("Invalid conversation id: {0}")]
    ConversationId(uuid::Error),

    #[error("Invalid input command: {0}")]
    InputCommand(String),

    #[error("Invalid template rendering params: {0}")]
    Template(handlebars::RenderError),
}

pub type Result<A> = std::result::Result<A, Error>;
