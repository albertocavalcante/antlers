//! High-level API for JVM dependency resolution.
//!
//! Antlers is a native Rust resolver for JVM dependencies. This crate provides
//! a unified API that combines artifact model, parsing, and resolution.
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

// Re-export format-specific parsers
pub use grale::{GradleModule, Variant};
pub use pomace::{Pom, PomParser, Properties};

// Re-export resolution types
pub use dendro::{
    ConflictStrategy, HighestWins, NearestWins, Resolution, ResolvedArtifact, Resolver,
    ResolverConfig, StrictFails, VersionConflict,
};

// Re-export fetch types
pub use gather::{
    Cache, Checksum, ChecksumAlgo, Fetcher, FileCache, MavenRepository, MemoryCache, Repository,
    RepositoryList,
};

// Re-export lockfile types
pub use antlers_lock::{
    Conflict, LOCKFILE_FORMAT, LOCKFILE_VERSION, LockedArtifact, Lockfile, LockfileFormat,
    LockfileMetadata, Repository as LockRepository, from_resolution, to_resolution,
};

mod antlers;
mod error;

pub use crate::antlers::Antlers;
pub use error::{Error, Result};
