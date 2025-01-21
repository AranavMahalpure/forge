use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::sql_types::{Bool, Nullable, Text, Timestamp};
use forge_domain::{Context, Conversation, ConversationId, ConversationMeta};

use crate::schema::conversations;
use crate::service::Service;
use crate::sqlite::Sqlite;

#[derive(Debug, Insertable, Queryable, QueryableByName)]
#[diesel(table_name = conversations)]
struct RawConversation {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Timestamp)]
    created_at: NaiveDateTime,
    #[diesel(sql_type = Timestamp)]
    updated_at: NaiveDateTime,
    #[diesel(sql_type = Text)]
    content: String,
    #[diesel(sql_type = Bool)]
    archived: bool,
    #[diesel(sql_type = Nullable<Text>)]
    title: Option<String>,
}

impl TryFrom<RawConversation> for Conversation {
    type Error = forge_domain::Error;

    fn try_from(raw: RawConversation) -> Result<Self, Self::Error> {
        Ok(Conversation {
            id: ConversationId::parse(raw.id)?,
            meta: Some(ConversationMeta {
                created_at: DateTime::from_naive_utc_and_offset(raw.created_at, Utc),
                updated_at: DateTime::from_naive_utc_and_offset(raw.updated_at, Utc),
            }),
            context: serde_json::from_str(&raw.content)?,
            archived: raw.archived,
            title: raw.title,
        })
    }
}
#[async_trait::async_trait]
pub trait ConversationRepository: Send + Sync {
    async fn set_conversation(
        &self,
        request: &Context,
        id: Option<ConversationId>,
    ) -> Result<Conversation>;
    async fn get_conversation(&self, id: ConversationId) -> Result<Conversation>;
    async fn list_conversations(&self) -> Result<Vec<Conversation>>;
    async fn archive_conversation(&self, id: ConversationId) -> Result<Conversation>;
    async fn set_conversation_title(
        &self,
        id: &ConversationId,
        title: String,
    ) -> Result<Conversation>;
}

pub struct Live<P: Sqlite> {
    pool_service: P,
}

impl<P: Sqlite> Live<P> {
    pub fn new(pool_service: P) -> Self {
        Self { pool_service }
    }
}

#[async_trait::async_trait]
impl<P: Sqlite + Send + Sync> ConversationRepository for Live<P> {
    async fn set_conversation(
        &self,
        request: &Context,
        id: Option<ConversationId>,
    ) -> Result<Conversation> {
        let pool = self.pool_service.pool().await?;
        let mut conn = pool.get()?;
        let id = id.unwrap_or_else(ConversationId::generate);

        let raw = RawConversation {
            id: id.into_string(),
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            content: serde_json::to_string(request)?,
            archived: false,
            title: None,
        };

        diesel::insert_into(conversations::table)
            .values(&raw)
            .on_conflict(conversations::id)
            .do_update()
            .set((
                conversations::content.eq(&raw.content),
                conversations::updated_at.eq(&raw.updated_at),
            ))
            .execute(&mut conn)?;

        let raw: RawConversation = conversations::table
            .find(id.into_string())
            .first(&mut conn)?;

        Ok(Conversation::try_from(raw)?)
    }

    async fn get_conversation(&self, id: ConversationId) -> Result<Conversation> {
        let pool = self.pool_service.pool().await?;
        let mut conn = pool.get()?;
        let raw: RawConversation = conversations::table
            .find(id.into_string())
            .first(&mut conn)?;

        Ok(Conversation::try_from(raw)?)
    }

    async fn list_conversations(&self) -> Result<Vec<Conversation>> {
        let pool = self.pool_service.pool().await?;
        let mut conn = pool.get()?;
        let raw: Vec<RawConversation> = conversations::table
            .filter(conversations::archived.eq(false))
            .load(&mut conn)?;

        Ok(raw
            .into_iter()
            .map(Conversation::try_from)
            .collect::<Result<Vec<_>, _>>()?)
    }

