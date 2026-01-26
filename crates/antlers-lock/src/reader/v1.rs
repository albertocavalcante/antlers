//! Reader for `rules_jvm_external` V1 lockfile format (legacy).
//!
//! V1 is the legacy format, superseded by V2. It uses a different structure
//! with a flat `dependency_tree` array instead of maps.
//!
//! # Format Reference
//!
//! The V1 format was the original format in `rules_jvm_external`:
//! - Historical context: <https://github.com/bazelbuild/rules_jvm_external/issues/365>
//!
//! # V1 Structure
//! ```json
//! {
//!   "dependency_tree": {
//!     "version": "0.1.0",
//!     "dependencies": [
//!       {
//!         "coord": "com.google.guava:guava:33.0.0-jre",
//!         "file": "v1/https/repo1.maven.org/maven2/com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar",
//!         "directDependencies": ["com.google.guava:failureaccess:1.0.2"],
//!         "dependencies": ["com.google.guava:failureaccess:1.0.2"],
//!         "sha256": "abc123..."
//!       }
//!     ]
//!   }
//! }
//! ```

use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;
use tracing::warn;

use crate::error::{Error, Result};
use crate::{LockedArtifact, Lockfile, LockfileMetadata, Repository};

/// Raw V1 format structure for deserialization.
#[derive(Debug, Deserialize)]
struct V1Lockfile {
    dependency_tree: V1DependencyTree,

    #[serde(flatten)]
    extensions: IndexMap<String, Value>,
}

/// V1 dependency tree.
#[derive(Debug, Deserialize)]
struct V1DependencyTree {
    #[allow(dead_code)]
    version: String,

    #[serde(default)]
    dependencies: Vec<V1Dependency>,

    /// Preserved for future use or debugging.
    #[allow(dead_code)]
    #[serde(flatten)]
    extensions: IndexMap<String, Value>,
}

/// V1 dependency entry.
#[derive(Debug, Deserialize)]
struct V1Dependency {
    /// Full coordinate string (group:artifact:version)
    coord: String,

    /// File path (used to extract repository URL)
    #[serde(default)]
    file: Option<String>,

    /// Direct dependencies (as full coordinates)
    #[serde(rename = "directDependencies", default)]
    direct_dependencies: Vec<String>,

    /// All transitive dependencies (preserved for debugging).
    #[allow(dead_code)]
    #[serde(default)]
    dependencies: Vec<String>,

    /// SHA-256 checksum
    #[serde(default)]
    sha256: Option<String>,

    /// Packages provided
    #[serde(default)]
    packages: Vec<String>,

    #[serde(flatten)]
    extensions: IndexMap<String, Value>,
}

/// Reads a `rules_jvm_external` V1 lockfile and converts it to our format.
pub fn read_v1(content: &str) -> Result<Lockfile> {
    let v1: V1Lockfile = serde_json::from_str(content)?;

    let mut artifacts = IndexMap::new();
    let mut repositories = IndexMap::new();

    for dep in &v1.dependency_tree.dependencies {
        let (key, version) = parse_coordinate(&dep.coord)?;

        let sha256 = dep
            .sha256
            .clone()
            .ok_or_else(|| Error::MissingField(format!("sha256 for artifact {}", dep.coord)))?;

        let mut artifact = LockedArtifact::new(&version, sha256);

        // Extract repository URL from file path
        if let Some(url) = dep.file.as_ref().and_then(|fp| extract_repository_url(fp)) {
            artifact.repository = Some(url.clone());
            repositories.insert(url, ());
        }

        // Convert direct dependencies from full coordinates to group:artifact
        let deps: Vec<String> = dep
            .direct_dependencies
            .iter()
            .filter_map(|c| parse_coordinate(c).map(|(k, _)| k).ok())
            .collect();
        artifact.dependencies = deps;

        // Add packages
        artifact.packages.clone_from(&dep.packages);

        // Preserve unknown fields
        artifact.extensions.clone_from(&dep.extensions);

        artifacts.insert(key, artifact);
    }

    // Convert repositories to list
    let repo_list: Vec<Repository> = repositories.keys().map(Repository::maven).collect();

    // Build metadata
    let metadata = LockfileMetadata::with_generator("rules_jvm_external v1 (converted)");

    // Filter extensions
    let mut extensions = v1.extensions;
    extensions.retain(|k, _| k != "dependency_tree");

    Ok(Lockfile {
        version: "1".to_string(),
        format: "antlers-lock".to_string(),
        artifacts,
        repositories: repo_list,
        conflicts: Vec::new(),
        metadata,
        extensions,
    })
}

/// Parses a coordinate string (group:artifact:version) into (key, version).
/// Returns (group:artifact, version).
fn parse_coordinate(coord: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = coord.split(':').collect();
    if parts.len() < 3 {
        return Err(Error::InvalidCoordinate(format!(
            "expected group:artifact:version, got: {coord}"
        )));
    }

    let key = format!("{}:{}", parts[0], parts[1]);
    let version = parts[2].to_string();

    Ok((key, version))
}

