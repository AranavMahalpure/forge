use std::fmt::Display;

use derive_setters::Setters;
use serde::{Deserialize, Serialize};

use crate::{ToolCallFull, ToolCallId, ToolName};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Setters)]
#[setters(strip_option, into)]
pub struct ToolResult {
    pub name: ToolName,
    pub call_id: Option<ToolCallId>,
    #[setters(skip)]
    pub content: String,
    #[setters(skip)]
    pub is_error: bool,
}

#[derive(Default, Serialize, Setters)]
#[serde(rename_all = "snake_case", rename = "tool_result")]
#[setters(strip_option)]
struct ToolResultXML {
    tool_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    success: Option<String>,
}

impl ToolResult {
    pub fn new(name: ToolName) -> ToolResult {
        Self {
            name,
            call_id: None,
            content: String::default(),
            is_error: false,
        }
    }

    pub fn success(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self.is_error = false;
        self
    }

    pub fn failure(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self.is_error = true;
        self
    }
}

impl From<ToolCallFull> for ToolResult {
    fn from(value: ToolCallFull) -> Self {
        Self {
            name: value.name,
            call_id: value.call_id,
            content: String::default(),
            is_error: false,
        }
    }
}

impl Display for ToolResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let xml = {
            let xml = ToolResultXML::default().tool_name(self.name.as_str().to_owned());
            if self.is_error {
                xml.error(self.content.clone())
            } else {
                xml.success(self.content.clone())
            }
        };

        // First serialize to string
        let mut out = String::new();
        let ser = quick_xml::se::Serializer::new(&mut out);
        xml.serialize(ser).unwrap();

        write!(f, "{}", out)
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use serde_json::json;

    use super::*;

    #[test]
    fn test_snapshot_minimal() {
        let result = ToolResult::new(ToolName::new("test_tool"));
        assert_snapshot!(result);
    }

    #[test]
    fn test_snapshot_full() {
        let result = ToolResult::new(ToolName::new("complex_tool"))
            .call_id(ToolCallId::new("123"))
            .failure(json!({"key": "value", "number": 42}).to_string());
        assert_snapshot!(result);
    }

    #[test]
    fn test_snapshot_with_special_chars() {
        let result = ToolResult::new(ToolName::new("xml_tool")).success(
            json!({
                "text": "Special chars: < > & ' \"",
                "nested": {
                    "html": "<div>Test</div>"
                }
            })
            .to_string(),
        );
        assert_snapshot!(result);
    }

    #[test]
    fn test_display_minimal() {
        let result = ToolResult::new(ToolName::new("test_tool"));
        assert_snapshot!(result.to_string());
    }

    #[test]
    fn test_display_full() {
        let result = ToolResult::new(ToolName::new("complex_tool"))
            .call_id(ToolCallId::new("123"))
            .success(
                json!({
                    "user": "John Doe",
                    "age": 42,
                    "address": [{"city": "New York"}, {"city": "Los Angeles"}]
                })
                .to_string(),
            );
        assert_snapshot!(result.to_string());
    }

    #[test]
    fn test_display_special_chars() {
        let result = ToolResult::new(ToolName::new("xml_tool")).success(
            json!({
                "text": "Special chars: < > & ' \"",
                "nested": {
                    "html": "<div>Test</div>"
                }
            })
            .to_string(),
        );
        assert_snapshot!(result.to_string());
    }

    #[test]
    fn test_success_and_failure_content() {
        let success = ToolResult::new(ToolName::new("test_tool")).success("success message");
        assert!(!success.is_error);
        assert_eq!(success.content, "success message");

        let failure = ToolResult::new(ToolName::new("test_tool")).failure("error message");
        assert!(failure.is_error);
        assert_eq!(failure.content, "error message");
    }
}
