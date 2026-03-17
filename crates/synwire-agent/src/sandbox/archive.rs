//! Archive backend for tar/gzip/zip operations.

use std::io::{Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use synwire_core::BoxFuture;
use synwire_core::vfs::error::VfsError;
use synwire_core::vfs::types::ArchiveInfo;

/// Conflict resolution policy for extraction.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ConflictPolicy {
    /// Skip existing files.
    Skip,
    /// Overwrite existing files.
    Overwrite,
    /// Fail with an error.
    Fail,
}

/// Archive format.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ArchiveFormat {
    /// Tar with gzip compression.
    TarGz,
    /// Tar with bzip2 compression.
    TarBz2,
    /// Tar without compression.
    Tar,
    /// Zip archive.
    Zip,
}

impl ArchiveFormat {
    /// Detect format from file extension.
    pub fn from_path(path: &str) -> Option<Self> {
        let p = std::path::Path::new(path);
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        let ext = ext.as_deref().unwrap_or("");
        if path.ends_with(".tar.gz") || ext == "tgz" {
            Some(Self::TarGz)
        } else if path.ends_with(".tar.bz2") || ext == "tbz2" {
            Some(Self::TarBz2)
        } else if ext == "tar" {
            Some(Self::Tar)
        } else if ext == "zip" {
            Some(Self::Zip)
        } else {
            None
        }
    }
}

/// Archive backend for creating, extracting, and listing archives.
#[derive(Debug, Default, Clone)]
pub struct ArchiveManager;

impl ArchiveManager {
    /// Create a new archive backend.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Create an archive from a directory.
    pub fn create_archive<'a>(
        &'a self,
        source_dir: &'a str,
        output_path: &'a str,
        format: ArchiveFormat,
    ) -> BoxFuture<'a, Result<ArchiveInfo, VfsError>> {
        Box::pin(async move {
            let source = Path::new(source_dir);
            if !source.exists() {
                return Err(VfsError::NotFound(source_dir.to_string()));
            }

            match format {
                ArchiveFormat::TarGz => create_tar_gz(source, output_path).await,
                ArchiveFormat::Tar => create_tar(source, output_path).await,
                ArchiveFormat::Zip => create_zip(source, output_path).await,
                ArchiveFormat::TarBz2 => create_tar_bz2(source, output_path).await,
            }
        })
    }

    /// Extract an archive to a directory.
    pub fn extract_archive<'a>(
        &'a self,
        archive_path: &'a str,
        dest_dir: &'a str,
        policy: ConflictPolicy,
    ) -> BoxFuture<'a, Result<(), VfsError>> {
        Box::pin(async move {
            let format = ArchiveFormat::from_path(archive_path).ok_or_else(|| {
                VfsError::Unsupported(format!("unknown archive format: {archive_path}"))
            })?;
            let dest = Path::new(dest_dir);
            tokio::fs::create_dir_all(dest)
                .await
                .map_err(VfsError::Io)?;

            match format {
                ArchiveFormat::TarGz | ArchiveFormat::Tar | ArchiveFormat::TarBz2 => {
                    extract_tar(archive_path, dest, format, policy).await
                }
                ArchiveFormat::Zip => extract_zip(archive_path, dest, policy).await,
            }
        })
    }

    /// List archive contents.
    pub fn list_contents<'a>(
        &'a self,
        archive_path: &'a str,
    ) -> BoxFuture<'a, Result<ArchiveInfo, VfsError>> {
        Box::pin(async move {
            let format = ArchiveFormat::from_path(archive_path).ok_or_else(|| {
                VfsError::Unsupported(format!("unknown archive format: {archive_path}"))
            })?;
            match format {
                ArchiveFormat::TarGz | ArchiveFormat::Tar | ArchiveFormat::TarBz2 => {
                    list_tar(archive_path, format).await
                }
                ArchiveFormat::Zip => list_zip(archive_path).await,
            }
        })
    }
}

// ── tar helpers ─────────────────────────────────────────────────────────────

async fn create_tar_gz(source: &Path, output_path: &str) -> Result<ArchiveInfo, VfsError> {
    let output = output_path.to_string();
    let source = source.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::create(&output).map_err(VfsError::Io)?;
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tar = tar::Builder::new(encoder);
        tar.append_dir_all(".", &source).map_err(VfsError::Io)?;
        tar.finish().map_err(VfsError::Io)?;
        let compressed_size = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
        Ok(ArchiveInfo {
            entries: Vec::new(),
            format: "tar.gz".to_string(),
            compressed_size,
        })
    })
    .await
    .map_err(|e| VfsError::Unsupported(e.to_string()))?
}

