use std::collections::HashMap;
use std::sync::Arc;

use derive_more::derive::Display;
use forge_domain::ModelId;

#[derive(Debug, Display, derive_more::From)]
pub enum Error {
    // Custom display message for provider error
    EmptyContent,
    ModelNotFound(ModelId),
    #[from(ignore)]
    #[display("Upstream: {message}")]
    Upstream {
        code: u32,
        message: String,
        metadata: HashMap<String, serde_json::Value>,
    },
    Reqwest(#[from] reqwest::Error),
    SerdeJson(#[from] serde_json::Error),
    EventSource(#[from] reqwest_eventsource::Error),
    ToolCallMissingName,
    Arc(Arc<Error>),
}

pub type Result<T> = std::result::Result<T, Error>;
