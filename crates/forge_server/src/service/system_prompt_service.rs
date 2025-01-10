use std::sync::Arc;

use forge_domain::{Environment, ModelId, ToolService};
use forge_provider::ProviderService;
use handlebars::Handlebars;
use serde::Serialize;
use tracing::info;

use super::Service;
use crate::Result;

#[async_trait::async_trait]
pub trait SystemPromptService: Send + Sync {
    async fn get_system_prompt(&self, model: &ModelId) -> Result<String>;
}

impl Service {
    pub fn system_prompt(
        env: Environment,
        tool: Arc<dyn ToolService>,
        provider: Arc<dyn ProviderService>,
        template: String,
    ) -> impl SystemPromptService {
        Live::new(env, tool, provider, template)
    }
}

#[derive(Clone, Serialize)]
struct Context {
    env: Environment,
    tool_information: String,
    tool_supported: bool,
}

#[derive(Clone)]
struct Live {
    env: Environment,
    tool: Arc<dyn ToolService>,
    provider: Arc<dyn ProviderService>,
    template: String,
}

impl Live {
    pub fn new(
        env: Environment,
        tool: Arc<dyn ToolService>,
        provider: Arc<dyn ProviderService>,
        template: String,
    ) -> Self {
        Self { env, tool, provider, template }
    }
}

#[async_trait::async_trait]
impl SystemPromptService for Live {
    async fn get_system_prompt(&self, model: &ModelId) -> Result<String> {
        let mut hb = Handlebars::new();
        hb.set_strict_mode(true);
        hb.register_escape_fn(|str| str.to_string());

        let tool_supported = self.provider.parameters(model).await?.tool_supported;
        info!("Tool support for {}: {}", model.as_str(), tool_supported);
        let ctx = Context {
            env: self.env.clone(),
            tool_information: self.tool.usage_prompt(),
            tool_supported,
        };

        Ok(hb.render_template(self.template.as_str(), &ctx)?)
    }
}

// #[cfg(test)]
// mod tests {
//     use forge_domain::Parameters;
//     use insta::assert_snapshot;

//     use super::*;
//     use crate::service::tests::TestProvider;

//     fn test_env() -> Environment {
//         Environment {
//             os: "linux".to_string(),
//             cwd: "/home/user/project".to_string(),
//             shell: "/bin/bash".to_string(),
//             home: Some("/home/user".to_string()),
//             files: vec!["file1.txt".to_string(), "file2.txt".to_string()],
//             api_key: "test".to_string(),
//             large_model_id: "open-ai/gpt-4o".to_string(),
//             small_model_id: "open-ai/gpt-4o-mini".to_string(),
//         }
//     }

//     #[tokio::test]
//     async fn test_tool_supported() {
//         let env = test_env();
//         let tools = Arc::new(forge_tool::Service::tool_service());
//         let provider = Arc::new(
//             TestProvider::default().parameters(vec![(ModelId::default(),
// Parameters::new(true))]),         );
//         let prompt = Live::new(env, tools, provider)
//             .get_system_prompt(&ModelId::default())
//             .await
//             .unwrap();
//         assert_snapshot!(prompt);
//     }

//     #[tokio::test]
//     async fn test_tool_unsupported() {
//         let env = test_env();
//         let tools = Arc::new(forge_tool::Service::tool_service());
//         let provider = Arc::new(
//             TestProvider::default().parameters(vec![(ModelId::default(),
// Parameters::new(false))]),         );
//         let prompt = Live::new(env, tools, provider)
//             .get_system_prompt(&ModelId::default())
//             .await
//             .unwrap();
//         assert_snapshot!(prompt);
//     }
// }
