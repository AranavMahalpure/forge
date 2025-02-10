use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sql_types::{Binary, Float, Text, Timestamp};
use forge_domain::{Embedding, EmbeddingsRepository, Information};
use uuid::Uuid;

use crate::embeddings::Embedder;
use crate::sqlite::Sqlite;
use crate::Service;

// Table reference for the diesel.
diesel::table! {
    embedding_index (id) {
        id -> Text,
        data -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        tags -> Text,
        embedding -> Binary,
    }
}

#[derive(Queryable, Insertable, QueryableByName)]
#[diesel(table_name = embedding_index)]
struct RawEmbedding {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    data: String,
    #[diesel(sql_type = Timestamp)]
    created_at: NaiveDateTime,
    #[diesel(sql_type = Timestamp)]
    updated_at: NaiveDateTime,
    #[diesel(sql_type = Text)]
    tags: String,
    #[diesel(sql_type = Binary)]
    embedding: Vec<u8>,
}

#[derive(QueryableByName)]
struct SearchResult {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    data: String,
    #[diesel(sql_type = Timestamp)]
    created_at: NaiveDateTime,
    #[diesel(sql_type = Timestamp)]
    updated_at: NaiveDateTime,
    #[diesel(sql_type = Text)]
    tags: String,
    #[diesel(sql_type = Binary)]
    embedding: Vec<u8>,
    #[diesel(sql_type = Float)]
    distance: f32,
}

pub struct Live {
    pool_service: Arc<dyn Sqlite>,
}

impl Service {
    pub fn embedding_repository(sql: Arc<dyn Sqlite>) -> impl EmbeddingsRepository {
        Live::new(sql)
    }
}

impl Live {
    pub fn new(pool_service: Arc<dyn Sqlite>) -> Self {
        Self { pool_service }
    }
}

impl TryFrom<RawEmbedding> for Information {
    type Error = anyhow::Error;
    fn try_from(row: RawEmbedding) -> Result<Self> {
        let id = Uuid::parse_str(&row.id)?;
        let tags: Vec<String> = serde_json::from_str(&row.tags)?;
        let embedding_vec = bytes_to_vec(&row.embedding)?;

        Ok(Information {
            id,
            data: row.data.clone(),
            embedding: Embedding::new(embedding_vec),
            created_at: row.created_at,
            updated_at: row.updated_at,
            tags,
            distance: None,
        })
    }
}

impl TryFrom<SearchResult> for Information {
    type Error = anyhow::Error;
    fn try_from(row: SearchResult) -> Result<Self> {
        let id = Uuid::parse_str(&row.id)?;
        let tags: Vec<String> = serde_json::from_str(&row.tags)?;
        let embedding_vec = bytes_to_vec(&row.embedding)?;

        Ok(Information {
            id,
            data: row.data.clone(),
            embedding: Embedding::new(embedding_vec),
            created_at: row.created_at,
            updated_at: row.updated_at,
            tags,
            distance: Some(row.distance),
        })
    }
}

#[async_trait::async_trait]
impl EmbeddingsRepository for Live {
    async fn get(&self, id: Uuid) -> Result<Option<Information>> {
        let mut conn = self.pool_service.connection().await.with_context(|| {
            "Failed to acquire database connection to get embedding".to_string()
        })?;

        let result = embedding_index::table
            .filter(embedding_index::id.eq(id.to_string()))
            .first::<RawEmbedding>(&mut conn)
            .optional()?;

        Ok(result.map(Information::try_from).transpose()?)
    }

    async fn insert(&self, data: String, tags: Vec<String>) -> Result<Embedding> {
        let mut conn = self.pool_service.connection().await.with_context(|| {
            "Failed to acquire database connection to insert embedding".to_string()
        })?;

        let id = Uuid::new_v4();
        let now = chrono::Local::now().naive_local();
        let embedding = Embedder::embed(data.clone())?;
        let embedding_bytes = vec_to_bytes(embedding.as_slice());
        let tags_json = serde_json::to_string(&tags)?;

        let new_embedding = RawEmbedding {
            id: id.to_string(),
            data,
            created_at: now,
            updated_at: now,
            tags: tags_json,
            embedding: embedding_bytes,
        };

        diesel::insert_into(embedding_index::table)
            .values(&new_embedding)
            .execute(&mut conn)?;
        Ok(embedding)
    }

    async fn search(
        &self,
        embedding: Embedding,
        tags: Vec<String>,
        k: usize,
    ) -> Result<Vec<Information>> {
        let mut conn = self.pool_service.connection().await.with_context(|| {
            "Failed to acquire database connection to search embedding".to_string()
        })?;

        let query_embedding = vec_to_bytes(embedding.as_slice());
        let tags_json = serde_json::to_string(&tags)?;

        let results = if tags.is_empty() {
            diesel::sql_query(
                "SELECT id, data, created_at, updated_at, tags, embedding, distance
                FROM embedding_index
                WHERE embedding MATCH ?
                AND k = ?
                ORDER BY distance ASC",
            )
            .bind::<Binary, _>(query_embedding)
            .bind::<diesel::sql_types::Integer, _>(k as i32)
            .load::<SearchResult>(&mut conn)?
        } else {
            diesel::sql_query(
                "SELECT le.id, le.data, le.created_at, le.updated_at, le.tags, le.embedding, distance
                FROM embedding_index le
                WHERE embedding MATCH ?
                AND k = ?
                AND EXISTS (
                    SELECT 1 
                    FROM json_each(le.tags) t
                    WHERE t.value IN (SELECT value FROM json_each(?))
                )
                ORDER BY distance ASC"
            )
            .bind::<Binary, _>(query_embedding)
            .bind::<diesel::sql_types::Integer, _>(k as i32)
            .bind::<Text, _>(tags_json)
            .load::<SearchResult>(&mut conn)?
        };

        let information = results
            .into_iter()
            .map(Information::try_from)
            .collect::<Result<Vec<_>>>()?;

        Ok(information)
    }
}

