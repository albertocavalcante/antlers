//! Maven POM file parsing and manipulation.
//!
//! This crate provides complete support for parsing Maven POM (Project Object Model)
//! XML files, including:
//!
//! - [`Pom`] - Parsed POM structure with all metadata
//! - [`Properties`] - Property substitution (`${property}` syntax)
//! - [`Profile`] - Maven profile support with activation conditions
//! - [`DependencyManagement`] - Centralized dependency version management
//! - [`PomParser`] - XML parsing utilities
//!
//! # Example
//!
//! ```
//! use pomace::PomParser;
//!
//! let xml = r#"
//!     <project>
//!         <groupId>com.example</groupId>
//!         <artifactId>my-lib</artifactId>
//!         <version>1.0.0</version>
//!         <dependencies>
//!             <dependency>
//!                 <groupId>org.slf4j</groupId>
//!                 <artifactId>slf4j-api</artifactId>
//!                 <version>2.0.0</version>
//!             </dependency>
//!         </dependencies>
//!     </project>
//! "#;
//!
//! let pom = PomParser::parse(xml).unwrap();
//! assert_eq!(pom.group_id, Some("com.example".to_string()));
//! assert_eq!(pom.artifact_id, Some("my-lib".to_string()));
//!
//! let deps = pom.direct_dependencies();
//! assert_eq!(deps.len(), 1);
//! assert_eq!(deps[0].artifact_id, "slf4j-api");
//! ```
//!
//! # Property Substitution
//!
//! Maven POMs support property substitution using `${property.name}` syntax.
//! The [`Properties`] type handles this substitution:
//!
//! ```
//! use pomace::Properties;
//!
//! let mut props = Properties::new();
//! props.insert("kotlin.version".to_string(), "2.0.0".to_string());
//!
//! assert_eq!(props.substitute("${kotlin.version}"), "2.0.0");
//! ```
//!
//! # Dependency Resolution
//!
//! The [`Pom`] type provides methods for resolving dependency versions through
//! the dependency management section:
//!
//! ```
//! use pomace::PomParser;
//!
//! let xml = r#"
//!     <project>
//!         <groupId>com.example</groupId>
//!         <artifactId>parent</artifactId>
//!         <version>1.0.0</version>
//!         <packaging>pom</packaging>
//!         <dependencyManagement>
//!             <dependencies>
//!                 <dependency>
//!                     <groupId>org.slf4j</groupId>
//!                     <artifactId>slf4j-api</artifactId>
//!                     <version>2.0.0</version>
//!                 </dependency>
//!             </dependencies>
//!         </dependencyManagement>
//!     </project>
//! "#;
//!
//! let pom = PomParser::parse(xml).unwrap();
//! let managed = pom.managed_dependencies();
//! assert_eq!(managed.len(), 1);
//! ```

mod error;
mod parser;
mod pom;
mod profile;
mod properties;

pub use error::{Error, Result};
pub use parser::PomParser;
pub use pom::{
    Dependencies, Dependency, DependencyManagement, Exclusion, Exclusions, Packaging, Parent, Pom,
    Scope,
};
pub use profile::{
    Activation, FileActivation, OsActivation, Profile, Profiles, PropertyActivation,
};
pub use properties::Properties;

// Re-export jvm-artifact types for convenience
pub use pom::{
    Artifact, Coordinates, Extension, JvmDependency, JvmExclusion, JvmExclusions, JvmScope,
    ManagedDependency, ParentRef, Version, VersionConstraint,
};
