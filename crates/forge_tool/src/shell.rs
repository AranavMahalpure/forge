use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Result;
use forge_domain::{NamedTool, ToolCallService, ToolDescription, ToolName};
use forge_tool_macros::ToolDescription;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct ShellInput {
    /// The shell command to execute.
    pub command: String,
    /// The working directory where the command should be executed.
    pub cwd: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct ShellOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    pub success: bool,
}

/// Execute shell commands with safety checks and validation. This tool provides
/// controlled access to system shell commands while preventing dangerous
/// operations through a comprehensive blacklist and validation system.
#[derive(ToolDescription)]
pub struct Shell {
    blacklist: HashSet<String>,
}

impl Default for Shell {
    fn default() -> Self {
        let mut blacklist = HashSet::new();
        // File System Destruction Commands
        blacklist.insert("rm".to_string());
        blacklist.insert("rmdir".to_string());
        blacklist.insert("del".to_string());

        // Disk/Filesystem Commands
        blacklist.insert("format".to_string());
        blacklist.insert("mkfs".to_string());
        blacklist.insert("dd".to_string());

        // Permission/Ownership Commands
        blacklist.insert("chmod".to_string());
        blacklist.insert("chown".to_string());

        // Privilege Escalation Commands
        blacklist.insert("sudo".to_string());
        blacklist.insert("su".to_string());

        // Code Execution Commands
        blacklist.insert("exec".to_string());
        blacklist.insert("eval".to_string());

        // System Communication Commands
        blacklist.insert("write".to_string());
        blacklist.insert("wall".to_string());

        // System Control Commands
        blacklist.insert("shutdown".to_string());
        blacklist.insert("reboot".to_string());
        blacklist.insert("init".to_string());

        Shell { blacklist }
    }
}

impl Shell {
    fn validate_command(&self, command: &str) -> Result<(), String> {
        let base_command = command
            .split_whitespace()
            .next()
            .ok_or_else(|| "Empty command".to_string())?;

        if self.blacklist.contains(base_command) {
            return Err(format!("Command '{}' is not allowed", base_command));
        }

        Ok(())
    }

    async fn execute_command(&self, command: &str, cwd: PathBuf) -> Result<ShellOutput, String> {
        // Use shell from current environment
        let shell_path = if cfg!(target_os = "windows") {
            std::env::var("ComSpec").unwrap_or_else(|_| "cmd.exe".to_string())
        } else {
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
        };

        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new(shell_path);
            c.args(["/C", command]);
            c
        } else {
            let mut c = Command::new(shell_path);
            c.args(["-c", command]);
            c
        };

        cmd.current_dir(cwd);

        let output = cmd.output().await.map_err(|e| e.to_string())?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(ShellOutput {
            stdout: if stdout.is_empty() {
                None
            } else {
                Some(stdout)
            },
            stderr: if stderr.is_empty() {
                None
            } else {
                Some(stderr)
            },
            success: output.status.success(),
        })
    }
}

impl NamedTool for Shell {
    fn tool_name(&self) -> ToolName {
        ToolName::new("execute_command")
    }
}

#[async_trait::async_trait]
impl ToolCallService for Shell {
    type Input = ShellInput;
    type Output = ShellOutput;

    async fn call(&self, input: Self::Input) -> Result<Self::Output, String> {
        self.validate_command(&input.command)?;
        self.execute_command(&input.command, input.cwd).await
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use pretty_assertions::assert_eq;

    use super::*;

    #[tokio::test]
    async fn test_shell_echo() {
        let shell = Shell::default();
        let result = shell
            .call(ShellInput {
                command: "echo 'Hello, World!'".to_string(),
                cwd: env::current_dir().unwrap(),
            })
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.stdout.as_ref().unwrap().contains("Hello, World!"));
        assert!(result.stderr.is_none());
    }

    #[tokio::test]
    async fn test_shell_with_working_directory() {
        let shell = Shell::default();
        let temp_dir = fs::canonicalize(env::temp_dir()).unwrap();

        let result = shell
            .call(ShellInput {
                command: if cfg!(target_os = "windows") {
                    "cd".to_string()
                } else {
                    "pwd".to_string()
                },
                cwd: temp_dir.clone(),
            })
            .await
            .unwrap();

        assert!(result.success);
        let output_path = PathBuf::from(result.stdout.as_ref().unwrap().trim());
        let actual_path = match fs::canonicalize(output_path.clone()) {
            Ok(path) => path,
            Err(_) => output_path,
        };
        let expected_path = temp_dir.as_path();

        assert_eq!(
            actual_path, expected_path,
            "\nExpected path: {:?}\nActual path: {:?}",
            expected_path, actual_path
        );
        assert!(result.stderr.is_none());
    }

    #[tokio::test]
    async fn test_shell_invalid_command() {
        let shell = Shell::default();
        let result = shell
            .call(ShellInput {
                command: "nonexistentcommand".to_string(),
                cwd: env::current_dir().unwrap(),
            })
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.stderr.is_some());
    }

    #[tokio::test]
    async fn test_shell_blacklisted_command() {
        let shell = Shell::default();
        let result = shell
            .call(ShellInput {
                command: "rm -rf /".to_string(),
                cwd: env::current_dir().unwrap(),
            })
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not allowed"));
    }

    #[tokio::test]
    async fn test_shell_empty_command() {
        let shell = Shell::default();
        let result = shell
            .call(ShellInput { command: "".to_string(), cwd: env::current_dir().unwrap() })
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Empty command"));
    }

    #[tokio::test]
    async fn test_shell_pwd() {
        let shell = Shell::default();
        
        // Create a temporary directory and set it as current dir
        let temp_dir = tempfile::tempdir().unwrap();
        env::set_current_dir(&temp_dir).unwrap();
        
        // Get the canonical path to compare against
        let temp_path = fs::canonicalize(&temp_dir).unwrap();
        
        let result = shell
            .call(ShellInput {
                command: if cfg!(target_os = "windows") {
                    "cd".to_string()
                } else {
                    "pwd".to_string()
                },
                cwd: temp_path.clone(),
            })
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.stderr.is_none());
        
        let stdout = result.stdout.as_ref().expect("Expected stdout").trim();
        
        // Clean up paths by removing /private prefix (macOS specific)
        let stdout_path = PathBuf::from(stdout.strip_prefix("/private").unwrap_or(stdout));
        let temp_path_str = temp_path.to_string_lossy();
        let temp_path_clean = temp_path_str.strip_prefix("/private").unwrap_or(&temp_path_str);
        
        assert_eq!(stdout_path.to_string_lossy(), temp_path_clean);
    }

    #[tokio::test]
    async fn test_shell_multiple_commands() {
        let shell = Shell::default();
        let result = shell
            .call(ShellInput {
                command: "echo 'first' && echo 'second'".to_string(),
                cwd: env::current_dir().unwrap(),
            })
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.stdout.as_ref().unwrap().contains("first"));
        assert!(result.stdout.as_ref().unwrap().contains("second"));
        assert!(result.stderr.is_none());
    }

    #[tokio::test]
    async fn test_shell_with_environment_variables() {
        let shell = Shell::default();
        let result = shell
            .call(ShellInput {
                command: "echo $PATH".to_string(),
                cwd: env::current_dir().unwrap(),
            })
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.stdout.is_some());
        assert!(result.stderr.is_none());
    }

    #[test]
    fn test_description() {
        assert!(Shell::default().description().len() > 100)
    }
}
