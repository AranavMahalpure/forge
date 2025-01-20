use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Parser as TsParser, Query};

use crate::error::Error;
use crate::symbol::{Location, Symbol, SymbolKind};

/// Functions for getting language-specific tree-sitter parsers.
/// These functions initialize the appropriate tree-sitter language parser
/// for each supported programming language.
fn language_rust() -> Language {
    tree_sitter_rust::LANGUAGE.into()
}

fn language_javascript() -> Language {
    tree_sitter_javascript::LANGUAGE.into()
}

fn language_python() -> Language {
    tree_sitter_python::LANGUAGE.into()
}

fn language_typescript() -> Language {
    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
}

fn language_c() -> Language {
    tree_sitter_c::LANGUAGE.into()
}

fn language_cpp() -> Language {
    tree_sitter_cpp::LANGUAGE.into()
}

fn language_csharp() -> Language {
    tree_sitter_c_sharp::LANGUAGE.into()
}

fn language_java() -> Language {
    tree_sitter_java::LANGUAGE.into()
}

fn language_go() -> Language {
    tree_sitter_go::LANGUAGE.into()
}

fn language_ruby() -> Language {
    tree_sitter_ruby::LANGUAGE.into()
}

fn language_php() -> Language {
    tree_sitter_php::LANGUAGE_PHP.into()
}

/// Parser responsible for analyzing source code and extracting symbols.
///
/// The Parser uses tree-sitter to parse source code and extract information
/// about defined symbols such as functions, classes, and variables. It
/// maintains separate parsers and queries for each supported programming
/// language.
pub struct Parser {
    /// Tree-sitter language parsers for each supported language
    parsers: HashMap<String, Language>,
    /// Tree-sitter queries for extracting symbols from each language
    queries: HashMap<String, Query>,
}

impl Parser {
    pub fn new() -> Result<Self, Error> {
        let mut parsers = HashMap::new();
        let mut queries = HashMap::new();

        // Initialize parsers
        parsers.insert("rust".to_string(), language_rust());
        parsers.insert("javascript".to_string(), language_javascript());
        parsers.insert("python".to_string(), language_python());
        parsers.insert("typescript".to_string(), language_typescript());
        parsers.insert("c".to_string(), language_c());
        parsers.insert("cpp".to_string(), language_cpp());
        parsers.insert("csharp".to_string(), language_csharp());
        parsers.insert("java".to_string(), language_java());
        parsers.insert("go".to_string(), language_go());
        parsers.insert("ruby".to_string(), language_ruby());
        parsers.insert("php".to_string(), language_php());

        // Load queries from embedded files in the lang directory
        Self::load_query(&mut queries, "rust", include_str!("lang/tree-sitter-rust-tags.scm"))?;
        Self::load_query(&mut queries, "javascript", include_str!("lang/tree-sitter-javascript-tags.scm"))?;
        Self::load_query(&mut queries, "python", include_str!("lang/tree-sitter-python-tags.scm"))?;
        Self::load_query(&mut queries, "typescript", include_str!("lang/tree-sitter-typescript-tags.scm"))?;
        Self::load_query(&mut queries, "c", include_str!("lang/tree-sitter-c-tags.scm"))?;
        Self::load_query(&mut queries, "cpp", include_str!("lang/tree-sitter-cpp-tags.scm"))?;
        Self::load_query(&mut queries, "csharp", include_str!("lang/tree-sitter-c_sharp-tags.scm"))?;
        Self::load_query(&mut queries, "java", include_str!("lang/tree-sitter-java-tags.scm"))?;
        Self::load_query(&mut queries, "go", include_str!("lang/tree-sitter-go-tags.scm"))?;
        Self::load_query(&mut queries, "ruby", include_str!("lang/tree-sitter-ruby-tags.scm"))?;
        Self::load_query(&mut queries, "php", include_str!("lang/tree-sitter-php-tags.scm"))?;

        Ok(Self { parsers, queries })
    }

