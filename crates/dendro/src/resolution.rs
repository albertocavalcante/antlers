//! Resolution result types.
//!
//! This module provides [`Resolution`] and [`ResolvedArtifact`] for representing
//! the results of dependency resolution.

use std::collections::HashMap;

use gav::{Artifact, Coordinates};
use serde::{Deserialize, Serialize};

use crate::VersionConflict;

/// A resolved artifact with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedArtifact {
    /// The artifact coordinates.
    pub artifact: Artifact,

    /// SHA1 checksum of the artifact.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,

    /// SHA256 checksum of the artifact.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,

    /// The repository where the artifact was found.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

impl ResolvedArtifact {
    /// Creates a new resolved artifact.
    #[must_use]
    pub const fn new(artifact: Artifact) -> Self {
        Self {
            artifact,
            sha1: None,
            sha256: None,
            repository: None,
        }
    }

    /// Sets the SHA1 checksum.
    #[must_use]
    pub fn with_sha1(mut self, sha1: impl Into<String>) -> Self {
        self.sha1 = Some(sha1.into());
        self
    }

    /// Sets the SHA256 checksum.
    #[must_use]
    pub fn with_sha256(mut self, sha256: impl Into<String>) -> Self {
        self.sha256 = Some(sha256.into());
        self
    }

    /// Sets the repository.
    #[must_use]
    pub fn with_repository(mut self, repository: impl Into<String>) -> Self {
        self.repository = Some(repository.into());
        self
    }

    /// Returns the coordinates (group:artifact) for this artifact.
    #[must_use]
    pub fn coordinates(&self) -> Coordinates {
        self.artifact.coordinates.clone()
    }

    /// Returns the full coordinate string (group:artifact:version).
    #[must_use]
    pub fn coordinate(&self) -> String {
        self.artifact.coordinate()
    }
}

/// The result of dependency resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    /// The root artifact that was resolved.
    pub root: Artifact,

    /// All resolved artifacts (including transitive dependencies).
    pub artifacts: Vec<ResolvedArtifact>,

    /// Version conflicts encountered during resolution.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub conflicts: Vec<VersionConflict>,
}

impl Resolution {
    /// Creates a new resolution result.
    #[must_use]
    pub const fn new(root: Artifact) -> Self {
        Self {
            root,
            artifacts: Vec::new(),
            conflicts: Vec::new(),
        }
    }

    /// Adds a resolved artifact.
    pub fn add_artifact(&mut self, artifact: ResolvedArtifact) {
        self.artifacts.push(artifact);
    }

    /// Adds a version conflict.
    pub fn add_conflict(&mut self, conflict: VersionConflict) {
        self.conflicts.push(conflict);
    }

    /// Returns the total number of resolved artifacts.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.artifacts.len()
    }

    /// Returns true if no artifacts were resolved.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }

    /// Returns a slice of all resolved artifacts.
    #[must_use]
    pub fn artifacts(&self) -> &[ResolvedArtifact] {
        &self.artifacts
    }

    /// Returns the artifacts indexed by their coordinates (group:artifact).
    #[must_use]
    pub fn by_coordinates(&self) -> HashMap<Coordinates, &ResolvedArtifact> {
        self.artifacts
            .iter()
            .map(|a| (a.coordinates(), a))
            .collect()
    }

    /// Returns the artifacts indexed by their full coordinate string.
    #[must_use]
    pub fn by_coordinate_string(&self) -> HashMap<String, &ResolvedArtifact> {
        self.artifacts.iter().map(|a| (a.coordinate(), a)).collect()
    }

    /// Returns an iterator over artifacts that have conflicts.
    pub fn conflicting_artifacts(&self) -> impl Iterator<Item = &str> {
        self.conflicts.iter().map(|c| c.artifact.as_str())
    }

    /// Returns true if there were any version conflicts.
    #[must_use]
    pub const fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    /// Returns the number of conflicts.
    #[must_use]
    pub const fn conflict_count(&self) -> usize {
        self.conflicts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolved_artifact() {
        let artifact = Artifact::new("com.example", "lib", "1.0.0");
        let resolved = ResolvedArtifact::new(artifact)
            .with_sha1("abc123")
            .with_sha256("def456")
            .with_repository("central");

        assert_eq!(resolved.sha1, Some("abc123".to_string()));
        assert_eq!(resolved.sha256, Some("def456".to_string()));
        assert_eq!(resolved.repository, Some("central".to_string()));
        assert_eq!(resolved.coordinate(), "com.example:lib:1.0.0");
    }

    #[test]
    fn test_resolution() {
        let root = Artifact::new("com.example", "root", "1.0.0");
        let mut resolution = Resolution::new(root);

        assert!(resolution.is_empty());
        assert_eq!(resolution.len(), 0);

        let dep1 = ResolvedArtifact::new(Artifact::new("com.example", "dep1", "2.0.0"));
        let dep2 = ResolvedArtifact::new(Artifact::new("com.example", "dep2", "3.0.0"));

        resolution.add_artifact(dep1);
        resolution.add_artifact(dep2);

        assert!(!resolution.is_empty());
        assert_eq!(resolution.len(), 2);

        let by_coord = resolution.by_coordinates();
        assert!(by_coord.contains_key(&Coordinates::new("com.example", "dep1")));
    }

    #[test]
    fn test_resolution_conflicts() {
        let root = Artifact::new("com.example", "root", "1.0.0");
        let mut resolution = Resolution::new(root);

        assert!(!resolution.has_conflicts());
        assert_eq!(resolution.conflict_count(), 0);

        resolution.add_conflict(VersionConflict::new(
            "com.example:lib",
            vec!["1.0".to_string(), "2.0".to_string()],
            "2.0",
            "highest-wins",
        ));

        assert!(resolution.has_conflicts());
        assert_eq!(resolution.conflict_count(), 1);
    }
}
