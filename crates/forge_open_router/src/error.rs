use std::collections::HashMap;

use derive_more::derive::Display;
use thiserror::Error;

#[derive(Debug, Display, derive_more::From, Error)]
pub enum Error {
    EmptyContent,
    #[from(ignore)]
    #[display("Upstream: {}", message)]
    Upstream {
        code: u32,
        message: String,
        metadata: HashMap<String, String>,
    },
    SerdeJson(serde_json::Error),
    ToolCallMissingName,
}
