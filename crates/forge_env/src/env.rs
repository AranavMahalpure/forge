use derive_setters::Setters;
use forge_walker::Walker;
use serde::Serialize;

use crate::Result;

#[derive(Default, Serialize, Debug, Setters, Clone)]
#[serde(rename_all = "camelCase")]
#[setters(strip_option)]
/// Represents the environment in which the application is running.
pub struct Environment {
    /// The operating system of the environment.
    pub os: String,
    /// The current working directory.
    pub cwd: String,
    /// The shell being used.
    pub shell: String,
    /// The home directory, if available.
    pub home: Option<String>,
    /// A list of files in the current working directory.
    pub cwd_files: Vec<String>,
}

impl Environment {
    pub async fn from_env() -> Result<Self> {
        let cwd = std::env::current_dir()?;
        let files = match Walker::new(cwd.clone()).get().await {
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
            cwd_files: files,
        })
    }
}
