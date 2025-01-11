use std::collections::HashSet;
use std::path::{Path, PathBuf};

use forge_domain::{Environment, ToolCallService, ToolDescription};
use forge_tool_macros::ToolDescription;
use forge_walker::Walker;
use regex::Regex;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
pub struct FSSearchInput {
    /// The path of the directory to search in (relative to the current working
    /// directory). This directory will be recursively searched.
    pub path: String,
    /// The regular expression pattern to search for. Uses Rust regex syntax.
    pub regex: String,
    /// Glob pattern to filter files (e.g., '*.ts' for TypeScript files). If not
    /// provided, it will search all files (*).
    pub file_pattern: Option<String>,
}

/// Request to perform a regex search across files in a specified directory,
/// providing context-rich results. This tool searches for patterns or specific
/// content across multiple files, displaying each match with encapsulating
/// context.
#[derive(ToolDescription)]
pub struct FSSearch {
    environment: Environment,
}

impl FSSearch {
    pub fn new(environment: Environment) -> Self {
        Self { environment }
    }
}

#[async_trait::async_trait]
impl ToolCallService for FSSearch {
    type Input = FSSearchInput;
    type Output = Vec<String>;

    async fn call(&self, input: Self::Input) -> Result<Self::Output, String> {
        let path = PathBuf::from(&input.path);

        // Validate the path before proceeding
        if !self.validate_path(&path, &self.environment).await? {
            return Err("Access to this path is not allowed".to_string());
        }

        if !path.exists() {
            return Err(format!("Directory '{}' does not exist", input.path));
        }

        // Create regex pattern - case-insensitive by default
        let pattern = format!("(?i){}", input.regex);
        let regex = Regex::new(&pattern).map_err(|e| format!("Invalid regex pattern: {}", e))?;

        let walker = Walker::new(path.clone());
        let files = walker
            .get()
            .await
            .map_err(|e| format!("Failed to walk directory: {}", e))?;

        let mut matches = Vec::new();
        let mut seen_paths = HashSet::new();

        for file in files {
            if file.is_dir {
                continue;
            }

            let file_path = Path::new(&file.path);
            let full_path = path.join(file_path);

            // Apply file pattern filter if provided
            if let Some(ref pattern) = input.file_pattern {
                let glob = glob::Pattern::new(pattern)
                    .map_err(|e| format!("Invalid glob pattern: {}", e))?;
                if let Some(filename) = file_path
                    .file_name()
                    .unwrap_or(file_path.as_os_str())
                    .to_str()
                {
                    if !glob.matches(filename) {
                        continue;
                    }
                }
            }

            // Skip if we've already processed this file
            if !seen_paths.insert(full_path.clone()) {
                continue;
            }

            // Try to read the file content
            let content = match tokio::fs::read_to_string(&full_path).await {
                Ok(content) => content,
                Err(e) => {
                    // Skip binary or unreadable files silently
                    if e.kind() != std::io::ErrorKind::InvalidData {
                        matches.push(format!("Error reading {:?}: {}", full_path.display(), e));
                    }
                    continue;
                }
            };

            // Process the file line by line
            for (line_num, line) in content.lines().enumerate() {
                if regex.is_match(line) {
                    // Format match in ripgrep style: filepath:line_num:content
                    matches.push(format!("{}:{}:{}", full_path.display(), line_num + 1, line));
                }
            }
        }

        if matches.is_empty() {
            Ok(vec![format!(
                "No matches found for pattern '{}' in path '{}'",
                input.regex, input.path
            )])
        } else {
            Ok(matches)
        }
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;
    use tokio::fs;

    use super::*;
    use crate::test_utils::setup_test_env;

    #[tokio::test]
    async fn test_fs_search_content() {
        let temp_dir = TempDir::new().unwrap();
        let environment = setup_test_env(&temp_dir).await;

        fs::write(temp_dir.path().join("test1.txt"), "Hello test world")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("test2.txt"), "Another test case")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("other.txt"), "No match here")
            .await
            .unwrap();

