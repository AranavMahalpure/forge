use forge_domain::{
    NamedTool, PermissionRequest, ToolCallService, ToolDescription, ToolName, ToolPermissions,
};
use forge_tool_macros::ToolDescription;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
pub struct FSFileInfoInput {
    /// The path of the file or directory to inspect (relative to the current
    /// working directory)
    pub path: String,
}

/// Request to retrieve detailed metadata about a file or directory at the
/// specified path. Returns comprehensive information including size, creation
/// time, last modified time, permissions, and type. Use this when you need to
/// understand file characteristics without reading the actual content.
#[derive(ToolDescription)]
pub struct FSFileInfo;

impl ToolPermissions for FSFileInfo {
    fn required_permissions(&self) -> Vec<forge_domain::Permission> {
        vec![]
    }
}

impl NamedTool for FSFileInfo {
    fn tool_name(&self) -> ToolName {
        ToolName::new("file_information")
    }
}

#[async_trait::async_trait]
impl ToolCallService for FSFileInfo {
    type Input = FSFileInfoInput;
    type Output = String;

    async fn call(&self, input: Self::Input) -> Result<Self::Output, String> {
        let meta = tokio::fs::metadata(input.path)
            .await
            .map_err(|e| e.to_string())?;
        Ok(format!("{:?}", meta))
    }
    async fn permission_check(&self, _input: Self::Input) -> PermissionRequest {
        PermissionRequest::new(self.required_permissions(), None)
    }
}

#[cfg(test)]
mod test {
    use tempfile::TempDir;
    use tokio::fs;

    use super::*;

    #[tokio::test]
    async fn test_fs_file_info_on_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").await.unwrap();

        let fs_info = FSFileInfo;
        let result = fs_info
            .call(FSFileInfoInput { path: file_path.to_string_lossy().to_string() })
            .await
            .unwrap();

        assert!(result.contains("FileType"));
        assert!(result.contains("permissions"));
        assert!(result.contains("modified"));
    }

    #[tokio::test]
    async fn test_fs_file_info_on_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("test_dir");
        fs::create_dir(&dir_path).await.unwrap();

        let fs_info = FSFileInfo;
        let result = fs_info
            .call(FSFileInfoInput { path: dir_path.to_string_lossy().to_string() })
            .await
            .unwrap();

        assert!(result.contains("FileType"));
        assert!(result.contains("permissions"));
        assert!(result.contains("modified"));
    }

    #[tokio::test]
    async fn test_fs_file_info_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent");

        let fs_info = FSFileInfo;
        let result = fs_info
            .call(FSFileInfoInput { path: nonexistent_path.to_string_lossy().to_string() })
            .await;

        assert!(result.is_err());
    }
}
