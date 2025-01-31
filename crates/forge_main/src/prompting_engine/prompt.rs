use std::borrow::Cow;

use derive_setters::Setters;
use forge_domain::Usage;
use nu_ansi_term::{Color, Style};
use reedline::{Prompt, PromptHistorySearchStatus};

// Constants
pub const MAX_LEN: usize = 30;
const AI_INDICATOR: &str = "⚡";
const MULTILINE_INDICATOR: &str = "::: ";
const RIGHT_CHEVRON: &str = "❯";

/// Very Specialized Prompt for the Agent Chat
#[derive(Clone, Default, Setters)]
#[setters(strip_option, borrow_self)]
pub struct ForgePrompt {
    title: Option<String>,
    usage: Option<Usage>,
}

impl Prompt for ForgePrompt {
    fn render_prompt_left(&self) -> Cow<str> {
        if let Some(title) = self.title.as_ref() {
            let title = title.chars().take(MAX_LEN).collect::<String>();
            Cow::Owned(format!(
                "{AI_INDICATOR} {}",
                Style::new().fg(Color::Cyan).paint(title),
            ))
        } else {
            Cow::Borrowed(AI_INDICATOR)
        }
    }

    fn render_prompt_right(&self) -> Cow<str> {
        if let Some(usage) = self.usage.as_ref() {
            let usage_text = format!(
                "[{}/{}/{}]",
                usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
            );
            Cow::Owned(
                Style::new()
                    .bold()
                    .fg(Color::DarkGray)
                    .paint(usage_text)
                    .to_string(),
            )
        } else {
            Cow::Borrowed("")
        }
    }

    fn render_prompt_indicator(&self, _prompt_mode: reedline::PromptEditMode) -> Cow<str> {
        if self.title.is_some() {
            Cow::Owned(
                Style::new()
                    .fg(Color::LightYellow)
                    .paint(format!(" {RIGHT_CHEVRON} "))
                    .to_string(),
            )
        } else {
            Cow::Borrowed("")
        }
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<str> {
        Cow::Borrowed(MULTILINE_INDICATOR)
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: reedline::PromptHistorySearch,
    ) -> Cow<str> {
        let prefix = match history_search.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };
        let input = format!("({}reverse-search: {}) ", prefix, history_search.term);
        Cow::Owned(Style::new().fg(Color::White).paint(input).to_string())
    }
}

#[cfg(test)]
mod tests {
    use nu_ansi_term::Style;

    use super::*;

    #[test]
    fn test_render_prompt_left_with_title() {
        let mut prompt = ForgePrompt::default();
        prompt.title("test-title".to_string());
        let title_style = Style::new().fg(Color::Cyan).paint("test-title").to_string();
        assert_eq!(
            prompt.render_prompt_left(),
            format!("{AI_INDICATOR} {title_style}")
        );
    }

    #[test]
    fn test_render_prompt_left_without_title() {
        let prompt = ForgePrompt::default();
        assert_eq!(prompt.render_prompt_left(), AI_INDICATOR);
    }

    #[test]
    fn test_render_prompt_left_with_long_title() {
        let long_title = "a".repeat(MAX_LEN + 10);
        let mut prompt = ForgePrompt::default();
        prompt.title(long_title);
        let truncated_title = "a".repeat(MAX_LEN);
        let title_style = Style::new()
            .fg(Color::Cyan)
            .paint(truncated_title)
            .to_string();
        assert_eq!(
            prompt.render_prompt_left(),
            format!("{AI_INDICATOR} {title_style}")
        );
    }

    #[test]
    fn test_render_prompt_right_with_usage() {
        let usage = Usage { prompt_tokens: 10, completion_tokens: 20, total_tokens: 30 };
        let mut prompt = ForgePrompt::default();
        prompt.usage(usage);
        let usage_style = Style::new()
            .bold()
            .fg(Color::DarkGray)
            .paint("[10/20/30]")
            .to_string();
        assert_eq!(prompt.render_prompt_right(), usage_style);
    }

    #[test]
    fn test_render_prompt_right_without_usage() {
        let prompt = ForgePrompt::default();
        assert_eq!(prompt.render_prompt_right(), "");
    }

    #[test]
    fn test_render_prompt_indicator_with_title() {
        let mut prompt = ForgePrompt::default();
        prompt.title("test".to_string());
        let indicator_style = Style::new()
            .fg(Color::LightYellow)
            .paint(format!(" {RIGHT_CHEVRON} "))
            .to_string();
        assert_eq!(
            prompt.render_prompt_indicator(reedline::PromptEditMode::Default),
            indicator_style
        );
    }

    #[test]
    fn test_render_prompt_indicator_without_title() {
        let prompt = ForgePrompt::default();
        assert_eq!(
            prompt.render_prompt_indicator(reedline::PromptEditMode::Default),
            ""
        );
    }

    #[test]
    fn test_render_prompt_multiline_indicator() {
        let prompt = ForgePrompt::default();
        assert_eq!(
            prompt.render_prompt_multiline_indicator(),
            MULTILINE_INDICATOR
        );
    }

    #[test]
    fn test_render_prompt_history_search_indicator_passing() {
        let prompt = ForgePrompt::default();
        let history_search = reedline::PromptHistorySearch {
            status: PromptHistorySearchStatus::Passing,
            term: "test".to_string(),
        };
        assert_eq!(
            prompt.render_prompt_history_search_indicator(history_search),
            " (reverse-search: test) "
        );
    }

    #[test]
    fn test_render_prompt_history_search_indicator_failing() {
        let prompt = ForgePrompt::default();
        let history_search = reedline::PromptHistorySearch {
            status: PromptHistorySearchStatus::Failing,
            term: "test".to_string(),
        };
        assert_eq!(
            prompt.render_prompt_history_search_indicator(history_search),
            " (failing reverse-search: test) "
        );
    }
}
