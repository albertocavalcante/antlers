//! Reader for `rules_jvm_external` V2 lockfile format.
//!
//! V2 is the current format used by `rules_jvm_external` for Bazel.
//!
//! # Format Reference
//!
//! The V2 format is defined in the `rules_jvm_external` repository:
//! - Format spec: <https://github.com/bazelbuild/rules_jvm_external/blob/master/docs/api.md#maven_installjson-lockfile>
//! - Writer impl: <https://github.com/bazelbuild/rules_jvm_external/blob/master/private/tools/java/com/github/bazelbuild/rules_jvm_external/resolver/LockfileWriter.java>
//!
//! # V2 Structure
//!
//! ```json
//! {
//!   "version": "2",
//!   "artifacts": {
//!     "com.google.guava:guava": {
//!       "version": "33.0.0-jre",
//!       "shasums": { "sha256": "..." }
//!     }
//!   },
//!   "dependencies": {
//!     "com.google.guava:guava": ["com.google.guava:failureaccess", ...]
//!   },
//!   "packages": {
//!     "com.google.guava:guava": ["com.google.common.base", ...]
//!   },
//!   "services": {
//!     "com.google.guava:guava": { "java.nio.file.spi.FileSystemProvider": [...] }
//!   },
//!   "repositories": {
//!     "https://repo1.maven.org/maven2/": ["com.google.guava:guava", ...]
//!   }
//! }
//! ```
//!
//! # Mapping to Our Format
//!
//! | V2 Field | Our Field | Notes |
//! |----------|-----------|-------|
//! | `artifacts[key].version` | `artifacts[key].version` | Direct mapping |
//! | `artifacts[key].shasums.sha256` | `artifacts[key].sha256` | We only use SHA-256 |
//! | `dependencies[key]` | `artifacts[key].dependencies` | Flattened into artifact |
//! | `packages[key]` | `artifacts[key].packages` | Flattened into artifact |
//! | `services[key]` | `artifacts[key].services` | Flattened into artifact |
//! | `repositories` (url→artifacts) | `artifacts[key].repository` | Reversed lookup |
//! | `__INPUT_ARTIFACTS_HASH` | `metadata.input_hash` | Renamed |
//! | `__RESOLVED_ARTIFACTS_HASH` | `metadata.resolved_hash` | Renamed |

use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;
use tracing::warn;

use crate::error::{Error, Result};
use crate::{Conflict, LockedArtifact, Lockfile, LockfileMetadata, Repository};

/// Raw V2 format structure for deserialization.
#[derive(Debug, Deserialize)]
struct V2Lockfile {
    #[allow(dead_code)]
    version: String,

    #[serde(default)]
    artifacts: IndexMap<String, V2Artifact>,

    #[serde(default)]
    dependencies: IndexMap<String, Vec<String>>,

    #[serde(default)]
    packages: IndexMap<String, Vec<String>>,

    #[serde(default)]
    services: IndexMap<String, IndexMap<String, Vec<String>>>,

    #[serde(default)]
    repositories: IndexMap<String, Vec<String>>,

    #[serde(default)]
    conflict_resolution: IndexMap<String, String>,

    #[serde(rename = "__INPUT_ARTIFACTS_HASH")]
    input_hash: Option<i64>,

    #[serde(rename = "__RESOLVED_ARTIFACTS_HASH")]
    resolved_hash: Option<i64>,

    #[serde(flatten)]
    extensions: IndexMap<String, Value>,
}

/// V2 artifact entry.
#[derive(Debug, Deserialize)]
struct V2Artifact {
    version: String,

    #[serde(default)]
    shasums: Option<V2Shasums>,

    #[serde(flatten)]
    extensions: IndexMap<String, Value>,
}

/// V2 checksum structure.
#[derive(Debug, Deserialize)]
struct V2Shasums {
    #[serde(default)]
    sha256: Option<String>,

    #[serde(default)]
    sha512: Option<String>,

    #[serde(default)]
    sha1: Option<String>,

    #[serde(default)]
    md5: Option<String>,
}