async fn create_tar_bz2(source: &Path, output_path: &str) -> Result<ArchiveInfo, VfsError> {
    let output = output_path.to_string();
    let source = source.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::create(&output).map_err(VfsError::Io)?;
        let encoder = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
        let mut tar = tar::Builder::new(encoder);
        tar.append_dir_all(".", &source).map_err(VfsError::Io)?;
        let encoder = tar.into_inner().map_err(VfsError::Io)?;
        let _ = encoder.finish().map_err(VfsError::Io)?;
        let compressed_size = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
        Ok(ArchiveInfo {
            entries: Vec::new(),
            format: "tar.bz2".to_string(),
            compressed_size,
        })
    })
    .await
    .map_err(|e| VfsError::Unsupported(e.to_string()))?
}

async fn create_tar(source: &Path, output_path: &str) -> Result<ArchiveInfo, VfsError> {
    let output = output_path.to_string();
    let source = source.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::create(&output).map_err(VfsError::Io)?;
        let mut tar = tar::Builder::new(file);
        tar.append_dir_all(".", &source).map_err(VfsError::Io)?;
        tar.finish().map_err(VfsError::Io)?;
        let compressed_size = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
        Ok(ArchiveInfo {
            entries: Vec::new(),
            format: "tar".to_string(),
            compressed_size,
        })
    })
    .await
    .map_err(|e| VfsError::Unsupported(e.to_string()))?
}

async fn extract_tar(
    archive_path: &str,
    dest: &Path,
    format: ArchiveFormat,
    _policy: ConflictPolicy,
) -> Result<(), VfsError> {
    let archive = archive_path.to_string();
    let dest = dest.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&archive).map_err(VfsError::Io)?;
        match format {
            ArchiveFormat::TarGz => {
                let decoder = flate2::read::GzDecoder::new(file);
                let mut tar = tar::Archive::new(decoder);
                tar.unpack(&dest).map_err(VfsError::Io)?;
            }
            ArchiveFormat::TarBz2 => {
                let decoder = bzip2::read::BzDecoder::new(file);
                let mut tar = tar::Archive::new(decoder);
                tar.unpack(&dest).map_err(VfsError::Io)?;
            }
            ArchiveFormat::Tar => {
                let mut tar = tar::Archive::new(file);
                tar.unpack(&dest).map_err(VfsError::Io)?;
            }
            ArchiveFormat::Zip => {
                return Err(VfsError::Unsupported(
                    "zip extraction handled separately".into(),
                ));
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| VfsError::Unsupported(e.to_string()))?
}

async fn list_tar(archive_path: &str, format: ArchiveFormat) -> Result<ArchiveInfo, VfsError> {
    let archive = archive_path.to_string();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&archive).map_err(VfsError::Io)?;
        let mut entries_out = Vec::new();
        match format {
            ArchiveFormat::TarGz => {
                let decoder = flate2::read::GzDecoder::new(file);
                let mut tar = tar::Archive::new(decoder);
                for e in tar.entries().map_err(VfsError::Io)? {
                    let e = e.map_err(VfsError::Io)?;
                    entries_out.push(synwire_core::vfs::types::ArchiveEntry {
                        path: e.path().map_err(VfsError::Io)?.display().to_string(),
                        is_dir: e.header().entry_type().is_dir(),
                        size: e.header().size().unwrap_or(0),
                    });
                }
            }
            ArchiveFormat::TarBz2 => {
                let decoder = bzip2::read::BzDecoder::new(file);
                let mut tar = tar::Archive::new(decoder);
                for e in tar.entries().map_err(VfsError::Io)? {
                    let e = e.map_err(VfsError::Io)?;
                    entries_out.push(synwire_core::vfs::types::ArchiveEntry {
                        path: e.path().map_err(VfsError::Io)?.display().to_string(),
                        is_dir: e.header().entry_type().is_dir(),
                        size: e.header().size().unwrap_or(0),
                    });
                }
            }
            ArchiveFormat::Tar => {
                let mut tar = tar::Archive::new(file);
                for e in tar.entries().map_err(VfsError::Io)? {
                    let e = e.map_err(VfsError::Io)?;
                    entries_out.push(synwire_core::vfs::types::ArchiveEntry {
                        path: e.path().map_err(VfsError::Io)?.display().to_string(),
                        is_dir: e.header().entry_type().is_dir(),
                        size: e.header().size().unwrap_or(0),
                    });
                }
            }
            ArchiveFormat::Zip => {
                return Err(VfsError::Unsupported(
                    "zip listing handled separately".into(),
                ));
            }
        }
        let compressed_size = std::fs::metadata(&archive).map(|m| m.len()).unwrap_or(0);
        Ok(ArchiveInfo {
            entries: entries_out,
            format: "tar".to_string(),
            compressed_size,
        })
    })
    .await
    .map_err(|e| VfsError::Unsupported(e.to_string()))?
}