    fn load_query(
        queries: &mut HashMap<String, Query>,
        language: &str,
        query_source: &str,
    ) -> Result<(), Error> {
        if let Some(lang) = get_language(language) {
            let query = Query::new(&lang, query_source)
                .map_err(|e| Error::Parse(format!("Failed to parse {} query: {}", language, e)))?;
            queries.insert(language.to_string(), query);
            Ok(())
        } else {
            Err(Error::UnsupportedLanguage(language.to_string()))
        }
    }

    pub fn parse_file(&self, path: &Path, content: &str) -> Result<Vec<Symbol>, Error> {
        let language = self.detect_language(path)?;
        let lang = self
            .parsers
            .get(language)
            .ok_or_else(|| Error::UnsupportedLanguage(language.to_string()))?;

        let mut parser = TsParser::new();
        parser
            .set_language(lang)
            .map_err(|e| Error::TreeSitter(e.to_string()))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| Error::Parse("Failed to parse file".to_string()))?;

        let query = self.queries.get(language).ok_or_else(|| {
            Error::Parse(format!("No query available for language: {}", language))
        })?;

        let mut symbols = Vec::new();

        // Extract symbols using the appropriate query
        let mut query_cursor = tree_sitter::QueryCursor::new();
        let mut cursor = query_cursor.matches(query, tree.root_node(), content.as_bytes());
        while let Some(match_) = cursor.next() {
            for capture in match_.captures {
                let capture_name = &query.capture_names()[capture.index as usize].to_string();
                let node = capture.node;

                if let Some(symbol) = self.create_symbol(capture_name, node, path, content)? {
                    symbols.push(symbol);
                }
            }
        }

        Ok(symbols)
    }

    fn detect_language(&self, path: &Path) -> Result<&str, Error> {
        match path.extension().and_then(|s| s.to_str()) {
            Some("rs") => Ok("rust"),
            Some("js") => Ok("javascript"),
            Some("py") => Ok("python"),
            Some("ts") | Some("tsx") => Ok("typescript"),
            Some("c") | Some("h") => Ok("c"),
            Some("cpp") | Some("hpp") | Some("cc") | Some("hh") => Ok("cpp"),
            Some("cs") => Ok("csharp"),
            Some("java") => Ok("java"),
            Some("go") => Ok("go"),
            Some("rb") => Ok("ruby"),
            Some("php") => Ok("php"),
            _ => Err(Error::UnsupportedLanguage(format!(
                "Unsupported file extension: {:?}",
                path.extension()
            ))),
        }
    }

    fn create_symbol(
        &self,
        capture_name: &str,
        node: tree_sitter::Node,
        file_path: &Path,
        source: &str,
    ) -> Result<Option<Symbol>, Error> {
        let name = node
            .utf8_text(source.as_bytes())
            .map_err(|e| Error::Parse(e.to_string()))?
            .to_string();

        let kind = if capture_name.starts_with("name.definition.") {
            match &capture_name["name.definition.".len()..] {
                "function" => Some(SymbolKind::Function),
                "method" => Some(SymbolKind::Method),
                "class" => Some(SymbolKind::Class),
                "interface" => Some(SymbolKind::Interface),
                "struct" => Some(SymbolKind::Struct),
                "enum" => Some(SymbolKind::Enum),
                "constant" => Some(SymbolKind::Constant),
                "variable" => Some(SymbolKind::Variable),
                "module" => Some(SymbolKind::Module),
                "trait" => Some(SymbolKind::Trait),
                _ => None,
            }
        } else {
            None
        };

        let kind = match kind {
            Some(k) => k,
            None => return Ok(None),
        };

        let location = Location {
            path: Rc::new(file_path.to_path_buf()),
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
            start_col: node.start_position().column,
            end_col: node.end_position().column,
        };

        Ok(Some(Symbol::new(name, kind, location)))
    }
}

fn get_language(language: &str) -> Option<Language> {
    match language {
        "rust" => Some(language_rust()),
        "javascript" => Some(language_javascript()),
        "python" => Some(language_python()),
        "typescript" => Some(language_typescript()),
        "c" => Some(language_c()),
        "cpp" => Some(language_cpp()),
        "csharp" => Some(language_csharp()),
        "java" => Some(language_java()),
        "go" => Some(language_go()),
        "ruby" => Some(language_ruby()),
        "php" => Some(language_php()),
        _ => None,
    }
}
