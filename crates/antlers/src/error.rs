//! Unified error types for the antler crate.
//!
//! This module provides a unified [`Error`] type that wraps errors from all
//! the lower-level crates (jvm-artifact, maven-pom, gradle-module-metadata,
//! jvm-resolver, jvm-fetch).

use thiserror::Error;

/// A specialized Result type for antler operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Unified error type for all antler operations.
///
/// This enum wraps errors from all the lower-level crates to provide a
/// single error type for the high-level API.
#[derive(Error, Debug)]
pub enum Error {
    /// An error from the jvm-artifact crate (coordinate/version parsing).
    #[error("artifact error: {0}")]
    Artifact(#[from] gav::Error),

    /// An error from the maven-pom crate (POM parsing).
    #[error("POM parsing error: {0}")]
    #[cfg(feature = "pom")]
    Pom(#[from] pomace::Error),

    /// An error from the gradle-module-metadata crate (GMM parsing).
    #[error("Gradle module error: {0}")]
    #[cfg(feature = "gmm")]
    GradleModule(#[from] grale::Error),

    /// An error from the jvm-resolver crate (dependency resolution).
    #[error("resolution error: {0}")]
    Resolver(#[from] dendro::Error),

    /// An error from the jvm-fetch crate (artifact fetching).
    #[error("fetch error: {0}")]
    #[cfg(feature = "fetch")]
    Fetch(#[from] gather::Error),

    /// A custom resolution error message.
    #[error("{0}")]
    Resolution(String),
}
