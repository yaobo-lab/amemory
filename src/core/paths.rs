//! Filesystem paths for Amemory runtime, config, data, and assets.

use std::path::{Path, PathBuf};

/// Get the project root directory.
#[must_use]
pub fn project_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Get the assets directory path.
#[must_use]
pub fn assets_dir() -> PathBuf {
    project_root().join("assets")
}

/// Resolve a path relative to the project root unless it is already absolute.
#[must_use]
pub fn resolve_relative(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root().join(path)
    }
}
