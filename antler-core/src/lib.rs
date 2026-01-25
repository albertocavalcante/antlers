//! # antler-core
//!
//! Core library for resolving JVM dependencies from Maven repositories.
//!
//! ## Features
//!
//! - Parse Maven POM files
//! - Parse Gradle Module Metadata (.module files)
//! - Resolve transitive dependencies
//! - Download artifacts with checksum verification
//!
//! ## Example
//!
//! ```rust,ignore
//! use antler_core::{Artifact, Resolver, Repository};
//!
//! let resolver = Resolver::new()
//!     .with_repository(Repository::maven_central());
//!
//! let artifact = Artifact::parse("org.jetbrains.kotlin:kotlin-stdlib:2.3.0")?;
//! let resolution = resolver.resolve(&artifact).await?;
//! ```

// Builder pattern methods returning Self are idiomatic in Rust
#![allow(clippy::return_self_not_must_use)]
// if-let-else is often more readable than map_or_else
#![allow(clippy::option_if_let_else)]

pub mod artifact;
pub mod error;
pub mod pom;
pub mod repository;
pub mod resolver;

pub use artifact::Artifact;
pub use error::{Error, Result};
pub use repository::Repository;
pub use resolver::{Resolution, Resolver};
