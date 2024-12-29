use std::collections::HashMap;

use forge_env::Environment;
use inflector::Inflector;
use schemars::schema::RootSchema;
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    Description, FSFileInfo, FSList, FSRead, FSReplace, FSSearch, FSWrite, Outline, Shell,
    ToolTrait,
};

struct JsonTool<T>(T);

#[async_trait::async_trait]
impl<T: ToolTrait + Sync> ToolTrait for JsonTool<T>
where
    T::Input: serde::de::DeserializeOwned + JsonSchema,
    T::Output: serde::Serialize + JsonSchema,
{
    type Input = Value;
    type Output = Value;

    async fn call(&self, input: Self::Input) -> Result<Self::Output, String> {
        let input: T::Input = serde_json::from_value(input).map_err(|e| e.to_string())?;
        let output: T::Output = self.0.call(input).await?;
        Ok(serde_json::to_value(output).map_err(|e| e.to_string())?)
    }
}

struct Executor {
    executable: Box<dyn ToolTrait<Input = Value, Output = Value> + Send + Sync + 'static>,
    tool: ToolDefinition,
}

pub struct ToolEngine {
    tools: HashMap<ToolName, Executor>,
}

///
/// Refer to the specification over here:
/// https://glama.ai/blog/2024-11-25-model-context-protocol-quickstart#server
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDefinition {
    pub name: ToolName,
    pub description: String,
    pub input_schema: RootSchema,
    pub output_schema: Option<RootSchema>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolName(String);

impl ToolName {
    pub fn new(value: impl ToString) -> Self {
        ToolName(value.to_string())
    }
}

impl ToolName {
    pub fn into_string(self) -> String {
        self.0
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ToolEngine {
    pub async fn call(&self, name: &ToolName, input: Value) -> Result<Value, String> {
        let output = match self.tools.get(name) {
            Some(tool) => tool.executable.call(input).await,
            None => Err(format!("No such tool found: {}", name.as_str())),
        };

        output
    }

    pub fn list(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|tool| tool.tool.clone()).collect()
    }
}

struct ToolImporter {
    env: Environment,
}

impl ToolImporter {
    fn new(env: Environment) -> Self {
        Self { env }
    }

    fn import<T>(&self, tool: T) -> (ToolName, Executor)
    where
        T: ToolTrait + Description + Send + Sync + 'static,
        T::Input: serde::de::DeserializeOwned + JsonSchema,
        T::Output: serde::Serialize + JsonSchema,
    {
        let name = std::any::type_name::<T>()
            .split("::")
            .last()
            .unwrap()
            .to_snake_case();
        let executable = Box::new(JsonTool(tool));

        let input: RootSchema = schema_for!(T::Input);
        let input: RootSchema = serde_json::from_str(
            &self
                .env
                .render(&serde_json::to_string(&input).unwrap())
                .unwrap(),
        )
        .unwrap();

        let output: RootSchema = schema_for!(T::Output);
        let output: RootSchema = serde_json::from_str(
            &self
                .env
                .render(&serde_json::to_string(&output).unwrap())
                .unwrap(),
        )
        .unwrap();

        let description = self.env.render(T::description()).unwrap_or_else(|err| {
            panic!(
                "Unable to render description for tool {}, err: {:?}",
                name, err
            )
        });

        assert!(
            description.len() < 1024,
            "Description for tool {} is longer than 1024",
            name
        );

        let tool = ToolDefinition {
            name: ToolName(name.clone()),
            description,
            input_schema: input,
            output_schema: Some(output),
        };

        (ToolName(name), Executor { executable, tool })
    }
}

impl ToolEngine {
    pub fn new(env: Environment) -> Self {
        let importer = ToolImporter::new(env);

        let tools: HashMap<ToolName, Executor> = HashMap::from([
            importer.import(FSRead),
            importer.import(FSWrite),
            importer.import(FSList),
            importer.import(FSSearch),
            importer.import(FSFileInfo),
            importer.import(FSReplace),
            importer.import(Outline),
            importer.import(Shell::default()),
            // TODO: uncomment them later on, as of now we only need the above tools.
            // importer.import(Think::default()),
            // importer::import(AskFollowUpQuestion),
        ]);

        Self { tools }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::think::Think;
    use crate::{FSFileInfo, FSSearch};

    fn test_importer() -> ToolImporter {
        ToolImporter::new(test_env())
    }

    fn test_env() -> Environment {
        Environment {
            cwd: "/Users/test".into(),
            os: "TestOS".into(),
            shell: "ZSH".into(),
            home: Some("/Users".into()),
            files: vec!["test.txt".into()],
        }
    }

    #[test]
    fn test_id() {
        let importer = test_importer();

        assert!(importer.import(FSRead).0.into_string().ends_with("fs_read"));
        assert!(importer
            .import(FSSearch)
            .0
            .into_string()
            .ends_with("fs_search"));
        assert!(importer.import(FSList).0.into_string().ends_with("fs_list"));
        assert!(importer
            .import(FSFileInfo)
            .0
            .into_string()
            .ends_with("file_info"));
        assert!(importer
            .import(Think::default())
            .0
            .into_string()
            .ends_with("think"));
    }

    #[test]
    fn test_description() {
        let tool_engine = ToolEngine::new(test_env());

        for tool in tool_engine.list() {
            let tool_str = serde_json::to_string_pretty(&tool).unwrap();
            insta::assert_snapshot!(tool.name.as_str(), tool_str);
        }
    }
}
