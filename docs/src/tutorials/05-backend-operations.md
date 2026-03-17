# Working with Files and Shell

**Time**: ~35 minutes
**Prerequisites**: Completed `01-first-agent.md`, basic familiarity with async Rust

Agents often need to read configuration files, write output artefacts, search source code,
or navigate a directory hierarchy. Synwire provides a uniform interface for all of these
through the `Vfs` trait. Two concrete implementations are built in:

- `MemoryProvider` — ephemeral, in-memory storage. No files on disk. Ideal for tests and
  agent scratchpads.
- `LocalProvider` — real OS filesystem, scoped to a root directory. Suitable for
  agents that must persist output between runs.

Both backends enforce safety boundaries that prevent an agent from escaping its allowed
working directory. This tutorial teaches you to use both, understand the error model, and
search file content with ripgrep-style options.

---

## What you are building

1. Writing, reading, and navigating with `MemoryProvider`.
2. Attempting a path traversal and observing the rejection.
3. Using `LocalProvider` scoped to a temporary directory.
4. Searching file content with `grep` and `GrepOptions`.
5. Reading `GrepMatch` fields.

---

## Step 1: Add dependencies

```toml
[dependencies]
synwire-core  = { path = "../../crates/synwire-core" }
synwire-agent = { path = "../../crates/synwire-agent" }
tokio = { version = "1", features = ["full"] }
```

---

## Step 2: Understand Vfs

`Vfs` is the trait all backends implement. Every method returns a
`BoxFuture<'_, Result<T, VfsError>>`, so operations are always `async`:

```rust
pub trait Vfs: Send + Sync {
    fn read(&self, path: &str)    -> BoxFuture<'_, Result<FileContent, VfsError>>;
    fn write(&self, path: &str, content: &[u8])
                                  -> BoxFuture<'_, Result<WriteResult, VfsError>>;
    fn ls(&self, path: &str)      -> BoxFuture<'_, Result<Vec<DirEntry>, VfsError>>;
    fn grep(&self, pattern: &str, opts: GrepOptions)
                                  -> BoxFuture<'_, Result<Vec<GrepMatch>, VfsError>>;
    fn pwd(&self)                 -> BoxFuture<'_, Result<String, VfsError>>;
    fn cd(&self, path: &str)      -> BoxFuture<'_, Result<(), VfsError>>;
    // ... and rm, cp, mv_file, edit, glob, upload, download, capabilities
}
```

You call `backend.read("file.txt").await?` exactly the same way for both
`MemoryProvider` and `LocalProvider`.

---

## Step 3: MemoryProvider — write and read

```rust
use synwire_core::vfs::state_backend::MemoryProvider;
use synwire_core::vfs::protocol::Vfs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = MemoryProvider::new();

    // Write a file. Content is raw bytes; use b"..." for UTF-8 string literals.
    backend.write("notes.txt", b"hello from synwire").await?;

    // Read it back. FileContent.content is Vec<u8>.
    let file = backend.read("notes.txt").await?;
    let text = String::from_utf8(file.content)?;
    assert_eq!(text, "hello from synwire");

    println!("read back: {text}");
    Ok(())
}
```

`MemoryProvider::new()` creates an empty backend with `/` as the working directory. There is
no persistence — when the `MemoryProvider` is dropped, all data is lost.

---

## Step 4: Navigate the working directory

Paths in `MemoryProvider` are resolved relative to the current working directory, exactly
like a real shell. Absolute paths (starting with `/`) are always resolved from root:

