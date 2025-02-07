use std::fmt;
use std::path::{Path, PathBuf};

use console::{style, Style};
use similar::{ChangeTag, TextDiff};

struct Line(Option<usize>);

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            None => write!(f, "    "),
            Some(idx) => write!(f, "{:<4}", idx + 1),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Source {
    /// Content from a file path
    Path { path: PathBuf, content: String },
    /// Direct string content
    #[allow(dead_code)]
    Content(String),
}

impl Source {
    pub async fn file(path: PathBuf) -> std::io::Result<Self> {
        let content = tokio::fs::read(path.clone()).await?;
        Ok(Source::Path { path, content: String::from_utf8(content).unwrap() })
    }
    /// Get the content of the source
    pub fn content(&self) -> &str {
        match self {
            Source::Path { content, .. } => content,
            Source::Content(content) => content,
        }
    }

    /// Get the path if this source is a Path variant
    pub fn path(&self) -> Option<&Path> {
        match self {
            Source::Path { path, .. } => Some(path),
            Source::Content(_) => None,
        }
    }
}

pub struct DiffPrinter {
    old: Source,
    new: Source,
}

impl DiffPrinter {
    pub fn new(old: Source, new: Source) -> Self {
        DiffPrinter { old, new }
    }

    /// Display the paths if they exist.
    fn format_file_paths_section(
        &self,
        old_path: Option<&Path>,
        new_path: Option<&Path>,
        mut output: String,
    ) -> String {
        // Only show file paths section if at least one path is present
        if old_path.is_some() || new_path.is_some() {
            output.push_str(&format!(
                "\n{}\n",
                style("┌─── File Changes ").bold().cyan()
            ));

            match (old_path.as_deref(), new_path.as_deref()) {
                (Some(old), Some(new)) => {
                    // Check if paths are the same
                    if old == new {
                        output.push_str(&format!(
                            "{}  {} {}",
                            style("│").bold().cyan(),
                            style("Path:").dim(),
                            style(old.display()).bold().underlined()
                        ));
                        output.push_str(&format!("\n"));
                    } else {
                        // Different paths
                        output.push_str(&format!(
                            "{}  {} {}\n",
                            style("│").bold().cyan(),
                            style("Old:").dim(),
                            style(old.display()).bold().underlined()
                        ));
                        output.push_str(&format!(
                            "{}  {} {}\n",
                            style("│").bold().cyan(),
                            style("New:").dim(),
                            style(new.display()).bold().underlined()
                        ));
                    }
                }
                (Some(path), None) | (None, Some(path)) => {
                    // Only one path available
                    output.push_str(&format!(
                        "{}  {} {}\n",
                        style("│").bold().cyan(),
                        style("Path:").dim(),
                        style(path.display()).bold().underlined()
                    ));
                }
                _ => {
                    // no-op, we won't reach here bcoz of the if condition
                }
            }

            output.push_str(&format!("{}\n", style("└───────────────").bold().cyan()));
        }
        output
    }

    pub fn diff(&self) -> String {
        let old_content = self.old.content();
        let new_content = self.new.content();
        let new_file_path = self.new.path();
        let old_file_path = self.old.path();

        let diff = TextDiff::from_lines(old_content, new_content);

        let mut output =
            self.format_file_paths_section(old_file_path, new_file_path, String::new());

        for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
            if idx > 0 {
                output.push_str(&format!("{:-^1$}\n", "-", 80));
            }
            for op in group {
                for change in diff.iter_inline_changes(op) {
                    let (sign, s) = match change.tag() {
                        ChangeTag::Delete => ("-", Style::new().red()),
                        ChangeTag::Insert => ("+", Style::new().green()),
                        ChangeTag::Equal => (" ", Style::new().dim()),
                    };

                    output.push_str(&format!(
                        "{}{} |{}",
                        style(Line(change.old_index())).dim(),
                        style(Line(change.new_index())).dim(),
                        s.apply_to(sign).bold(),
                    ));

                    for (emphasized, value) in change.iter_strings_lossy() {
                        if emphasized {
                            output.push_str(&format!(
                                "{}",
                                s.apply_to(value).underlined().on_black()
                            ));
                        } else {
                            output.push_str(&format!("{}", s.apply_to(value)));
                        }
                    }
                    if change.missing_newline() {
                        output.push('\n');
                    }
                }
            }
        }
        output
    }
}
