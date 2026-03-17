//! Language detection from file extension.

use std::path::Path;

/// Supported source language for AST-aware chunking.
///
/// Variants with no tree-sitter grammar that is compatible with tree-sitter 0.24
/// (e.g. Toml and Markdown) are present so that callers can record the language
/// even when AST chunking is unavailable; those paths fall back to the text
/// splitter automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Language {
    /// Rust source files.
    Rust,
    /// Python source files.
    Python,
    /// JavaScript source files.
    JavaScript,
    /// TypeScript source files (`.ts` and `.tsx`).
    TypeScript,
    /// Go source files.
    Go,
    /// Java source files.
    Java,
    /// C source and header files.
    C,
    /// C++ source and header files.
    Cpp,
    /// C# source files.
    CSharp,
    /// Ruby source files.
    Ruby,
    /// Bash/shell scripts.
    Bash,
    /// JSON data files.
    Json,
    /// TOML configuration files.
    Toml,
    /// YAML configuration files.
    Yaml,
    /// HTML documents.
    Html,
    /// CSS stylesheets.
    Css,
    /// Markdown documents.
    Markdown,
}

/// Detect the programming language from a file path using its extension.
///
/// Returns `None` for unrecognised extensions — the caller should fall back
/// to the text splitter.
///
/// # Examples
///
/// ```
/// use synwire_chunker::detect_language;
/// use synwire_chunker::Language;
/// use std::path::Path;
///
/// assert_eq!(detect_language(Path::new("main.rs")), Some(Language::Rust));
/// assert_eq!(detect_language(Path::new("index.ts")), Some(Language::TypeScript));
/// assert_eq!(detect_language(Path::new("unknown.xyz")), None);
/// ```
pub fn detect_language(path: &Path) -> Option<Language> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "rs" => Some(Language::Rust),
        "py" => Some(Language::Python),
        "js" | "mjs" | "cjs" => Some(Language::JavaScript),
        "ts" | "tsx" => Some(Language::TypeScript),
        "go" => Some(Language::Go),
        "java" => Some(Language::Java),
        "c" | "h" => Some(Language::C),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some(Language::Cpp),
        "cs" => Some(Language::CSharp),
        "rb" => Some(Language::Ruby),
        "sh" | "bash" => Some(Language::Bash),
        "json" => Some(Language::Json),
        "toml" => Some(Language::Toml),
        "yaml" | "yml" => Some(Language::Yaml),
        "html" | "htm" => Some(Language::Html),
        "css" => Some(Language::Css),
        "md" | "markdown" => Some(Language::Markdown),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detects_rust() {
        assert_eq!(detect_language(Path::new("foo.rs")), Some(Language::Rust));
    }

    #[test]
    fn detects_typescript_tsx() {
        assert_eq!(
            detect_language(Path::new("app.tsx")),
            Some(Language::TypeScript)
        );
    }

    #[test]
    fn detects_cpp_variants() {
        for ext in &["cpp", "cc", "cxx", "hpp", "hxx"] {
            let path = format!("file.{ext}");
            assert_eq!(
                detect_language(Path::new(&path)),
                Some(Language::Cpp),
                "failed for extension {ext}"
            );
        }
    }

    #[test]
    fn unknown_extension_returns_none() {
        assert_eq!(detect_language(Path::new("archive.zip")), None);
    }

    #[test]
    fn no_extension_returns_none() {
        assert_eq!(detect_language(Path::new("Makefile")), None);
    }
}