```rust
#[tokio::test]
async fn navigate_directories() {
    let backend = MemoryProvider::new();

    // Write a file at an absolute path.
    backend.write("/project/src/main.rs", b"fn main() {}").await.expect("write");

    // Check the initial working directory.
    let cwd = backend.pwd().await.expect("pwd");
    assert_eq!(cwd, "/");

    // Change into the project directory.
    backend.cd("/project").await.expect("cd");
    assert_eq!(backend.pwd().await.expect("pwd"), "/project");

    // Relative read now works from /project.
    let file = backend.read("src/main.rs").await.expect("read relative");
    assert_eq!(file.content, b"fn main() {}");

    // cd with a relative path.
    backend.cd("src").await.expect("cd src");
    assert_eq!(backend.pwd().await.expect("pwd"), "/project/src");
}
```

`cd` to a path that does not exist returns `VfsError::NotFound`. The working directory
is not changed on error.

---

## Step 5: Path traversal protection

Both backends block traversal attempts that would escape the root. In `MemoryProvider` the
root is always `/`; in `LocalProvider` it is the directory you passed to `new`:

```rust
#[tokio::test]
async fn path_traversal_is_rejected() {
    use synwire_core::vfs::error::VfsError;

    let backend = MemoryProvider::new();

    // Attempt to cd above root using "..".
    let err = backend.cd("/../etc").await.expect_err("should be blocked");

    // The error is PathTraversal, not NotFound.
    assert!(
        matches!(err, VfsError::PathTraversal { .. }),
        "expected PathTraversal, got: {err}"
    );

    // The working directory is unchanged.
    assert_eq!(backend.pwd().await.expect("pwd"), "/");
}
```

`VfsError::PathTraversal` carries two fields:

```rust
VfsError::PathTraversal {
    attempted: String,  // The normalised path that was attempted
    root: String,       // The root boundary that was violated
}
```

---

## Step 6: VfsError — the full error model

`VfsError` is `#[non_exhaustive]`. The variants you will encounter most often:

| Variant | Meaning |
|---|---|
| `NotFound(path)` | File or directory does not exist |
| `PermissionDenied(path)` | OS-level permission refused |
| `IsDirectory(path)` | Expected file but found directory |
| `PathTraversal { attempted, root }` | Path normalised outside the root boundary |
| `ScopeViolation { path, scope }` | Operation outside the configured allowed scope |
| `Unsupported(msg)` | Operation not implemented by this backend |
| `Io(io::Error)` | Underlying OS I/O error (filesystem backend only) |
| `Timeout(msg)` | Operation exceeded a time limit |
| `OperationDenied(msg)` | User or policy denied the operation |

Always handle `VfsError` with a match and a catch-all arm:

```rust
use synwire_core::vfs::error::VfsError;

match err {
    VfsError::NotFound(p) => eprintln!("missing: {p}"),
    VfsError::PathTraversal { attempted, root } => {
        eprintln!("traversal blocked: {attempted} outside {root}")
    }
    VfsError::PermissionDenied(p) => eprintln!("permission denied: {p}"),
    other => eprintln!("backend error: {other}"),
}
```

---

## Step 7: LocalProvider — real files on disk

`LocalProvider` operates on the real filesystem but confines all operations to the
`root` directory you pass to `new`. Attempting to access anything outside that root is
treated as a `PathTraversal` error.

```rust
use synwire_agent::vfs::filesystem::LocalProvider;
use synwire_core::vfs::protocol::Vfs;

#[tokio::test]
async fn filesystem_backend_write_and_read() {
    // Use a temporary directory so the test cleans up after itself.
    let dir = tempfile::tempdir().expect("tmpdir");
    let root = dir.path();

    // new() canonicalises root and verifies it exists.
    let backend = LocalProvider::new(root).expect("create backend");

    // Write a file. Parent directories are created automatically.
    backend
        .write("output/result.txt", b"42")
        .await
        .expect("write");

    // Read it back.
    let content = backend.read("output/result.txt").await.expect("read");
    assert_eq!(content.content, b"42");
}
```

To use `tempfile` in tests, add it to your `[dev-dependencies]`:

```toml
[dev-dependencies]
tempfile = "3"
```

---

## Step 8: Path traversal with LocalProvider

