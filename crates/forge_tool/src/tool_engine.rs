use std::collections::HashMap;

use forge_env::{Environment, EnvironmentValue};
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

struct ToolDefinition {
    executable: Box<dyn ToolTrait<Input = Value, Output = Value> + Send + Sync + 'static>,
    tool: Tool,
}

pub struct ToolEngine {
    tools: HashMap<ToolName, ToolDefinition>,
}

///
/// Refer to the specification over here:
/// https://glama.ai/blog/2024-11-25-model-context-protocol-quickstart#server
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tool {
    pub name: ToolName,
    pub description: String,
    pub input_schema: RootSchema,
    pub output_schema: Option<RootSchema>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolName(String);

impl<A: ToString> From<A> for ToolName {
    fn from(value: A) -> Self {
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
        println!("{}({})", name.as_str(), input);
        let output = match self.tools.get(name) {
            Some(tool) => {
                let output = tool.executable.call(input).await;
                println!(
                    "{}(...) -> {:?}",
                    name.as_str(),
                    serde_json::to_string(&output.clone().ok().unwrap_or_default()).ok()
                );

                output
            }
            None => Err(format!("No such tool found: {}", name.as_str())),
        };

        output
    }

    pub fn list(&self) -> Vec<Tool> {
        self.tools.values().map(|tool| tool.tool.clone()).collect()
    }
}

struct ToolImporter<'a, C> {
    ctx: &'a C,
}

impl<'a, C: Serialize> ToolImporter<'a, C> {
    fn new(ctx: &'a C) -> Self {
        Self { ctx }
    }

    fn import<T>(&self, tool: T) -> (ToolName, ToolDefinition)
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
            &Environment::render(&serde_json::to_string(&input).unwrap(), self.ctx).unwrap(),
        )
        .unwrap();

        let output: RootSchema = schema_for!(T::Output);
        let output: RootSchema = serde_json::from_str(
            &Environment::render(&serde_json::to_string(&output).unwrap(), self.ctx).unwrap(),
        )
        .unwrap();

        let tool = Tool {
            name: ToolName(name.clone()),
            description: Environment::render(T::description(), self.ctx)
                .unwrap_or_else(|_| panic!("Unable to render description for tool {}", name)),
            input_schema: input,
            output_schema: Some(output),
        };

        (ToolName(name), ToolDefinition { executable, tool })
    }
}

impl Default for ToolEngine {
    fn default() -> Self {
        let ctx = EnvironmentValue::build();
        let importer = ToolImporter::new(&ctx);

        let tools: HashMap<ToolName, ToolDefinition> = HashMap::from([
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
    use serde_json::json;

    use super::*;
    use crate::think::Think;
    use crate::{FSFileInfo, FSSearch};

    fn env_ctx() -> Value {
        json!({
            "current_working_directory": "./test-dir",
            "operating_system": "test-os"
        })
    }

    impl ToolEngine {
        fn build<C: Serialize>(importer: ToolImporter<C>) -> Self {
            let tools: HashMap<ToolName, ToolDefinition> = HashMap::from([
                importer.import(FSRead),
                importer.import(FSWrite),
                importer.import(FSList),
                importer.import(FSSearch),
                importer.import(FSFileInfo),
                importer.import(Think::default()),
            ]);
            Self { tools }
        }
    }

    #[test]
    fn test_id() {
        let env_ctx = env_ctx();
        let importer = ToolImporter::new(&env_ctx);

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
        let env_ctx = env_ctx();
        let tool_engine = ToolEngine::build(ToolImporter::new(&env_ctx));

        for tool in tool_engine.list() {
            let tool_str = serde_json::to_string_pretty(&tool).unwrap();
            insta::assert_snapshot!(tool.name.as_str(), tool_str);
        }
    }
}
