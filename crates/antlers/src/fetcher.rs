//! Project fetcher implementation for Maven POMs.
//!
//! This module provides [`PomFetcher`] which implements the [`ProjectFetcher`](dendro::ProjectFetcher)
//! trait for fetching and parsing Maven POM files.

use dendro::{Checksums, ProjectFetcher};
use gather::{ChecksumAlgo, Fetcher, RepositoryList};
use gav::{Artifact, Coordinates, Dependency, ManagedDependency, ParentRef, Project, Version};
use pomace::{Pom, PomParser};
use std::sync::Arc;

use crate::{Error, Result};

/// A wrapper around a parsed POM that implements the Project trait.
///
/// This allows the resolver to work with POMs fetched from Maven repositories.
#[derive(Debug, Clone)]
pub struct PomProject {
    /// The underlying parsed POM.
    pom: Pom,
    /// Coordinates (computed once).
    coordinates: Coordinates,
    /// Version (computed once).
    version: Version,
    /// Dependencies converted to jvm-artifact format.
    dependencies: Vec<Dependency>,
    /// Managed dependencies converted to jvm-artifact format.
    managed_dependencies: Vec<ManagedDependency>,
    /// Parent reference if any.
    parent_ref: Option<ParentRef>,
    /// Properties as key-value pairs.
    properties: Vec<(String, String)>,
}

impl PomProject {
    /// Creates a new `PomProject` from a parsed [`Pom`].
    ///
    /// # Errors
    ///
    /// Returns an error if the POM is missing required fields (group ID, artifact ID, version).
    pub fn new(pom: Pom) -> Result<Self> {
        let group_id = pom
            .effective_group_id()
            .ok_or_else(|| Error::Resolution("POM missing group ID".to_string()))?;
        let artifact_id = pom
            .artifact_id
            .as_deref()
            .ok_or_else(|| Error::Resolution("POM missing artifact ID".to_string()))?;
        let version_str = pom
            .effective_version()
            .ok_or_else(|| Error::Resolution("POM missing version".to_string()))?;

        let coordinates = Coordinates::new(group_id, artifact_id);
        let version = Version::new(version_str);

        // Convert dependencies with property substitution
        let dependencies = Self::convert_dependencies(&pom);

        // Convert managed dependencies with property substitution
        let managed_dependencies = Self::convert_managed_dependencies(&pom);

        // Convert parent reference
        let parent_ref = pom.to_parent_ref();

        // Get properties
        let properties = pom.properties_as_vec();

        Ok(Self {
            pom,
            coordinates,
            version,
            dependencies,
            managed_dependencies,
            parent_ref,
            properties,
        })
    }

    /// Converts POM dependencies to jvm-artifact Dependencies with property substitution.
    fn convert_dependencies(pom: &Pom) -> Vec<Dependency> {
        pom.direct_dependencies()
            .iter()
            .filter_map(|dep| {
                // Get version with property substitution
                let version = pom.resolve_dependency_version(dep)?;

                // Skip if version still contains unresolved properties
                if version.contains("${") {
                    return None;
                }

                // Parse the version constraint
                let version_constraint = gav::VersionConstraint::parse(&version).ok()?;

                // Build the dependency
                let mut jvm_dep = Dependency::new(dep.to_coordinates())
                    .with_version(version_constraint)
                    .with_scope(dep.parsed_scope().to_jvm_scope())
                    .with_exclusions(dep.to_jvm_exclusions());

                if dep.optional.unwrap_or(false) {
                    jvm_dep = jvm_dep.optional();
                }

                if let Some(ref classifier) = dep.classifier {
                    jvm_dep = jvm_dep.with_classifier(classifier.as_str());
                }

                if let Some(ref dep_type) = dep.dep_type {
                    jvm_dep = jvm_dep.with_type(dep_type.as_str());
                }

                Some(jvm_dep)
            })
            .collect()
    }

