//! Lockfile metadata.
//!
//! This module provides [`LockfileMetadata`] for storing information about
//! how and when the lockfile was generated.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Metadata about the lockfile generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockfileMetadata {
    /// The tool that generated this lockfile (e.g., "antler 0.1.0").
    pub generated_by: String,

    /// ISO 8601 timestamp of when the lockfile was generated.
    pub generated_at: String,

    /// The root artifacts that were resolved (as coordinate strings).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub root_artifacts: Vec<String>,

    /// Hash of the input artifacts (for change detection).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_hash: Option<i64>,

    /// Hash of the resolved artifacts (for integrity checking).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_hash: Option<i64>,

    /// Unknown fields preserved for forward/backward compatibility.
    #[serde(flatten)]
    pub extensions: IndexMap<String, Value>,
}

impl LockfileMetadata {
    /// Creates new metadata with the current timestamp.
    #[must_use]
    pub fn new() -> Self {
        let now = chrono::Utc::now();
        Self {
            generated_by: format!("antler {}", env!("CARGO_PKG_VERSION")),
            generated_at: now.to_rfc3339(),
            root_artifacts: Vec::new(),
            input_hash: None,
            resolved_hash: None,
            extensions: IndexMap::new(),
        }
    }

    /// Creates metadata with a custom generator string.
    #[must_use]
    pub fn with_generator(generator: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            generated_by: generator.into(),
            generated_at: now.to_rfc3339(),
            root_artifacts: Vec::new(),
            input_hash: None,
            resolved_hash: None,
            extensions: IndexMap::new(),
        }
    }

    /// Sets the root artifacts.
    #[must_use]
    pub fn with_root_artifacts(mut self, roots: Vec<String>) -> Self {
        self.root_artifacts = roots;
        self
    }

    /// Sets the input hash.
    #[must_use]
    pub const fn with_input_hash(mut self, hash: i64) -> Self {
        self.input_hash = Some(hash);
        self
    }

    /// Sets the resolved hash.
    #[must_use]
    pub const fn with_resolved_hash(mut self, hash: i64) -> Self {
        self.resolved_hash = Some(hash);
        self
    }

    /// Adds a root artifact.
    pub fn add_root_artifact(&mut self, artifact: impl Into<String>) {
        self.root_artifacts.push(artifact.into());
    }
}

impl Default for LockfileMetadata {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_metadata() {
        let meta = LockfileMetadata::new();
        assert!(meta.generated_by.starts_with("antler"));
        assert!(!meta.generated_at.is_empty());
        assert!(meta.root_artifacts.is_empty());
        assert!(meta.input_hash.is_none());
        assert!(meta.resolved_hash.is_none());
    }

    #[test]
    fn test_with_generator() {
        let meta = LockfileMetadata::with_generator("test-tool 1.0");
        assert_eq!(meta.generated_by, "test-tool 1.0");
    }

    #[test]
    fn test_builder_pattern() {
        let meta = LockfileMetadata::new()
            .with_root_artifacts(vec!["com.example:lib:1.0".to_string()])
            .with_input_hash(12345)
            .with_resolved_hash(67890);

        assert_eq!(meta.root_artifacts, vec!["com.example:lib:1.0"]);
        assert_eq!(meta.input_hash, Some(12345));
        assert_eq!(meta.resolved_hash, Some(67890));
    }

    #[test]
    fn test_serialization() {
        let meta = LockfileMetadata::new()
            .with_root_artifacts(vec!["com.example:lib:1.0".to_string()])
            .with_input_hash(12345);

        let json = serde_json::to_string_pretty(&meta).unwrap();
        let parsed: LockfileMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(meta.generated_by, parsed.generated_by);
        assert_eq!(meta.generated_at, parsed.generated_at);
        assert_eq!(meta.root_artifacts, parsed.root_artifacts);
        assert_eq!(meta.input_hash, parsed.input_hash);
    }
}
