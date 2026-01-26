//! Locked artifact representation.
//!
//! This module provides [`LockedArtifact`] for representing a resolved artifact
//! with its checksum, dependencies, and metadata.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A resolved artifact in the lockfile.
///
/// Contains all the information needed to reproducibly fetch and verify an artifact.
/// The artifact key (not stored here) is always `group_id:artifact_id`.
///
/// # Required Fields
///
/// - `version` - The exact resolved version
/// - `sha256` - SHA-256 checksum for integrity verification
///
/// # Optional Fields
///
/// Most fields are optional and only serialized when non-default/non-empty.
/// This keeps lockfiles compact while supporting rich metadata.
///
/// # Compatibility
///
/// The `extensions` field captures any unknown JSON fields, ensuring we can
/// read lockfiles from newer versions without data loss.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedArtifact {
    /// The resolved version string (e.g., "33.0.0-jre", "2.0.0").
    pub version: String,

    /// SHA-256 checksum of the artifact file (hex-encoded, 64 characters).
    /// Used for integrity verification during fetch.
    pub sha256: String,

    /// The repository URL where the artifact was resolved from.
    /// Useful for reproducibility when multiple repositories are configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,

    /// Maven packaging type. Defaults to "jar".
    /// Common values: "jar", "aar" (Android), "pom", "war".
    #[serde(
        default = "default_packaging",
        skip_serializing_if = "is_default_packaging"
    )]
    pub packaging: String,

    /// Maven classifier for variant artifacts.
    /// Common values: "sources", "javadoc", "tests", "linux-x86_64".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classifier: Option<String>,

    /// Direct dependencies as `group:artifact` keys (without version).
    /// These reference other entries in the lockfile's artifacts map.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,

    /// Java packages exported by this artifact.
    /// Used by Bazel's strict deps checking to validate imports.
    /// See: <https://bazel.build/docs/be/java#java_library.deps>
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<String>,

    /// Java `ServiceLoader` (SPI) implementations.
    /// Maps service interface FQCN to list of implementation FQCNs.
    /// Discovered from `META-INF/services/` in the JAR.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub services: IndexMap<String, Vec<String>>,

    // === Future KMP fields (reserved for Kotlin Multiplatform support) ===
    /// Target platforms for Kotlin Multiplatform artifacts.
    /// Values: "jvm", "js", "linuxX64", "macosArm64", "iosArm64", etc.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<String>,

    /// Kotlin compiler version this artifact was built with.
    /// Important for KMP binary compatibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kotlin_version: Option<String>,

    /// Unknown fields preserved for forward/backward compatibility.
    /// Uses `#[serde(flatten)]` to capture any fields we don't recognize,
    /// allowing us to round-trip lockfiles from newer antler versions.
    #[serde(flatten)]
    pub extensions: IndexMap<String, Value>,
}

impl LockedArtifact {
    /// Creates a new locked artifact with minimal required fields.
    #[must_use]
    pub fn new(version: impl Into<String>, sha256: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            sha256: sha256.into(),
            repository: None,
            packaging: default_packaging(),
            classifier: None,
            dependencies: Vec::new(),
            packages: Vec::new(),
            services: IndexMap::new(),
            targets: Vec::new(),
            kotlin_version: None,
            extensions: IndexMap::new(),
        }
    }

    /// Sets the repository URL.
    #[must_use]
    pub fn with_repository(mut self, repository: impl Into<String>) -> Self {
        self.repository = Some(repository.into());
        self
    }

    /// Sets the packaging type.
    #[must_use]
    pub fn with_packaging(mut self, packaging: impl Into<String>) -> Self {
        self.packaging = packaging.into();
        self
    }

    /// Sets the classifier.
    #[must_use]
    pub fn with_classifier(mut self, classifier: impl Into<String>) -> Self {
        self.classifier = Some(classifier.into());
        self
    }

    /// Adds a dependency.
    pub fn add_dependency(&mut self, dependency: impl Into<String>) {
        self.dependencies.push(dependency.into());
    }

    /// Sets the dependencies.
    #[must_use]
    pub fn with_dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }

    /// Sets the packages.
    #[must_use]
    pub fn with_packages(mut self, packages: Vec<String>) -> Self {
        self.packages = packages;
        self
    }

    /// Sets the services.
    #[must_use]
    pub fn with_services(mut self, services: IndexMap<String, Vec<String>>) -> Self {
        self.services = services;
        self
    }

    /// Sets the target platforms.
    #[must_use]
    pub fn with_targets(mut self, targets: Vec<String>) -> Self {
        self.targets = targets;
        self
    }

    /// Sets the Kotlin version.
    #[must_use]
    pub fn with_kotlin_version(mut self, kotlin_version: impl Into<String>) -> Self {
        self.kotlin_version = Some(kotlin_version.into());
        self
    }

    /// Returns true if this artifact is a JAR.
    #[must_use]
    pub fn is_jar(&self) -> bool {
        self.packaging == "jar"
    }

    /// Returns true if this artifact is an AAR (Android).
    #[must_use]
    pub fn is_aar(&self) -> bool {
        self.packaging == "aar"
    }

    /// Returns true if this artifact has a classifier.
    #[must_use]
    pub const fn has_classifier(&self) -> bool {
        self.classifier.is_some()
    }
}

