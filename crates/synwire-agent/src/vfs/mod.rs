//! VFS provider implementations.
//!
//! Each provider exposes a different data source through the [`Vfs`] trait,
//! allowing LLMs to interact with it using filesystem-like operations.

pub mod clone;
pub mod composite;
pub mod local;
pub mod store;

pub use composite::{CompositeProvider, Mount};
pub use local::LocalProvider;
pub use store::StoreProvider;

use synwire_core::BoxFuture;
use synwire_core::vfs::types::{CpOptions, LsOptions, RmOptions};
use synwire_core::vfs::{Vfs, VfsError};

/// Translate a bash-style command string into a VFS operation.
///
/// Supported commands: `ls`, `cat`, `rm`, `cp`, `mv`, `pwd`, `cd`.
/// Unrecognised commands return [`VfsError::Unsupported`].
pub fn bash_command<'a>(
    vfs: &'a dyn Vfs,
    cmd: &'a str,
    args: &'a [&'a str],
) -> BoxFuture<'a, Result<String, VfsError>> {
    Box::pin(async move {
        match cmd {
            "ls" => {
                let path = args.first().copied().unwrap_or(".");
                let entries = vfs.ls(path, LsOptions::default()).await?;
                let names: Vec<String> = entries.iter().map(|e| e.name.clone()).collect();
                Ok(names.join("\n"))
            }
            "cat" => {
                let path = args
                    .first()
                    .ok_or_else(|| VfsError::Unsupported("cat requires a path argument".into()))?;
                let content = vfs.read(path).await?;
                String::from_utf8(content.content)
                    .map_err(|_| VfsError::Unsupported("binary file".into()))
            }
            "rm" => {
                let path = args
                    .first()
                    .ok_or_else(|| VfsError::Unsupported("rm requires a path argument".into()))?;
                vfs.rm(path, RmOptions::default()).await?;
                Ok(String::new())
            }
            "cp" => {
                if args.len() < 2 {
                    return Err(VfsError::Unsupported("cp requires src and dst".into()));
                }
                let result = vfs.cp(args[0], args[1], CpOptions::default()).await?;
                Ok(format!(
                    "copied {} bytes to {}",
                    result.bytes_transferred, result.path
                ))
            }
            "mv" => {
                if args.len() < 2 {
                    return Err(VfsError::Unsupported("mv requires src and dst".into()));
                }
                let result = vfs.mv_file(args[0], args[1]).await?;
                Ok(format!("moved to {}", result.path))
            }
            "pwd" => vfs.pwd().await,
            "cd" => {
                let path = args
                    .first()
                    .ok_or_else(|| VfsError::Unsupported("cd requires a path argument".into()))?;
                vfs.cd(path).await?;
                Ok(String::new())
            }
            other => Err(VfsError::Unsupported(format!("unknown command: {other}"))),
        }
    })
}
