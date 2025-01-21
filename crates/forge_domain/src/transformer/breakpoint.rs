use std::collections::HashSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{Context, ContextMessage};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq)]
pub enum BreakPoint {
    /// Keep only messages that match any condition
    Any,

    /// Keep messages whose role matches the given string.
    Role(MessageRole),

    /// Keep only messages at or after the given index
    MinIndex(usize),

    /// Keep only the Nth (0-based) message that matches the given breakpoint.
    /// For example, Nth(1, Role(User)) matches the second user message.
    Nth(usize, Box<BreakPoint>),

    /// Keep only the first message that matches the given breakpoint.
    First(Box<BreakPoint>),

    /// Keep only the last message that matches the given breakpoint.
    Last(Box<BreakPoint>),

    /// Logical AND of two breakpoints.
    And(Box<BreakPoint>, Box<BreakPoint>),

    /// Logical OR of two breakpoints.
    Or(Box<BreakPoint>, Box<BreakPoint>),

    /// Keep messages *after* the first match of the first breakpoint,
    /// as filtered by the second. (One possible interpretation)
    After(Box<BreakPoint>, Box<BreakPoint>),

    /// Keep messages *before* the first match of the first breakpoint,
    /// as filtered by the second. (One possible interpretation)
    Before(Box<BreakPoint>, Box<BreakPoint>),

    /// Negate a breakpoint. (i.e. keep all messages *not* matched by the inner
    /// breakpoint)
    Not(Box<BreakPoint>),

    /// Keep messages whose tool name matches the given string.
    ToolName(String),

    /// Keep messages whose tool result is a success.
    ToolCallSuccess(Box<BreakPoint>),

    /// Keep messages whose tool result is a failure.
    ToolCallFailure(Box<BreakPoint>),
    //TODO: Add a breakpoint for number of Tokens
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq)]
pub enum MessageRole {
    Role(crate::Role),
    Tool,
}

impl From<&ContextMessage> for MessageRole {
    fn from(msg: &ContextMessage) -> Self {
        match msg {
            ContextMessage::ContentMessage(msg) => MessageRole::Role(msg.role.clone()),
            ContextMessage::ToolMessage(_) => MessageRole::Tool,
        }
    }
}

