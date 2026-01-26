//! Error types for the antler-lock crate.

use thiserror::Error;

/// A specialized Result type for antler-lock operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during lockfile operations.
#[derive(Error, Debug)]
pub enum Error {
    /// Failed to parse lockfile JSON.
    ///
    /// This usually means the file is not valid JSON or has structural issues.
    #[error("failed to parse lockfile: {0}")]
    Parse(#[from] serde_json::Error),

    /// Failed to read/write lockfile from disk.
    #[error("failed to read lockfile: {0}")]
    Io(#[from] std::io::Error),

    /// Unknown or unsupported lockfile format.
    ///
    /// The lockfile was valid JSON but doesn't match any known format
    /// (antler-lock, `rules_jvm_external` V1, or V2).
    #[error("unknown lockfile format: {0}")]
    UnknownFormat(String),

    /// Invalid lockfile version.
    ///
    /// The format was recognized but the version is not supported.
    #[error("unsupported lockfile version: {0}")]
    UnsupportedVersion(String),

    /// Missing required field in the lockfile.
    ///
    /// For example, an artifact missing its SHA-256 checksum.
    #[error("missing required field: {0}")]
    MissingField(String),

    /// Invalid artifact coordinate format.
    ///
    /// Coordinates should be `group:artifact` or `group:artifact:version`.
    #[error("invalid artifact coordinate: {0}")]
    InvalidCoordinate(String),

    /// Invalid checksum format or algorithm.
    #[error("invalid checksum: {0}")]
    InvalidChecksum(String),
}

impl Error {
    /// Returns `true` if this error is due to a missing file.
    #[must_use]
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::Io(e) if e.kind() == std::io::ErrorKind::NotFound)
    }

    /// Returns `true` if this error is due to invalid JSON.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // matches! macro prevents const
    pub fn is_parse_error(&self) -> bool {
        matches!(self, Self::Parse(_))
    }

    /// Returns `true` if this error is due to an unknown format.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // matches! macro prevents const
    pub fn is_unknown_format(&self) -> bool {
        matches!(self, Self::UnknownFormat(_))
    }
}
