//! Writer for `rules_jvm_external` V2 format (for interoperability).
//!
//! This writer converts our native format back to V2 format,
//! allowing integration with Bazel and `rules_jvm_external`.

use indexmap::IndexMap;
use serde::Serialize;
use serde_json::Value;

use crate::Lockfile;
use crate::error::Result;

/// V2 lockfile structure for serialization.
#[derive(Debug, Serialize)]
struct V2Lockfile {
    version: String,
    artifacts: IndexMap<String, V2Artifact>,
    dependencies: IndexMap<String, Vec<String>>,
    packages: IndexMap<String, Vec<String>>,
    services: IndexMap<String, IndexMap<String, Vec<String>>>,
    repositories: IndexMap<String, Vec<String>>,

    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    conflict_resolution: IndexMap<String, String>,

    #[serde(
        rename = "__INPUT_ARTIFACTS_HASH",
        skip_serializing_if = "Option::is_none"
    )]
    input_hash: Option<i64>,

    #[serde(
        rename = "__RESOLVED_ARTIFACTS_HASH",
        skip_serializing_if = "Option::is_none"
    )]
    resolved_hash: Option<i64>,

    #[serde(flatten)]
    extensions: IndexMap<String, Value>,
}

/// V2 artifact entry.
#[derive(Debug, Serialize)]
struct V2Artifact {
    version: String,
    shasums: V2Shasums,

    #[serde(flatten)]
    extensions: IndexMap<String, Value>,
}

/// V2 checksum structure.
#[derive(Debug, Serialize)]
struct V2Shasums {
    sha256: String,
}

/// Writes a lockfile to `rules_jvm_external` V2 format.
///
/// This is useful for integration with Bazel workspaces that use
/// `rules_jvm_external`.
pub fn write_v2(lockfile: &Lockfile) -> Result<String> {
    let v2 = convert_to_v2(lockfile);
    let json = serde_json::to_string_pretty(&v2)?;
    Ok(json)
}

/// Writes a lockfile to compact V2 format.
pub fn write_v2_compact(lockfile: &Lockfile) -> Result<String> {
    let v2 = convert_to_v2(lockfile);
    let json = serde_json::to_string(&v2)?;
    Ok(json)
}

