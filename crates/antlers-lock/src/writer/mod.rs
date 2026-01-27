//! Lockfile writers for various formats.
//!
//! This module provides writers for different lockfile formats:
//!
//! - [`antlers`] - Our native antlers-lock format
//! - [`v2`] - `rules_jvm_external` V2 format (for interoperability)
//! - [`coursier`] - Coursier JSON format (for tools using Coursier cache)
//! - [`gradle`] - Gradle version catalog format (libs.versions.toml)
//!
//! # Example
//!
//! ```ignore
//! use antlers_lock::{Lockfile, writer};
//!
//! let lockfile = Lockfile::new();
//!
//! // Write in our format
//! let json = writer::write_antlers(&lockfile)?;
//!
//! // Write in V2 format for Bazel compatibility
//! let v2_json = writer::write_v2(&lockfile)?;
//!
//! // Write in Coursier format
//! let coursier_json = writer::write_coursier(&lockfile)?;
//!
//! // Write as Gradle version catalog
//! let toml = writer::write_gradle_catalog(&lockfile)?;
//! ```

pub mod antlers;
pub mod coursier;
pub mod gradle;
pub mod v2;

pub use antlers::{write_antlers, write_antlers_compact};
pub use coursier::{write_coursier, write_coursier_compact, write_coursier_with_cache_path};
pub use gradle::{GradleCatalogOptions, write_gradle_catalog, write_gradle_catalog_with_options};
pub use v2::{write_v2, write_v2_compact};
