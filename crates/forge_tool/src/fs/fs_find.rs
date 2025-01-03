use std::collections::HashSet;
use std::path::Path;

use forge_tool_macros::Description as DescriptionDerive;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{Description, ToolCallService};

#[derive(Deserialize, JsonSchema, Serialize, Clone)]
pub struct FSSearchInput {
    /// The path of the directory to search in (relative to the current working
    /// directory). This directory will be recursively searched.
    #[serde(rename = "@path")]
    pub path: String,
    /// The regular expression pattern to search for. Uses Rust regex syntax.
    #[serde(rename = "@regex")]
    pub regex: String,
    /// Glob pattern to filter files (e.g., '*.ts' for TypeScript files). If not
    /// provided, it will search all files (*).
    #[serde(rename = "@file_pattern")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_pattern: Option<String>,
}

/// Request to perform a regex search across files in a specified directory,
/// providing context-rich results. This tool searches for patterns or specific
/// content across multiple files, displaying each match with encapsulating
/// context.
#[derive(DescriptionDerive)]
pub struct FSSearch;

#[derive(Serialize, JsonSchema)]
pub struct FSSearchOutput {
    #[serde(flatten)]
    args: FSSearchInput,
    matches: Vec<String>,
}

#[async_trait::async_trait]
impl ToolCallService for FSSearch {
    type Input = FSSearchInput;
    type Output = FSSearchOutput;

    async fn call(&self, input: Self::Input) -> Result<Self::Output, String> {
        use regex::Regex;
        use walkdir::WalkDir;

        let dir = Path::new(&input.path);
        if !dir.exists() {
            return Err("Directory does not exist".to_string());
        }

        // Create case-insensitive regex pattern
        let pattern = if input.regex.is_empty() {
            ".*".to_string()
        } else {
            format!("(?i){}", regex::escape(&input.regex)) // Add back regex::escape for literal matches
        };
        let regex = Regex::new(&pattern).map_err(|e| e.to_string())?;

        let mut matches = Vec::new();
        let mut seen_paths = HashSet::new();
        let walker = WalkDir::new(dir)
            .follow_links(false)
            .same_file_system(true)
            .into_iter();

        let entries = if let Some(ref pattern) = input.file_pattern {
            let glob = glob::Pattern::new(pattern).map_err(|e| e.to_string())?;
            walker
                .filter_entry(move |e| {
                    if !e.file_type().is_file() {
                        return true; // Keep traversing directories
                    }
                    e.file_name()
                        .to_str()
                        .map(|name| glob.matches(name))
                        .unwrap_or(false)
                })
                .filter_map(Result::ok)
                .collect::<Vec<_>>()
        } else {
            walker.filter_map(Result::ok).collect::<Vec<_>>()
        };

        for entry in entries {
            let path = entry.path().to_string_lossy();

            let name = entry.file_name().to_string_lossy();
            let is_file = entry.file_type().is_file();
            // let is_dir = entry.file_type().is_dir();

            // For empty pattern, only match files
            if input.regex.is_empty() {
                if is_file && seen_paths.insert(path.to_string()) {
                    matches.push(format!("File: {}\nLines 1-1:\n{}", path, path));
                }
                continue;
            }

            // Check filename and directory name for match
            if regex.is_match(&name) {
                if seen_paths.insert(path.to_string()) {
                    matches.push(format!("File: {}\nLines 1-1:\n{}", path, name));
                }
                if !is_file {
                    continue;
                }
            }

            // Skip content check for directories
            if !is_file {
                continue;
            }

            // Skip content check if already matched by name
            if seen_paths.contains(&path.to_string()) {
                continue;
            }

            // Check file content
            let content = match tokio::fs::read_to_string(entry.path()).await {
                Ok(content) => content,
                Err(_) => continue,
            };

            let lines: Vec<&str> = content.lines().collect();
            let mut content_matches = Vec::new();

            for (line_num, line) in lines.iter().enumerate() {
                if regex.is_match(line) {
                    // Get context (3 lines before and after)
                    let start = line_num.saturating_sub(3);
                    let end = (line_num + 4).min(lines.len());
                    let context = lines[start..end].join("\n");

                    content_matches.push(format!(
                        "File: {}\nLines {}-{}:\n{}\n",
                        path,
                        start + 1,
                        end,
                        context
                    ));
                }
            }

            if !content_matches.is_empty() {
                matches.extend(content_matches);
                seen_paths.insert(path.to_string());
            }
        }

        Ok(FSSearchOutput { matches, args: input.clone() })
    }
}

#[cfg(test)]
mod test {
    use tempfile::TempDir;
    use tokio::fs;

    use super::*;

