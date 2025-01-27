use std::path::Path;

use forge_domain::{NamedTool, ToolCallService, ToolDescription, ToolName};
use forge_tool_macros::ToolDescription;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::syn;
use crate::utils::assert_absolute_path;

#[derive(Deserialize, JsonSchema)]
pub struct FSWriteInput {
    /// The path of the file to write to (absolute path required)
    pub path: String,
    /// The content to write to the file. ALWAYS provide the COMPLETE intended
    /// content of the file, without any truncation or omissions. You MUST
    /// include ALL parts of the file, even if they haven't been modified.
    pub content: String,
    /// When set to true, allows overwriting of existing files. Defaults to
    /// false.
    pub overwrite: Option<bool>,
}

/// Use it to create a new file at a specified path with the provided content.
/// Always provide absolute paths for file locations. By default, if the file
/// already exists, the tool will return an error to prevent overwriting. Set
/// overwrite=true to allow overwriting existing files. The tool automatically
/// handles the creation of any missing intermediary directories in the
/// specified path.
#[derive(ToolDescription)]
pub struct FSWrite;

impl NamedTool for FSWrite {
    fn tool_name(&self) -> ToolName {
        ToolName::new("tool_forge_fs_write")
    }
}

#[async_trait::async_trait]
impl ToolCallService for FSWrite {
    type Input = FSWriteInput;

    async fn call(&self, input: Self::Input) -> Result<String, String> {
        // Validate absolute path requirement
        let path = Path::new(&input.path);
        assert_absolute_path(path)?;

        // Check if file already exists
        let file_exists = tokio::fs::metadata(&input.path).await.is_ok();
        if file_exists && !input.overwrite.unwrap_or(false) {
            return Err(format!(
                "File {} already exists. Set overwrite=true to overwrite.",
                input.path
            ));
        }

        // Validate file content if it's a supported language file
        let syntax_warning = syn::validate(&input.path, &input.content);

        // Create parent directories if they don't exist
        if let Some(parent) = Path::new(&input.path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create directories: {}", e))?;
        }

        // Write file only after validation passes and directories are created
        tokio::fs::write(&input.path, &input.content)
            .await
            .map_err(|e| e.to_string())?;

        let mut result = format!(
            "Successfully wrote {} bytes to {}",
            input.content.len(),
            input.path
        );
        if let Some(warning) = syntax_warning {
            result.push_str("\nWarning: ");
            result.push_str(&warning.to_string());
        }

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use pretty_assertions::assert_eq;
    use tokio::fs;

    use super::*;
    use crate::utils::TempDir;

    async fn assert_path_exists(path: impl AsRef<Path>) {
        assert!(fs::metadata(path).await.is_ok(), "Path should exist");
    }

    #[tokio::test]
    async fn test_fs_write_success() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let content = "Hello, World!";

        let fs_write = FSWrite;
        let output = fs_write
            .call(FSWriteInput {
                path: file_path.to_string_lossy().to_string(),
                content: content.to_string(),
                overwrite: Some(false),
            })
            .await
            .unwrap();

        assert!(output.contains("Successfully wrote"));
        assert!(output.contains(&file_path.display().to_string()));
        assert!(output.contains(&content.len().to_string()));

        // Verify file was actually written
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Hello, World!")
    }

    #[tokio::test]
    async fn test_fs_write_invalid_rust() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        let fs_write = FSWrite;
        let result = fs_write
            .call(FSWriteInput {
                path: file_path.to_string_lossy().to_string(),
                content: "fn main() { let x = ".to_string(),
                overwrite: Some(false),
            })
            .await;