        let fs_search = FSSearch::new(environment);
        let result = fs_search
            .call(FSSearchInput {
                path: temp_dir.path().to_string_lossy().to_string(),
                regex: "test".to_string(),
                file_pattern: None,
            })
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|p| p.contains("test1.txt")));
        assert!(result.iter().any(|p| p.contains("test2.txt")));
    }

    #[tokio::test]
    async fn test_fs_search_with_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let environment = setup_test_env(&temp_dir).await;

        fs::write(temp_dir.path().join("test1.txt"), "Hello test world")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("test2.rs"), "fn test() {}")
            .await
            .unwrap();

        let fs_search = FSSearch::new(environment);
        let result = fs_search
            .call(FSSearchInput {
                path: temp_dir.path().to_string_lossy().to_string(),
                regex: "test".to_string(),
                file_pattern: Some("*.rs".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert!(result.iter().any(|p| p.contains("test2.rs")));
    }

    #[tokio::test]
    async fn test_fs_search_with_context() {
        let temp_dir = TempDir::new().unwrap();
        let environment = setup_test_env(&temp_dir).await;
        let content = "line 1\nline 2\ntest line\nline 4\nline 5";

        fs::write(temp_dir.path().join("test.txt"), content)
            .await
            .unwrap();

        let fs_search = FSSearch::new(environment);
        let result = fs_search
            .call(FSSearchInput {
                path: temp_dir.path().to_string_lossy().to_string(),
                regex: "test".to_string(),
                file_pattern: None,
            })
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert!(result[0].contains("test line"));
    }

    #[tokio::test]
    async fn test_fs_search_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let environment = setup_test_env(&temp_dir).await;

        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir).await.unwrap();

        fs::write(temp_dir.path().join("test1.txt"), "test content")
            .await
            .unwrap();
        fs::write(sub_dir.join("test2.txt"), "more test content")
            .await
            .unwrap();
        fs::write(sub_dir.join("best.txt"), "this is proper\n test content")
            .await
            .unwrap();

        let fs_search = FSSearch::new(environment);
        let result = fs_search
            .call(FSSearchInput {
                path: temp_dir.path().to_string_lossy().to_string(),
                regex: "test".to_string(),
                file_pattern: None,
            })
            .await
            .unwrap();

        assert_eq!(result.len(), 3);
        assert!(result.iter().any(|p| p.contains("test1.txt")));
        assert!(result.iter().any(|p| p.contains("test2.txt")));
        assert!(result.iter().any(|p| p.contains("best.txt")));
    }

    #[tokio::test]
    async fn test_fs_search_case_insensitive() {
        let temp_dir = TempDir::new().unwrap();
        let environment = setup_test_env(&temp_dir).await;

        fs::write(
            temp_dir.path().join("test.txt"),
            "TEST CONTENT\ntest content",
        )
        .await
        .unwrap();

        let fs_search = FSSearch::new(environment);
        let result = fs_search
            .call(FSSearchInput {
                path: temp_dir.path().to_string_lossy().to_string(),
                regex: "test".to_string(),
                file_pattern: None,
            })
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|p| p.contains("TEST CONTENT")));
        assert!(result.iter().any(|p| p.contains("test content")));
    }

    #[tokio::test]
    async fn test_fs_search_no_matches() {
        let temp_dir = TempDir::new().unwrap();
        let environment = setup_test_env(&temp_dir).await;

        fs::write(temp_dir.path().join("test.txt"), "content")
            .await
            .unwrap();

        let fs_search = FSSearch::new(environment);
        let result = fs_search
            .call(FSSearchInput {
                path: temp_dir.path().to_string_lossy().to_string(),
                regex: "nonexistent".to_string(),
                file_pattern: None,
            })
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert!(result[0].contains("No matches found"));
    }

    #[tokio::test]
    async fn test_fs_search_invalid_regex() {
        let temp_dir = TempDir::new().unwrap();
        let environment = setup_test_env(&temp_dir).await;

        let fs_search = FSSearch::new(environment);
        let result = fs_search
            .call(FSSearchInput {
                path: temp_dir.path().to_string_lossy().to_string(),
                regex: "[invalid".to_string(),
                file_pattern: None,
            })
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid regex pattern"));
    }

    #[tokio::test]
    async fn test_fs_search_hidden_file() {
        let temp_dir = TempDir::new().unwrap();
        let environment = setup_test_env(&temp_dir).await;

        fs::write(temp_dir.path().join(".hidden.txt"), "hidden test content")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("visible.txt"), "visible test content")
            .await
            .unwrap();

        let fs_search = FSSearch::new(environment);
        let result = fs_search
            .call(FSSearchInput {
                path: temp_dir.path().to_string_lossy().to_string(),
                regex: "test".to_string(),
                file_pattern: None,
            })
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert!(result.iter().any(|p| p.contains("visible.txt")));
        assert!(!result.iter().any(|p| p.contains(".hidden.txt")));
    }

    #[tokio::test]
    async fn test_fs_search_gitignored_file() {
        let temp_dir = TempDir::new().unwrap();
        let environment = setup_test_env(&temp_dir).await;

        fs::write(temp_dir.path().join("ignored.txt"), "ignored test content")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("tracked.txt"), "tracked test content")
            .await
            .unwrap();

        let fs_search = FSSearch::new(environment);
        let result = fs_search
            .call(FSSearchInput {
                path: temp_dir.path().to_string_lossy().to_string(),
                regex: "test".to_string(),
                file_pattern: None,
            })
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert!(result.iter().any(|p| p.contains("tracked.txt")));
        assert!(!result.iter().any(|p| p.contains("ignored.txt")));
    }
}