/// Reads a `rules_jvm_external` V2 lockfile and converts it to our format.
///
/// # Errors
///
/// Returns an error if:
/// - JSON parsing fails
/// - Any artifact is missing a SHA-256 checksum
pub fn read_v2(content: &str) -> Result<Lockfile> {
    let v2: V2Lockfile = serde_json::from_str(content)?;

    // V2 stores repositories as { url: [artifact_keys...] }
    // We need the reverse: artifact_key -> url
    let repo_lookup = build_repository_lookup(&v2.repositories);

    // Convert artifacts, flattening the separate V2 maps into our per-artifact structure.
    // V2 splits data across multiple top-level maps (artifacts, dependencies, packages, services)
    // while we embed everything in each artifact entry.
    let mut artifacts = IndexMap::new();
    for (key, v2_artifact) in v2.artifacts {
        let sha256 = extract_sha256(v2_artifact.shasums.as_ref(), &key)?;

        let mut artifact = LockedArtifact::new(&v2_artifact.version, sha256);

        // Reverse lookup: find which repository URL contains this artifact
        if let Some(url) = repo_lookup.get(&key) {
            artifact.repository = Some(url.clone());
        }

        // V2 stores dependencies in a separate top-level map
        if let Some(deps) = v2.dependencies.get(&key) {
            artifact.dependencies.clone_from(deps);
        }

        // V2 stores Java packages (for strict deps) in a separate map
        if let Some(pkgs) = v2.packages.get(&key) {
            artifact.packages.clone_from(pkgs);
        }

        // V2 stores SPI services in a separate map
        if let Some(svc) = v2.services.get(&key) {
            artifact.services.clone_from(svc);
        }

        // Preserve any unknown fields for forward compatibility
        artifact.extensions = v2_artifact.extensions;

        artifacts.insert(key, artifact);
    }

    // Convert repository map keys to our list format
    let repositories: Vec<Repository> = v2.repositories.keys().map(Repository::maven).collect();

    // V2's conflict_resolution only stores artifact -> selected_version,
    // it doesn't preserve the list of requested versions or strategy used
    let conflicts: Vec<Conflict> = v2
        .conflict_resolution
        .iter()
        .map(|(artifact, selected)| {
            Conflict {
                artifact: artifact.clone(),
                requested: vec![], // V2 format limitation: doesn't store requested versions
                selected: selected.clone(),
                strategy: "unknown".to_string(), // V2 format limitation: doesn't store strategy
            }
        })
        .collect();

    // Build metadata
    let mut metadata = LockfileMetadata::with_generator("rules_jvm_external (converted)");
    metadata.input_hash = v2.input_hash;
    metadata.resolved_hash = v2.resolved_hash;

    // Filter out known fields from extensions
    let mut extensions = v2.extensions;
    extensions.retain(|k, _| {
        !matches!(
            k.as_str(),
            "version"
                | "artifacts"
                | "dependencies"
                | "packages"
                | "services"
                | "repositories"
                | "conflict_resolution"
                | "__INPUT_ARTIFACTS_HASH"
                | "__RESOLVED_ARTIFACTS_HASH"
        )
    });

    Ok(Lockfile {
        version: "1".to_string(),
        format: "antlers-lock".to_string(),
        artifacts,
        repositories,
        conflicts,
        metadata,
        extensions,
    })
}

/// Builds a reverse lookup from artifact key to repository URL.
fn build_repository_lookup(
    repositories: &IndexMap<String, Vec<String>>,
) -> IndexMap<String, String> {
    let mut lookup = IndexMap::new();
    for (url, artifacts) in repositories {
        for artifact in artifacts {
            lookup.insert(artifact.clone(), url.clone());
        }
    }
    lookup
}

