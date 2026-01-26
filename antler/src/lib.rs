//! High-level API for JVM dependency resolution.
//!
//! Antler is a native Rust resolver for JVM dependencies. This crate provides
//! a unified API that combines artifact model, parsing, and resolution.
//!
//! # Example
//!
//! ```no_run
//! use antler::{Antler, Artifact};
//!
//! # async fn example() -> antler::Result<()> {
//! let resolution = Antler::new()
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
pub use jvm_artifact::{
    Artifact, Classifier, Coordinates, Dependency, Exclusion, Exclusions, Extension,
    ManagedDependency, ParentRef, Project, Scope, Version, VersionConstraint, VersionInterval,
};

// Re-export format-specific parsers
pub use gradle_module_metadata::{GradleModule, Variant};
pub use maven_pom::{Pom, PomParser, Properties};

// Re-export resolution types
pub use jvm_resolver::{
    ConflictStrategy, HighestWins, NearestWins, Resolution, ResolvedArtifact, Resolver,
    ResolverConfig, StrictFails, VersionConflict,
};

// Re-export fetch types
pub use jvm_fetch::{
    Cache, Checksum, ChecksumAlgo, Fetcher, FileCache, MavenRepository, MemoryCache, Repository,
    RepositoryList,
};

// Re-export lockfile types
pub use antler_lock::{
    Conflict, LOCKFILE_FORMAT, LOCKFILE_VERSION, LockedArtifact, Lockfile, LockfileFormat,
    LockfileMetadata, Repository as LockRepository, from_resolution, to_resolution,
};

mod antler;
mod error;

pub use crate::antler::Antler;
pub use error::{Error, Result};