impl BreakPoint {
    pub fn get_breakpoints(&self, ctx: &Context) -> HashSet<usize> {
        let mut breakpoints = HashSet::new();
        match self {
            BreakPoint::Any => {
                breakpoints.extend(0..ctx.messages.len());
            }
            BreakPoint::Role(role) => {
                for (i, msg) in ctx.messages.iter().enumerate() {
                    if &MessageRole::from(msg) == role {
                        breakpoints.insert(i);
                    }
                }
            }
            BreakPoint::MinIndex(n) => {
                for i in *n..ctx.messages.len() {
                    breakpoints.insert(i);
                }
            }
            BreakPoint::First(bp) => {
                let matches = bp.get_breakpoints(ctx);
                if let Some(first) = matches.into_iter().min() {
                    breakpoints.insert(first);
                }
            }
            BreakPoint::Last(bp) => {
                let matches = bp.get_breakpoints(ctx);
                if let Some(last) = matches.into_iter().max() {
                    breakpoints.insert(last);
                }
            }
            BreakPoint::Nth(n, bp) => {
                let mut matches: Vec<_> = bp.get_breakpoints(ctx).into_iter().collect();
                matches.sort(); // Sort to ensure consistent order
                if let Some(nth) = matches.get(*n) {
                    breakpoints.insert(*nth);
                }
            }
            BreakPoint::And(bp1, bp2) => {
                let mut bp1_set = bp1.get_breakpoints(ctx);
                let bp2_set = bp2.get_breakpoints(ctx);
                bp1_set.retain(|i| bp2_set.contains(i));
                breakpoints.extend(bp1_set);
            }
            BreakPoint::Or(bp1, bp2) => {
                let bp1_set = bp1.get_breakpoints(ctx);
                let bp2_set = bp2.get_breakpoints(ctx);
                breakpoints.extend(bp1_set);
                breakpoints.extend(bp2_set);
            }
            BreakPoint::After(bp1, bp2) => {
                let bp1_set = bp1.get_breakpoints(ctx);
                let bp2_set = bp2.get_breakpoints(ctx);
                let mut after = false;
                for i in 0..ctx.messages.len() {
                    if bp1_set.contains(&i) {
                        after = true;
                    }
                    if after && bp2_set.contains(&i) {
                        breakpoints.insert(i);
                    }
                }
            }
            BreakPoint::Before(bp1, bp2) => {
                let bp1_set = bp1.get_breakpoints(ctx);
                let bp2_set = bp2.get_breakpoints(ctx);
                let mut before = true;
                for i in 0..ctx.messages.len() {
                    if bp1_set.contains(&i) {
                        before = false;
                    }
                    if before && bp2_set.contains(&i) {
                        breakpoints.insert(i);
                    }
                }
            }
            BreakPoint::Not(bp) => {
                let bp_set = bp.get_breakpoints(ctx);
                for i in 0..ctx.messages.len() {
                    if !bp_set.contains(&i) {
                        breakpoints.insert(i);
                    }
                }
            }
            BreakPoint::ToolName(name) => {
                for (i, msg) in ctx.messages.iter().enumerate() {
                    if let ContextMessage::ToolMessage(tool) = msg {
                        if tool.name.as_str() == name {
                            breakpoints.insert(i);
                        }
                    }
                }
            }
            BreakPoint::ToolCallSuccess(bp) => {
                let bp_indices = bp.get_breakpoints(ctx);
                for i in bp_indices {
                    if let Some(ContextMessage::ToolMessage(tool)) = ctx.messages.get(i) {
                        if !tool.is_error {
                            breakpoints.insert(i);
                        }
                    }
                }
            }
            BreakPoint::ToolCallFailure(bp) => {
                let bp_indices = bp.get_breakpoints(ctx);
                for i in bp_indices {
                    if let Some(ContextMessage::ToolMessage(tool)) = ctx.messages.get(i) {
                        if tool.is_error {
                            breakpoints.insert(i);
                        }
                    }
                }
            }
        }
        breakpoints
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Role, ToolName, ToolResult};

    /// Helper function to create a test context with mixed message types
    fn create_test_context() -> Context {
        Context::default()
            .add_message(ContextMessage::user("user message")) // 0
            .add_message(ContextMessage::assistant("assistant message", None)) // 1
            .add_message(ContextMessage::ToolMessage(
                // 2
                ToolResult::new(ToolName::new("test_tool")).success("test output"),
            ))
            .add_message(ContextMessage::user("another user message")) // 3
            .add_message(ContextMessage::ToolMessage(
                // 4
                ToolResult::new(ToolName::new("test_tool")).success("another test output"),
            ))
            .add_message(ContextMessage::ToolMessage(
                // 5
                ToolResult::new(ToolName::new("failed_tool")).failure("error output"),
            ))
            .add_message(ContextMessage::ToolMessage(
                // 6
                ToolResult::new(ToolName::new("other_tool")).success("success output"),
            ))
            .add_message(ContextMessage::assistant("final message", None)) // 7
    }

    #[test]
    fn test_and_breakpoint() {
        let ctx = create_test_context();

        // Test AND between Role(User) and Not(Tool)
        let bp = BreakPoint::And(
            Box::new(BreakPoint::Role(MessageRole::Role(Role::User))),
            Box::new(BreakPoint::Not(Box::new(BreakPoint::Role(
                MessageRole::Tool,
            )))),
        );

        let result = bp.get_breakpoints(&ctx);
        assert_eq!(result, [0, 3].into_iter().collect::<HashSet<_>>());
    }

    #[test]
    fn test_or_breakpoint() {
        let ctx = create_test_context();

        // Test OR between First and Last messages
        let bp = BreakPoint::Or(
            Box::new(BreakPoint::First(Box::new(BreakPoint::Any))),
            Box::new(BreakPoint::Last(Box::new(BreakPoint::Any))),
        );

        let result = bp.get_breakpoints(&ctx);
        assert_eq!(result, [0, 7].into_iter().collect::<HashSet<_>>());
    }

