//! Lockfile writers for various formats.
//!
//! This module provides writers for different lockfile formats:
//!
//! - [`antlers`] - Our native antlers-lock format
//! - [`v2`] - `rules_jvm_external` V2 format (for interoperability)
//! - [`coursier`] - Coursier JSON format (for tools using Coursier cache)
//! - [`gradle`] - Gradle version catalog format (libs.versions.toml)
//! - [`starlark`] - Starlark formats (dict, module extensions, legacy macros)
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
//!
//! // Write as Starlark dict (.bzl)
//! let bzl = writer::write_starlark_dict(&lockfile)?;
//!
//! // Write as bzlmod module extension (.bzl)
//! let bzl = writer::write_module_extension(&lockfile)?;
//!
//! // Write as legacy WORKSPACE macro (.bzl)
//! let bzl = writer::write_starlark_macro(&lockfile)?;
//! ```

pub mod antlers;
pub mod coursier;
pub mod gradle;
pub mod starlark;
pub mod v2;

pub use antlers::{write_antlers, write_antlers_compact};
pub use coursier::{write_coursier, write_coursier_compact, write_coursier_with_cache_path};
pub use gradle::{GradleCatalogOptions, write_gradle_catalog, write_gradle_catalog_with_options};
pub use starlark::{
    MAVEN_CENTRAL_URL, ModuleExtensionOptions, StarlarkDictOptions, StarlarkMacroOptions,
    coordinate_to_target_name, write_module_extension, write_module_extension_with_options,
    write_starlark_dict, write_starlark_dict_with_options, write_starlark_macro,
    write_starlark_macro_with_options,
};
pub use v2::{write_v2, write_v2_compact};
