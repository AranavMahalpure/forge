use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use forge_domain::{
    ChatRequest, ChatResponse, Config, Context, Conversation, ConversationId, Environment, Model,
    ProviderService, ResultStream, ToolDefinition, ToolService,
};

use super::chat::ConversationHistory;
use super::completion::CompletionService;
use super::env::EnvironmentService;
use super::{File, Service, UIService};
use crate::{ConfigRepository, ConversationRepository};

#[async_trait::async_trait]
pub trait APIService: Send + Sync {
    async fn completions(&self) -> Result<Vec<File>>;
    async fn tools(&self) -> Vec<ToolDefinition>;
    async fn context(&self, conversation_id: ConversationId) -> Result<Context>;
    async fn models(&self) -> Result<Vec<Model>>;
    async fn chat(&self, chat: ChatRequest) -> ResultStream<ChatResponse, anyhow::Error>;
    async fn conversations(&self) -> Result<Vec<Conversation>>;
    async fn conversation(&self, conversation_id: ConversationId) -> Result<ConversationHistory>;
    async fn get_config(&self) -> Result<Config>;
    async fn set_config(&self, request: Config) -> Result<Config>;
    async fn environment(&self) -> Result<Environment>;
}

impl Service {
    pub async fn api_service() -> Result<impl APIService> {
        Live::new(std::env::current_dir()?).await
    }
}

#[derive(Clone)]
struct Live {
    provider: Arc<dyn ProviderService>,
    tool: Arc<dyn ToolService>,
    completions: Arc<dyn CompletionService>,
    ui_service: Arc<dyn UIService>,
    storage: Arc<dyn ConversationRepository>,
    config_storage: Arc<dyn ConfigRepository>,
    environment: Environment,
}

impl Live {
    async fn new(cwd: PathBuf) -> Result<Self> {
        let env = Service::environment_service(cwd).get().await?;
        let cwd: String = env.cwd.clone();

        let embedding_repository = Arc::new(Service::embedding_repository(&cwd).await?);

        let provider = Arc::new(Service::provider_service(env.api_key.clone()));
        let tool = Arc::new(Service::tool_service(embedding_repository.clone()));
        let file_read = Arc::new(Service::file_read_service());

        let system_prompt = Arc::new(Service::system_prompt(
            env.clone(),
            tool.clone(),
            provider.clone(),
            file_read.clone(),
            embedding_repository,
        ));

        let user_prompt = Arc::new(Service::user_prompt_service(file_read.clone()));
        let storage = Arc::new(Service::storage_service(&cwd)?);

        let chat_service = Arc::new(Service::chat_service(
            provider.clone(),
            system_prompt.clone(),
            tool.clone(),
            user_prompt,
        ));
        let completions = Arc::new(Service::completion_service(cwd.clone()));

        let title_service = Arc::new(Service::title_service(provider.clone()));

        let chat_service = Arc::new(Service::ui_service(
            storage.clone(),
            chat_service,
            title_service,
        ));
        let config_storage = Arc::new(Service::config_service(&cwd)?);

        Ok(Self {
            provider,
            tool,
            completions,
            ui_service: chat_service,
            storage,
            config_storage,
            environment: env,
        })
    }
}

#[async_trait::async_trait]
impl APIService for Live {
    async fn completions(&self) -> Result<Vec<File>> {
        self.completions.list().await
    }

    async fn tools(&self) -> Vec<ToolDefinition> {
        self.tool.list()
    }

    async fn context(&self, conversation_id: ConversationId) -> Result<Context> {
        Ok(self
            .storage
            .get_conversation(conversation_id)
            .await?
            .context)
    }

    async fn models(&self) -> Result<Vec<Model>> {
        Ok(self.provider.models().await?)
    }

    async fn chat(&self, chat: ChatRequest) -> ResultStream<ChatResponse, anyhow::Error> {
        Ok(self.ui_service.chat(chat).await?)
    }

    async fn conversations(&self) -> Result<Vec<Conversation>> {
        self.storage.list_conversations().await
    }

    async fn conversation(&self, conversation_id: ConversationId) -> Result<ConversationHistory> {
        Ok(self
            .storage
            .get_conversation(conversation_id)
            .await?
            .context
            .into())
    }

    async fn get_config(&self) -> Result<Config> {
        Ok(self.config_storage.get().await?)
    }

    async fn set_config(&self, request: Config) -> Result<Config> {
        self.config_storage.set(request).await
    }

    async fn environment(&self) -> Result<Environment> {
        Ok(self.environment.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use forge_domain::ModelId;
    use futures::future::join_all;
    use tokio_stream::StreamExt;

    use super::*;

    #[tokio::test]
    async fn test_e2e() {
        let api = Live::new(Path::new("../../").to_path_buf()).await.unwrap();
        let task = include_str!("./api_task.md");

        const MAX_RETRIES: usize = 3;
        const MATCH_THRESHOLD: f64 = 0.7; // 70% of crates must be found
        const SUPPORTED_MODELS: &[&str] = &[
            "anthropic/claude-3.5-sonnet:beta",
            "openai/gpt-4o-2024-11-20",
            "anthropic/claude-3.5-sonnet",
            "openai/gpt-4o",
            "openai/gpt-4o-mini",
            "google/gemini-flash-1.5",
            "anthropic/claude-3-sonnet",
        ];

        let test_futures = SUPPORTED_MODELS.iter().map(|&model| {
            let api = api.clone();
            let task = task.to_string();

            async move {
                let request = ChatRequest::new(ModelId::new(model), task);
                let expected_crates = [
                    "forge_app",
                    "forge_ci",
                    "forge_domain",
                    "forge_main",
                    "forge_open_router",
                    "forge_prompt",
                    "forge_tool",
                    "forge_tool_macros",
                    "forge_walker",
                ];

                for attempt in 0..MAX_RETRIES {
                    let response = api
                        .chat(request.clone())
                        .await
                        .unwrap()
                        .filter_map(|message| match message.unwrap() {
                            ChatResponse::Text(text) => Some(text),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .await
                        .join("")
                        .trim()
                        .to_string();

                    let found_crates: Vec<&str> = expected_crates
                        .iter()
                        .filter(|&crate_name| {
                            response.contains(&format!("<crate>{}</crate>", crate_name))
                        })
                        .cloned()
                        .collect();

                    let match_percentage = found_crates.len() as f64 / expected_crates.len() as f64;

                    if match_percentage >= MATCH_THRESHOLD {
                        println!(
                            "[{}] Successfully found {:.2}% of expected crates",
                            model,
                            match_percentage * 100.0
                        );
                        return Ok::<_, String>(());
                    }

                    if attempt < MAX_RETRIES - 1 {
                        println!(
                            "[{}] Attempt {}/{}: Found {}/{} crates: {:?}",
                            model,
                            attempt + 1,
                            MAX_RETRIES,
                            found_crates.len(),
                            expected_crates.len(),
                            found_crates
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    } else {
                        return Err(format!(
                            "[{}] Failed: Found only {}/{} crates: {:?}",
                            model,
                            found_crates.len(),
                            expected_crates.len(),
                            found_crates
                        ));
                    }
                }

                unreachable!()
            }
        });

        let results = join_all(test_futures).await;
        let errors: Vec<_> = results.into_iter().filter_map(Result::err).collect();

        if !errors.is_empty() {
            panic!("Test failures:\n{}", errors.join("\n"));
        }
    }
}