fn vec_to_bytes(v: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(v.len() * 4);
    for &x in v {
        bytes.extend_from_slice(&x.to_le_bytes());
    }
    bytes
}

fn bytes_to_vec(v: &[u8]) -> Result<Vec<f32>> {
    let mut vec = Vec::with_capacity(v.len() / 4);
    for chunk in v.chunks_exact(4) {
        vec.push(f32::from_le_bytes(chunk.try_into().unwrap()));
    }
    Ok(vec)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::sqlite::TestDriver;

    pub struct EmbeddingRepositoryTest;

    impl EmbeddingRepositoryTest {
        pub fn init() -> impl EmbeddingsRepository {
            let pool_service = Arc::new(TestDriver::new().unwrap());
            Live::new(pool_service)
        }
    }

    #[tokio::test]
    async fn test_insertion() {
        let repo = EmbeddingRepositoryTest::init();
        let data = "learning about vector indexing".to_string();
        let tags = vec!["learning".to_owned()];
        let result = repo.insert(data, tags).await;
        assert!(result.is_ok());
        assert!(result.unwrap().as_slice().len() == 384);
    }

    #[tokio::test]
    async fn test_search() {
        let repo = EmbeddingRepositoryTest::init();

        // Insert some test data
        let data1 = "learning about vector indexing".to_string();
        let data2 = "vector similarity search methods".to_string();
        let data3 = "cooking recipes for pasta".to_string();

        let tags1 = vec!["learning".to_owned(), "vectors".to_owned()];
        let tags2 = vec!["vectors".to_owned(), "search".to_owned()];
        let tags3 = vec!["cooking".to_owned(), "food".to_owned()];

        let embedding1 = repo.insert(data1.clone(), tags1.clone()).await.unwrap();
        repo.insert(data2.clone(), tags2.clone()).await.unwrap();
        repo.insert(data3.clone(), tags3.clone()).await.unwrap();

        // Search using the first embedding
        let results = repo.search(embedding1.clone(), vec![], 2).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].data, data1); // Most similar should be itself

        // Search with tags

        let new_search = Embedder::embed("i like eating food".to_string()).unwrap();
        let results = repo.search(new_search, vec![], 2).await.unwrap();
        assert!(!results.is_empty());
        assert!(results[0].data.contains("cooking"));
    }

    #[tokio::test]
    async fn test_get() {
        let repo = EmbeddingRepositoryTest::init();

        // Insert test data
        let data = "test embedding data".to_string();
        let tags = vec!["test".to_owned(), "data".to_owned()];
        let embedding = repo.insert(data.clone(), tags.clone()).await.unwrap();

        // Get the ID from a search
        let results = repo.search(embedding, vec![], 1).await.unwrap();
        assert_eq!(results.len(), 1);
        let id = results[0].id;

        // Test successful retrieval
        let result = repo.get(id).await.unwrap();
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.data, data);
        assert_eq!(info.tags, tags);
        assert_eq!(info.embedding.as_slice().len(), 384);
    }

    #[tokio::test]
    async fn test_comprehensive_search() {
        let repo = EmbeddingRepositoryTest::init();
        // Insert multiple learning examples about different topics
        let rust_async = vec![
            "Rust's async/await syntax enables concurrent programming without data races.",
            "Tokio provides async primitives like channels and mutexes for safe concurrency.",
            "Spawn multiple tasks to handle concurrent operations efficiently.",
        ];
        for text in rust_async {
            repo.insert(text.to_string(), vec!["learning".to_owned()])
                .await
                .unwrap();
        }

        let rust_threading = vec![
            "Rust's ownership system and lifetimes ensure thread safety.",
            "Send and Sync traits enable safe concurrent data sharing.",
            "Arc and Mutex provide thread-safe reference counting and locking.",
        ];
        for text in rust_threading {
            repo.insert(text.to_string(), vec!["learning".to_owned()])
                .await
                .unwrap();
        }

        let graphql = vec![
            "GraphQL schemas define the structure of your API.",
            "Query resolvers handle data fetching in GraphQL.",
        ];
        for text in graphql {
            repo.insert(text.to_string(), vec!["learning".to_owned()])
                .await
                .unwrap();
        }

        let docker = vec![
            "Docker images are built using Dockerfiles.",
            "Container orchestration manages deployment and scaling.",
        ];
        for text in docker {
            repo.insert(text.to_string(), vec!["learning".to_owned()])
                .await
                .unwrap();
        }

        // Search with tags
        let query = "Tell me about APIs";

        let query_embedding = Embedder::embed(query.to_string()).unwrap();
        let results = repo.search(query_embedding, vec![], 2).await.unwrap();

        // Verify we get GraphQL results when searching with the graphql tag
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.data.contains("GraphQL")));
    }
}
