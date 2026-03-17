//! `Vfs` conformance test harness.
//!
//! Call [`run_vfs_conformance`] from any `#[tokio::test]` to validate that
//! a `Vfs` implementation satisfies the trait contract.

#![allow(clippy::expect_used, clippy::panic, clippy::unnested_or_patterns)]

use synwire_core::vfs::protocol::Vfs;
use synwire_core::vfs::types::{LsOptions, RmOptions};

/// Run the full `Vfs` conformance suite against `backend`.
///
/// # Panics
/// Panics with a descriptive message if any assertion fails.
pub async fn run_vfs_conformance(backend: &(impl Vfs + ?Sized)) {
    test_pwd(backend).await;
    test_write_read(backend).await;
    test_ls(backend).await;
    test_rm(backend).await;
    test_edit(backend).await;
    test_glob(backend).await;
    test_grep(backend).await;
    test_path_traversal(backend).await;
}

async fn test_pwd(backend: &(impl Vfs + ?Sized)) {
    let pwd = backend.pwd().await.expect("pwd should succeed");
    assert!(!pwd.is_empty(), "pwd must return a non-empty path");
}

async fn test_write_read(backend: &(impl Vfs + ?Sized)) {
    let _ = backend
        .write("conformance_test.txt", b"hello conformance")
        .await
        .expect("write should succeed");

    let content = backend
        .read("conformance_test.txt")
        .await
        .expect("read should succeed");

    assert_eq!(
        content.content, b"hello conformance",
        "read should return exactly what was written"
    );
}

async fn test_ls(backend: &(impl Vfs + ?Sized)) {
    let _ = backend
        .write("ls_test/file.txt", b"data")
        .await
        .expect("write for ls test");

    let entries = backend
        .ls("ls_test", LsOptions::default())
        .await
        .expect("ls should succeed");
    assert!(
        entries.iter().any(|e| e.name == "file.txt"),
        "ls should list the written file"
    );
}

async fn test_rm(backend: &(impl Vfs + ?Sized)) {
    let _ = backend
        .write("rm_test.txt", b"delete me")
        .await
        .expect("write for rm test");

    backend
        .rm("rm_test.txt", RmOptions::default())
        .await
        .expect("rm should succeed");

    let result = backend.read("rm_test.txt").await;
    assert!(result.is_err(), "reading a deleted file should fail");
}

async fn test_edit(backend: &(impl Vfs + ?Sized)) {
    let _ = backend
        .write("edit_test.txt", b"old content")
        .await
        .expect("write for edit test");

    let _ = backend
        .edit("edit_test.txt", "old content", "new content")
        .await
        .expect("edit should succeed");

    let content = backend
        .read("edit_test.txt")
        .await
        .expect("read after edit");

    assert!(
        content.content.windows(11).any(|w| w == b"new content"),
        "edited content should contain new text"
    );
}

async fn test_glob(backend: &(impl Vfs + ?Sized)) {
    if !backend
        .capabilities()
        .contains(synwire_core::vfs::types::VfsCapabilities::GLOB)
    {
        return;
    }
    let _ = backend
        .write("glob_a.rs", b"fn a() {}")
        .await
        .expect("write for glob test");
    let _ = backend
        .write("glob_b.rs", b"fn b() {}")
        .await
        .expect("write for glob test");

    let matches = backend.glob("*.rs").await.expect("glob should succeed");
    assert!(
        matches.iter().any(|e| e.path.ends_with("glob_a.rs")),
        "glob *.rs should match glob_a.rs"
    );
}

async fn test_grep(backend: &(impl Vfs + ?Sized)) {
    use synwire_core::vfs::grep_options::GrepOptions;
    if !backend
        .capabilities()
        .contains(synwire_core::vfs::types::VfsCapabilities::GREP)
    {
        return;
    }
    let _ = backend
        .write("grep_test.txt", b"hello world\nfoo bar\nhello again")
        .await
        .expect("write for grep test");

    let opts = GrepOptions {
        case_insensitive: false,
        line_numbers: true,
        ..GrepOptions::default()
    };
    let matches = backend
        .grep("hello", opts)
        .await
        .expect("grep should succeed");
    assert!(
        matches.len() >= 2,
        "grep 'hello' should find at least 2 matches"
    );
}

async fn test_path_traversal(backend: &(impl Vfs + ?Sized)) {
    let result = backend.read("../escape.txt").await;
    match result {
        Err(_) => {} // PathTraversal, PermissionDenied, and other errors are all acceptable.
        Ok(_) => panic!("path traversal above root must not succeed"),
    }
}
