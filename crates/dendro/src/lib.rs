//! Format-agnostic dependency resolution algorithm.
//!
//! This crate provides the core resolution algorithm that works with any
//! project format (Maven POM, Gradle Module Metadata, etc.):
//!
//! - [`Resolver`] - Main resolver with configurable strategies
//! - [`Resolution`] - Result of dependency resolution
//! - [`ConflictStrategy`] - Pluggable version conflict resolution
//! - [`ResolverConfig`] - Resolution configuration options
//!
//! # Example
//!
//! ```ignore
//! use dendro::{Resolver, ResolverConfig, NearestWins};
//!
//! // Create a fetcher that implements ProjectFetcher
//! let fetcher = MyProjectFetcher::new();
//!
//! // Create a resolver with configuration
//! let resolver = Resolver::new(fetcher)
//!     .with_config(ResolverConfig::new().transitive(true));
//!
//! // Resolve an artifact
//! let artifact = Artifact::parse("com.example:lib:1.0.0").unwrap();
//! let resolution = resolver.resolve(&artifact).await?;
//!
//! // Print resolved artifacts
//! for artifact in resolution.artifacts() {
//!     println!("{}", artifact.artifact);
//! }
//! ```
//!
//! # Conflict Resolution Strategies
//!
//! The resolver supports pluggable conflict resolution strategies:
//!
//! - [`NearestWins`] - Maven's default: the nearest definition wins
//! - [`HighestWins`] - Always select the highest version
//! - [`StrictFails`] - Fail on any version conflict
//!
//! # Architecture
//!
//! The resolver is generic over:
//!
//! - [`ProjectFetcher`] - Fetches project metadata from repositories
//! - [`ConflictStrategy`] - Resolves version conflicts
//!
//! This allows the same resolution algorithm to work with different
//! project formats and repository implementations.

mod config;
mod conflict;
mod error;
mod graph;
mod resolution;
mod resolver;

pub use config::ResolverConfig;
pub use conflict::{
    ConflictStrategy, HighestWins, NearestWins, StrictFails, VersionChoice, VersionConflict,
};
pub use error::{Error, Result};
pub use graph::{DependencyGraph, GraphNode};
pub use resolution::{Resolution, ResolvedArtifact};
pub use resolver::{Checksums, ProjectFetcher, Resolver};
