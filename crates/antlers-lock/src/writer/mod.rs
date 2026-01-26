//! Lockfile writers for various formats.
//!
//! This module provides writers for different lockfile formats:
//!
//! - [`antlers`] - Our native antlers-lock format
//! - [`v2`] - `rules_jvm_external` V2 format (for interoperability)
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
//! ```

pub mod antlers;
pub mod v2;

pub use antlers::{write_antlers, write_antlers_compact};
pub use v2::{write_v2, write_v2_compact};
