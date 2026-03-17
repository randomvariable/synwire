# How to: Perform Advanced Search

**Goal:** Use `Vfs::grep` with `GrepOptions` to run ripgrep-style content searches against any backend.

---

## GrepOptions reference

```rust
use synwire_core::vfs::grep_options::{GrepOptions, GrepOutputMode};

let opts = GrepOptions {
    path: None,                  // search root; None = backend's current working directory
    case_insensitive: false,
    invert: false,               // true = return non-matching lines
    line_numbers: false,         // include line numbers in results
    output_mode: GrepOutputMode::Content,
    before_context: 0,           // lines to include before each match
    after_context: 0,            // lines to include after each match
    context: None,               // symmetric context (overrides before/after when set)
    max_matches: None,           // stop after N total matches
    glob: None,                  // filename glob filter, e.g. "*.rs"
    file_type: None,             // ripgrep-style type: "rust", "python", "go", etc.
    multiline: false,
    fixed_string: false,         // treat pattern as literal, not regex
};
```

All fields have `Default` implementations. Start from `GrepOptions::default()` and override only what you need.

---

## GrepMatch fields

```rust
pub struct GrepMatch {
    pub file: String,
    pub line_number: usize,    // 1-indexed when line_numbers: true; 0 otherwise
    pub column: usize,         // 0-indexed byte offset of match start
    pub line_content: String,  // the matching line
    pub before: Vec<String>,   // lines before the match (up to before_context)
    pub after: Vec<String>,    // lines after the match (up to after_context)
}
```

In `Count` mode, `line_number` holds the match count for the file and `line_content` is its string representation.

---

## Case-insensitive search

```rust
use synwire_core::vfs::grep_options::GrepOptions;
use synwire_core::vfs::protocol::Vfs;

let opts = GrepOptions {
    case_insensitive: true,
    line_numbers: true,
    ..Default::default()
};

let matches = backend.grep("todo", opts).await?;
for m in &matches {
    println!("{}:{} {}", m.file, m.line_number, m.line_content);
}
```

---

## Context lines

```rust
let opts = GrepOptions {
    context: Some(2),     // 2 lines before AND after each match
    line_numbers: true,
    ..Default::default()
};

// Or use asymmetric context:
let opts = GrepOptions {
    before_context: 3,
    after_context: 1,
    ..Default::default()
};
```

When both `context` and `before_context`/`after_context` are set, `context` takes precedence.

---

## File-type filter

Accepts ripgrep-style type names. Supported aliases include `rust`/`rs`, `python`/`py`, `js`/`javascript`, `ts`/`typescript`, `go`, `json`, `yaml`/`yml`, `toml`, `md`/`markdown`, `sh`/`bash`.

```rust
let opts = GrepOptions {
    file_type: Some("rust".to_string()),
    ..Default::default()
};

let matches = backend.grep("unwrap", opts).await?;
// Returns only matches in *.rs files.
```

---

## Glob filter

Restricts results to files whose name matches the glob pattern. `*` matches any sequence of non-separator characters; `**` matches anything.

```rust
let opts = GrepOptions {
    glob: Some("*.toml".to_string()),
    ..Default::default()
};

let matches = backend.grep("version", opts).await?;
```

---

## Inverted match

Returns lines that do NOT match the pattern.

```rust
let opts = GrepOptions {
    invert: true,
    ..Default::default()
};

// Lines that do not contain "TODO".
let matches = backend.grep("TODO", opts).await?;
```

---

## Files-with-matches mode

Returns one entry per file (with empty `line_content`) rather than one entry per matching line.

```rust
use synwire_core::vfs::grep_options::GrepOutputMode;

let opts = GrepOptions {
    output_mode: GrepOutputMode::FilesWithMatches,
    ..Default::default()
};

let matches = backend.grep("panic!", opts).await?;
let files: Vec<&str> = matches.iter().map(|m| m.file.as_str()).collect();
```

---

## Count mode

Returns one entry per file with `line_number` set to the number of matches in that file.

```rust
let opts = GrepOptions {
    output_mode: GrepOutputMode::Count,
    ..Default::default()
};

let counts = backend.grep("error", opts).await?;
for c in &counts {
    println!("{}: {} occurrences", c.file, c.line_number);
}
```

---

## Limiting results

```rust
let opts = GrepOptions {
    max_matches: Some(50),
    ..Default::default()
};
```

The search stops after `max_matches` total matches across all files. Useful for large codebases where you only need the first few hits.

---

## Scoped search path

```rust
let opts = GrepOptions {
    path: Some("src/backends".to_string()),
    file_type: Some("rust".to_string()),
    ..Default::default()
};

let matches = backend.grep("VfsError", opts).await?;
```

`path` is resolved relative to the backend's current working directory.

---

**See also**

- [How to: Use the Backend Implementations](vfs.md) — `Vfs` operations
- [How to: Configure the Middleware Stack](middleware.md)
- [Reference: Feature Flags](../reference/feature-flags.md)
