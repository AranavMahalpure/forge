use anyhow::{Context, Result};
use forge_domain::{ChatCompletionMessage, Context as ChatContext, HostType, Model, ModelId, Parameters, ProviderService, ResultStream};
use forge_open_router::OpenRouter;
use moka2::future::Cache;
use forge_ollama::Ollama;

use super::Service;

impl Service {
    pub fn provider_service(api_key: Option<impl ToString>, host_type: HostType) -> impl ProviderService {
        Live::new(api_key, host_type)
    }
}

struct Live {
    provider: Box<dyn ProviderService>,
    cache: Cache<ModelId, Parameters>,
}

impl Live {
    fn new(api_key: Option<impl ToString>, host_type: HostType) -> Self {
        let provider: Box<dyn ProviderService> = match host_type {
            HostType::Ollama => Box::new(Ollama::new()),
            HostType::OpenRouter => Box::new(OpenRouter::new(api_key.expect("API key is required").to_string())),
        };

        Self { provider, cache: Cache::new(1024) }
    }
}

#[async_trait::async_trait]
impl ProviderService for Live {
    async fn chat(
        &self,
        model_id: &ModelId,
        request: ChatContext,
    ) -> ResultStream<ChatCompletionMessage, anyhow::Error> {
        self.provider.chat(model_id, request).await
    }

    async fn models(&self) -> Result<Vec<Model>> {
        self.provider.models().await
    }

    async fn parameters(&self, model: &ModelId) -> anyhow::Result<Parameters> {
        Ok(self
            .cache
            .try_get_with_by_ref(model, async {
                self.provider
                    .parameters(model)
                    .await
                    .with_context(|| format!("Failed to get parameters for model: {}", model))
            })
            .await
            .map_err(|e| anyhow::anyhow!(e))?)
    }
}