/// Extracts SHA-256 checksum, preferring it over other algorithms.
fn extract_sha256(shasums: Option<&V2Shasums>, artifact_key: &str) -> Result<String> {
    let shasums = shasums
        .ok_or_else(|| Error::MissingField(format!("shasums for artifact {artifact_key}")))?;

    // Prefer SHA-256
    if let Some(ref sha256) = shasums.sha256 {
        return Ok(sha256.clone());
    }

    // Fall back to SHA-512 if available (we'll note this)
    if let Some(ref sha512) = shasums.sha512 {
        warn!(
            artifact = artifact_key,
            "using SHA-512 instead of SHA-256 for artifact"
        );
        return Ok(sha512.clone());
    }

    // Warn about weak checksums
    if shasums.sha1.is_some() || shasums.md5.is_some() {
        warn!(
            artifact = artifact_key,
            "artifact only has weak checksums (SHA-1/MD5), skipping"
        );
    }

    Err(Error::MissingField(format!(
        "sha256 checksum for artifact {artifact_key}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    const V2_LOCKFILE: &str = r#"{
        "version": "2",
        "artifacts": {
            "com.google.guava:guava": {
                "version": "33.0.0-jre",
                "shasums": {
                    "sha256": "abc123def456"
                }
            },
            "com.google.guava:failureaccess": {
                "version": "1.0.2",
                "shasums": {
                    "sha256": "789ghi012jkl"
                }
            }
        },
        "dependencies": {
            "com.google.guava:guava": [
                "com.google.guava:failureaccess"
            ]
        },
        "packages": {
            "com.google.guava:guava": [
                "com.google.common.base",
                "com.google.common.collect"
            ]
        },
        "services": {
            "com.google.guava:guava": {
                "java.nio.file.spi.FileSystemProvider": [
                    "com.google.common.jimfs.JimfsFileSystemProvider"
                ]
            }
        },
        "repositories": {
            "https://repo1.maven.org/maven2/": [
                "com.google.guava:guava",
                "com.google.guava:failureaccess"
            ]
        },
        "__INPUT_ARTIFACTS_HASH": 12345,
        "__RESOLVED_ARTIFACTS_HASH": 67890
    }"#;

    #[test]
    fn test_read_v2_basic() {
        let lockfile = read_v2(V2_LOCKFILE).unwrap();

        assert_eq!(lockfile.version, "1");
        assert_eq!(lockfile.format, "antlers-lock");
        assert_eq!(lockfile.artifacts.len(), 2);
    }

    #[test]
    fn test_read_v2_artifacts() {
        let lockfile = read_v2(V2_LOCKFILE).unwrap();

        let guava = lockfile.get("com.google.guava:guava").unwrap();
        assert_eq!(guava.version, "33.0.0-jre");
        assert_eq!(guava.sha256, "abc123def456");
        assert_eq!(
            guava.repository,
            Some("https://repo1.maven.org/maven2/".to_string())
        );
    }

    #[test]
    fn test_read_v2_dependencies() {
        let lockfile = read_v2(V2_LOCKFILE).unwrap();

        let guava = lockfile.get("com.google.guava:guava").unwrap();
        assert_eq!(guava.dependencies, vec!["com.google.guava:failureaccess"]);
    }

    #[test]
    fn test_read_v2_packages() {
        let lockfile = read_v2(V2_LOCKFILE).unwrap();

        let guava = lockfile.get("com.google.guava:guava").unwrap();
        assert_eq!(
            guava.packages,
            vec!["com.google.common.base", "com.google.common.collect"]
        );
    }

    #[test]
    fn test_read_v2_services() {
        let lockfile = read_v2(V2_LOCKFILE).unwrap();

        let guava = lockfile.get("com.google.guava:guava").unwrap();
        assert!(
            guava
                .services
                .contains_key("java.nio.file.spi.FileSystemProvider")
        );
    }

    #[test]
    fn test_read_v2_repositories() {
        let lockfile = read_v2(V2_LOCKFILE).unwrap();

        assert_eq!(lockfile.repositories.len(), 1);
        assert_eq!(
            lockfile.repositories[0].url,
            "https://repo1.maven.org/maven2/"
        );
    }

    #[test]
    fn test_read_v2_metadata() {
        let lockfile = read_v2(V2_LOCKFILE).unwrap();

        assert_eq!(lockfile.metadata.input_hash, Some(12345));
        assert_eq!(lockfile.metadata.resolved_hash, Some(67890));
    }
}
