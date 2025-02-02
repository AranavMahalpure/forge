use std::path::PathBuf;

use async_trait::async_trait;
use derive_setters::Setters;
use serde::Serialize;

#[derive(Default, Serialize, Debug, Setters, Clone)]
#[serde(rename_all = "camelCase")]
#[setters(strip_option)]
/// Represents the environment in which the application is running.
pub struct Environment {
    /// The operating system of the environment.
    pub os: String,
    /// The current working directory.
    pub cwd: PathBuf,
    /// The home directory.
    pub home: Option<PathBuf>,
    /// The shell being used.
    pub shell: String,
    /// The Forge API key.
    pub api_key: Option<String>,
    /// The large model ID.
    pub large_model_id: String,
    /// The small model ID.
    pub small_model_id: String,

    /// The base path relative to which everything else stored.
    pub base_path: PathBuf,
    /// The host type.
    /// For example, it could be Ollama or OpenRouter.
    pub host_type: HostType,
}

#[derive(Default, Serialize, Debug, Clone, strum_macros::EnumString)]
#[serde(rename_all = "camelCase")]
pub enum HostType {
    #[default]
    #[strum(ascii_case_insensitive)]
    Ollama,
    #[strum(ascii_case_insensitive)]
    OpenRouter,
}

impl Environment {
    pub fn db_path(&self) -> PathBuf {
        self.base_path.clone()
    }

    pub fn log_path(&self) -> PathBuf {
        self.base_path.join("logs")
    }

    pub fn history_path(&self) -> PathBuf {
        self.base_path.clone()
    }
    /// Config dir for Forge.
    pub db_path: String,
    /// The host type.
    /// For example, it could be Ollama or OpenRouter.
    pub host_type: HostType,
}

#[derive(Default, Serialize, Debug, Clone, strum_macros::EnumString)]
#[serde(rename_all = "camelCase")]
pub enum HostType {
    #[default]
    #[strum(ascii_case_insensitive)]
    Ollama,
    #[strum(ascii_case_insensitive)]
    OpenRouter,
}

/// Repository for accessing system environment information
#[async_trait]
pub trait EnvironmentRepository {
    /// Get the current environment information including:
    /// - Operating system
    /// - Current working directory
    /// - Home directory
    /// - Default shell
    async fn get_environment(&self) -> anyhow::Result<Environment>;
}
