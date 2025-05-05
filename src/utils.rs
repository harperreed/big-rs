// ABOUTME: Utility functions for the big-slides application
// ABOUTME: Provides various helper functions for validation, path handling, etc.

use crate::errors::{BigError, Result};
use log::warn;
use std::path::{Path, PathBuf};

/// Validate that a file exists
pub fn validate_file_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(BigError::PathNotFoundError(path.to_path_buf()));
    }
    if !path.is_file() {
        return Err(BigError::ValidationError(format!(
            "Path is not a file: {:?}",
            path
        )));
    }
    Ok(())
}

/// Validate that a directory exists
pub fn validate_directory_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(BigError::PathNotFoundError(path.to_path_buf()));
    }
    if !path.is_dir() {
        return Err(BigError::ValidationError(format!(
            "Path is not a directory: {:?}",
            path
        )));
    }
    Ok(())
}

/// Ensure a directory exists, creating it if necessary
pub fn ensure_directory_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path).map_err(BigError::FileReadError)?;
    } else if !path.is_dir() {
        return Err(BigError::ValidationError(format!(
            "Path exists but is not a directory: {:?}",
            path
        )));
    }
    Ok(())
}

/// Ensure a file's parent directory exists
pub fn ensure_parent_directory_exists(file_path: &Path) -> Result<()> {
    if let Some(parent) = file_path.parent() {
        ensure_directory_exists(parent)?;
    }
    Ok(())
}

/// Validate write permissions for a directory
pub fn validate_directory_writable(path: &Path) -> Result<()> {
    // First ensure it exists
    ensure_directory_exists(path)?;

    // Try to create a temporary file to test write permissions
    let test_file = path.join(format!("test_write_{}.tmp", uuid::Uuid::new_v4()));
    match std::fs::File::create(&test_file) {
        Ok(_) => {
            // Clean up the test file
            if let Err(e) = std::fs::remove_file(&test_file) {
                warn!("Failed to clean up test file {:?}: {}", test_file, e);
            }
            Ok(())
        }
        Err(e) => Err(BigError::ValidationError(format!(
            "Directory is not writable: {:?} - {}",
            path, e
        ))),
    }
}

/// Get the absolute path
pub fn get_absolute_path(path: &Path) -> Result<PathBuf> {
    std::fs::canonicalize(path).map_err(|e| {
        BigError::ValidationError(format!("Failed to get absolute path for {:?}: {}", path, e))
    })
}
