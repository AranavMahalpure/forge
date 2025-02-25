use async_trait::async_trait;
use postgrest::Postgrest;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::data::models::ApiKey;
use crate::error::{Error, Result};
use crate::NewApiKey;

// Table name for the API keys.
const API_KEYS_TABLE: &str = "api_keys_table";

#[async_trait]
pub trait ApiKeyRepository: Send + Sync {
    async fn save(&self, api_key: NewApiKey) -> Result<ApiKey>;
    async fn find_by_key_id(&self, user_id: &str, key_id: Uuid) -> Result<Option<ApiKey>>;
    async fn list_by_user_id(&self, user_id: &str) -> Result<Vec<ApiKey>>;
    async fn delete_by_key_id(&self, user_id: &str, key_id: Uuid) -> Result<()>;
    async fn find_by_key(&self, key: &str) -> Result<Option<ApiKey>>;
}

/// A Supabase-backed implementation of the API key repository
pub struct ApiKeyRepositoryImpl {
    client: Postgrest,
}

impl ApiKeyRepositoryImpl {
    pub fn new(client: Postgrest) -> Self {
        Self { client }
    }
}

#[async_trait]
impl ApiKeyRepository for ApiKeyRepositoryImpl {
    async fn save(&self, api_key: NewApiKey) -> Result<ApiKey> {
        debug!("Creating API key in database");
        let response = self
            .client
            .from(API_KEYS_TABLE)
            .insert(api_key)
            .execute()
            .await
            .map_err(|e| {
                error!("Database error while creating API key: {}", e);
                Error::Database(e.to_string())
            })?;

        if !response.status().is_success() {
            error!("Failed to create API key, status: {}", response.status());
            return Err(Error::Database("Failed to create API key".to_string()));
        }

        let created_key = response
            .json()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        info!("API key created successfully");
        Ok(created_key)
    }

    async fn find_by_key_id(&self, user_id: &str, key_id: Uuid) -> Result<Option<ApiKey>> {
        debug!("Fetching API key from database");
        let response = self
            .client
            .from(API_KEYS_TABLE)
            .select("*")
            .eq("id", key_id.to_string())
            .eq("user_id", user_id)
            .eq("is_deleted", "false")
            .execute()
            .await
            .map_err(|e| {
                error!("Database error while fetching API key: {}", e);
                Error::Database(e.to_string())
            })?;

        if !response.status().is_success() {
            error!("Failed to fetch API key, status: {}", response.status());
            return Err(Error::Database("Failed to get API key".to_string()));
        }

        let api_keys: Vec<ApiKey> = response.json().await.map_err(|e| {
            error!("Failed to deserialize API key: {}", e);
            Error::Database(e.to_string())
        })?;

        info!(found = !api_keys.is_empty(), "API key lookup complete");
        Ok(api_keys.into_iter().next())
    }

    async fn list_by_user_id(&self, user_id: &str) -> Result<Vec<ApiKey>> {
        debug!("Listing API keys from database");
        let response = self
            .client
            .from(API_KEYS_TABLE)
            .select("*")
            .eq("is_deleted", "false")
            .eq("user_id", user_id)
            .execute()
            .await
            .map_err(|e| {
                error!("Database error while listing API keys: {}", e);
                Error::Database(e.to_string())
            })?;

        if !response.status().is_success() {
            error!("Failed to list API keys, status: {}", response.status());
            return Err(Error::Database("Failed to list API keys".to_string()));
        }

        let api_keys: Vec<ApiKey> = response.json().await.map_err(|e| {
            error!("Failed to deserialize API keys: {}", e);
            Error::Database(e.to_string())
        })?;

        info!(count = api_keys.len(), "Successfully retrieved API keys");
        Ok(api_keys)
    }

    async fn delete_by_key_id(&self, user_id: &str, key_id: Uuid) -> Result<()> {
        // TODO: use soft delete to delete the key.
        debug!("Deleting API key from database");
        let response = self
            .client
            .from(API_KEYS_TABLE)
            .delete()
            .eq("id", key_id.to_string())
            .eq("user_id", user_id)
            .eq("is_deleted", "false")
            .execute()
            .await
            .map_err(|e| {
                error!("Database error while deleting API key: {}", e);
                Error::Database(e.to_string())
            })?;

        if !response.status().is_success() {
            error!("Failed to delete API key, status: {}", response.status());
            return Err(Error::Database("Failed to delete API key".to_string()));
        }

        info!("API key deleted successfully");
        Ok(())
    }

    async fn find_by_key(&self, key: &str) -> Result<Option<ApiKey>> {
        debug!("Finding API key from database");
        let response = self
            .client
            .from(API_KEYS_TABLE)
            .select("*")
            .eq("key", key)
            .eq("is_deleted", "false")
            .limit(1)
            .execute()
            .await
            .map_err(|e| {
                error!("Database error while finding API key: {}", e);
                Error::Database(e.to_string())
            })?;

        if !response.status().is_success() {
            error!("Failed to find API key, status: {}", response.status());
            return Err(Error::Database("Failed to find API key".to_string()));
        }

        let api_keys: Vec<ApiKey> = response.json().await.map_err(|e| {
            error!("Failed to deserialize API key: {}", e);
            Error::Database(e.to_string())
        })?;

        info!(found = !api_keys.is_empty(), "API key lookup complete");
        Ok(api_keys.into_iter().next())
    }
}
