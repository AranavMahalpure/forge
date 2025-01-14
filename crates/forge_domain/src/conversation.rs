use async_trait::async_trait;
use chrono::{DateTime, Utc};
use derive_more::derive::Display;
use derive_setters::Setters;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Context, Error};

#[derive(Debug, Display, Serialize, Deserialize, Clone, PartialEq, Eq, Copy)]
#[serde(transparent)]
pub struct ConversationId(Uuid);

impl ConversationId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn into_string(&self) -> String {
        self.0.to_string()
    }

    pub fn parse(value: impl ToString) -> Result<Self, Error> {
        Ok(Self(Uuid::parse_str(&value.to_string())?))
    }
}

#[derive(Debug, Setters, Serialize, Deserialize)]
pub struct Conversation {
    pub id: ConversationId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ConversationMeta>,
    pub context: Context,
    pub archived: bool,
    pub title: Option<String>,
}

impl Conversation {
    pub fn new(context: Context) -> Self {
        Self {
            id: ConversationId::generate(),
            meta: None,
            context,
            archived: false,
            title: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationMeta {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[async_trait]
pub trait ConversationRepository {
    /// Get a conversation by its ID
    async fn get_conversation(&self, id: ConversationId) -> anyhow::Result<Option<Conversation>>;

    /// Save a new conversation or update an existing one
    async fn save_conversation(&self, conversation: &Conversation) -> anyhow::Result<()>;

    /// List all conversations
    async fn list_conversations(&self) -> anyhow::Result<Vec<Conversation>>;

    /// Archive a conversation
    async fn archive_conversation(&self, id: ConversationId) -> anyhow::Result<()>;
}