impl Default for LockedArtifact {
    fn default() -> Self {
        Self {
            version: String::new(),
            sha256: String::new(),
            repository: None,
            packaging: default_packaging(),
            classifier: None,
            dependencies: Vec::new(),
            packages: Vec::new(),
            services: IndexMap::new(),
            targets: Vec::new(),
            kotlin_version: None,
            extensions: IndexMap::new(),
        }
    }
}

fn default_packaging() -> String {
    "jar".to_string()
}

fn is_default_packaging(packaging: &String) -> bool {
    packaging == "jar"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_artifact() {
        let artifact = LockedArtifact::new("1.0.0", "abc123");
        assert_eq!(artifact.version, "1.0.0");
        assert_eq!(artifact.sha256, "abc123");
        assert_eq!(artifact.packaging, "jar");
        assert!(artifact.classifier.is_none());
        assert!(artifact.dependencies.is_empty());
    }

    #[test]
    fn test_builder_pattern() {
        let artifact = LockedArtifact::new("2.0.0", "def456")
            .with_repository("https://repo1.maven.org/maven2/")
            .with_packaging("aar")
            .with_classifier("sources")
            .with_dependencies(vec!["com.example:dep1".to_string()]);

        assert_eq!(artifact.version, "2.0.0");
        assert_eq!(artifact.sha256, "def456");
        assert_eq!(
            artifact.repository,
            Some("https://repo1.maven.org/maven2/".to_string())
        );
        assert_eq!(artifact.packaging, "aar");
        assert_eq!(artifact.classifier, Some("sources".to_string()));
        assert_eq!(artifact.dependencies, vec!["com.example:dep1"]);
    }

    #[test]
    fn test_is_jar() {
        let jar = LockedArtifact::new("1.0", "abc");
        assert!(jar.is_jar());
        assert!(!jar.is_aar());

        let aar = LockedArtifact::new("1.0", "abc").with_packaging("aar");
        assert!(!aar.is_jar());
        assert!(aar.is_aar());
    }

    #[test]
    fn test_serialization() {
        let artifact = LockedArtifact::new("1.0.0", "abc123")
            .with_repository("https://repo1.maven.org/maven2/")
            .with_dependencies(vec!["com.example:dep".to_string()]);

        let json = serde_json::to_string_pretty(&artifact).unwrap();
        let parsed: LockedArtifact = serde_json::from_str(&json).unwrap();

        assert_eq!(artifact.version, parsed.version);
        assert_eq!(artifact.sha256, parsed.sha256);
        assert_eq!(artifact.repository, parsed.repository);
        assert_eq!(artifact.dependencies, parsed.dependencies);
    }

    #[test]
    fn test_default_packaging_not_serialized() {
        let artifact = LockedArtifact::new("1.0.0", "abc123");
        let json = serde_json::to_string(&artifact).unwrap();
        assert!(!json.contains("packaging"));
    }

    #[test]
    fn test_non_default_packaging_serialized() {
        let artifact = LockedArtifact::new("1.0.0", "abc123").with_packaging("aar");
        let json = serde_json::to_string(&artifact).unwrap();
        assert!(json.contains("packaging"));
        assert!(json.contains("aar"));
    }
}
