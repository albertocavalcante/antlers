//! Error types for gradle-module-metadata parsing.

use thiserror::Error;

/// A specialized Result type for Gradle module metadata operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur when parsing Gradle Module Metadata files.
#[derive(Error, Debug)]
pub enum Error {
    /// Failed to parse Gradle module metadata.
    #[error("failed to parse Gradle module metadata: {0}")]
    Parse(String),

    /// JSON parsing error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Unsupported format version.
    #[error("unsupported format version: {0}")]
    UnsupportedVersion(String),
}