/// Converts our lockfile to V2 structure.
fn convert_to_v2(lockfile: &Lockfile) -> V2Lockfile {
    let mut artifacts = IndexMap::new();
    let mut dependencies = IndexMap::new();
    let mut packages = IndexMap::new();
    let mut services = IndexMap::new();

    for (key, artifact) in &lockfile.artifacts {
        // Build V2 artifact
        artifacts.insert(
            key.clone(),
            V2Artifact {
                version: artifact.version.clone(),
                shasums: V2Shasums {
                    sha256: artifact.sha256.clone(),
                },
                extensions: artifact.extensions.clone(),
            },
        );

        // Add dependencies if present
        if !artifact.dependencies.is_empty() {
            dependencies.insert(key.clone(), artifact.dependencies.clone());
        }

        // Add packages if present
        if !artifact.packages.is_empty() {
            packages.insert(key.clone(), artifact.packages.clone());
        }

        // Add services if present
        if !artifact.services.is_empty() {
            services.insert(key.clone(), artifact.services.clone());
        }
    }

    // Build repository map (url -> list of artifacts)
    let mut repositories: IndexMap<String, Vec<String>> = IndexMap::new();
    for (key, artifact) in &lockfile.artifacts {
        if let Some(ref url) = artifact.repository {
            repositories
                .entry(url.clone())
                .or_default()
                .push(key.clone());
        }
    }

    // Also add any repositories from the lockfile that might not have artifacts
    for repo in &lockfile.repositories {
        repositories.entry(repo.url.clone()).or_default();
    }

    // Build conflict resolution map
    let mut conflict_resolution = IndexMap::new();
    for conflict in &lockfile.conflicts {
        conflict_resolution.insert(conflict.artifact.clone(), conflict.selected.clone());
    }

    V2Lockfile {
        version: "2".to_string(),
        artifacts,
        dependencies,
        packages,
        services,
        repositories,
        conflict_resolution,
        input_hash: lockfile.metadata.input_hash,
        resolved_hash: lockfile.metadata.resolved_hash,
        extensions: lockfile.extensions.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Conflict, LockedArtifact, LockfileMetadata, Repository};

    fn create_test_lockfile() -> Lockfile {
        let mut artifacts = IndexMap::new();

        let guava = LockedArtifact::new("33.0.0-jre", "abc123def456")
            .with_repository("https://repo1.maven.org/maven2/")
            .with_dependencies(vec!["com.google.guava:failureaccess".to_string()])
            .with_packages(vec![
                "com.google.common.base".to_string(),
                "com.google.common.collect".to_string(),
            ]);

        let failureaccess = LockedArtifact::new("1.0.2", "789ghi012jkl")
            .with_repository("https://repo1.maven.org/maven2/");

        artifacts.insert("com.google.guava:guava".to_string(), guava);
        artifacts.insert("com.google.guava:failureaccess".to_string(), failureaccess);

        Lockfile {
            version: "1".to_string(),
            format: "antler-lock".to_string(),
            artifacts,
            repositories: vec![Repository::maven("https://repo1.maven.org/maven2/")],
            conflicts: vec![Conflict {
                artifact: "com.example:lib".to_string(),
                requested: vec!["1.0".to_string(), "2.0".to_string()],
                selected: "2.0".to_string(),
                strategy: "highest-wins".to_string(),
            }],
            metadata: LockfileMetadata::with_generator("test")
                .with_input_hash(12345)
                .with_resolved_hash(67890),
            extensions: IndexMap::new(),
        }
    }

    #[test]
    fn test_write_v2_basic() {
        let lockfile = create_test_lockfile();
        let json = write_v2(&lockfile).unwrap();

        assert!(json.contains("\"version\": \"2\""));
        assert!(json.contains("\"com.google.guava:guava\""));
    }

    #[test]
    fn test_write_v2_artifacts() {
        let lockfile = create_test_lockfile();
        let json = write_v2(&lockfile).unwrap();

        // Parse back to check structure
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        let artifacts = parsed.get("artifacts").unwrap().as_object().unwrap();
        let guava = artifacts.get("com.google.guava:guava").unwrap();

        assert_eq!(
            guava.get("version").unwrap().as_str().unwrap(),
            "33.0.0-jre"
        );
        assert_eq!(
            guava
                .get("shasums")
                .unwrap()
                .get("sha256")
                .unwrap()
                .as_str()
                .unwrap(),
            "abc123def456"
        );
    }

    #[test]
    fn test_write_v2_dependencies() {
        let lockfile = create_test_lockfile();
        let json = write_v2(&lockfile).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let deps = parsed.get("dependencies").unwrap().as_object().unwrap();
        let guava_deps = deps
            .get("com.google.guava:guava")
            .unwrap()
            .as_array()
            .unwrap();

        assert_eq!(guava_deps.len(), 1);
        assert_eq!(
            guava_deps[0].as_str().unwrap(),
            "com.google.guava:failureaccess"
        );
    }

    #[test]
    fn test_write_v2_packages() {
        let lockfile = create_test_lockfile();
        let json = write_v2(&lockfile).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let pkgs = parsed.get("packages").unwrap().as_object().unwrap();
        let guava_pkgs = pkgs
            .get("com.google.guava:guava")
            .unwrap()
            .as_array()
            .unwrap();

        assert_eq!(guava_pkgs.len(), 2);
    }

    #[test]
    fn test_write_v2_repositories() {
        let lockfile = create_test_lockfile();
        let json = write_v2(&lockfile).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let repos = parsed.get("repositories").unwrap().as_object().unwrap();

        assert!(repos.contains_key("https://repo1.maven.org/maven2/"));
    }

    #[test]
    fn test_write_v2_hashes() {
        let lockfile = create_test_lockfile();
        let json = write_v2(&lockfile).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(
            parsed
                .get("__INPUT_ARTIFACTS_HASH")
                .unwrap()
                .as_i64()
                .unwrap(),
            12345
        );
        assert_eq!(
            parsed
                .get("__RESOLVED_ARTIFACTS_HASH")
                .unwrap()
                .as_i64()
                .unwrap(),
            67890
        );
    }

    #[test]
    fn test_write_v2_conflicts() {
        let lockfile = create_test_lockfile();
        let json = write_v2(&lockfile).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let conflicts = parsed
            .get("conflict_resolution")
            .unwrap()
            .as_object()
            .unwrap();

        assert_eq!(
            conflicts.get("com.example:lib").unwrap().as_str().unwrap(),
            "2.0"
        );
    }
}
