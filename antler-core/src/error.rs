//! Error types for antler-core.

use thiserror::Error;

/// Result type alias using antler's Error type.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during dependency resolution.
#[derive(Error, Debug)]
pub enum Error {
    /// Failed to parse artifact coordinates.
    #[error("invalid artifact coordinates: {0}")]
    InvalidCoordinates(String),

    /// Failed to parse a POM file.
    #[error("failed to parse POM for {artifact}: {details}")]
    PomParse {
        /// The artifact whose POM failed to parse.
        artifact: String,
        /// Details about the parse failure.
        details: String,
    },

    /// Failed to parse Gradle Module Metadata.
    #[error("failed to parse Gradle Module Metadata: {0}")]
    GmmParse(String),

    /// Artifact not found in any repository.
    #[error("artifact not found in any repository: {artifact} (searched: {repositories})")]
    NotFound {
        /// The artifact that was not found.
        artifact: String,
        /// Comma-separated list of repositories that were searched.
        repositories: String,
    },

    /// Network error during fetch.
    #[error("network error fetching {context}: {source}")]
    Network {
        /// What was being fetched when the error occurred.
        context: String,
        /// The underlying network error.
        #[source]
        source: reqwest::Error,
    },

    /// Checksum mismatch.
    #[error("checksum mismatch for {artifact}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        /// The artifact with mismatched checksum.
        artifact: String,
        /// The expected checksum value.
        expected: String,
        /// The actual checksum value.
        actual: String,
    },

    /// Version resolution conflict.
    #[error("version conflict for {artifact}: {details}")]
    VersionConflict {
        /// The artifact with version conflict.
        artifact: String,
        /// Details about the conflict.
        details: String,
    },

    /// Circular dependency detected.
    #[error("circular dependency detected: {0}")]
    CircularDependency(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// XML parsing error.
    #[error("XML parsing error: {0}")]
    Xml(#[from] quick_xml::Error),

    /// JSON parsing error.
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    /// Invalid version string.
    #[error("invalid version: {0}")]
    InvalidVersion(String),

    /// Invalid URL.
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// Parent POM resolution failed.
    #[error("failed to resolve parent POM for {artifact}: {details}")]
    ParentResolution {
        /// The artifact whose parent POM could not be resolved.
        artifact: String,
        /// Details about the resolution failure.
        details: String,
    },
}