`LocalProvider` normalises paths without requiring them to exist (it avoids calling
`canonicalize` on non-existent paths). The boundary check happens after normalisation:

```rust
#[tokio::test]
async fn filesystem_backend_blocks_traversal() {
    use synwire_core::vfs::error::VfsError;

    let dir = tempfile::tempdir().expect("tmpdir");
    let backend = LocalProvider::new(dir.path()).expect("create");

    // Attempt to read a file above the workspace root.
    let err = backend
        .read("../../etc/passwd")
        .await
        .expect_err("traversal must be blocked");

    assert!(
        matches!(err, VfsError::PathTraversal { .. }),
        "expected PathTraversal, got: {err}"
    );
}
```

---

## Step 9: Grep — searching file content

`Vfs::grep` accepts a regex pattern and a `GrepOptions` struct. The options
mirror ripgrep's command-line flags:

```rust
use synwire_core::vfs::state_backend::MemoryProvider;
use synwire_core::vfs::protocol::Vfs;
use synwire_core::vfs::grep_options::{GrepOptions, GrepOutputMode};

#[tokio::test]
async fn grep_with_context() {
    let backend = MemoryProvider::new();

    // Seed the backend with some files.
    backend
        .write("/src/lib.rs", b"// lib\npub fn add(a: u32, b: u32) -> u32 {\n    a + b\n}\n")
        .await
        .expect("write");
    backend
        .write("/src/main.rs", b"// main\nfn main() {\n    println!(\"hello\");\n}\n")
        .await
        .expect("write");

    let opts = GrepOptions {
        case_insensitive: false,
        line_numbers: true,
        // Show one line of context before and after each match.
        after_context: 1,
        before_context: 1,
        // Restrict to .rs files.
        glob: Some("*.rs".to_string()),
        ..GrepOptions::default()
    };

    let matches = backend.grep("pub fn", opts).await.expect("grep");

    // "pub fn" appears only in lib.rs.
    assert_eq!(matches.len(), 1);

    let m = &matches[0];
    assert!(m.file.ends_with("lib.rs"));
    assert_eq!(m.line_number, 2);          // 1-indexed
    assert!(m.line_content.contains("pub fn add"));
    assert!(!m.before.is_empty());         // "// lib" is the before context
    assert!(!m.after.is_empty());          // "    a + b" is the after context
}
```

---

## Step 10: GrepOptions reference

| Field | Type | Default | Description |
|---|---|---|---|
| `path` | `Option<String>` | `None` (= cwd) | Restrict search to this path |
| `after_context` | `u32` | `0` | Lines to show after each match |
| `before_context` | `u32` | `0` | Lines to show before each match |
| `context` | `Option<u32>` | `None` | Symmetric context (overrides before/after) |
| `case_insensitive` | `bool` | `false` | Case-insensitive match |
| `glob` | `Option<String>` | `None` | File name glob filter (e.g. `"*.rs"`) |
| `file_type` | `Option<String>` | `None` | Ripgrep-style type filter (`"rust"`, `"python"`, ...) |
| `max_matches` | `Option<usize>` | `None` | Stop after this many matches |
| `output_mode` | `GrepOutputMode` | `Content` | One of `Content`, `FilesWithMatches`, `Count` |
| `line_numbers` | `bool` | `false` | Include line numbers in output |
| `invert` | `bool` | `false` | Show non-matching lines |
| `fixed_string` | `bool` | `false` | Treat pattern as literal string, not regex |
| `multiline` | `bool` | `false` | Allow pattern to span lines |

---

## Step 11: GrepOutputMode variants

`GrepOutputMode` controls the shape of the `GrepMatch` values returned:

