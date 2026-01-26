//! Deterministic hashing for lockfile integrity.
//!
//! This module provides functions for computing deterministic hashes
//! of lockfile contents for change detection.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use indexmap::IndexMap;

use crate::LockedArtifact;

/// Computes a deterministic hash of the artifact map.
///
/// This hash can be used to detect changes in the resolved dependencies.
/// The hash is stable across runs as long as the artifacts are the same.
#[allow(clippy::cast_possible_wrap)]
pub fn compute_artifacts_hash(artifacts: &IndexMap<String, LockedArtifact>) -> i64 {
    let mut hasher = DefaultHasher::new();

    // Sort keys for deterministic ordering
    let mut keys: Vec<_> = artifacts.keys().collect();
    keys.sort();

    for key in keys {
        key.hash(&mut hasher);
        if let Some(artifact) = artifacts.get(key) {
            artifact.version.hash(&mut hasher);
            artifact.sha256.hash(&mut hasher);
            artifact.packaging.hash(&mut hasher);

            if let Some(ref classifier) = artifact.classifier {
                classifier.hash(&mut hasher);
            }

            if let Some(ref repository) = artifact.repository {
                repository.hash(&mut hasher);
            }

            // Hash dependencies in sorted order
            let mut deps: Vec<_> = artifact.dependencies.iter().collect();
            deps.sort();
            for dep in deps {
                dep.hash(&mut hasher);
            }
        }
    }

    // Convert u64 to i64 (for JSON compatibility)
    hasher.finish() as i64
}

/// Computes a deterministic hash of input artifact coordinates.
///
/// This hash can be used to detect changes in the input (requested) artifacts.
#[allow(clippy::cast_possible_wrap)]
pub fn compute_input_hash(roots: &[String]) -> i64 {
    let mut hasher = DefaultHasher::new();

    // Sort for deterministic ordering
    let mut sorted: Vec<_> = roots.iter().collect();
    sorted.sort();

    for root in sorted {
        root.hash(&mut hasher);
    }

    hasher.finish() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_artifacts_hash_deterministic() {
        let mut artifacts = IndexMap::new();
        artifacts.insert(
            "com.example:lib".to_string(),
            LockedArtifact::new("1.0.0", "abc123"),
        );
        artifacts.insert(
            "com.example:dep".to_string(),
            LockedArtifact::new("2.0.0", "def456"),
        );

        let hash1 = compute_artifacts_hash(&artifacts);
        let hash2 = compute_artifacts_hash(&artifacts);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_artifacts_hash_order_independent() {
        let mut artifacts1 = IndexMap::new();
        artifacts1.insert(
            "com.example:lib".to_string(),
            LockedArtifact::new("1.0.0", "abc123"),
        );
        artifacts1.insert(
            "com.example:dep".to_string(),
            LockedArtifact::new("2.0.0", "def456"),
        );

        let mut artifacts2 = IndexMap::new();
        artifacts2.insert(
            "com.example:dep".to_string(),
            LockedArtifact::new("2.0.0", "def456"),
        );
        artifacts2.insert(
            "com.example:lib".to_string(),
            LockedArtifact::new("1.0.0", "abc123"),
        );

        assert_eq!(
            compute_artifacts_hash(&artifacts1),
            compute_artifacts_hash(&artifacts2)
        );
    }

    #[test]
    fn test_compute_artifacts_hash_changes() {
        let mut artifacts1 = IndexMap::new();
        artifacts1.insert(
            "com.example:lib".to_string(),
            LockedArtifact::new("1.0.0", "abc123"),
        );

        let mut artifacts2 = IndexMap::new();
        artifacts2.insert(
            "com.example:lib".to_string(),
            LockedArtifact::new("2.0.0", "abc123"),
        );

        assert_ne!(
            compute_artifacts_hash(&artifacts1),
            compute_artifacts_hash(&artifacts2)
        );
    }

    #[test]
    fn test_compute_input_hash_deterministic() {
        let roots = vec!["com.example:lib:1.0".to_string()];

        let hash1 = compute_input_hash(&roots);
        let hash2 = compute_input_hash(&roots);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_input_hash_order_independent() {
        let roots1 = vec![
            "com.example:lib:1.0".to_string(),
            "com.example:dep:2.0".to_string(),
        ];
        let roots2 = vec![
            "com.example:dep:2.0".to_string(),
            "com.example:lib:1.0".to_string(),
        ];

        assert_eq!(compute_input_hash(&roots1), compute_input_hash(&roots2));
    }
}
