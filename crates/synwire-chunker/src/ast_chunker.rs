//! AST-based code chunking using tree-sitter.
//!
//! Parses source text into an AST and extracts top-level definition nodes
//! (functions, classes, structs, etc.) as individual [`Document`]s.

use std::collections::HashMap;

use serde_json::Value;
use synwire_core::documents::Document;
use tree_sitter::{Node, Parser};

use crate::language::Language;

/// Top-level AST node kinds that represent semantic units for the given language.
///
/// Only node kinds that correspond to meaningful top-level definitions are
/// returned so that the chunker produces useful, self-contained chunks.
const fn definition_kinds(lang: Language) -> &'static [&'static str] {
    match lang {
        Language::Rust => &[
            "function_item",
            "impl_item",
            "struct_item",
            "enum_item",
            "trait_item",
            "type_alias",
        ],
        Language::Python => &["function_definition", "class_definition"],
        Language::JavaScript => &[
            "function_declaration",
            "class_declaration",
            "method_definition",
            "arrow_function",
        ],
        Language::TypeScript => &[
            "function_declaration",
            "class_declaration",
            "method_definition",
            "interface_declaration",
            "type_alias_declaration",
        ],
        Language::Go => &[
            "function_declaration",
            "method_declaration",
            "type_declaration",
        ],
        Language::Java => &[
            "method_declaration",
            "class_declaration",
            "interface_declaration",
            "constructor_declaration",
        ],
        Language::C => &["function_definition", "struct_specifier"],
        Language::Cpp => &[
            "function_definition",
            "struct_specifier",
            "class_specifier",
            "namespace_definition",
        ],
        Language::CSharp => &[
            "method_declaration",
            "class_declaration",
            "interface_declaration",
            "property_declaration",
        ],
        Language::Ruby => &["method", "singleton_method", "class", "module"],
        Language::Bash => &["function_definition"],
        // Data / markup formats: no useful definition-level splitting.
        Language::Json
        | Language::Toml
        | Language::Yaml
        | Language::Html
        | Language::Css
        | Language::Markdown => &[],
    }
}

/// Retrieve the tree-sitter [`Language`][`tree_sitter::Language`] object for
/// the given [`Language`] variant.
///
/// Returns `None` for variants that do not have a compatible tree-sitter grammar
/// bundled with this crate.
fn ts_language(lang: Language) -> Option<tree_sitter::Language> {
    let tsl = match lang {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
        Language::Java => tree_sitter_java::LANGUAGE.into(),
        Language::C => tree_sitter_c::LANGUAGE.into(),
        Language::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        Language::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
        Language::Ruby => tree_sitter_ruby::LANGUAGE.into(),
        Language::Bash => tree_sitter_bash::LANGUAGE.into(),
        Language::Json => tree_sitter_json::LANGUAGE.into(),
        Language::Yaml => tree_sitter_yaml::LANGUAGE.into(),
        Language::Html => tree_sitter_html::LANGUAGE.into(),
        Language::Css => tree_sitter_css::LANGUAGE.into(),
        // No compatible crate is bundled for these formats.
        Language::Toml | Language::Markdown => return None,
    };
    Some(tsl)
}

/// Return the language name as a lowercase string for use in metadata.
const fn language_name(lang: Language) -> &'static str {
    match lang {
        Language::Rust => "rust",
        Language::Python => "python",
        Language::JavaScript => "javascript",
        Language::TypeScript => "typescript",
        Language::Go => "go",
        Language::Java => "java",
        Language::C => "c",
        Language::Cpp => "cpp",
        Language::CSharp => "csharp",
        Language::Ruby => "ruby",
        Language::Bash => "bash",
        Language::Json => "json",
        Language::Toml => "toml",
        Language::Yaml => "yaml",
        Language::Html => "html",
        Language::Css => "css",
        Language::Markdown => "markdown",
    }
}

/// Span information extracted from a tree-sitter node.
#[derive(Clone)]
struct NodeSpan {
    /// Optional symbol name.  May be a plain identifier (`"foo"`) or a
    /// qualified name (`"Foo::bar"`) for per-method chunks.
    symbol: Option<String>,
    byte_start: usize,
    byte_end: usize,
    /// 1-indexed first line.
    line_start: usize,
    /// 1-indexed last line.
    line_end: usize,
}

/// Build a [`Document`] from a source span.
fn build_doc(file_path: &str, content: &str, lang: Language, span: NodeSpan) -> Document {
    let page_content = content
        .get(span.byte_start..span.byte_end)
        .unwrap_or("")
        .to_owned();

    let mut metadata: HashMap<String, Value> = HashMap::new();
    let _ = metadata.insert("file".to_owned(), Value::String(file_path.to_owned()));
    let _ = metadata.insert(
        "language".to_owned(),
        Value::String(language_name(lang).to_owned()),
    );
    let _ = metadata.insert(
        "line_start".to_owned(),
        Value::Number(span.line_start.into()),
    );
    let _ = metadata.insert("line_end".to_owned(), Value::Number(span.line_end.into()));
    if let Some(sym) = span.symbol {
        let _ = metadata.insert("symbol".to_owned(), Value::String(sym));
    }

    Document::with_metadata(page_content, metadata)
}

