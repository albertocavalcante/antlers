//! Core data model for JVM artifacts.
//!
//! This crate provides the fundamental types for working with JVM artifacts:
//! - [`Artifact`] - Maven/Gradle artifact coordinates
//! - [`Version`] and [`VersionConstraint`] - Version handling with ranges
//! - [`Dependency`] - Dependency declarations with scope and exclusions
//! - [`Exclusions`] - Efficient exclusion pattern matching
//! - [`Project`] trait - Abstract project model for resolution

mod artifact;
mod dependency;
mod error;
mod exclusions;
mod project;
mod version;

pub use artifact::{Artifact, Classifier, Coordinates, Extension};
pub use dependency::{Dependency, Scope};
pub use error::{Error, Result};
pub use exclusions::{Exclusion, Exclusions};
pub use project::{ManagedDependency, ParentRef, Project};
pub use version::{Bound, LatestKind, Version, VersionConstraint, VersionInterval};
