use forge_domain::ToolCallService;
use insta::assert_snapshot;
use tempfile::TempDir;
use tokio::fs;

use super::super::{Outline, OutlineInput};
use crate::test_utils::setup_test_env;

#[tokio::test]
async fn scala_outline() {
    let temp_dir = TempDir::new().unwrap();
    let environment = setup_test_env(&temp_dir).await;

    let content = r#"
package com.example

sealed trait UserRole
case object Admin extends UserRole
case object Regular extends UserRole

case class User(name: String, age: Int, role: UserRole)

object UserService {
    def createUser(name: String, age: Int): User = {
        User(name, age, Regular)
    }

    def processUser[T](user: User)(f: User => T): T = {
        f(user)
    }
}

trait UserRepository {
    def findById(id: String): Option[User]
    def save(user: User): Unit
}

class UserServiceImpl extends UserRepository {
    private var users = Map.empty[String, User]

    override def findById(id: String): Option[User] = users.get(id)

    override def save(user: User): Unit = {
        users = users + (user.name -> user)
    }
}"#;
    let file_path = temp_dir.path().join("test.scala");
    fs::write(&file_path, content).await.unwrap();

    let outline = Outline::new(environment);
    let result = outline
        .call(OutlineInput { path: temp_dir.path().to_string_lossy().to_string() })
        .await
        .unwrap();

    assert_snapshot!("outline_scala", result);
}