    /// Converts POM managed dependencies to jvm-artifact `ManagedDependency` with property substitution.
    fn convert_managed_dependencies(pom: &Pom) -> Vec<ManagedDependency> {
        pom.managed_dependencies()
            .iter()
            .filter_map(|dep| {
                // Get version with property substitution
                let version = dep.version.as_ref()?;
                let resolved_version = pom.substitute_properties(version);

                // Skip if version still contains unresolved properties
                if resolved_version.contains("${") {
                    return None;
                }

                // Parse the version constraint
                let version_constraint = gav::VersionConstraint::parse(&resolved_version).ok()?;

                let managed = ManagedDependency::new(dep.to_coordinates())
                    .with_version(version_constraint)
                    .with_scope(dep.parsed_scope().to_jvm_scope())
                    .with_exclusions(dep.to_jvm_exclusions());

                Some(managed)
            })
            .collect()
    }

    /// Returns a reference to the underlying [`Pom`].
    #[must_use]
    pub const fn pom(&self) -> &Pom {
        &self.pom
    }
}

impl Project for PomProject {
    fn coordinates(&self) -> &Coordinates {
        &self.coordinates
    }

    fn version(&self) -> &Version {
        &self.version
    }

    fn dependencies(&self) -> Vec<&Dependency> {
        self.dependencies.iter().collect()
    }

    fn managed_dependencies(&self) -> Vec<&ManagedDependency> {
        self.managed_dependencies.iter().collect()
    }

    fn parent(&self) -> Option<&ParentRef> {
        self.parent_ref.as_ref()
    }

    fn properties(&self) -> &[(String, String)] {
        &self.properties
    }

    fn packaging(&self) -> &'static str {
        match self.pom.effective_packaging() {
            pomace::Packaging::Pom => "pom",
            pomace::Packaging::War => "war",
            pomace::Packaging::Ear => "ear",
            _ => "jar",
        }
    }
}

/// Fetcher for Maven POM files.
///
/// This implements the [`ProjectFetcher`] trait from dendro, allowing
/// the resolver to fetch and parse POMs from Maven repositories.
pub struct PomFetcher {
    fetcher: Arc<Fetcher>,
}

impl PomFetcher {
    /// Creates a new [`PomFetcher`] with the given repositories.
    pub fn new(repositories: RepositoryList) -> Self {
        Self {
            fetcher: Arc::new(Fetcher::new(repositories)),
        }
    }

    /// Creates a new [`PomFetcher`] with default repositories (Maven Central + Google).
    pub fn with_defaults() -> Self {
        Self::new(RepositoryList::with_defaults())
    }

    /// Returns a reference to the underlying fetcher.
    pub fn fetcher(&self) -> &Fetcher {
        &self.fetcher
    }
}

/// Error type for POM fetching operations.
#[derive(Debug)]
pub struct PomFetchError {
    message: String,
}

impl PomFetchError {
    /// Creates a new fetch error with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for PomFetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PomFetchError {}

impl PomFetcher {
    /// Recursively fetches and resolves parent POMs.
    fn fetch_pom_with_parents<'a>(
        &'a self,
        artifact: &'a Artifact,
        depth: usize,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<Pom, PomFetchError>> + Send + 'a>,
    > {
        Box::pin(async move {
            const MAX_PARENT_DEPTH: usize = 10;

            if depth > MAX_PARENT_DEPTH {
                return Err(PomFetchError::new(format!(
                    "Maximum parent depth ({MAX_PARENT_DEPTH}) exceeded for {artifact}"
                )));
            }

            // Fetch the POM content
            let pom_content = self.fetcher.fetch_pom(artifact).await.map_err(|e| {
                PomFetchError::new(format!("Failed to fetch POM for {artifact}: {e}"))
            })?;

            // Parse the POM
            let mut pom = PomParser::parse(&pom_content).map_err(|e| {
                PomFetchError::new(format!("Failed to parse POM for {artifact}: {e}"))
            })?;

            // If there's a parent, fetch it recursively
            if let Some(ref parent) = pom.parent {
                let parent_artifact =
                    Artifact::new(&parent.group_id, &parent.artifact_id, &parent.version)
                        .with_extension("pom");

                match self
                    .fetch_pom_with_parents(&parent_artifact, depth + 1)
                    .await
                {
                    Ok(parent_pom) => {
                        pom.resolved_parent = Some(Box::new(parent_pom));
                    }
                    Err(e) => {
                        // Log but continue - parent might not be resolvable
                        tracing::warn!("Failed to resolve parent POM for {artifact}: {e}");
                    }
                }
            }

            Ok(pom)
        })
    }
}

impl ProjectFetcher for PomFetcher {
    type Project = PomProject;
    type Error = PomFetchError;

