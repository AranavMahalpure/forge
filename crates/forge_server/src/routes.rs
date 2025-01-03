use std::sync::Arc;
use uuid::Uuid;

const SERVER_PORT: u16 = 8080;

use axum::extract::{Json, State};
use axum::response::sse::{Event, Sse};
use axum::response::Html;
use axum::routing::{get, post};
use axum::Router;
use forge_env::Environment;
use forge_provider::{Model, Request};
use forge_tool::ToolDefinition;
use serde::Serialize;
use tokio_stream::{Stream, StreamExt};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::context::ContextEngine;
use crate::{ChatRequest, Errata, File, Result, RootAPIService, Service};
use crate::{ChatResponse, Conversation};

pub struct API {
    // TODO: rename Conversation to Server and drop Server
    api_key: String,
}

impl Default for API {
    fn default() -> Self {
        dotenv::dotenv().ok();
        let api_key = std::env::var("FORGE_KEY").expect("FORGE_KEY must be set");
        Self { api_key }
    }
}

async fn context_html_handler(
    State(state): State<Arc<dyn RootAPIService>>, 
    axum::extract::Path(id): axum::extract::Path<Uuid>
) -> Html<String> {
    let context = state.context(id).await;
    let engine = ContextEngine::new(context);
    Html(engine.render_html())
}

impl API {
    pub async fn launch(self) -> Result<()> {
        tracing_subscriber::fmt().init();
        let env = Environment::from_env().await?;
        let state = Arc::new(Service::root_api_service(env, self.api_key));

        if dotenv::dotenv().is_ok() {
            info!("Loaded .env file");
        }

        // Setup HTTP server
        let app = Router::new()
            .route("/conversation", post(conversation_handler))
            .route("/completions", get(completions_handler))
            .route("/health", get(health_handler))
            .route("/tools", get(tools_handler))
            .route("/models", get(models_handler))
            .route("/context/{id}", get(context_handler))
            .route("/context/{id}/html", get(context_html_handler))
            .route("/conversations", get(conversations_handler))
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods([
                        axum::http::Method::GET,
                        axum::http::Method::POST,
                        axum::http::Method::OPTIONS,
                    ])
                    .allow_headers([
                        axum::http::header::CONTENT_TYPE,
                        axum::http::header::AUTHORIZATION,
                    ]),
            )
            .with_state(state.clone());

        // Spawn HTTP server
        let server = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{SERVER_PORT}"))
                .await
                .unwrap();
            info!("Server running on http://127.0.0.1:{SERVER_PORT}");
            axum::serve(listener, app).await.unwrap();
        });

        // Wait for server to complete (though it runs indefinitely)
        let _ = server.await;

        Ok(())
    }
}

async fn completions_handler(
    State(state): State<Arc<dyn RootAPIService>>,
) -> axum::Json<Vec<File>> {
    let files = state
        .completions()
        .await
        .expect("Failed to get completions");
    axum::Json(files)
}

#[axum::debug_handler]
async fn conversation_handler(
    State(state): State<Arc<dyn RootAPIService>>,
    Json(request): Json<ChatRequest>,
) -> Sse<impl Stream<Item = std::result::Result<Event, std::convert::Infallible>>> {
    let stream = state
        .chat(request)
        .await
        .expect("Engine failed to respond with a chat message");
    Sse::new(stream.map(|message| {
        let data =
            serde_json::to_string(&message.unwrap_or_else(|error| Errata::from(&error).into()))
                .expect("Failed to serialize message");
        Ok(Event::default().data(data))
    }))
}

#[axum::debug_handler]
async fn tools_handler(State(state): State<Arc<dyn RootAPIService>>) -> Json<ToolResponse> {
    let tools = state.tools().await;
    Json(ToolResponse { tools })
}

async fn health_handler() -> axum::response::Response {
    axum::response::Response::builder()
        .status(200)
        .body(axum::body::Body::empty())
        .unwrap()
}

async fn models_handler(State(state): State<Arc<dyn RootAPIService>>) -> Json<ModelResponse> {
    let models = state.models().await.unwrap_or_default();
    Json(ModelResponse { models })
}

async fn conversations_handler(State(state): State<Arc<dyn RootAPIService>>) -> Json<ConversationResponse> {
    let conversations = state.conversations().await.unwrap_or_default();
    Json(ConversationResponse { conversations })
}

#[axum::debug_handler]
async fn context_handler(
    State(state): State<Arc<dyn RootAPIService>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Json<ContextResponse> {
    let context = state.context(id).await;
    Json(ContextResponse { context })
}

#[derive(Serialize)]
pub struct ContextResponse {
    context: Request,
}

#[derive(Serialize)]
pub struct ModelResponse {
    models: Vec<Model>,
}

#[derive(Serialize)]
pub struct ToolResponse {
    tools: Vec<ToolDefinition>,
}

#[derive(Serialize)]
pub struct ConversationResponse {
    conversations: Vec<Conversation>,
}