    #[tokio::test]
    async fn test_fs_search_content() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("test1.txt"), "Hello test world")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("test2.txt"), "Another test case")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("other.txt"), "No match here")
            .await
            .unwrap();

        let fs_search = FSSearch;
        let result = fs_search
            .call(FSSearchInput {
                path: temp_dir.path().to_string_lossy().to_string(),
                regex: "test".to_string(),
                file_pattern: None,
            })
            .await
            .unwrap();

        assert_eq!(result.matches.len(), 2);
        assert!(result.matches.iter().any(|p| p.contains("test1.txt")));
        assert!(result.matches.iter().any(|p| p.contains("test2.txt")));
        assert!(result.matches.iter().all(|p| p.contains("Lines")));
    }

    #[tokio::test]
    async fn test_fs_search_with_pattern() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("test1.txt"), "Hello test world")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("test2.rs"), "fn test() {}")
            .await
            .unwrap();

        let fs_search = FSSearch;
        let result = fs_search
            .call(FSSearchInput {
                path: temp_dir.path().to_string_lossy().to_string(),
                regex: "test".to_string(),
                file_pattern: Some("*.rs".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(result.matches.len(), 1);
        assert!(result.matches.iter().any(|p| p.contains("test2.rs")));
    }

    #[tokio::test]
    async fn test_fs_search_with_context() {
        let temp_dir = TempDir::new().unwrap();
        let content = "line 1\nline 2\ntest line\nline 4\nline 5";

        fs::write(temp_dir.path().join("test.txt"), content)
            .await
            .unwrap();

        let fs_search = FSSearch;
        let result = fs_search
            .call(FSSearchInput {
                path: temp_dir.path().to_string_lossy().to_string(),
                regex: "test".to_string(),
                file_pattern: None,
            })
            .await
            .unwrap();

        assert_eq!(result.matches.len(), 1);
        let output = &result.matches[0];
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3);

        let output_path = lines[0].split(' ').last().unwrap();
        let output = std::fs::read_to_string(output_path).unwrap();

        assert!(output.contains("line 1"));
        assert!(output.contains("line 2"));
        assert!(output.contains("test line"));
        assert!(output.contains("line 4"));
        assert!(output.contains("line 5"));
    }

    #[tokio::test]
    async fn test_fs_search_recursive() {
        let temp_dir = TempDir::new().unwrap();

        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir).await.unwrap();

        fs::write(temp_dir.path().join("test1.txt"), "")
            .await
            .unwrap();
        fs::write(sub_dir.join("test2.txt"), "").await.unwrap();
        fs::write(sub_dir.join("other.txt"), "").await.unwrap();

        let fs_search = FSSearch;
        let result = fs_search
            .call(FSSearchInput {
                path: temp_dir.path().to_string_lossy().to_string(),
                regex: "test".to_string(),
                file_pattern: None,
            })
            .await
            .unwrap();

        assert_eq!(result.matches.len(), 2);
        assert!(result.matches.iter().any(|p| p.ends_with("test1.txt")));
        assert!(result.matches.iter().any(|p| p.ends_with("test2.txt")));
    }

    #[tokio::test]
    async fn test_fs_search_case_insensitive() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("TEST.txt"), "")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("TeSt2.txt"), "")
            .await
            .unwrap();

        let fs_search = FSSearch;
        let result = fs_search
            .call(FSSearchInput {
                path: temp_dir.path().to_string_lossy().to_string(),
                regex: "test".to_string(),
                file_pattern: None,
            })
            .await
            .unwrap();
        assert_eq!(result.matches.len(), 2);

        assert!(result.matches.iter().any(|p| p.ends_with("TEST.txt")));
        assert!(result.matches.iter().any(|p| p.ends_with("TeSt2.txt")));
    }

    #[tokio::test]
    async fn test_fs_search_empty_pattern() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("test.txt"), "")
            .await
            .unwrap();

        let fs_search = FSSearch;
        let result = fs_search
            .call(FSSearchInput {
                path: temp_dir.path().to_string_lossy().to_string(),
                regex: "".to_string(),
                file_pattern: None,
            })
            .await
            .unwrap();

        assert_eq!(result.matches.len(), 1);
        assert!(result.matches.iter().any(|p| p.ends_with("test.txt")));
    }

    #[tokio::test]
    async fn test_fs_search_nonexistent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_dir = temp_dir.path().join("nonexistent");

        let fs_search = FSSearch;
        let result = fs_search
            .call(FSSearchInput {
                path: nonexistent_dir.to_string_lossy().to_string(),
                regex: "test".to_string(),
                file_pattern: None,
            })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fs_search_directory_names() {
        let temp_dir = TempDir::new().unwrap();

        fs::create_dir(temp_dir.path().join("test_dir"))
            .await
            .unwrap();
        fs::create_dir(temp_dir.path().join("test_dir").join("nested"))
            .await
            .unwrap();
        fs::create_dir(temp_dir.path().join("other_dir"))
            .await
            .unwrap();

        let fs_search = FSSearch;
        let result = fs_search
            .call(FSSearchInput {
                path: temp_dir.path().to_string_lossy().to_string(),
                regex: "test".to_string(),
                file_pattern: None,
            })
            .await
            .unwrap();

        assert_eq!(result.matches.len(), 1);
        assert!(result.matches.iter().any(|p| p.ends_with("test_dir")));
    }

    #[test]
    fn serialize_to_xml() {
        let output = FSSearchOutput {
            matches:vec![
                "File: /var/folders/99/v0n6z0gj5yj3j5vvsyfmvx100000gn/T/.tmpCdUifn/TeSt2.txt\nLines 1-1:\nTeSt2.txt".to_string(),
                "File: /var/folders/99/v0n6z0gj5yj3j5vvsyfmvx100000gn/T/.tmpCdUifn/TEST.txt\nLines 1-1:\nTEST.txt".to_string(),
            ],
            args: FSSearchInput{
                path: ".".to_string(),
                regex: "*.txt".to_string(),
                file_pattern: None
            }
        };
        let mut buffer = Vec::new();
        let mut writer = quick_xml::Writer::new_with_indent(&mut buffer, b' ', 4);
        writer.write_serializable("fs_search", &output).unwrap();
        insta::assert_snapshot!(std::str::from_utf8(&buffer).unwrap());
    }
}
