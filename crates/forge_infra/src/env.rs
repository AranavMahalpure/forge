use std::path::PathBuf;

use forge_app::EnvironmentService;
use forge_domain::{Environment, ModelId};

pub struct ForgeEnvironmentService {
    restricted: bool,
}

impl ForgeEnvironmentService {
    /// Creates a new EnvironmentFactory with current working directory
    ///
    /// # Arguments
    /// * `unrestricted` - If true, use unrestricted shell mode (sh/bash) If
    ///   false, use restricted shell mode (rbash)
    pub fn new(restricted: bool) -> Self {
        Self { restricted }
    }

    /// Get path to appropriate shell based on platform and mode
    fn get_shell_path(&self) -> String {
        if cfg!(target_os = "windows") {
            std::env::var("COMSPEC").unwrap_or("cmd.exe".to_string())
        } else if self.restricted {
            // Default to rbash in restricted mode
            "/bin/rbash".to_string()
        } else {
            // Use user's preferred shell or fallback to sh
            std::env::var("SHELL").unwrap_or("/bin/sh".to_string())
        }
    }

    pub fn get(&self) -> Environment {
        dotenv::dotenv().ok();
        let cwd = std::env::current_dir().unwrap_or(PathBuf::from("."));
        let api_key = std::env::var("OPEN_ROUTER_KEY").expect("OPEN_ROUTER_KEY must be set");
        let large_model_id = ModelId::new(
            std::env::var("FORGE_LARGE_MODEL").unwrap_or("anthropic/claude-3.5-sonnet".to_owned()),
        );
        let small_model_id = ModelId::new(
            std::env::var("FORGE_SMALL_MODEL").unwrap_or("anthropic/claude-3.5-haiku".to_owned()),
        );

        Environment {
            os: std::env::consts::OS.to_string(),
            cwd,
            shell: self.get_shell_path(),
            api_key,
            large_model_id,
            small_model_id,
            base_path: dirs::config_dir()
                .map(|a| a.join("forge"))
                .unwrap_or(PathBuf::from(".").join(".forge")),
            home: dirs::home_dir(),
        }
    }
}

impl EnvironmentService for ForgeEnvironmentService {
    fn get_environment(&self) -> Environment {
        self.get()
    }
}

pub struct TestEnvironmentService {
    large_model_id: ModelId,
    small_model_id: ModelId,
}

impl TestEnvironmentService {
    pub fn new(large_model_id: ModelId, small_model_id: ModelId) -> Self {
        Self { large_model_id, small_model_id }
    }
}

impl EnvironmentService for TestEnvironmentService {
    fn get_environment(&self) -> Environment {
        dotenv::dotenv().ok();
        let cwd = std::env::current_dir().unwrap_or(PathBuf::from("."));
        let api_key = std::env::var("OPEN_ROUTER_KEY").expect("OPEN_ROUTER_KEY must be set");
        Environment {
            os: std::env::consts::OS.to_string(),
            cwd,
            shell: "/bin/sh".to_string(),
            api_key,
            large_model_id: self.large_model_id.clone(),
            small_model_id: self.small_model_id.clone(),
            base_path: dirs::config_dir()
                .map(|a| a.join("forge"))
                .unwrap_or(PathBuf::from(".").join(".forge")),
            home: dirs::home_dir(),
        }
    }
}
