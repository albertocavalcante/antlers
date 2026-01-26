//! High-level API for JVM dependency resolution.
//!
//! Antlers is a native Rust resolver for JVM dependencies. This crate provides
//! a unified API that combines artifact model, parsing, and resolution.
//!
//! # Features
//!
//! - `pom` (default) - Maven POM parsing support via pomace
//! - `gmm` (default) - Gradle Module Metadata support via grale
//! - `fetch` (default) - HTTP fetching support via gather
//!
//! # Example
//!
//! ```no_run
//! use antlers::{Antlers, Artifact};
//!
//! # async fn example() -> antlers::Result<()> {
//! let resolution = Antlers::new()
//!     .with_maven_central()
//!     .resolve(&Artifact::parse("org.jetbrains.kotlin:kotlin-stdlib:2.0.0")?)
//!     .await?;
//!
//! for artifact in resolution.artifacts() {
//!     println!("{}", artifact.artifact);
//! }
//! # Ok(())
//! # }
//! ```

// Re-export core types from jvm-artifact
pub use gav::{
    Artifact, Classifier, Coordinates, Dependency, Exclusion, Exclusions, Extension,
    ManagedDependency, ParentRef, Project, Scope, Version, VersionConstraint, VersionInterval,
};

// Re-export format-specific parsers (feature-gated)
#[cfg(feature = "gmm")]
pub use grale::{GradleModule, Variant};
#[cfg(feature = "pom")]
pub use pomace::{Pom, PomParser, Properties};

// Re-export resolution types
pub use dendro::{
    ConflictStrategy, HighestWins, NearestWins, Resolution, ResolvedArtifact, Resolver,
    ResolverConfig, StrictFails, VersionConflict,
};

// Re-export fetch types (feature-gated)
#[cfg(feature = "fetch")]
pub use gather::{
    Cache, Checksum, ChecksumAlgo, Credentials, Ecosystem, Fetcher, FileCache, MavenRepository,
    MemoryCache, Netrc, ProxyConfig, Repository, RepositoryList, RepositoryPreset,
    RepositoryRegistry,
};

// Re-export lockfile types
pub use antlers_lock::{
    Conflict, LOCKFILE_FORMAT, LOCKFILE_VERSION, LockedArtifact, Lockfile, LockfileFormat,
    LockfileMetadata, Repository as LockRepository, from_resolution, to_resolution,
};

pub mod constants;
pub mod registry;

#[cfg(all(feature = "pom", feature = "gmm", feature = "fetch"))]
mod antlers;
#[cfg(feature = "fetch")]
pub mod config;
mod error;
#[cfg(all(feature = "pom", feature = "fetch"))]
mod fetcher;
#[cfg(all(feature = "pom", feature = "gmm", feature = "fetch"))]
mod gmm;
#[cfg(all(feature = "pom", feature = "fetch"))]
pub mod migrate;
#[cfg(all(feature = "pom", feature = "fetch"))]
mod parallel;

#[cfg(all(feature = "pom", feature = "gmm", feature = "fetch"))]
pub use crate::antlers::Antlers;
#[cfg(feature = "fetch")]
pub use config::{
    AntlerConfig, AntlersToml, CacheConfig, CacheMode, ConfigEditor, EnvConfig, FormatError,
    HermeticConfig, HermeticLevel, NetworkConfig, ParallelismConfig, RetryConfig, TomlFormatter,
};
pub use error::{Error, Result};
#[cfg(all(feature = "pom", feature = "fetch"))]
pub use fetcher::{PomFetcher, PomProject};
#[cfg(all(feature = "pom", feature = "gmm", feature = "fetch"))]
pub use gmm::{GradleModuleProject, HybridFetcher, HybridProject, VariantSelection};
#[cfg(all(feature = "pom", feature = "fetch"))]
pub use migrate::{IvyParser, MigrationError, MigrationSource, SourceFormat};
#[cfg(all(feature = "pom", feature = "fetch"))]
pub use parallel::ParallelPomFetcher;
