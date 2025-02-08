use std::fmt;
use std::path::PathBuf;

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

pub struct Format;

impl Format {
    pub fn format(path: PathBuf, old: &str, new: &str) -> String {
        let diff = TextDiff::from_lines(old, new);
        let ops = diff.grouped_ops(3);

        let mut output = format!(
            "{} {}\n",
            style("File:").bold(),
            style(path.display()).dim()
        );

        if ops.is_empty() {
            output.push_str(&format!("{}\n", style("No changes found").dim()));
            return output;
        }

        for (idx, group) in ops.iter().enumerate() {
            if idx > 0 {
                output.push_str(&format!("{}\n", style("...").dim()));
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
                            output.push_str(&format!("{}", s.apply_to(value).underlined()));
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

#[cfg(test)]
mod tests {
    use console::strip_ansi_codes;
    use insta::assert_snapshot;

    use super::*;

    #[test]
    fn test_diff_printer_no_differences() {
        let content = "line 1\nline 2\nline 3";
        let diff = Format::format("xyz.txt".into(), content, content);
        assert!(diff.contains("No changes found"));
    }

    #[test]
    fn test_file_source() {
        let old = "line 1\nline 2\nline 3\nline 4\nline 5";
        let new = "line 1\nline 2\nline 3";
        let diff = Format::format("xya.txt".into(), old, new);
        let clean_diff = strip_ansi_codes(&diff);
        assert_snapshot!(clean_diff);
    }

    #[test]
    fn test_diff_printer_simple_diff() {
        let old = "line 1\nline 2\nline 3\nline 5\nline 6\nline 7\nline 8\nline 9";
        let new = "line 1\nmodified line\nline 3\nline 5\nline 6\nline 7\nline 8\nline 9";
        let diff = Format::format("abc.txt".into(), old, new);
        let clean_diff = strip_ansi_codes(&diff);
        assert_snapshot!(clean_diff);
    }
}
