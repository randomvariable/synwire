//! Content hash registry for skipping unchanged files during re-indexing.
//!
//! Stores a mapping of `file_path → xxh128 hash` in `hashes.json` alongside the
//! index metadata.  Before chunking and embedding a file, the pipeline computes
//! the xxh128 of its content and compares against the stored hash.  Files whose
//! content has not changed are skipped.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// On-disk format for the hash registry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HashRegistry {
    /// Map from canonical file path to hex-encoded xxh128 hash.
    pub files: HashMap<String, String>,
}

/// Compute the xxh128 hash of `data` and return it as a hex string.
pub fn xxh128_hex(data: &[u8]) -> String {
    let hash = xxhash_rust::xxh3::xxh3_128(data);
    format!("{hash:032x}")
}

/// Read the hash registry from `<cache_dir>/hashes.json`.
///
/// Returns an empty registry if the file is absent or corrupt.
pub fn read_hashes(cache_dir: &Path) -> HashRegistry {
    let path = cache_dir.join("hashes.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

/// Write the hash registry to `<cache_dir>/hashes.json`.
///
/// # Errors
///
/// Returns an I/O error if the directory cannot be created or the file written.
pub fn write_hashes(cache_dir: &Path, registry: &HashRegistry) -> std::io::Result<()> {
    std::fs::create_dir_all(cache_dir)?;
    let json = serde_json::to_string_pretty(registry).map_err(std::io::Error::other)?;
    std::fs::write(cache_dir.join("hashes.json"), json)
}