    #[test]
    fn test_not_breakpoint() {
        let ctx = create_test_context();

        // Test NOT of Tool messages
        let bp = BreakPoint::Not(Box::new(BreakPoint::Role(MessageRole::Tool)));

        let result = bp.get_breakpoints(&ctx);
        // Should not include messages 2, 4, 5, and 6 (tool messages)
        assert_eq!(result, [0, 1, 3, 7].into_iter().collect::<HashSet<_>>());
    }

    #[test]
    fn test_after_breakpoint() {
        let ctx = create_test_context();

        // Test messages after first Tool message that are Assistant
        let bp = BreakPoint::After(
            Box::new(BreakPoint::Role(MessageRole::Tool)),
            Box::new(BreakPoint::Role(MessageRole::Role(Role::Assistant))),
        );

        let result = bp.get_breakpoints(&ctx);
        assert_eq!(result, [7].into_iter().collect::<HashSet<_>>());
    }

    #[test]
    fn test_before_breakpoint() {
        let ctx = create_test_context();

        // Test messages before first Tool message that are User
        let bp = BreakPoint::Before(
            Box::new(BreakPoint::Role(MessageRole::Tool)),
            Box::new(BreakPoint::Role(MessageRole::Role(Role::User))),
        );

        let result = bp.get_breakpoints(&ctx);
        assert_eq!(result, [0].into_iter().collect::<HashSet<_>>());
    }

    #[test]
    fn test_nth_breakpoint() {
        let ctx = create_test_context();

        let bp = BreakPoint::Nth(2, Box::new(BreakPoint::Any));
        let result = bp.get_breakpoints(&ctx);
        assert_eq!(result, [2].into_iter().collect::<HashSet<_>>());
    }

    #[test]
    fn test_role_breakpoint() {
        let ctx = create_test_context();

        // Test finding all Assistant messages
        let bp = BreakPoint::Role(MessageRole::Role(Role::Assistant));
        let result = bp.get_breakpoints(&ctx);
        // Should be messages 1 and 7
        assert_eq!(result, [1, 7].into_iter().collect::<HashSet<_>>());
    }

    #[test]
    fn test_first_last_breakpoint() {
        let ctx = create_test_context();

        let first = BreakPoint::First(Box::new(BreakPoint::Any));
        let last = BreakPoint::Last(Box::new(BreakPoint::Any));

        assert_eq!(
            first.get_breakpoints(&ctx),
            [0].into_iter().collect::<HashSet<_>>()
        );
        assert_eq!(
            last.get_breakpoints(&ctx),
            [7].into_iter().collect::<HashSet<_>>()
        );
    }

    #[test]
    fn test_complex_breakpoint_combination() {
        let ctx = create_test_context();

        // Test (User OR Tool) AND Not(Last)
        let bp = BreakPoint::And(
            Box::new(BreakPoint::Or(
                Box::new(BreakPoint::Role(MessageRole::Role(Role::User))),
                Box::new(BreakPoint::Role(MessageRole::Tool)),
            )),
            Box::new(BreakPoint::Not(Box::new(BreakPoint::Last(Box::new(
                BreakPoint::Any,
            ))))),
        );

        let result = bp.get_breakpoints(&ctx);
        assert_eq!(
            result,
            [0, 2, 3, 4, 5, 6].into_iter().collect::<HashSet<_>>()
        );
    }

    #[test]
    fn test_tool_name_breakpoint() {
        let ctx = create_test_context();

        let bp = BreakPoint::ToolName("test_tool".to_string());
        let result = bp.get_breakpoints(&ctx);
        assert_eq!(result, [2, 4].into_iter().collect::<HashSet<_>>());
    }

    #[test]
    fn test_tool_call_success_breakpoint() {
        let ctx = create_test_context();

        let bp =
            BreakPoint::ToolCallSuccess(Box::new(BreakPoint::ToolName("test_tool".to_string())));
        let result = bp.get_breakpoints(&ctx);
        assert_eq!(result, [2, 4].into_iter().collect::<HashSet<_>>());
    }

