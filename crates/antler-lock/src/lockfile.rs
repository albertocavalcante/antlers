//! Core lockfile types.
//!
//! This module provides the main [`Lockfile`] struct and related types.

use std::fs;
use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;
use crate::hash::{compute_artifacts_hash, compute_input_hash};
use crate::reader;
use crate::writer;
use crate::{LockedArtifact, LockfileMetadata};

/// Current lockfile format version.
pub const LOCKFILE_VERSION: &str = "1";

/// Format identifier for our lockfile.
pub const LOCKFILE_FORMAT: &str = "antler-lock";

/// A repository entry in the lockfile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    /// The repository URL.
    pub url: String,

    /// The repository type (default: "maven").
    #[serde(default = "default_repo_type", skip_serializing_if = "is_maven_type")]
    pub repo_type: String,

    /// Unknown fields preserved for compatibility.
    #[serde(flatten)]
    pub extensions: IndexMap<String, Value>,
}

impl Repository {
    /// Creates a new Maven repository entry.
    #[must_use]
    pub fn maven(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            repo_type: "maven".to_string(),
            extensions: IndexMap::new(),
        }
    }

    /// Creates a new repository with a custom type.
    #[must_use]
    pub fn new(url: impl Into<String>, repo_type: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            repo_type: repo_type.into(),
            extensions: IndexMap::new(),
        }
    }
}

fn default_repo_type() -> String {
    "maven".to_string()
}

fn is_maven_type(repo_type: &String) -> bool {
    repo_type == "maven"
}

/// A version conflict that was resolved during dependency resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// The artifact coordinates (group:artifact) that had a conflict.
    pub artifact: String,

    /// The versions that were requested.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requested: Vec<String>,

    /// The version that was selected.
    pub selected: String,

    /// The strategy that was used to resolve the conflict.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub strategy: String,
}

impl Conflict {
    /// Creates a new conflict record.
    #[must_use]
    pub fn new(
        artifact: impl Into<String>,
        requested: Vec<String>,
        selected: impl Into<String>,
        strategy: impl Into<String>,
    ) -> Self {
        Self {
            artifact: artifact.into(),
            requested,
            selected: selected.into(),
            strategy: strategy.into(),
        }
    }
}

/// A lockfile containing resolved dependencies.
///
/// The lockfile format is designed to be:
/// - Forward/backward compatible (unknown fields are preserved)
/// - Human-readable (JSON with clear structure)
/// - Deterministic (sorted maps, consistent ordering)
/// - Interoperable (can read/write `rules_jvm_external` formats)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    /// The lockfile format version.
    pub version: String,

    /// The format identifier (always "antler-lock" for our format).
    pub format: String,

    /// Resolved artifacts, keyed by group:artifact.
    pub artifacts: IndexMap<String, LockedArtifact>,

    /// List of repositories used.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repositories: Vec<Repository>,

    /// Version conflicts that were resolved.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<Conflict>,

    /// Metadata about the lockfile generation.
    pub metadata: LockfileMetadata,

    /// Unknown fields preserved for forward/backward compatibility.
    #[serde(flatten)]
    pub extensions: IndexMap<String, Value>,
}

