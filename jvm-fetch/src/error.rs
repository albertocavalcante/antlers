//! Error types for the jvm-fetch crate.

use thiserror::Error;

/// A specialized Result type for jvm-fetch operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur when fetching artifacts.
#[derive(Error, Debug)]
pub enum Error {
    /// Artifact was not found in any repository.
    #[error("artifact not found: {artifact} (searched: {repositories})")]
    NotFound {
        /// The artifact that was not found.
        artifact: String,
        /// The repositories that were searched.
        repositories: String,
    },

    /// Network error during fetch.
    #[error("network error fetching {context}: {source}")]
    Network {
        /// Description of what was being fetched.
        context: String,
        /// The underlying network error.
        #[source]
        source: reqwest::Error,
    },

    /// Checksum verification failed.
    #[error("checksum mismatch for {artifact}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        /// The artifact being verified.
        artifact: String,
        /// The expected checksum.
        expected: String,
        /// The actual checksum.
        actual: String,
    },

    /// Invalid URL.
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Artifact parsing error.
    #[error("artifact error: {0}")]
    Artifact(#[from] jvm_artifact::Error),
}