    #[test]
    fn test_tool_call_failure_breakpoint() {
        let ctx = create_test_context();

        let bp =
            BreakPoint::ToolCallFailure(Box::new(BreakPoint::ToolName("failed_tool".to_string())));
        let result = bp.get_breakpoints(&ctx);
        assert_eq!(result, [5].into_iter().collect::<HashSet<_>>());
    }

    #[test]
    fn test_tool_call_success_and_tool_name() {
        let ctx = create_test_context();

        let bp = BreakPoint::And(
            Box::new(BreakPoint::ToolCallSuccess(Box::new(BreakPoint::ToolName(
                "test_tool".to_string(),
            )))),
            Box::new(BreakPoint::ToolName("test_tool".to_string())),
        );
        let result = bp.get_breakpoints(&ctx);
        assert_eq!(result, [2, 4].into_iter().collect::<HashSet<_>>());
    }

    #[test]
    fn test_tool_call_failure_or_tool_name() {
        let ctx = create_test_context();

        let bp = BreakPoint::Or(
            Box::new(BreakPoint::ToolCallFailure(Box::new(BreakPoint::ToolName(
                "failed_tool".to_string(),
            )))),
            Box::new(BreakPoint::ToolName("other_tool".to_string())),
        );
        let result = bp.get_breakpoints(&ctx);
        assert_eq!(result, [5, 6].into_iter().collect::<HashSet<_>>());
    }

    #[test]
    fn test_not_tool_call_success() {
        let ctx = create_test_context();

        let bp = BreakPoint::Not(Box::new(BreakPoint::ToolCallSuccess(Box::new(
            BreakPoint::ToolName("test_tool".to_string()),
        ))));
        let result = bp.get_breakpoints(&ctx);
        assert_eq!(
            result,
            [0, 1, 3, 5, 6, 7].into_iter().collect::<HashSet<_>>()
        );
    }

    #[test]
    fn test_not_tool_failure_and_tool_name() {
        let ctx = create_test_context();

        let bp = BreakPoint::And(
            Box::new(BreakPoint::Not(Box::new(BreakPoint::ToolCallFailure(
                Box::new(BreakPoint::ToolName("failed_tool".to_string())),
            )))),
            Box::new(BreakPoint::ToolName("test_tool".to_string())),
        );
        let result = bp.get_breakpoints(&ctx);
        assert_eq!(result, [2, 4].into_iter().collect::<HashSet<_>>());
    }

    #[test]
    fn test_after_tool_call_success() {
        let ctx = create_test_context();

        let bp = BreakPoint::After(
            Box::new(BreakPoint::ToolCallSuccess(Box::new(BreakPoint::ToolName(
                "test_tool".to_string(),
            )))),
            Box::new(BreakPoint::ToolName("failed_tool".to_string())),
        );
        let result = bp.get_breakpoints(&ctx);
        assert_eq!(result, [5].into_iter().collect::<HashSet<_>>());
    }

    #[test]
    fn test_before_tool_call_failure() {
        let ctx = create_test_context();

        let bp = BreakPoint::Before(
            Box::new(BreakPoint::ToolCallFailure(Box::new(BreakPoint::ToolName(
                "failed_tool".to_string(),
            )))),
            Box::new(BreakPoint::ToolName("test_tool".to_string())),
        );
        let result = bp.get_breakpoints(&ctx);
        assert_eq!(result, [2, 4].into_iter().collect::<HashSet<_>>());
    }

    #[test]
    fn test_tool_name_after_tool_call_success() {
        let ctx = create_test_context();

        let bp = BreakPoint::And(
            Box::new(BreakPoint::After(
                Box::new(BreakPoint::ToolCallSuccess(Box::new(BreakPoint::ToolName(
                    "test_tool".to_string(),
                )))),
                Box::new(BreakPoint::ToolName("other_tool".to_string())),
            )),
            Box::new(BreakPoint::Not(Box::new(BreakPoint::Last(Box::new(
                BreakPoint::Any,
            ))))),
        );
        let result = bp.get_breakpoints(&ctx);
        assert_eq!(result, [6].into_iter().collect::<HashSet<_>>());
    }