impl Lockfile {
    /// Creates a new empty lockfile.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: LOCKFILE_VERSION.to_string(),
            format: LOCKFILE_FORMAT.to_string(),
            artifacts: IndexMap::new(),
            repositories: Vec::new(),
            conflicts: Vec::new(),
            metadata: LockfileMetadata::new(),
            extensions: IndexMap::new(),
        }
    }

    /// Creates a lockfile with custom metadata.
    #[must_use]
    pub fn with_metadata(metadata: LockfileMetadata) -> Self {
        Self {
            version: LOCKFILE_VERSION.to_string(),
            format: LOCKFILE_FORMAT.to_string(),
            artifacts: IndexMap::new(),
            repositories: Vec::new(),
            conflicts: Vec::new(),
            metadata,
            extensions: IndexMap::new(),
        }
    }

    // === Reading ===

    /// Reads a lockfile from a string, auto-detecting the format.
    ///
    /// Supports:
    /// - Our native antler-lock format
    /// - `rules_jvm_external` V2 format
    /// - `rules_jvm_external` V1 format (legacy)
    ///
    /// # Errors
    ///
    /// Returns an error if the content cannot be parsed or the format
    /// is not recognized.
    pub fn read(content: &str) -> Result<Self> {
        reader::read(content)
    }

    /// Reads a lockfile from a file, auto-detecting the format.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn read_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        Self::read(&content)
    }

    // === Writing ===

    /// Writes the lockfile to our native format (pretty-printed JSON).
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn write(&self) -> Result<String> {
        writer::write_antler(self)
    }

    /// Writes the lockfile to a file in our native format.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn write_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = self.write()?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Writes the lockfile to `rules_jvm_external` V2 format.
    ///
    /// This is useful for integration with Bazel workspaces.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn write_v2(&self) -> Result<String> {
        writer::write_v2(self)
    }

    /// Writes the lockfile to a file in V2 format.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn write_v2_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = self.write_v2()?;
        fs::write(path, content)?;
        Ok(())
    }

    // === Artifact access ===

    /// Gets an artifact by its key (group:artifact).
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&LockedArtifact> {
        self.artifacts.get(key)
    }

    /// Gets a mutable reference to an artifact by its key.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut LockedArtifact> {
        self.artifacts.get_mut(key)
    }

    /// Inserts an artifact into the lockfile.
    pub fn insert(&mut self, key: impl Into<String>, artifact: LockedArtifact) {
        self.artifacts.insert(key.into(), artifact);
    }

    /// Returns an iterator over all artifacts.
    pub fn artifacts(&self) -> impl Iterator<Item = (&String, &LockedArtifact)> {
        self.artifacts.iter()
    }

    /// Returns the number of artifacts in the lockfile.
    #[must_use]
    pub fn len(&self) -> usize {
        self.artifacts.len()
    }

    /// Returns true if the lockfile has no artifacts.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }

    /// Returns true if the lockfile contains the given artifact key.
    #[must_use]
    pub fn contains(&self, key: &str) -> bool {
        self.artifacts.contains_key(key)
    }

    // === Repository management ===

    /// Adds a repository to the lockfile.
    pub fn add_repository(&mut self, repository: Repository) {
        // Avoid duplicates
        if !self.repositories.iter().any(|r| r.url == repository.url) {
            self.repositories.push(repository);
        }
    }

    // === Conflict management ===

    /// Adds a conflict record to the lockfile.
    pub fn add_conflict(&mut self, conflict: Conflict) {
        self.conflicts.push(conflict);
    }

    /// Returns true if there are any conflicts.
    #[must_use]
    pub const fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    // === Hash computation ===

    /// Computes a deterministic hash of the resolved artifacts.
    ///
    /// This hash can be used to detect changes in the lockfile.
    #[must_use]
    pub fn compute_hash(&self) -> i64 {
        compute_artifacts_hash(&self.artifacts)
    }

    /// Computes a hash of the root artifacts.
    ///
    /// This hash can be used to detect changes in the input.
    #[must_use]
    pub fn compute_input_hash(&self) -> i64 {
        compute_input_hash(&self.metadata.root_artifacts)
    }

    /// Updates the metadata hashes.
    pub fn update_hashes(&mut self) {
        self.metadata.input_hash = Some(self.compute_input_hash());
        self.metadata.resolved_hash = Some(self.compute_hash());
    }

    // === Sorting ===

    /// Sorts the artifacts by key for deterministic output.
    pub fn sort_artifacts(&mut self) {
        self.artifacts.sort_keys();
    }
}

