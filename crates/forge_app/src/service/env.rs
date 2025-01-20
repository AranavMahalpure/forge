use anyhow::Result;
use forge_domain::Environment;
use forge_walker::Walker;
use tokio::sync::Mutex;

use super::Service;

#[async_trait::async_trait]
pub trait EnvironmentService {
    async fn get(&self) -> Result<Environment>;
}

impl Service {
    pub fn environment_service() -> impl EnvironmentService {
        Live::new()
    }
}

struct Live(Mutex<Option<Environment>>);

impl Live {
    pub fn new() -> Self {
        Self(Mutex::new(None))
    }

    async fn from_env() -> Result<Environment> {
        dotenv::dotenv().ok();
        let api_key = std::env::var("FORGE_KEY").expect("FORGE_KEY must be set");
        let large_model_id =
            std::env::var("FORGE_LARGE_MODEL").unwrap_or("anthropic/claude-3.5-sonnet".to_owned());
        let small_model_id =
            std::env::var("FORGE_SMALL_MODEL").unwrap_or("anthropic/claude-3.5-haiku".to_owned());

        let cwd = std::env::current_dir()?;
        let files = match Walker::new(cwd.clone())
            .with_max_depth(usize::MAX)
            .get()
            .await
        {
            Ok(files) => files
                .into_iter()
                .filter(|f| !f.is_dir)
                .map(|f| f.path)
                .collect(),
            Err(_) => vec![],
        };

        Ok(Environment {
            os: std::env::consts::OS.to_string(),
            cwd: cwd.display().to_string(),
            shell: if cfg!(windows) {
                std::env::var("COMSPEC")?
            } else {
                std::env::var("SHELL").unwrap_or("/bin/sh".to_string())
            },
            home: dirs::home_dir().map(|a| a.display().to_string()),
            files,
            api_key,
            large_model_id,
            small_model_id,
        })
    }
}

#[async_trait::async_trait]
impl EnvironmentService for Live {
    async fn get(&self) -> Result<Environment> {
        let mut guard = self.0.lock().await;

        if let Some(env) = guard.as_ref() {
            return Ok(env.clone());
        } else {
            *guard = Some(Live::from_env().await?);
            Ok(guard.as_ref().unwrap().clone())
        }
    }
}
