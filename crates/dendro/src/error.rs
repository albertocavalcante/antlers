//! Error types for the jvm-resolver crate.

use thiserror::Error;

/// A specialized Result type for jvm-resolver operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during dependency resolution.
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

    /// A version conflict was detected and the strict strategy was used.
    #[error("version conflict for {artifact}: {details}")]
    VersionConflict {
        /// The artifact with conflicting versions.
        artifact: String,
        /// Details about the conflict.
        details: String,
    },

    /// A circular dependency was detected.
    #[error("circular dependency detected: {0}")]
    CircularDependency(String),

    /// The maximum resolution depth was exceeded.
    #[error("max resolution depth exceeded: {0}")]
    MaxDepthExceeded(usize),

    /// A general resolution error.
    #[error("resolution error: {0}")]
    Resolution(String),

    /// An error from the project fetcher.
    #[error("fetch error: {0}")]
    Fetch(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl Error {
    /// Creates a new fetch error from any error type.
    pub fn fetch<E: std::error::Error + Send + Sync + 'static>(err: E) -> Self {
        Self::Fetch(Box::new(err))
    }
}
