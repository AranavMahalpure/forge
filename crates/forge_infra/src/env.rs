use std::path::PathBuf;

use forge_app::EnvironmentService;
use forge_domain::Environment;

pub struct ForgeEnvironmentService {
    cwd: PathBuf,
    unrestricted: bool,
}

impl ForgeEnvironmentService {
    /// Creates a new EnvironmentFactory with current working directory
    ///
    /// # Arguments
    /// * `cwd` - The current working directory for the environment
    /// * `unrestricted` - If true, use unrestricted shell mode (sh/bash) If
    ///   false, use restricted shell mode (rbash)
    pub fn new(cwd: PathBuf, unrestricted: bool) -> Self {
        Self { cwd, unrestricted }
    }

    /// Get path to appropriate shell based on platform and mode
    fn get_shell_path(unrestricted: bool) -> String {
        if cfg!(target_os = "windows") {
            if unrestricted {
                std::env::var("COMSPEC").unwrap_or("cmd.exe".to_string())
            } else {
                // TODO: Add Windows restricted shell implementation
                std::env::var("COMSPEC").unwrap_or("cmd.exe".to_string())
            }
        } else if unrestricted {
            // Use user's preferred shell or fallback to sh
            std::env::var("SHELL").unwrap_or("/bin/sh".to_string())
        } else {
            // Default to rbash in restricted mode
            "/bin/rbash".to_string()
        }
    }

    pub fn get(&self) -> Environment {
        dotenv::dotenv().ok();
        let cwd = self.cwd.clone();
        let api_key = std::env::var("OPEN_ROUTER_KEY").expect("OPEN_ROUTER_KEY must be set");
        let large_model_id =
            std::env::var("FORGE_LARGE_MODEL").unwrap_or("anthropic/claude-3.5-sonnet".to_owned());
        let small_model_id =
            std::env::var("FORGE_SMALL_MODEL").unwrap_or("anthropic/claude-3.5-haiku".to_owned());

        Environment {
            os: std::env::consts::OS.to_string(),
            cwd,
            shell: Self::get_shell_path(self.unrestricted),
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
