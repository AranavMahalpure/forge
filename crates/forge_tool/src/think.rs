use std::collections::HashMap;

use anyhow::Result;
use colorize::AnsiColor;
use forge_domain::{
    NamedTool, Permission, PermissionRequest, ToolCallService, ToolDescription, ToolName, ToolPermissions
};
use forge_tool_macros::ToolDescription;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A framework for iterative reasoning in problem-solving.
///
/// Purpose:
/// Tracks reasoning steps ("thoughts") to solve complex problems:
/// - Breaks tasks into steps
/// - Supports revisions and branching
/// - Tracks confidence and verifies solutions
///
/// Key Features:
/// - Adjust `total_thoughts` for complexity
/// - Link revisions with `revises_thought`
/// - Branch paths via `branch_from_thought`
/// - Update `solution_confidence`
/// - Mark completion with `solution_reached`
///
/// Workflow:
/// 1. Initialize `Think` with `total_thoughts`.
/// 2. Add steps to `thought_history`.
/// 3. Revise or branch as needed.
/// 4. Update confidence and validate.
/// 5. Mark `solution_reached` when done.
///
/// Fields:
/// - `thought_history`: Steps taken.
/// - `branches`: Alternate paths.
/// - `solution_reached`: Final solution.
#[derive(Clone, Default, ToolDescription)]
pub struct Think {
    thought_history: Vec<ThoughtInput>,
    branches: HashMap<String, Vec<ThoughtInput>>,
    solution_reached: bool,
}

impl ToolPermissions for Think {
    fn required_permissions(&self) -> Vec<Permission> {
        vec![]
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct ThoughtInput {
    /// The description of the current thought or reasoning step.
    pub thought: String,
    /// Whether another thought is needed to reach a solution.
    pub next_thought_needed: bool,
    /// The number of the current thought or reasoning step.
    pub thought_number: i32,
    /// The total number of thoughts or reasoning steps expected to reach a
    /// solution.
    pub total_thoughts: i32,
    /// Whether this thought is a revision of a previous thought.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_revision: Option<bool>,
    /// The number of the thought being revised, if this is a revision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revises_thought: Option<i32>,
    /// The number of the thought from which this thought branches, if this is a
    /// branch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_from_thought: Option<i32>,
    /// A unique identifier for the branch, if this is a branch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    /// Whether additional thoughts are needed to reach a solution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_more_thoughts: Option<bool>,
    /// The current confidence in the solution, ranging from 0.0 to 1.0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solution_confidence: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct ThoughtResult {
    pub thought_number: i32,
    pub total_thoughts: i32,
    pub next_thought_needed: bool,
    pub solution_reached: bool,
    pub solution_confidence: f32,
    pub branches: Vec<String>,
    pub thought_history_length: usize,
}

impl Think {
    fn validate_thought_data(&self, mut input: ThoughtInput) -> Result<ThoughtInput> {
        if input.thought_number <= 0 {
            return Err(anyhow::anyhow!("Invalid thoughtNumber: must be positive"));
        }
        if input.total_thoughts <= 0 {
            return Err(anyhow::anyhow!("Invalid totalThoughts: must be positive"));
        }

        // If no confidence is provided, calculate it based on progress
        if input.solution_confidence.is_none() {
            input.solution_confidence =
                Some(input.thought_number as f32 / input.total_thoughts as f32);
        }

        Ok(input)
    }

    fn format_thought(&self, thought_data: &ThoughtInput) -> String {
        let (prefix, context) = match (thought_data.is_revision, &thought_data.branch_from_thought)
        {
            (Some(true), _) => (
                "🔄 Revision".yellow().to_string(),
                format!(
                    " (revising thought {})",
                    thought_data.revises_thought.unwrap_or(0)
                ),
            ),
            (_, Some(branch)) => (
                "🌿 Branch".green().to_string(),
                format!(
                    " (from thought {}, ID: {})",
                    branch,
                    thought_data.branch_id.as_ref().unwrap_or(&String::new())
                ),
            ),
            _ => ("💭 Thought".blue().to_string(), String::new()),
        };

        let header = format!(
            "{} {}/{}{} (Confidence: {:.2}%)",
            prefix,
            thought_data.thought_number,
            thought_data.total_thoughts,
            context,
            thought_data.solution_confidence.unwrap_or(0.0) * 100.0
        );
        let border_len = header.len().max(thought_data.thought.len()) + 4;
        let border = "─".repeat(border_len);

        let thought_data = format!("{:width$}", thought_data.thought, width = border_len - 2);

        format!(
            "\n┌{}┐\n│ {} │\n├{}┤\n│ {} │\n└{}┘",
            border, header, border, thought_data, border
        )
    }

    fn process_thought(&mut self, input: ThoughtInput) -> Result<ThoughtResult> {
        let mut thought_data = self.validate_thought_data(input)?;

        // Adjust total thoughts if needed
        if thought_data.thought_number > thought_data.total_thoughts {
            thought_data.total_thoughts = thought_data.thought_number;
        }

        // Evaluate solution confidence
        if let Some(confidence) = thought_data.solution_confidence {
            if confidence >= 0.8 {
                self.solution_reached = true;
                thought_data.next_thought_needed = false;
            }
        }

        // Terminate thinking if max thoughts reached or solution found
        if thought_data.thought_number >= thought_data.total_thoughts || self.solution_reached {
            thought_data.next_thought_needed = false;
        }

        // Always allow at least one thought to be processed
        if self.thought_history.is_empty() {
            thought_data.next_thought_needed = true;
        }

        self.thought_history.push(thought_data.clone());

        // Branch handling remains the same
        if let (Some(_), Some(branch_id)) =
            (thought_data.branch_from_thought, &thought_data.branch_id)
        {
            self.branches
                .entry(branch_id.clone())
                .or_default()
                .push(thought_data.clone());
        }

        eprintln!("{}", self.format_thought(&thought_data));

        Ok(ThoughtResult {
            thought_number: thought_data.thought_number,
            total_thoughts: thought_data.total_thoughts,
            next_thought_needed: thought_data.next_thought_needed,
            solution_reached: self.solution_reached,
            solution_confidence: thought_data.solution_confidence.unwrap_or(0.0),
            branches: self.branches.keys().cloned().collect(),
            thought_history_length: self.thought_history.len(),
        })
    }
}

impl NamedTool for Think {
    fn tool_name(&self) -> ToolName {
        ToolName::new("think_step")
    }
}

#[async_trait::async_trait]
impl ToolCallService for Think {
    type Input = ThoughtInput;
    type Output = ThoughtResult;

    async fn call(&self, input: Self::Input) -> Result<Self::Output, String> {
        let mut thinker = self.clone();
        let thought_result = thinker.process_thought(input).map_err(|e| e.to_string())?;
        Ok(thought_result)
    }

    async fn permission_check(&self, _input: Self::Input) -> PermissionRequest {
        PermissionRequest::new(self.required_permissions(), None)
    }
}
