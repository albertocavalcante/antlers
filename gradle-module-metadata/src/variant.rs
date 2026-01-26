//! Gradle variant definitions.
//!
//! A variant in Gradle Module Metadata represents a specific configuration
//! of an artifact, such as compile vs runtime, different JVM targets, etc.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::Attributes;

/// A variant of a Gradle module.
///
/// Variants represent different configurations of a module, such as:
/// - `apiElements` - API dependencies (compile-time)
/// - `runtimeElements` - Runtime dependencies
/// - Platform-specific variants (JVM 8, JVM 11, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variant {
    /// The name of the variant (e.g., "apiElements", "runtimeElements").
    pub name: String,

    /// Attributes describing this variant.
    #[serde(default)]
    pub attributes: Attributes,

    /// Dependencies of this variant.
    #[serde(default)]
    pub dependencies: Vec<VariantDependency>,

    /// Dependency constraints (version hints without requiring the dependency).
    #[serde(default)]
    pub dependency_constraints: Vec<DependencyConstraint>,

    /// Files published in this variant.
    #[serde(default)]
    pub files: Vec<File>,

    /// Capabilities provided by this variant.
    #[serde(default)]
    pub capabilities: Vec<Capability>,

    /// Reference to another module where this variant is defined.
    /// Used for relocated/redirected modules.
    #[serde(default)]
    pub available_at: Option<AvailableAt>,
}

impl Variant {
    /// Returns the primary JAR file for this variant, if any.
    #[must_use]
    pub fn primary_jar(&self) -> Option<&File> {
        self.files
            .iter()
            .find(|f| f.name.to_ascii_lowercase().ends_with(".jar"))
    }

    /// Returns true if this variant has an `available_at` redirect.
    #[must_use]
    pub const fn is_redirect(&self) -> bool {
        self.available_at.is_some()
    }
}

/// A dependency declared in a variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariantDependency {
    /// The group ID of the dependency.
    pub group: String,

    /// The module/artifact ID of the dependency.
    pub module: String,

    /// Version requirement for the dependency.
    #[serde(default)]
    pub version: Option<VersionRequirement>,

    /// Reason for this dependency (documentation).
    #[serde(default)]
    pub reason: Option<String>,

    /// Attributes to apply when resolving this dependency.
    #[serde(default)]
    pub attributes: Attributes,

    /// Capabilities requested from this dependency.
    #[serde(default, rename = "requestedCapabilities")]
    pub requested_capabilities: Vec<Capability>,

    /// Exclusions for transitive dependencies.
    #[serde(default)]
    pub excludes: Vec<Exclude>,

    /// Whether this dependency should be endorsed (strict version alignment).
    #[serde(default, rename = "endorseStrictVersions")]
    pub endorse_strict_versions: bool,

    /// Optional artifact selector for non-default artifacts.
    #[serde(default, rename = "thirdPartyCompatibility")]
    pub third_party_compatibility: Option<ThirdPartyCompatibility>,
}

impl VariantDependency {
    /// Returns the coordinates as "group:module".
    #[must_use]
    pub fn coordinates(&self) -> String {
        format!("{}:{}", self.group, self.module)
    }

    /// Returns the version string if specified.
    #[must_use]
    pub fn version_string(&self) -> Option<&str> {
        self.version.as_ref().and_then(|v| v.effective_version())
    }
}

/// Version requirement with multiple constraint types.
///
/// Gradle supports rich version constraints:
/// - `requires`: The default version to use.
/// - `strictly`: A strict version that cannot be upgraded.
/// - `prefers`: A preferred version (soft constraint).
/// - `rejects`: Versions to reject.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionRequirement {
    /// The required version (default constraint).
    #[serde(default)]
    pub requires: Option<String>,

    /// A strict version constraint (cannot be upgraded).
    #[serde(default)]
    pub strictly: Option<String>,

    /// A preferred version (soft constraint).
    #[serde(default)]
    pub prefers: Option<String>,

    /// Versions to reject.
    #[serde(default)]
    pub rejects: Vec<String>,
}

impl VersionRequirement {
    /// Returns the effective version string.
    ///
    /// Priority: strictly > requires > prefers
    #[must_use]
    pub fn effective_version(&self) -> Option<&str> {
        self.strictly
            .as_deref()
            .or(self.requires.as_deref())
            .or(self.prefers.as_deref())
    }

    /// Returns true if this is a strict version constraint.
    #[must_use]
    pub const fn is_strict(&self) -> bool {
        self.strictly.is_some()
    }

    /// Returns true if this requirement rejects the given version.
    #[must_use]
    pub fn rejects_version(&self, version: &str) -> bool {
        self.rejects.iter().any(|r| r == version)
    }
}

/// A dependency constraint (version hint without requiring the dependency).
///
/// Constraints are used to suggest versions for dependencies that may be
/// brought in transitively, without directly depending on them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyConstraint {
    /// The group ID.
    pub group: String,

    /// The module/artifact ID.
    pub module: String,

    /// Version constraint.
    #[serde(default)]
    pub version: Option<VersionRequirement>,

    /// Reason for this constraint (documentation).
    #[serde(default)]
    pub reason: Option<String>,

    /// Attributes for the constraint.
    #[serde(default)]
    pub attributes: Attributes,
}

impl DependencyConstraint {
    /// Returns the coordinates as "group:module".
    #[must_use]
    pub fn coordinates(&self) -> String {
        format!("{}:{}", self.group, self.module)
    }
}