impl Default for Lockfile {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_lockfile() {
        let lockfile = Lockfile::new();
        assert_eq!(lockfile.version, "1");
        assert_eq!(lockfile.format, "antler-lock");
        assert!(lockfile.is_empty());
    }

    #[test]
    fn test_insert_and_get() {
        let mut lockfile = Lockfile::new();

        lockfile.insert("com.example:lib", LockedArtifact::new("1.0.0", "abc123"));

        assert!(!lockfile.is_empty());
        assert_eq!(lockfile.len(), 1);
        assert!(lockfile.contains("com.example:lib"));

        let artifact = lockfile.get("com.example:lib").unwrap();
        assert_eq!(artifact.version, "1.0.0");
        assert_eq!(artifact.sha256, "abc123");
    }

    #[test]
    fn test_artifacts_iterator() {
        let mut lockfile = Lockfile::new();

        lockfile.insert("com.example:a", LockedArtifact::new("1.0", "abc"));
        lockfile.insert("com.example:b", LockedArtifact::new("2.0", "def"));

        let keys: Vec<_> = lockfile.artifacts().map(|(k, _)| k.as_str()).collect();
        assert!(keys.contains(&"com.example:a"));
        assert!(keys.contains(&"com.example:b"));
    }

    #[test]
    fn test_add_repository() {
        let mut lockfile = Lockfile::new();

        lockfile.add_repository(Repository::maven("https://repo1.maven.org/maven2/"));
        lockfile.add_repository(Repository::maven("https://repo1.maven.org/maven2/")); // duplicate

        assert_eq!(lockfile.repositories.len(), 1);
    }

    #[test]
    fn test_add_conflict() {
        let mut lockfile = Lockfile::new();

        assert!(!lockfile.has_conflicts());

        lockfile.add_conflict(Conflict::new(
            "com.example:lib",
            vec!["1.0".to_string(), "2.0".to_string()],
            "2.0",
            "highest-wins",
        ));

        assert!(lockfile.has_conflicts());
        assert_eq!(lockfile.conflicts.len(), 1);
    }

    #[test]
    fn test_compute_hash_deterministic() {
        let mut lockfile = Lockfile::new();
        lockfile.insert("com.example:lib", LockedArtifact::new("1.0.0", "abc123"));

        let hash1 = lockfile.compute_hash();
        let hash2 = lockfile.compute_hash();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_sort_artifacts() {
        let mut lockfile = Lockfile::new();
        lockfile.insert("com.example:z", LockedArtifact::new("1.0", "abc"));
        lockfile.insert("com.example:a", LockedArtifact::new("1.0", "def"));
        lockfile.insert("com.example:m", LockedArtifact::new("1.0", "ghi"));

        lockfile.sort_artifacts();

        let keys: Vec<_> = lockfile.artifacts().map(|(k, _)| k.as_str()).collect();
        assert_eq!(
            keys,
            vec!["com.example:a", "com.example:m", "com.example:z"]
        );
    }

    #[test]
    fn test_repository_types() {
        let maven = Repository::maven("https://repo1.maven.org/maven2/");
        assert_eq!(maven.repo_type, "maven");

        let custom = Repository::new("https://example.com/", "custom");
        assert_eq!(custom.repo_type, "custom");
    }

    #[test]
    fn test_roundtrip() {
        let mut lockfile = Lockfile::new();
        lockfile.insert(
            "com.example:lib",
            LockedArtifact::new("1.0.0", "abc123")
                .with_repository("https://repo1.maven.org/maven2/")
                .with_dependencies(vec!["com.example:dep".to_string()]),
        );
        lockfile.add_repository(Repository::maven("https://repo1.maven.org/maven2/"));

        let json = lockfile.write().unwrap();
        let parsed = Lockfile::read(&json).unwrap();

        assert_eq!(lockfile.len(), parsed.len());
        assert_eq!(
            lockfile.get("com.example:lib").unwrap().version,
            parsed.get("com.example:lib").unwrap().version
        );
    }
}