// ── zip helpers ──────────────────────────────────────────────────────────────

async fn create_zip(source: &Path, output_path: &str) -> Result<ArchiveInfo, VfsError> {
    let output = output_path.to_string();
    let source = source.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::create(&output).map_err(VfsError::Io)?;
        let mut zip = zip::ZipWriter::new(file);
        let opts: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
        write_dir_to_zip(&mut zip, &source, &source, opts)?;
        let _ = zip
            .finish()
            .map_err(|e| VfsError::Unsupported(e.to_string()))?;
        let compressed_size = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
        Ok(ArchiveInfo {
            entries: Vec::new(),
            format: "zip".to_string(),
            compressed_size,
        })
    })
    .await
    .map_err(|e| VfsError::Unsupported(e.to_string()))?
}

fn write_dir_to_zip(
    zip: &mut zip::ZipWriter<std::fs::File>,
    base: &Path,
    dir: &Path,
    opts: zip::write::FileOptions<'_, ()>,
) -> Result<(), VfsError> {
    for entry in std::fs::read_dir(dir).map_err(VfsError::Io)? {
        let entry = entry.map_err(VfsError::Io)?;
        let path = entry.path();
        let name = path
            .strip_prefix(base)
            .map_err(|e| VfsError::Unsupported(e.to_string()))?
            .display()
            .to_string();
        if path.is_dir() {
            zip.add_directory(&name, opts)
                .map_err(|e| VfsError::Unsupported(e.to_string()))?;
            write_dir_to_zip(zip, base, &path, opts)?;
        } else {
            zip.start_file(&name, opts)
                .map_err(|e| VfsError::Unsupported(e.to_string()))?;
            let mut f = std::fs::File::open(&path).map_err(VfsError::Io)?;
            let mut buf = Vec::new();
            let _ = f.read_to_end(&mut buf).map_err(VfsError::Io)?;
            zip.write_all(&buf)
                .map_err(|e| VfsError::Unsupported(e.to_string()))?;
        }
    }
    Ok(())
}

async fn extract_zip(
    archive_path: &str,
    dest: &Path,
    policy: ConflictPolicy,
) -> Result<(), VfsError> {
    let archive = archive_path.to_string();
    let dest = dest.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&archive).map_err(VfsError::Io)?;
        let mut zip =
            zip::ZipArchive::new(file).map_err(|e| VfsError::Unsupported(e.to_string()))?;
        for i in 0..zip.len() {
            let mut entry = zip
                .by_index(i)
                .map_err(|e| VfsError::Unsupported(e.to_string()))?;
            let out_path = dest.join(
                entry
                    .enclosed_name()
                    .ok_or_else(|| VfsError::Unsupported("circular symlink".into()))?,
            );
            if entry.is_dir() {
                std::fs::create_dir_all(&out_path).map_err(VfsError::Io)?;
            } else {
                if out_path.exists() {
                    match policy {
                        ConflictPolicy::Skip => continue,
                        ConflictPolicy::Fail => {
                            return Err(VfsError::Unsupported(format!(
                                "conflict: {} already exists",
                                out_path.display()
                            )));
                        }
                        ConflictPolicy::Overwrite => {}
                    }
                }
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent).map_err(VfsError::Io)?;
                }
                let mut out_file = std::fs::File::create(&out_path).map_err(VfsError::Io)?;
                let _ = std::io::copy(&mut entry, &mut out_file).map_err(VfsError::Io)?;
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| VfsError::Unsupported(e.to_string()))?
}