```rust
use synwire_core::vfs::grep_options::{GrepOptions, GrepOutputMode};

// Content mode (default): full line content with context.
let content_opts = GrepOptions {
    output_mode: GrepOutputMode::Content,
    line_numbers: true,
    ..GrepOptions::default()
};

// FilesWithMatches: one entry per file that has at least one match.
// GrepMatch.line_content and context fields are empty.
let files_opts = GrepOptions {
    output_mode: GrepOutputMode::FilesWithMatches,
    ..GrepOptions::default()
};

// Count: one entry per file; GrepMatch.line_number holds the match count.
let count_opts = GrepOptions {
    output_mode: GrepOutputMode::Count,
    ..GrepOptions::default()
};
```

Count mode example:

```rust
#[tokio::test]
async fn grep_count_mode() {
    let backend = MemoryProvider::new();
    backend.write("/f.txt", b"foo\nfoo\nbar\nfoo").await.expect("write");

    let matches = backend
        .grep(
            "foo",
            GrepOptions {
                output_mode: GrepOutputMode::Count,
                ..GrepOptions::default()
            },
        )
        .await
        .expect("grep");

    // One GrepMatch per file; line_number holds the count.
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].line_number, 3);
}
```

---

## Step 12: GrepMatch fields

`GrepMatch` carries all information about a single match:

```rust
pub struct GrepMatch {
    /// File path where the match was found.
    pub file: String,
    /// Line number (1-indexed). 0 when line_numbers: false or in FilesWithMatches mode.
    pub line_number: usize,
    /// Column of the match start (0-indexed). 0 in invert or FilesWithMatches mode.
    pub column: usize,
    /// Full content of the matched line.
    pub line_content: String,
    /// Lines before the match (up to before_context).
    pub before: Vec<String>,
    /// Lines after the match (up to after_context).
    pub after: Vec<String>,
}
```

In `Count` mode, `line_number` is repurposed to hold the match count and `line_content`
holds the count as a string. All other fields are empty.

---

## Step 13: Case-insensitive and invert search

```rust
#[tokio::test]
async fn case_insensitive_and_invert() {
    let backend = MemoryProvider::new();
    backend
        .write("/log.txt", b"INFO: start\nERROR: fail\nINFO: end")
        .await
        .expect("write");

    // Case-insensitive: "error" matches "ERROR".
    let errors = backend
        .grep(
            "error",
            GrepOptions {
                case_insensitive: true,
                line_numbers: true,
                ..GrepOptions::default()
            },
        )
        .await
        .expect("grep");
    assert_eq!(errors.len(), 1);
    assert!(errors[0].line_content.contains("ERROR"));

    // Invert: show lines that do NOT match "ERROR".
    let non_errors = backend
        .grep(
            "ERROR",
            GrepOptions {
                invert: true,
                ..GrepOptions::default()
            },
        )
        .await
        .expect("grep");
    assert_eq!(non_errors.len(), 2);
    assert!(non_errors.iter().all(|m| !m.line_content.contains("ERROR")));
}
```

---

## What you have learned

- `Vfs` is the uniform interface for file operations across backends.
- `MemoryProvider` is fully in-memory — perfect for tests and agent scratchpads.
- `LocalProvider` is scoped to a root directory and enforces path traversal
  protection using normalised path comparison.
- Both backends reject `../../etc/passwd`-style traversal attempts with
  `VfsError::PathTraversal`.
- `grep` supports case insensitivity, context lines, file type/glob filters, output
  modes, invert matching, and match limits through `GrepOptions`.
- `GrepMatch` carries the file path, line number, column, matched content, and context lines.

---

## Next steps

- **Composite backends**: See `../how-to/vfs.md` for composing `MemoryProvider`,
  `LocalProvider`, and custom backends through the `CompositeProvider` pipeline.
- **Shell execution**: See `../how-to/shell.md` for running commands with
  `Shell` and reading `ExecuteResponse`.
- **Architecture**: See `../explanation/backend_protocol.md` for a deeper explanation of
  how backends integrate with the agent runner, middleware, and approval gates.
- **Previous tutorial**: `04-plugin-state-isolation.md` — composing plugins with
  type-safe state.
