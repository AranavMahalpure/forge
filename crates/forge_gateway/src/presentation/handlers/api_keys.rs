use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;
use validator::Validate;

use crate::presentation::dto::{ApiKeyResponse, CreateApiKeyRequest};
use crate::{
    error::{Error, Result}, presentation::middleware::auth::AuthUser, service::api_keys::ApiKeyService,
};

#[axum::debug_handler]
pub async fn create_api_key(
    State(service): State<Arc<ApiKeyService>>,
    auth_user: AuthUser,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKeyResponse>> {
    if let Err(e) = request.validate() {
        return Err(Error::BadRequest(e));
    }
    let api_key = service.create_api_key(&auth_user.id, &request.name).await?;
    Ok(Json(api_key.into()))
}

#[axum::debug_handler]
pub async fn list_api_keys(
    State(service): State<Arc<ApiKeyService>>,
    auth_user: AuthUser,
) -> Result<Json<Vec<ApiKeyResponse>>> {
    let api_keys = service.list_api_keys(&auth_user.id).await?;
    Ok(Json(api_keys.into_iter().map(Into::into).collect()))
}

#[axum::debug_handler]
pub async fn delete_api_key(
    State(service): State<Arc<ApiKeyService>>,
    auth_user: AuthUser,
    Path(key_id): Path<Uuid>,
) -> Result<()> {
    service.delete_api_key(&auth_user.id, key_id).await
}

pub async fn get_by_key_id(
    State(service): State<Arc<ApiKeyService>>,
    auth_user: AuthUser,
    Path(key_id): Path<Uuid>,
) -> Result<Json<ApiKeyResponse>> {
    let api_key = service.get_by_key_id(&auth_user.id, key_id).await?;
    Ok(Json(api_key.into()))
}