    async fn archive_conversation(&self, id: ConversationId) -> Result<Conversation> {
        let pool = self.pool_service.pool().await?;
        let mut conn = pool.get()?;

        diesel::update(conversations::table.find(id.into_string()))
            .set(conversations::archived.eq(true))
            .execute(&mut conn)?;

        let raw: RawConversation = conversations::table
            .find(id.into_string())
            .first(&mut conn)?;

        Ok(Conversation::try_from(raw)?)
    }

    async fn set_conversation_title(
        &self,
        id: &ConversationId,
        title: String,
    ) -> Result<Conversation> {
        let pool = self.pool_service.pool().await?;
        let mut conn = pool.get()?;

        diesel::update(conversations::table.find(id.into_string()))
            .set(conversations::title.eq(title))
            .execute(&mut conn)?;

        let raw: RawConversation = conversations::table
            .find(id.into_string())
            .first(&mut conn)?;

        Ok(raw.try_into()?)
    }
}

impl Service {
    pub fn storage_service(database_url: &str) -> Result<impl ConversationRepository> {
        let pool_service = Service::db_pool_service(database_url)?;
        Ok(Live::new(pool_service))
    }
}

#[cfg(test)]
pub mod tests {

    use pretty_assertions::assert_eq;

    use super::*;
    use crate::sqlite::tests::TestSqlite;

    pub struct TestStorage;
    impl TestStorage {
        pub fn in_memory() -> Result<impl ConversationRepository> {
            let pool_service = TestSqlite::new()?;
            Ok(Live::new(pool_service))
        }
    }

    async fn setup_storage() -> Result<impl ConversationRepository> {
        TestStorage::in_memory()
    }

    async fn create_conversation(
        storage: &impl ConversationRepository,
        id: Option<ConversationId>,
    ) -> Result<Conversation> {
        let request = Context::default();
        storage.set_conversation(&request, id).await
    }

    #[tokio::test]
    async fn conversation_can_be_stored_and_retrieved() {
        let storage = setup_storage().await.unwrap();
        let id = ConversationId::generate();

        let saved = create_conversation(&storage, Some(id)).await.unwrap();
        let retrieved = storage.get_conversation(id).await.unwrap();

        assert_eq!(saved.id, retrieved.id);
        assert_eq!(saved.context, retrieved.context);
    }

    #[tokio::test]
    async fn list_returns_active_conversations() {
        let storage = setup_storage().await.unwrap();

        let conv1 = create_conversation(&storage, None).await.unwrap();
        let conv2 = create_conversation(&storage, None).await.unwrap();
        let conv3 = create_conversation(&storage, None).await.unwrap();

        // Archive one conversation
        storage.archive_conversation(conv2.id).await.unwrap();

        let conversations = storage.list_conversations().await.unwrap();

        assert_eq!(conversations.len(), 2);
        assert!(conversations.iter().all(|c| !c.archived));
        assert!(conversations.iter().any(|c| c.id == conv1.id));
        assert!(conversations.iter().any(|c| c.id == conv3.id));
        assert!(conversations.iter().all(|c| c.id != conv2.id));
    }

    #[tokio::test]
    async fn archive_marks_conversation_as_archived() {
        let storage = setup_storage().await.unwrap();
        let conversation = create_conversation(&storage, None).await.unwrap();

        let archived = storage.archive_conversation(conversation.id).await.unwrap();

        assert!(archived.archived);
        assert_eq!(archived.id, conversation.id);
    }

    #[tokio::test]
    async fn test_set_title_for_conversation() {
        let storage = setup_storage().await.unwrap();
        let conversation = create_conversation(&storage, None).await.unwrap();
        let result = storage
            .set_conversation_title(&conversation.id, "test-title".to_string())
            .await
            .unwrap();

        assert!(result.title.is_some());
        assert_eq!(result.id, conversation.id);
    }
}