    async fn fetch(&self, artifact: &Artifact) -> std::result::Result<Self::Project, Self::Error> {
        // Fetch the POM with parent resolution
        let pom = self.fetch_pom_with_parents(artifact, 0).await?;

        // Convert to PomProject
        PomProject::new(pom)
            .map_err(|e| PomFetchError::new(format!("Invalid POM for {artifact}: {e}")))
    }

    async fn fetch_checksums(&self, artifact: &Artifact) -> Checksums {
        let mut checksums = Checksums::none();

        // Fetch SHA1
        if let Ok(Some(sha1)) = self
            .fetcher
            .fetch_checksum(artifact, ChecksumAlgo::Sha1)
            .await
        {
            checksums = checksums.with_sha1(sha1);
        }

        // Fetch SHA256
        if let Ok(Some(sha256)) = self
            .fetcher
            .fetch_checksum(artifact, ChecksumAlgo::Sha256)
            .await
        {
            checksums = checksums.with_sha256(sha256);
        }

        // Set repository name
        if let Some(repo) = self.fetcher.repositories().iter().next() {
            checksums = checksums.with_repository(&repo.name);
        }

        checksums
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pomace::{Dependencies, Dependency as PomDependency, Properties};

    #[test]
    fn test_pom_project_from_pom() {
        let pom = Pom {
            group_id: Some("com.example".to_string()),
            artifact_id: Some("my-lib".to_string()),
            version: Some("1.0.0".to_string()),
            ..Default::default()
        };

        let project = PomProject::new(pom).unwrap();
        assert_eq!(project.coordinates().group_id, "com.example");
        assert_eq!(project.coordinates().artifact_id, "my-lib");
        assert_eq!(project.version().as_str(), "1.0.0");
    }

    #[test]
    fn test_pom_project_missing_group_id() {
        let pom = Pom {
            artifact_id: Some("my-lib".to_string()),
            version: Some("1.0.0".to_string()),
            ..Default::default()
        };

        let result = PomProject::new(pom);
        assert!(result.is_err());
    }

    #[test]
    fn test_pom_project_missing_artifact_id() {
        let pom = Pom {
            group_id: Some("com.example".to_string()),
            version: Some("1.0.0".to_string()),
            ..Default::default()
        };

        let result = PomProject::new(pom);
        assert!(result.is_err());
    }

    #[test]
    fn test_pom_project_missing_version() {
        let pom = Pom {
            group_id: Some("com.example".to_string()),
            artifact_id: Some("my-lib".to_string()),
            ..Default::default()
        };

        let result = PomProject::new(pom);
        assert!(result.is_err());
    }

    #[test]
    fn test_pom_project_with_dependencies() {
        let pom = Pom {
            group_id: Some("com.example".to_string()),
            artifact_id: Some("my-lib".to_string()),
            version: Some("1.0.0".to_string()),
            dependencies: Some(Dependencies {
                dependencies: vec![PomDependency {
                    group_id: "org.slf4j".to_string(),
                    artifact_id: "slf4j-api".to_string(),
                    version: Some("2.0.0".to_string()),
                    ..Default::default()
                }],
            }),
            ..Default::default()
        };

        let project = PomProject::new(pom).unwrap();
        let deps = project.dependencies();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].group_id(), "org.slf4j");
        assert_eq!(deps[0].artifact_id(), "slf4j-api");
    }