        let output = result.unwrap();
        assert!(output.contains("Warning:"));
    }

    #[tokio::test]
    async fn test_fs_write_valid_rust() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        let fs_write = FSWrite;
        let content = "fn main() { let x = 42; }";
        let result = fs_write
            .call(FSWriteInput {
                path: file_path.to_string_lossy().to_string(),
                content: content.to_string(),
                overwrite: Some(false),
            })
            .await;

        let output = result.unwrap();
        assert!(output.contains("Successfully wrote"));
        assert!(output.contains(&file_path.display().to_string()));
        assert!(output.contains(&content.len().to_string()));
        assert!(!output.contains("Warning:"));
        // Verify file contains valid Rust code
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "fn main() { let x = 42; }");
    }

    #[tokio::test]
    async fn test_fs_write_file_exists() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create the file first
        fs::write(&file_path, "Existing content").await.unwrap();

        let fs_write = FSWrite;
        let result = fs_write
            .call(FSWriteInput {
                path: file_path.to_string_lossy().to_string(),
                content: "New content".to_string(),
                overwrite: Some(false),
            })
            .await;

        // Check that the result is an error
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));

        // Verify original content remains unchanged
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Existing content");
    }

    #[tokio::test]
    async fn test_fs_write_single_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("new_dir").join("test.txt");
        let content = "Hello from nested file!";

        let fs_write = FSWrite;
        let result = fs_write
            .call(FSWriteInput {
                path: nested_path.to_string_lossy().to_string(),
                content: content.to_string(),
                overwrite: Some(false),
            })
            .await
            .unwrap();

        assert!(result.contains("Successfully wrote"));
        // Verify both directory and file were created
        assert_path_exists(&nested_path).await;
        assert_path_exists(nested_path.parent().unwrap()).await;

        // Verify content
        let written_content = fs::read_to_string(&nested_path).await.unwrap();
        assert_eq!(written_content, content);
    }

    #[tokio::test]
    async fn test_fs_write_deep_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let deep_path = temp_dir
            .path()
            .join("level1")
            .join("level2")
            .join("level3")
            .join("deep.txt");
        let content = "Deep in the directory structure";

        let fs_write = FSWrite;
        let result = fs_write
            .call(FSWriteInput {
                path: deep_path.to_string_lossy().to_string(),
                content: content.to_string(),
                overwrite: Some(false),
            })
            .await
            .unwrap();

        assert!(result.contains("Successfully wrote"));

        // Verify entire path was created
        assert_path_exists(&deep_path).await;
        let mut current = deep_path.parent().unwrap();
        while current != temp_dir.path() {
            assert_path_exists(current).await;
            current = current.parent().unwrap();
        }

        // Verify content
        let written_content = fs::read_to_string(&deep_path).await.unwrap();
        assert_eq!(written_content, content);
    }

    #[tokio::test]
    async fn test_fs_write_with_different_separators() {
        let temp_dir = TempDir::new().unwrap();

        // Use forward slashes regardless of platform
        let path_str = format!("{}/dir_a/dir_b/file.txt", temp_dir.path().to_string_lossy());
        let content = "Testing path separators";

        let fs_write = FSWrite;
        let result = fs_write
            .call(FSWriteInput {
                path: path_str,
                content: content.to_string(),
                overwrite: Some(false),
            })
            .await
            .unwrap();

        assert!(result.contains("Successfully wrote"));

        // Convert to platform path and verify
        let platform_path = Path::new(&temp_dir.path())
            .join("dir_a")
            .join("dir_b")
            .join("file.txt");

        assert_path_exists(&platform_path).await;

        // Verify content
        let written_content = fs::read_to_string(&platform_path).await.unwrap();
        assert_eq!(written_content, content);
    }

    #[tokio::test]
    async fn test_fs_write_with_overwrite() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create initial file
        let initial_content = "Initial content";
        fs::write(&file_path, initial_content).await.unwrap();

        // Try to overwrite with overwrite flag
        let new_content = "New content";
        let fs_write = FSWrite;
        let result = fs_write
            .call(FSWriteInput {
                path: file_path.to_string_lossy().to_string(),
                content: new_content.to_string(),
                overwrite: Some(true),
            })
            .await;

        // Verify overwrite was successful
        assert!(result.is_ok());
        let written_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(written_content, new_content);
    }

    #[tokio::test]
    async fn test_fs_write_without_overwrite() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create initial file
        let initial_content = "Initial content";
        fs::write(&file_path, initial_content).await.unwrap();

        // Try to write without overwrite flag
        let fs_write = FSWrite;
        let result = fs_write
            .call(FSWriteInput {
                path: file_path.to_string_lossy().to_string(),
                content: "New content".to_string(),
                overwrite: Some(false),
            })
            .await;

        // Verify write was prevented
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Set overwrite=true to overwrite"));

        // Verify original content remains unchanged
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, initial_content);
    }

    #[tokio::test]
    async fn test_fs_write_relative_path() {
        let fs_write = FSWrite;
        let result = fs_write
            .call(FSWriteInput {
                path: "relative/path/file.txt".to_string(),
                content: "test content".to_string(),
                overwrite: None,
            })
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Path must be absolute"));
    }

    #[tokio::test]
    async fn test_fs_write_overwrite_not_provided() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create initial file
        let initial_content = "Initial content";
        fs::write(&file_path, initial_content).await.unwrap();

        // Try to write without providing overwrite parameter
        let fs_write = FSWrite;
        let result = fs_write
            .call(FSWriteInput {
                path: file_path.to_string_lossy().to_string(),
                content: "New content".to_string(),
                overwrite: None,
            })
            .await;

        // Verify write was prevented
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Set overwrite=true to overwrite"));

        // Verify original content remains unchanged
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, initial_content);
    }
}
