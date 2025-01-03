use forge_tool_macros::Description as DescriptionDerive;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{Description, ToolCallService};

#[derive(Deserialize, JsonSchema, Serialize, Debug, PartialEq, Clone)]
pub struct FSReadInput {
    /// The path of the file to read (relative to the current working directory)
    #[serde(rename = "@path")]
    pub path: String,
}

/// Request to read the contents of a file at the specified path. Use this when
/// you need to examine the contents of an existing file you do not know the
/// contents of, for example to analyze code, review text files, or extract
/// information from configuration files. Automatically extracts raw text from
/// PDF and DOCX files. May not be suitable for other types of binary files, as
/// it returns the raw content as a string.
#[derive(DescriptionDerive)]
pub struct FSRead;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename = "fs_read")]
pub struct FSReadOutput {
    #[serde(rename = "$value")]
    pub content: String,
}

#[async_trait::async_trait]
impl ToolCallService for FSRead {
    type Input = FSReadInput;
    type Output = String;

    async fn call(&self, input: Self::Input) -> Result<Self::Output, String> {
        let content = tokio::fs::read_to_string(&input.path)
            .await
            .map_err(|e| e.to_string())?;
        Ok(content)
    }
}

#[cfg(test)]
mod test {
    use tempfile::TempDir;
    use tokio::fs;

    use super::*;

    #[tokio::test]
    async fn test_fs_read_success() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let test_content = "Hello, World!";
        fs::write(&file_path, test_content).await.unwrap();

        let fs_read = FSRead;
        let result = fs_read
            .call(FSReadInput { path: file_path.to_string_lossy().to_string() })
            .await
            .unwrap();

        assert_eq!(result, test_content);
    }

    #[tokio::test]
    async fn test_fs_read_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_file = temp_dir.path().join("nonexistent.txt");

        let fs_read = FSRead;
        let result = fs_read
            .call(FSReadInput { path: nonexistent_file.to_string_lossy().to_string() })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fs_read_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.txt");
        fs::write(&file_path, "").await.unwrap();

        let fs_read = FSRead;
        let result = fs_read
            .call(FSReadInput { path: file_path.to_string_lossy().to_string() })
            .await
            .unwrap();

        assert_eq!(result, "");
    }

    #[test]
    fn test_description() {
        assert!(FSRead::description().len() > 100)
    }

    #[test]
    fn serialize_to_xml() {
        let output = FSReadOutput { content: "Hello, World!".to_string() };
        let mut buffer = Vec::new();
        let mut writer = quick_xml::Writer::new_with_indent(&mut buffer, b' ', 4);
        writer.write_serializable("fs_read", &output).unwrap();

        let xml_str = std::str::from_utf8(&buffer).unwrap();
        insta::assert_snapshot!(xml_str);
    }
}
