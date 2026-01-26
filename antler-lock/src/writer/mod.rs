//! Lockfile writers for various formats.
//!
//! This module provides writers for different lockfile formats:
//!
//! - [`antler`] - Our native antler-lock format
//! - [`v2`] - `rules_jvm_external` V2 format (for interoperability)
//!
//! # Example
//!
//! ```ignore
//! use antler_lock::{Lockfile, writer};
//!
//! let lockfile = Lockfile::new();
//!
//! // Write in our format
//! let json = writer::write_antler(&lockfile)?;
//!
//! // Write in V2 format for Bazel compatibility
//! let v2_json = writer::write_v2(&lockfile)?;
//! ```

pub mod antler;
pub mod v2;

pub use antler::{write_antler, write_antler_compact};
pub use v2::{write_v2, write_v2_compact};