    #[test]
    fn test_first_tool_failure() {
        let ctx = create_test_context();

        let bp = BreakPoint::First(Box::new(BreakPoint::ToolCallFailure(Box::new(
            BreakPoint::Role(MessageRole::Tool),
        ))));
        let result = bp.get_breakpoints(&ctx);
        // Should return index 5 - first failed tool call
        assert_eq!(result, [5].into_iter().collect::<HashSet<_>>());
    }

    #[test]
    fn test_last_tool_success() {
        let ctx = create_test_context();

        let bp = BreakPoint::Last(Box::new(BreakPoint::ToolCallSuccess(Box::new(
            BreakPoint::Role(MessageRole::Tool),
        ))));
        let result = bp.get_breakpoints(&ctx);
        // Should return index 6 - last successful tool call
        assert_eq!(result, [6].into_iter().collect::<HashSet<_>>());
    }

    #[test]
    fn test_nth_user_message() {
        let ctx = create_test_context();

        let bp = BreakPoint::Nth(1, Box::new(BreakPoint::Role(MessageRole::Role(Role::User))));
        let result = bp.get_breakpoints(&ctx);
        assert_eq!(result, [3].into_iter().collect::<HashSet<_>>()); // Second user message
    }

    #[test]
    fn test_first_last_nth_basic() {
        let ctx = create_test_context();

        // Test direct usage of First, Last, Nth variants
        assert_eq!(
            BreakPoint::First(Box::new(BreakPoint::Any)).get_breakpoints(&ctx),
            [0].into_iter().collect::<HashSet<_>>()
        );
        assert_eq!(
            BreakPoint::Last(Box::new(BreakPoint::Any)).get_breakpoints(&ctx),
            [7].into_iter().collect::<HashSet<_>>()
        );
        assert_eq!(
            BreakPoint::Nth(2, Box::new(BreakPoint::Any)).get_breakpoints(&ctx),
            [2].into_iter().collect::<HashSet<_>>()
        );
    }

    #[test]
    fn test_first_last_nth_matching() {
        let ctx = create_test_context();

        // Test direct usage of First, Last, Nth variants with conditions
        let tool_condition = BreakPoint::Role(MessageRole::Tool);

        // First tool message
        assert_eq!(
            BreakPoint::First(Box::new(tool_condition.clone())).get_breakpoints(&ctx),
            [2].into_iter().collect::<HashSet<_>>() // First tool call at index 2
        );

        // Last tool message
        assert_eq!(
            BreakPoint::Last(Box::new(tool_condition.clone())).get_breakpoints(&ctx),
            [6].into_iter().collect::<HashSet<_>>() // Last tool call at index 6
        );

        // Third tool message (index 2: test_tool success, 4: test_tool success, 5:
        // failed_tool failure)
        assert_eq!(
            BreakPoint::Nth(2, Box::new(tool_condition)).get_breakpoints(&ctx),
            [5].into_iter().collect::<HashSet<_>>()
        );
    }

    #[test]
    fn test_complex_tool_breakpoint_combination() {
        let ctx = create_test_context();

        // Test ((ToolCallSuccess AND ToolName) OR (Not ToolCallFailure)) AND After
        // First
        let bp = BreakPoint::And(
            Box::new(BreakPoint::Or(
                Box::new(BreakPoint::And(
                    Box::new(BreakPoint::ToolCallSuccess(Box::new(BreakPoint::ToolName(
                        "test_tool".to_string(),
                    )))),
                    Box::new(BreakPoint::ToolName("test_tool".to_string())),
                )),
                Box::new(BreakPoint::Not(Box::new(BreakPoint::ToolCallFailure(
                    Box::new(BreakPoint::ToolName("failed_tool".to_string())),
                )))),
            )),
            Box::new(BreakPoint::After(
                Box::new(BreakPoint::First(Box::new(BreakPoint::Any))),
                Box::new(BreakPoint::Role(MessageRole::Tool)),
            )),
        );

        let result = bp.get_breakpoints(&ctx);
        assert_eq!(result, [2, 4, 6].into_iter().collect::<HashSet<_>>());
    }
}
