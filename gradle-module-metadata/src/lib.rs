//! Gradle Module Metadata (.module) file parsing.
//!
//! This crate provides complete support for Gradle's module metadata format,
//! including variant-aware dependency resolution.
//!
//! # Overview
//!
//! Gradle Module Metadata (GMM) is a JSON format that provides richer
//! information than Maven POMs:
//! - Variant-aware dependencies (API vs runtime, JVM version targeting)
//! - Rich version constraints (strictly, requires, prefers, rejects)
//! - Capability declarations for conflict detection
//! - Module relocations via `availableAt`
//!
//! # Example
//!
//! ```
//! use gradle_module_metadata::{GradleModuleParser, AllOf};
//!
//! let json = r#"{
//!     "formatVersion": "1.1",
//!     "component": {
//!         "group": "org.example",
//!         "module": "library",
//!         "version": "1.0.0"
//!     },
//!     "variants": [
//!         {
//!             "name": "runtimeElements",
//!             "attributes": {
//!                 "org.gradle.usage": "java-runtime",
//!                 "org.gradle.category": "library"
//!             },
//!             "dependencies": [
//!                 {
//!                     "group": "com.google.guava",
//!                     "module": "guava",
//!                     "version": { "requires": "31.1-jre" }
//!                 }
//!             ]
//!         }
//!     ]
//! }"#;
//!
//! let module = GradleModuleParser::parse(json).unwrap();
//!
//! // Select the runtime variant
//! if let Some(variant) = module.runtime_variant() {
//!     for dep in &variant.dependencies {
//!         println!("Depends on: {}:{}", dep.group, dep.module);
//!     }
//! }
//! ```
//!
//! # Variant Selection
//!
//! Use attribute matchers to select the appropriate variant:
//!
//! ```
//! use gradle_module_metadata::{AllOf, AttributeMatcher, JvmVersionMatcher};
//!
//! // Create a matcher for JVM 11 runtime dependencies
//! let matcher = AllOf::runtime_for_jvm(11);
//! ```
//!
//! # Key Types
//!
//! - [`GradleModule`] - The root metadata structure
//! - [`Variant`] - A module variant with attributes, dependencies, and files
//! - [`Attributes`] - Key-value pairs describing a variant
//! - [`AttributeMatcher`] - Trait for selecting variants by attributes

mod attributes;
mod error;
mod module;
mod parser;
mod variant;

// Re-export main types
pub use attributes::{
    AllOf, ApiMatcher, AttributeMatcher, Attributes, JarElementsMatcher, JvmVersionMatcher,
    LibraryMatcher, RuntimeMatcher, keys, values,
};
pub use error::{Error, Result};
pub use module::{Component, CreatedBy, GradleInfo, GradleModule};
pub use parser::GradleModuleParser;
pub use variant::{
    ArtifactSelector, AvailableAt, Capability, DependencyConstraint, Exclude, File,
    ThirdPartyCompatibility, Variant, VariantDependency, VersionRequirement,
};