async fn list_zip(archive_path: &str) -> Result<ArchiveInfo, VfsError> {
    let archive = archive_path.to_string();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&archive).map_err(VfsError::Io)?;
        let mut zip =
            zip::ZipArchive::new(file).map_err(|e| VfsError::Unsupported(e.to_string()))?;
        let mut entries = Vec::new();
        for i in 0..zip.len() {
            let entry = zip
                .by_index(i)
                .map_err(|e| VfsError::Unsupported(e.to_string()))?;
            entries.push(synwire_core::vfs::types::ArchiveEntry {
                path: entry.name().to_string(),
                is_dir: entry.is_dir(),
                size: entry.size(),
            });
        }
        let compressed_size = std::fs::metadata(&archive).map(|m| m.len()).unwrap_or(0);
        Ok(ArchiveInfo {
            entries,
            format: "zip".to_string(),
            compressed_size,
        })
    })
    .await
    .map_err(|e| VfsError::Unsupported(e.to_string()))?
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tar_gz_round_trip() {
        let tmp = tempdir();
        let src = tmp.join("src");
        std::fs::create_dir_all(&src).expect("mkdir");
        std::fs::write(src.join("hello.txt"), b"hello").expect("write");

        let archive_path = tmp.join("out.tar.gz").display().to_string();
        let backend = ArchiveManager::new();
        let _ = backend
            .create_archive(
                &src.display().to_string(),
                &archive_path,
                ArchiveFormat::TarGz,
            )
            .await
            .expect("create");

        let dst = tmp.join("dst");
        std::fs::create_dir_all(&dst).expect("mkdir dst");
        backend
            .extract_archive(
                &archive_path,
                &dst.display().to_string(),
                ConflictPolicy::Overwrite,
            )
            .await
            .expect("extract");

        assert!(dst.join("hello.txt").exists());
    }

    #[tokio::test]
    async fn test_tar_bz2_round_trip() {
        let tmp = tempdir();
        let src = tmp.join("src");
        std::fs::create_dir_all(&src).expect("mkdir");
        std::fs::write(src.join("hello.txt"), b"hello bz2").expect("write");

        let archive_path = tmp.join("out.tar.bz2").display().to_string();
        let backend = ArchiveManager::new();
        let info = backend
            .create_archive(
                &src.display().to_string(),
                &archive_path,
                ArchiveFormat::TarBz2,
            )
            .await
            .expect("create");
        assert_eq!(info.format, "tar.bz2");
        assert!(info.compressed_size > 0);

        let listing = backend.list_contents(&archive_path).await.expect("list");
        assert!(!listing.entries.is_empty());

        let dst = tmp.join("dst");
        std::fs::create_dir_all(&dst).expect("mkdir dst");
        backend
            .extract_archive(
                &archive_path,
                &dst.display().to_string(),
                ConflictPolicy::Overwrite,
            )
            .await
            .expect("extract");

        assert!(dst.join("hello.txt").exists());
        assert_eq!(
            std::fs::read_to_string(dst.join("hello.txt")).expect("read"),
            "hello bz2"
        );
    }

    #[tokio::test]
    async fn test_zip_round_trip() {
        let tmp = tempdir();
        let src = tmp.join("src");
        std::fs::create_dir_all(&src).expect("mkdir");
        std::fs::write(src.join("data.txt"), b"data").expect("write");

        let archive_path = tmp.join("out.zip").display().to_string();
        let backend = ArchiveManager::new();
        let _ = backend
            .create_archive(
                &src.display().to_string(),
                &archive_path,
                ArchiveFormat::Zip,
            )
            .await
            .expect("create");

        let dst = tmp.join("dst");
        std::fs::create_dir_all(&dst).expect("mkdir dst");
        backend
            .extract_archive(
                &archive_path,
                &dst.display().to_string(),
                ConflictPolicy::Overwrite,
            )
            .await
            .expect("extract");

        assert!(dst.join("data.txt").exists());
    }

    #[tokio::test]
    async fn test_conflict_policy_fail() {
        let tmp = tempdir();
        let src = tmp.join("src");
        std::fs::create_dir_all(&src).expect("mkdir");
        std::fs::write(src.join("f.txt"), b"original").expect("write");

        let archive_path = tmp.join("out.zip").display().to_string();
        let backend = ArchiveManager::new();
        let _ = backend
            .create_archive(
                &src.display().to_string(),
                &archive_path,
                ArchiveFormat::Zip,
            )
            .await
            .expect("create");

        let dst = tmp.join("dst");
        std::fs::create_dir_all(&dst).expect("mkdir");
        std::fs::write(dst.join("f.txt"), b"existing").expect("prewrite");

        let err = backend
            .extract_archive(
                &archive_path,
                &dst.display().to_string(),
                ConflictPolicy::Fail,
            )
            .await;
        assert!(err.is_err());
    }

    fn tempdir() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("synwire-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).expect("tempdir");
        path
    }
}