    #[test]
    fn test_pom_project_with_property_substitution() {
        let mut props = Properties::new();
        props.insert("slf4j.version".to_string(), "2.0.9".to_string());

        let pom = Pom {
            group_id: Some("com.example".to_string()),
            artifact_id: Some("my-lib".to_string()),
            version: Some("1.0.0".to_string()),
            properties: props,
            dependencies: Some(Dependencies {
                dependencies: vec![PomDependency {
                    group_id: "org.slf4j".to_string(),
                    artifact_id: "slf4j-api".to_string(),
                    version: Some("${slf4j.version}".to_string()),
                    ..Default::default()
                }],
            }),
            ..Default::default()
        };

        let project = PomProject::new(pom).unwrap();
        let deps = project.dependencies();
        assert_eq!(deps.len(), 1);
        // Version should be resolved
        assert!(deps[0].version.is_some());
    }

    #[test]
    fn test_pom_project_skips_unresolved_properties() {
        let pom = Pom {
            group_id: Some("com.example".to_string()),
            artifact_id: Some("my-lib".to_string()),
            version: Some("1.0.0".to_string()),
            dependencies: Some(Dependencies {
                dependencies: vec![PomDependency {
                    group_id: "org.slf4j".to_string(),
                    artifact_id: "slf4j-api".to_string(),
                    version: Some("${undefined.property}".to_string()),
                    ..Default::default()
                }],
            }),
            ..Default::default()
        };

        let project = PomProject::new(pom).unwrap();
        // Dependency with unresolved property should be skipped
        assert_eq!(project.dependencies().len(), 0);
    }

    #[test]
    fn test_pom_project_inherits_from_parent() {
        // Create parent POM with properties
        let mut parent_props = Properties::new();
        parent_props.insert("dep.version".to_string(), "3.0.0".to_string());

        let parent_pom = Pom {
            group_id: Some("com.example".to_string()),
            artifact_id: Some("parent".to_string()),
            version: Some("1.0.0".to_string()),
            properties: parent_props,
            ..Default::default()
        };

        // Create child POM that uses parent property
        let mut child_pom = Pom {
            group_id: Some("com.example".to_string()),
            artifact_id: Some("child".to_string()),
            version: Some("1.0.0".to_string()),
            dependencies: Some(Dependencies {
                dependencies: vec![PomDependency {
                    group_id: "org.example".to_string(),
                    artifact_id: "dep".to_string(),
                    version: Some("${dep.version}".to_string()),
                    ..Default::default()
                }],
            }),
            ..Default::default()
        };
        child_pom.resolved_parent = Some(Box::new(parent_pom));

        let project = PomProject::new(child_pom).unwrap();
        let deps = project.dependencies();
        assert_eq!(deps.len(), 1);
        // Should resolve version from parent property
        assert!(deps[0].version.is_some());
    }

    #[test]
    fn test_pom_project_packaging() {
        let pom = Pom {
            group_id: Some("com.example".to_string()),
            artifact_id: Some("my-lib".to_string()),
            version: Some("1.0.0".to_string()),
            packaging: Some("war".to_string()),
            ..Default::default()
        };

        let project = PomProject::new(pom).unwrap();
        assert_eq!(project.packaging(), "war");
    }

    #[test]
    fn test_pom_fetcher_new() {
        let fetcher = PomFetcher::with_defaults();
        assert_eq!(fetcher.fetcher().repositories().len(), 2);
    }

    #[test]
    fn test_pom_fetcher_custom_repositories() {
        let repos = RepositoryList::new();
        let fetcher = PomFetcher::new(repos);
        assert_eq!(fetcher.fetcher().repositories().len(), 0);
    }

    #[test]
    fn test_pom_fetch_error_display() {
        let error = PomFetchError::new("test error message");
        assert_eq!(error.to_string(), "test error message");
    }
}
