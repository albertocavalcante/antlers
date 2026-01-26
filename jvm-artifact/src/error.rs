//! Error types for the jvm-artifact crate.

use thiserror::Error;

/// A specialized Result type for jvm-artifact operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur when working with JVM artifacts.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Invalid artifact coordinates format.
    #[error("invalid artifact coordinates: {0}")]
    InvalidCoordinates(String),

    /// Invalid version string.
    #[error("invalid version: {0}")]
    InvalidVersion(String),

    /// Invalid version constraint (range) format.
    #[error("invalid version constraint: {0}")]
    InvalidConstraint(String),
}
