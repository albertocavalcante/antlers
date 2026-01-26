//! Abstract project model.
//!
//! This module defines the [`Project`] trait which provides a common interface
//! for Maven POMs and Gradle Module Metadata.

use serde::{Deserialize, Serialize};

use crate::artifact::{Artifact, Coordinates};
use crate::dependency::{Dependency, Scope};
use crate::exclusions::Exclusions;
use crate::version::{Version, VersionConstraint};

/// A reference to a parent project.
///
/// In Maven, projects can inherit from parent POMs. This struct
/// captures the reference to a parent project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParentRef {
    /// The coordinates of the parent project.
    pub coordinates: Coordinates,
    /// The version of the parent project.
    pub version: Version,
    /// Optional relative path to the parent POM (default "../pom.xml").
    pub relative_path: Option<String>,
}

impl ParentRef {
    /// Creates a new parent reference.
    #[must_use]
    pub const fn new(coordinates: Coordinates, version: Version) -> Self {
        Self {
            coordinates,
            version,
            relative_path: None,
        }
    }

    /// Creates a new parent reference from group, artifact, and version strings.
    #[must_use]
    pub fn from_gav(
        group_id: impl Into<String>,
        artifact_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            coordinates: Coordinates::new(group_id, artifact_id),
            version: Version::new(version),
            relative_path: None,
        }
    }

    /// Creates a new parent reference with a relative path.
    #[must_use]
    pub fn with_relative_path(mut self, path: impl Into<String>) -> Self {
        self.relative_path = Some(path.into());
        self
    }

    /// Converts this parent reference to an artifact.
    #[must_use]
    pub fn to_artifact(&self) -> Artifact {
        Artifact::new(
            &self.coordinates.group_id,
            &self.coordinates.artifact_id,
            self.version.as_str(),
        )
    }

    /// Returns the group ID.
    #[must_use]
    pub fn group_id(&self) -> &str {
        &self.coordinates.group_id
    }

    /// Returns the artifact ID.
    #[must_use]
    pub fn artifact_id(&self) -> &str {
        &self.coordinates.artifact_id
    }
}

/// A managed dependency declaration.
///
/// Managed dependencies are declared in the `dependencyManagement` section
/// of a POM. They provide default values (version, scope, exclusions) for
/// dependencies declared in projects that inherit from this project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedDependency {
    /// The artifact coordinates (group + artifact).
    pub coordinates: Coordinates,
    /// The version constraint (may be absent for BOM imports).
    pub version: Option<VersionConstraint>,
    /// Optional scope override.
    pub scope: Option<Scope>,
    /// Exclusions to apply to this dependency.
    pub exclusions: Exclusions,
}

impl ManagedDependency {
    /// Creates a new managed dependency.
    #[must_use]
    pub fn new(coordinates: Coordinates) -> Self {
        Self {
            coordinates,
            version: None,
            scope: None,
            exclusions: Exclusions::default(),
        }
    }

    /// Creates a new managed dependency from group, artifact, and version strings.
    #[must_use]
    pub fn from_gav(
        group_id: impl Into<String>,
        artifact_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            coordinates: Coordinates::new(group_id, artifact_id),
            version: Some(VersionConstraint::Exact(Version::new(version))),
            scope: None,
            exclusions: Exclusions::default(),
        }
    }

    /// Creates a new managed dependency with version.
    #[must_use]
    pub fn with_version(mut self, version: VersionConstraint) -> Self {
        self.version = Some(version);
        self
    }

    /// Creates a new managed dependency with scope.
    #[must_use]
    pub const fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = Some(scope);
        self
    }

    /// Creates a new managed dependency with exclusions.
    #[must_use]
    pub fn with_exclusions(mut self, exclusions: Exclusions) -> Self {
        self.exclusions = exclusions;
        self
    }

    /// Returns the group ID.
    #[must_use]
    pub fn group_id(&self) -> &str {
        &self.coordinates.group_id
    }

    /// Returns the artifact ID.
    #[must_use]
    pub fn artifact_id(&self) -> &str {
        &self.coordinates.artifact_id
    }

    /// Returns the key for this dependency (groupId:artifactId).
    #[must_use]
    pub fn key(&self) -> String {
        format!(
            "{}:{}",
            self.coordinates.group_id, self.coordinates.artifact_id
        )
    }
}

/// Abstract project model that both Maven POM and Gradle Module implement.
///
/// This trait provides a common interface for accessing project metadata,
/// dependencies, and parent information regardless of the underlying format.
pub trait Project {
    /// Returns the project coordinates (group + artifact).
    fn coordinates(&self) -> &Coordinates;

    /// Returns the project version.
    fn version(&self) -> &Version;

    /// Returns the project's direct dependencies.
    fn dependencies(&self) -> Vec<&Dependency>;

    /// Returns the project's managed dependencies.
    ///
    /// Managed dependencies provide default values for dependencies declared
    /// in this project or projects that inherit from it.
    fn managed_dependencies(&self) -> Vec<&ManagedDependency>;

    /// Returns the parent project reference, if any.
    fn parent(&self) -> Option<&ParentRef>;

    /// Returns the project properties as key-value pairs.
    ///
    /// Properties can be used for variable substitution in the project
    /// (e.g., `${project.version}`).
    fn properties(&self) -> &[(String, String)];

    /// Returns the packaging type (default "jar").
    fn packaging(&self) -> &'static str {
        "jar"
    }

    /// Returns the project name, if available.
    fn name(&self) -> Option<&str> {
        None
    }

    /// Returns the project description, if available.
    fn description(&self) -> Option<&str> {
        None
    }

    /// Returns the coordinate string (group:artifact:version).
    fn coordinate_string(&self) -> String {
        format!(
            "{}:{}:{}",
            self.coordinates().group_id,
            self.coordinates().artifact_id,
            self.version()
        )
    }

    /// Converts this project to an artifact.
    fn to_artifact(&self) -> Artifact {
        Artifact::new(
            &self.coordinates().group_id,
            &self.coordinates().artifact_id,
            self.version().as_str(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parent_ref() {
        let parent = ParentRef::new(
            Coordinates::new("org.springframework.boot", "spring-boot-starter-parent"),
            Version::new("3.2.0"),
        )
        .with_relative_path("../pom.xml");

        assert_eq!(parent.coordinates.group_id, "org.springframework.boot");
        assert_eq!(parent.coordinates.artifact_id, "spring-boot-starter-parent");
        assert_eq!(parent.version.as_str(), "3.2.0");
        assert_eq!(parent.relative_path, Some("../pom.xml".to_string()));
    }

    #[test]
    fn test_parent_ref_from_gav() {
        let parent = ParentRef::from_gav("com.example", "parent", "1.0.0");
        let artifact = parent.to_artifact();
        assert_eq!(artifact.group_id(), "com.example");
        assert_eq!(artifact.artifact_id(), "parent");
        assert_eq!(artifact.version.as_str(), "1.0.0");
    }

    #[test]
    fn test_managed_dependency() {
        let managed = ManagedDependency::new(Coordinates::new("com.google.guava", "guava"))
            .with_version(VersionConstraint::Exact(Version::new("31.1-jre")));

        assert_eq!(managed.group_id(), "com.google.guava");
        assert_eq!(managed.artifact_id(), "guava");
        assert_eq!(managed.key(), "com.google.guava:guava");
        assert!(managed.version.is_some());
    }

    #[test]
    fn test_managed_dependency_from_gav() {
        let managed = ManagedDependency::from_gav("com.example", "lib", "2.0.0");
        assert_eq!(managed.key(), "com.example:lib");
    }
}