/// Extracts the repository URL from a V1 file path.
///
/// V1 file paths look like:
/// - `v1/https/repo1.maven.org/maven2/com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar`
/// - `v1/http/example.com/repo/...`
fn extract_repository_url(file_path: &str) -> Option<String> {
    // Split by '/'
    let parts: Vec<&str> = file_path.split('/').collect();

    if parts.len() < 3 {
        return None;
    }

    // Check for v1 prefix
    if parts[0] != "v1" {
        warn!(
            path = file_path,
            "unexpected file path format in V1 lockfile"
        );
        return None;
    }

    // Get protocol (http/https)
    let protocol = parts[1];
    if protocol != "http" && protocol != "https" {
        warn!(
            path = file_path,
            protocol = protocol,
            "unexpected protocol in V1 file path"
        );
        return None;
    }

    // Get host
    let host = parts[2];

    // Find where the artifact path starts (after maven2, repo, etc.)
    // Look for common repository paths
    let repo_markers = ["maven2", "releases", "snapshots", "public", "repo"];
    let mut repo_path_end = 3;

    for (i, part) in parts.iter().enumerate().skip(3) {
        if repo_markers.contains(part) {
            repo_path_end = i + 1;
            break;
        }
        // If we hit a group ID pattern (has dots when joined), stop
        // This is a heuristic - actual group IDs are path segments
        repo_path_end = i;
    }

    // Build the URL
    let repo_path: String = parts[3..repo_path_end].join("/");
    let url = if repo_path.is_empty() {
        format!("{protocol}://{host}/")
    } else {
        format!("{protocol}://{host}/{repo_path}/")
    };

    Some(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    const V1_LOCKFILE: &str = r#"{
        "dependency_tree": {
            "version": "0.1.0",
            "dependencies": [
                {
                    "coord": "com.google.guava:guava:33.0.0-jre",
                    "file": "v1/https/repo1.maven.org/maven2/com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar",
                    "directDependencies": [
                        "com.google.guava:failureaccess:1.0.2"
                    ],
                    "dependencies": [
                        "com.google.guava:failureaccess:1.0.2"
                    ],
                    "sha256": "abc123def456"
                },
                {
                    "coord": "com.google.guava:failureaccess:1.0.2",
                    "file": "v1/https/repo1.maven.org/maven2/com/google/guava/failureaccess/1.0.2/failureaccess-1.0.2.jar",
                    "directDependencies": [],
                    "dependencies": [],
                    "sha256": "789ghi012jkl"
                }
            ]
        }
    }"#;

    #[test]
    fn test_read_v1_basic() {
        let lockfile = read_v1(V1_LOCKFILE).unwrap();

        assert_eq!(lockfile.version, "1");
        assert_eq!(lockfile.format, "antlers-lock");
        assert_eq!(lockfile.artifacts.len(), 2);
    }

    #[test]
    fn test_read_v1_artifacts() {
        let lockfile = read_v1(V1_LOCKFILE).unwrap();

        let guava = lockfile.get("com.google.guava:guava").unwrap();
        assert_eq!(guava.version, "33.0.0-jre");
        assert_eq!(guava.sha256, "abc123def456");
    }

    #[test]
    fn test_read_v1_dependencies() {
        let lockfile = read_v1(V1_LOCKFILE).unwrap();

        let guava = lockfile.get("com.google.guava:guava").unwrap();
        assert_eq!(guava.dependencies, vec!["com.google.guava:failureaccess"]);
    }

    #[test]
    fn test_read_v1_repository() {
        let lockfile = read_v1(V1_LOCKFILE).unwrap();

        let guava = lockfile.get("com.google.guava:guava").unwrap();
        assert_eq!(
            guava.repository,
            Some("https://repo1.maven.org/maven2/".to_string())
        );
    }

    #[test]
    fn test_parse_coordinate() {
        let (key, version) = parse_coordinate("com.google.guava:guava:33.0.0-jre").unwrap();
        assert_eq!(key, "com.google.guava:guava");
        assert_eq!(version, "33.0.0-jre");
    }

    #[test]
    fn test_parse_coordinate_with_classifier() {
        let (key, version) = parse_coordinate("com.example:lib:1.0:sources").unwrap();
        assert_eq!(key, "com.example:lib");
        assert_eq!(version, "1.0");
    }

    #[test]
    fn test_extract_repository_url() {
        let url = extract_repository_url(
            "v1/https/repo1.maven.org/maven2/com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar",
        );
        assert_eq!(url, Some("https://repo1.maven.org/maven2/".to_string()));
    }

    #[test]
    fn test_extract_repository_url_http() {
        let url =
            extract_repository_url("v1/http/example.com/maven2/com/example/lib/1.0/lib-1.0.jar");
        assert_eq!(url, Some("http://example.com/maven2/".to_string()));
    }
}
