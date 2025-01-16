use std::collections::{HashMap, HashSet};
use std::path::Path;

use forge_domain::{NamedTool, ToolCallService, ToolDescription, ToolName};
use forge_tool_macros::ToolDescription;
use schemars::JsonSchema;
use serde::Deserialize;
use streaming_iterator::{IntoStreamingIterator, StreamingIterator};
use tokio::fs;
use tree_sitter::{Language, Parser, Query, QueryCursor};
use walkdir::WalkDir;

const JAVASCRIPT: &str = include_str!("queries/javascript.rkt");
const PYTHON: &str = include_str!("queries/python.rkt");
const RUST: &str = include_str!("queries/rust.rkt");
const TYPESCRIPT: &str = include_str!("queries/typescript.rkt");
const CSS: &str = include_str!("queries/css.rkt");
const JAVA: &str = include_str!("queries/java.rkt");
const SCALA: &str = include_str!("queries/scala.rkt");

fn load_language_parser(language_name: &str) -> Result<Language, String> {
    match language_name {
        "rust" => Ok(tree_sitter_rust::LANGUAGE.into()),
        "javascript" => Ok(tree_sitter_javascript::LANGUAGE.into()),
        "python" => Ok(tree_sitter_python::LANGUAGE.into()),
        "typescript" => Ok(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" => Ok(tree_sitter_typescript::LANGUAGE_TSX.into()),
        "css" => Ok(tree_sitter_css::LANGUAGE.into()),
        "java" => Ok(tree_sitter_java::LANGUAGE.into()),
        "scala" => Ok(tree_sitter_scala::LANGUAGE.into()),
        x => Err(format!("Unsupported language: {}", x)),
    }
}

fn load_queries() -> HashMap<&'static str, &'static str> {
    let mut queries = HashMap::new();
    queries.insert("rust", RUST);
    queries.insert("javascript", JAVASCRIPT);
    queries.insert("python", PYTHON);
    queries.insert("typescript", TYPESCRIPT);
    queries.insert("tsx", TYPESCRIPT); // Use TypeScript query for TSX files
    queries.insert("css", CSS);
    queries.insert("java", JAVA);
    queries.insert("scala", SCALA);
    queries
}

fn parse_file(_file: &Path, content: &str, parser: &mut Parser, query: &Query) -> Option<String> {
    let tree = parser.parse(content, None)?;
    let mut cursor = QueryCursor::new();
    let mut formatted_output = String::new();
    let mut last_line: i64 = -1;
    let mut seen_lines = HashSet::new();

    let mut captures: Vec<_> = cursor
        .matches(query, tree.root_node(), content.as_bytes())
        .flat_map(|m| m.captures.into_streaming_iter())
        .filter_map(|capture| {
            let node = capture.node;
            let start_line = node.start_position().row;
            // let end_line = node.end_position().row;
            // Get the full text of the node instead of just the first line
            let text = node.utf8_text(content.as_bytes()).ok()?;
            // Get the first line of the definition which contains the signature
            let first_line = text.lines().next()?.trim().to_string();
            Some((start_line, first_line))
        })
        .fold(Vec::default(), |mut acc, x| {
            acc.push(x.to_owned());
            acc
        });

    captures.sort_by_key(|&(row, _)| row);

    for (start_line, text) in captures {
        let start_line = start_line.to_owned() as i64;
        if !seen_lines.insert(start_line) {
            continue;
        }

        if last_line != -1 && start_line > last_line + 1 {
            formatted_output.push_str("|----\n");
        }

        formatted_output.push_str(&format!("│{}\n", text.trim()));
        last_line = start_line;
    }

    if formatted_output.is_empty() {
        None
    } else {
        Some(formatted_output)
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct OutlineInput {
    /// The path to the directory containing the source code files to analyze.
    pub path: String,
}

/// This tool helps developers analyze source code by listing key definitions
/// like classes, functions, and methods, making it easier to understand code
/// structure and relationships. It's ideal for navigating large or unfamiliar
/// codebases, improving comprehension during onboarding, code reviews, and
/// refactoring. The tool visualizes inheritance hierarchies, identifies
/// implementations, and uncovers dependencies and architectural patterns.
/// It tracks type usage, locates definitions, and clarifies module
/// organization, providing deep insights into system interactions. Supporting
/// most programming languages, it enhances productivity in complex projects and
/// notifies users when encountering unsupported languages.
#[derive(ToolDescription)]
pub struct Outline;

impl NamedTool for Outline {
    fn tool_name(&self) -> ToolName {
        ToolName::new("outline")
    }
}

#[async_trait::async_trait]
impl ToolCallService for Outline {
    type Input = OutlineInput;
    type Output = String;

    async fn call(&self, input: Self::Input) -> Result<Self::Output, String> {
        let extensions_to_languages = HashMap::from([
            ("rs", "rust"),
            ("js", "javascript"),
            ("py", "python"),
            ("ts", "typescript"),
            ("tsx", "tsx"),
            ("css", "css"),
            ("scss", "css"),
            ("java", "java"),
            ("scala", "scala"),
        ]);

        let queries = load_queries();
        let mut parsers: HashMap<&str, (Parser, Query)> = HashMap::new();
        let mut result = String::new();

        let entries = WalkDir::new(&input.path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file()
                    && e.path()
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|ext| {
                            extensions_to_languages.contains_key(ext.to_lowercase().as_str())
                        })
                        .unwrap_or(false)
            })
            .collect::<Vec<_>>();

        for entry in entries {
            let path = entry.path().to_path_buf();
            if let Ok(content) = fs::read_to_string(&path).await {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if let Some(&lang_name) =
                        extensions_to_languages.get(ext.to_lowercase().as_str())
                    {
                        if !parsers.contains_key(lang_name) {
                            let language = load_language_parser(lang_name)?;
                            let mut parser = Parser::new();
                            parser.set_language(&language).map_err(|e| e.to_string())?;
                            let query = Query::new(&language, queries[lang_name])
                                .map_err(|e| e.to_string())?;
                            parsers.insert(lang_name, (parser, query));
                        }

                        if let Some((parser, query)) = parsers.get_mut(lang_name) {
                            if let Some(file_output) = parse_file(&path, &content, parser, query) {
                                if !result.is_empty() {
                                    result.push_str("|----\n");
                                }
                                result.push_str(&format!(
                                    "{}\n",
                                    path.file_name().unwrap().to_string_lossy()
                                ));
                                result.push_str(&file_output);
                            }
                        }
                    }
                }
            }
        }

        if result.is_empty() {
            Ok("No source code definitions found.".into())
        } else {
            Ok(result)
        }
    }
}

#[cfg(test)]
mod tests;