/// Extract the symbol name from a definition node by finding the first child
/// whose kind is `identifier`, `name`, or `field_identifier`.
fn extract_symbol<'a>(node: Node<'_>, source: &'a [u8]) -> Option<&'a str> {
    for i in 0..node.child_count() {
        let child = node.child(i)?;
        match child.kind() {
            "identifier" | "name" | "field_identifier" | "type_identifier" => {
                return child.utf8_text(source).ok();
            }
            _ => {}
        }
    }
    None
}

/// Container node kinds that hold per-method definitions for supported languages.
///
/// Returns `Some((container_kind, method_kind))` if `lang` supports per-method
/// chunking for the given top-level node kind, otherwise `None`.
///
/// The `container_kind` is the AST node kind of the container (e.g. `impl_item`
/// for Rust), and `method_kind` is the kind of the direct method children to
/// extract from within that container.
const fn method_container(lang: Language) -> Option<(&'static str, &'static str)> {
    match lang {
        Language::Rust => Some(("impl_item", "function_item")),
        Language::Python => Some(("class_definition", "function_definition")),
        Language::JavaScript | Language::TypeScript => {
            Some(("class_declaration", "method_definition"))
        }
        Language::Java | Language::CSharp => Some(("class_declaration", "method_declaration")),
        Language::Ruby => Some(("class", "method")),
        _ => None,
    }
}

/// Extract the parent type name from a container node.
///
/// For Rust `impl` blocks this extracts the implemented type name.
/// For class-based languages it extracts the class name.
fn extract_container_name<'a>(node: Node<'_>, source: &'a [u8]) -> Option<&'a str> {
    extract_symbol(node, source)
}

/// Walk the immediate children of `node` and collect definition nodes whose
/// `kind()` is in `kinds`.
///
/// For container nodes (e.g. `impl_item`, `class_declaration`) this recurses
/// one level deeper to extract per-method chunks, setting `symbol` to
/// `"ParentType::method_name"` format.  Top-level definitions are still
/// collected normally.
fn collect_definitions<'a>(
    node: Node<'a>,
    kinds: &[&str],
    lang: Language,
    source: &[u8],
    out: &mut Vec<(Node<'a>, Option<String>)>,
) {
    let container = method_container(lang);

    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else {
            continue;
        };
        if !kinds.contains(&child.kind()) {
            continue;
        }

        // Check whether this top-level node is a container that should be
        // split into per-method chunks.
        if let Some((container_kind, method_kind)) = container {
            if child.kind() == container_kind {
                let parent_name = extract_container_name(child, source);
                let mut found_methods = false;

                // Walk the body/children of the container for method nodes.
                for j in 0..child.child_count() {
                    let Some(body_or_child) = child.child(j) else {
                        continue;
                    };
                    // Some grammars nest methods inside a `block` or `body`
                    // child; try both the direct child and one level down.
                    if body_or_child.kind() == method_kind {
                        let sym = build_qualified_symbol(
                            parent_name,
                            extract_symbol(body_or_child, source),
                        );
                        out.push((body_or_child, sym));
                        found_methods = true;
                    } else {
                        for k in 0..body_or_child.child_count() {
                            let Some(method_node) = body_or_child.child(k) else {
                                continue;
                            };
                            if method_node.kind() == method_kind {
                                let sym = build_qualified_symbol(
                                    parent_name,
                                    extract_symbol(method_node, source),
                                );
                                out.push((method_node, sym));
                                found_methods = true;
                            }
                        }
                    }
                }

                // If we extracted methods, skip adding the container as a
                // whole-block chunk.  If the container was empty (no methods
                // found), fall through and add the container itself.
                if found_methods {
                    continue;
                }
            }
        }

        // Default: add the node as-is (symbol resolved later).
        out.push((child, None));
    }
}

/// Build a qualified `"ParentType::method_name"` symbol string.
///
/// Returns `Some(...)` only when both the parent and method names are known.
fn build_qualified_symbol(parent: Option<&str>, method: Option<&str>) -> Option<String> {
    match (parent, method) {
        (Some(p), Some(m)) => Some(format!("{p}::{m}")),
        (None, Some(m)) => Some(m.to_owned()),
        _ => None,
    }
}

