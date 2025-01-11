use forge_domain::ToolCallService;
use insta::assert_snapshot;
use tempfile::TempDir;
use tokio::fs;
use crate::test_utils::setup_test_env;

use super::super::{Outline, OutlineInput};

#[tokio::test]
async fn rust_outline() {
    let temp_dir = TempDir::new().unwrap();
    let environment = setup_test_env(&temp_dir).await;

    let content = r#"
struct User {
    name: String,
    age: u32,
}

fn calculate_age(birth_year: u32) -> u32 {
    2024 - birth_year
}

impl User {
    fn new(name: String, age: u32) -> Self {
        User { name, age }
    }
}
"#;
    let file_path = temp_dir.path().join("test.rs");
    fs::write(&file_path, content).await.unwrap();

    let outline = Outline::new(environment);
    let result = outline
        .call(OutlineInput { path: temp_dir.path().to_string_lossy().to_string() })
        .await
        .unwrap();

    assert_snapshot!("outline_rust", result);
}