/// A file published in a variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    /// The filename (e.g., "commons-lang3-3.12.0.jar").
    pub name: String,

    /// The URL path relative to the module (e.g., "commons-lang3-3.12.0.jar").
    pub url: String,

    /// File size in bytes.
    #[serde(default)]
    pub size: Option<u64>,

    /// SHA-1 checksum.
    #[serde(default)]
    pub sha1: Option<String>,

    /// SHA-256 checksum.
    #[serde(default)]
    pub sha256: Option<String>,

    /// SHA-512 checksum.
    #[serde(default)]
    pub sha512: Option<String>,

    /// MD5 checksum.
    #[serde(default)]
    pub md5: Option<String>,
}

impl File {
    /// Returns the strongest available checksum.
    ///
    /// Preference order: SHA-512 > SHA-256 > SHA-1 > MD5
    #[must_use]
    pub fn strongest_checksum(&self) -> Option<(&'static str, &str)> {
        self.sha512
            .as_deref()
            .map(|h| ("sha512", h))
            .or_else(|| self.sha256.as_deref().map(|h| ("sha256", h)))
            .or_else(|| self.sha1.as_deref().map(|h| ("sha1", h)))
            .or_else(|| self.md5.as_deref().map(|h| ("md5", h)))
    }

    /// Returns true if this file has any checksum.
    #[must_use]
    pub const fn has_checksum(&self) -> bool {
        self.sha512.is_some() || self.sha256.is_some() || self.sha1.is_some() || self.md5.is_some()
    }
}

/// A capability provided by a variant.
///
/// Capabilities allow multiple artifacts to declare that they provide
/// the same functionality, enabling conflict detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    /// The group ID of the capability.
    pub group: String,

    /// The name of the capability.
    pub name: String,

    /// The version of the capability.
    #[serde(default)]
    pub version: Option<String>,
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.group, self.name)?;
        if let Some(ref v) = self.version {
            write!(f, ":{v}")?;
        }
        Ok(())
    }
}

/// Reference to another module where a variant is available.
///
/// Used for module relocations and platform-specific artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableAt {
    /// The URL to the relocated module metadata.
    pub url: String,

    /// The group ID of the target module.
    pub group: String,

    /// The module/artifact ID of the target module.
    pub module: String,

    /// The version of the target module.
    pub version: String,
}

impl AvailableAt {
    /// Returns the coordinates as "group:module:version".
    #[must_use]
    pub fn coordinates(&self) -> String {
        format!("{}:{}:{}", self.group, self.module, self.version)
    }
}

/// Exclusion pattern for transitive dependencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exclude {
    /// The group pattern to exclude (can be "*" for all).
    pub group: String,

    /// The module pattern to exclude (can be "*" for all).
    pub module: String,
}

impl Exclude {
    /// Returns true if this exclusion matches the given coordinates.
    #[must_use]
    pub fn matches(&self, group: &str, module: &str) -> bool {
        let group_matches = self.group == "*" || self.group == group;
        let module_matches = self.module == "*" || self.module == module;
        group_matches && module_matches
    }
}

/// Third-party compatibility settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThirdPartyCompatibility {
    /// Artifact selectors.
    #[serde(default)]
    pub artifact_selector: Option<ArtifactSelector>,
}

/// Artifact selector for selecting non-default artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSelector {
    /// The artifact name.
    pub name: String,

    /// The artifact type/extension.
    #[serde(rename = "type")]
    pub artifact_type: String,

    /// Optional classifier.
    #[serde(default)]
    pub classifier: Option<String>,

    /// Optional extension (if different from type).
    #[serde(default)]
    pub extension: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_requirement_effective() {
        let req = VersionRequirement {
            requires: Some("1.0".to_string()),
            strictly: None,
            prefers: Some("1.1".to_string()),
            rejects: vec![],
        };
        assert_eq!(req.effective_version(), Some("1.0"));

        let strict_req = VersionRequirement {
            requires: Some("1.0".to_string()),
            strictly: Some("1.0.0".to_string()),
            prefers: None,
            rejects: vec![],
        };
        assert_eq!(strict_req.effective_version(), Some("1.0.0"));
        assert!(strict_req.is_strict());
    }

    #[test]
    fn test_version_requirement_rejects() {
        let req = VersionRequirement {
            requires: Some("1.0".to_string()),
            strictly: None,
            prefers: None,
            rejects: vec!["1.0.1".to_string(), "1.0.2".to_string()],
        };
        assert!(req.rejects_version("1.0.1"));
        assert!(!req.rejects_version("1.0.3"));
    }

    #[test]
    fn test_file_checksum() {
        let file = File {
            name: "test.jar".to_string(),
            url: "test.jar".to_string(),
            size: Some(1024),
            sha1: Some("abc123".to_string()),
            sha256: Some("def456".to_string()),
            sha512: None,
            md5: Some("789xyz".to_string()),
        };

        let (alg, hash) = file.strongest_checksum().unwrap();
        assert_eq!(alg, "sha256");
        assert_eq!(hash, "def456");
    }

    #[test]
    fn test_exclude_matches() {
        let exclude = Exclude {
            group: "org.example".to_string(),
            module: "*".to_string(),
        };

        assert!(exclude.matches("org.example", "foo"));
        assert!(exclude.matches("org.example", "bar"));
        assert!(!exclude.matches("com.other", "foo"));
    }

    #[test]
    fn test_capability_to_string() {
        let cap = Capability {
            group: "org.example".to_string(),
            name: "feature".to_string(),
            version: Some("1.0".to_string()),
        };
        assert_eq!(cap.to_string(), "org.example:feature:1.0");

        let cap_no_version = Capability {
            group: "org.example".to_string(),
            name: "feature".to_string(),
            version: None,
        };
        assert_eq!(cap_no_version.to_string(), "org.example:feature");
    }
}