/// Chunk a source file into semantic units using tree-sitter AST parsing.
///
/// Returns one [`Document`] per top-level definition found.  Falls back to
/// the entire file as a single chunk if:
/// - the language has no compatible tree-sitter grammar,
/// - parsing fails, or
/// - no definition-level nodes are found.
pub fn chunk_ast(file_path: &str, content: &str, language: Language) -> Vec<Document> {
    let kinds = definition_kinds(language);

    // For formats without definition kinds, return empty so the caller falls
    // back to the text splitter.
    if kinds.is_empty() {
        return Vec::new();
    }

    let Some(ts_lang) = ts_language(language) else {
        return Vec::new();
    };

    let mut parser = Parser::new();
    if let Err(err) = parser.set_language(&ts_lang) {
        tracing::warn!(
            file = file_path,
            language = language_name(language),
            error = %err,
            "tree-sitter failed to set language; falling back to text splitter",
        );
        return Vec::new();
    }

    let source_bytes = content.as_bytes();
    let Some(tree) = parser.parse(content, None) else {
        tracing::warn!(
            file = file_path,
            language = language_name(language),
            "tree-sitter parse returned None; falling back to text splitter",
        );
        return Vec::new();
    };

    let root = tree.root_node();
    let mut def_nodes: Vec<(Node<'_>, Option<String>)> = Vec::new();
    collect_definitions(root, kinds, language, source_bytes, &mut def_nodes);

    if def_nodes.is_empty() {
        return Vec::new();
    }

    def_nodes
        .into_iter()
        .map(|(node, qualified_sym)| {
            let start = node.start_position();
            let end = node.end_position();
            // Use the pre-computed qualified symbol (e.g. "Foo::bar") if
            // available; otherwise fall back to the plain extracted name.
            let symbol =
                qualified_sym.or_else(|| extract_symbol(node, source_bytes).map(str::to_owned));
            let span = NodeSpan {
                symbol,
                byte_start: node.start_byte(),
                byte_end: node.end_byte(),
                // row is 0-indexed; convert to 1-indexed line numbers.
                line_start: start.row + 1,
                line_end: end.row + 1,
            };
            build_doc(file_path, content, language, span)
        })
        .collect()
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::manual_contains,
    clippy::map_unwrap_or
)]
mod tests {
    use super::*;

    const RUST_SRC: &str = r"
/// A simple add function.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

struct Point {
    x: f64,
    y: f64,
}
";

    #[test]
    fn chunks_rust_functions_and_structs() {
        let docs = chunk_ast("src/lib.rs", RUST_SRC, Language::Rust);
        assert!(
            docs.len() >= 2,
            "expected at least 2 chunks, got {}",
            docs.len()
        );
        let has_add = docs
            .iter()
            .any(|d| d.metadata.get("symbol").and_then(|v| v.as_str()) == Some("add"));
        assert!(has_add, "expected chunk with symbol 'add'");
    }

    #[test]
    fn returns_empty_for_unrecognised_format() {
        // Toml has no bundled grammar.
        let docs = chunk_ast("config.toml", "[package]\nname = \"foo\"", Language::Toml);
        assert!(docs.is_empty());
    }

    #[test]
    fn line_numbers_are_one_indexed() {
        let docs = chunk_ast("src/lib.rs", RUST_SRC, Language::Rust);
        for doc in &docs {
            let line_start = doc
                .metadata
                .get("line_start")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            assert!(
                line_start >= 1,
                "line_start should be >= 1, got {line_start}"
            );
        }
    }

    const RUST_IMPL_SRC: &str = r"
struct Foo;

impl Foo {
    fn bar(&self) -> i32 {
        1
    }

    fn baz(&self) -> i32 {
        2
    }
}

fn top_level() -> i32 {
    42
}
";

    /// Per-method chunking: `impl` blocks are split into individual method
    /// chunks with `"ParentType::method_name"` symbols.
    #[test]
    fn per_method_chunking_rust_impl() {
        let docs = chunk_ast("src/lib.rs", RUST_IMPL_SRC, Language::Rust);

        // Expect at least the two impl methods and the top-level function.
        assert!(
            docs.len() >= 3,
            "expected at least 3 chunks (bar, baz, top_level), got {}",
            docs.len()
        );

        let symbols: Vec<&str> = docs
            .iter()
            .filter_map(|d| d.metadata.get("symbol").and_then(|v| v.as_str()))
            .collect();

        assert!(
            symbols.iter().any(|&s| s == "Foo::bar"),
            "expected chunk with symbol 'Foo::bar', got {symbols:?}"
        );
        assert!(
            symbols.iter().any(|&s| s == "Foo::baz"),
            "expected chunk with symbol 'Foo::baz', got {symbols:?}"
        );
    }

    /// Top-level functions still produce their own chunks with a plain symbol
    /// and are not duplicated inside impl-level output.
    #[test]
    fn top_level_functions_not_duplicated() {
        let docs = chunk_ast("src/lib.rs", RUST_IMPL_SRC, Language::Rust);

        let top_level_chunks: Vec<_> = docs
            .iter()
            .filter(|d| {
                d.metadata
                    .get("symbol")
                    .and_then(|v| v.as_str())
                    .map(|s| s == "top_level")
                    .unwrap_or(false)
            })
            .collect();

        assert!(
            !top_level_chunks.is_empty(),
            "expected at least one top_level chunk"
        );
        assert_eq!(
            top_level_chunks.len(),
            1,
            "top_level function should appear exactly once, got {}",
            top_level_chunks.len()
        );
    }
}
