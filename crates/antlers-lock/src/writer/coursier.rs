//! Writer for Coursier JSON format.
//!
//! This writer converts our native lockfile format to Coursier's JSON report format,
//! which is used by tools like `cs fetch --json-output-file`.
//!
//! ## Format Overview
//!
//! The Coursier format uses an array of dependencies rather than a map, and includes
//! local file paths instead of checksums:
//!
//! ```json
//! {
//!   "version": "0.0.1",
//!   "conflict_resolution": {
//!     "org.example:lib:1.0": "org.example:lib:2.0"
//!   },
//!   "dependencies": [
//!     {
//!       "coord": "org.example:lib:2.0",
//!       "files": [
//!         ["", "/path/to/lib-2.0.jar"],
//!         ["sources", "/path/to/lib-2.0-sources.jar"]
//!       ],
//!       "dependencies": ["org.example:dep1:1.0"]
//!     }
//!   ]
//! }
//! ```

use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde::Serialize;

use crate::Lockfile;
use crate::error::Result;

/// The Coursier report format version.
const COURSIER_VERSION: &str = "0.0.1";

/// Default cache path for Coursier (Maven Central).
const DEFAULT_CACHE_PATH: &str = ".cache/coursier/v1/https/repo1.maven.org/maven2";

/// Coursier JSON report structure.
#[derive(Debug, Serialize)]
struct CoursierReport {
    /// Format version (always "0.0.1").
    version: String,

    /// Conflict resolutions mapping original coords to selected coords.
    ///
    /// Format: `group:artifact:requested_version` -> `group:artifact:selected_version`
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    conflict_resolution: IndexMap<String, String>,

    /// Array of resolved dependencies.
    dependencies: Vec<CoursierDependency>,
}

/// A dependency entry in the Coursier format.
#[derive(Debug, Serialize)]
struct CoursierDependency {
    /// Full coordinate with version: "group:artifact:version"
    coord: String,

    /// File classifier and path tuples.
    ///
    /// Example: `[["", "/path/to/file.jar"], ["sources", "/path/to/file-sources.jar"]]`
    files: Vec<(String, String)>,

    /// Direct dependencies as full coordinates with versions.
    dependencies: Vec<String>,
}

/// Writes a lockfile to Coursier JSON format using the default cache path.
///
/// The default cache path is `~/.cache/coursier/v1/https/repo1.maven.org/maven2`.
///
/// # Example
///
/// ```ignore
/// use antlers_lock::{Lockfile, writer};
///
/// let lockfile = Lockfile::read_file("deps.lock.json")?;
/// let coursier_json = writer::write_coursier(&lockfile)?;
/// std::fs::write("coursier-report.json", coursier_json)?;
/// ```
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn write_coursier(lockfile: &Lockfile) -> Result<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let cache_path = PathBuf::from(home).join(DEFAULT_CACHE_PATH);
    write_coursier_with_cache_path(lockfile, &cache_path)
}

/// Writes a lockfile to Coursier JSON format with a custom cache path.
///
/// # Arguments
///
/// * `lockfile` - The lockfile to convert
/// * `cache_path` - The base path for cached artifacts (e.g., `/home/user/.cache/coursier/v1/https/repo1.maven.org/maven2`)
///
/// # Example
///
/// ```ignore
/// use std::path::Path;
/// use antlers_lock::{Lockfile, writer};
///
/// let lockfile = Lockfile::read_file("deps.lock.json")?;
/// let cache_path = Path::new("/custom/cache/path");
/// let coursier_json = writer::write_coursier_with_cache_path(&lockfile, cache_path)?;
/// ```
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn write_coursier_with_cache_path(lockfile: &Lockfile, cache_path: &Path) -> Result<String> {
    let report = convert_to_coursier(lockfile, cache_path);
    let json = serde_json::to_string_pretty(&report)?;
    Ok(json)
}

/// Writes a lockfile to compact Coursier JSON format (no extra whitespace).
pub fn write_coursier_compact(lockfile: &Lockfile, cache_path: &Path) -> Result<String> {
    let report = convert_to_coursier(lockfile, cache_path);
    let json = serde_json::to_string(&report)?;
    Ok(json)
}

/// Converts our lockfile to Coursier report structure.
fn convert_to_coursier(lockfile: &Lockfile, cache_path: &Path) -> CoursierReport {
    // Build conflict resolution map
    // In Coursier format: "group:artifact:requested" -> "group:artifact:selected"
    let mut conflict_resolution = IndexMap::new();
    for conflict in &lockfile.conflicts {
        // For each requested version that differs from selected, add an entry
        for requested in &conflict.requested {
            if requested != &conflict.selected {
                let from_coord = format!("{}:{}", conflict.artifact, requested);
                let to_coord = format!("{}:{}", conflict.artifact, conflict.selected);
                conflict_resolution.insert(from_coord, to_coord);
            }
        }
    }

    // Build a map of artifact key -> version for resolving dependency versions
    let version_map: IndexMap<&str, &str> = lockfile
        .artifacts
        .iter()
        .map(|(key, artifact)| (key.as_str(), artifact.version.as_str()))
        .collect();

    // Convert artifacts to Coursier dependencies
    let mut dependencies = Vec::new();
    for (key, artifact) in &lockfile.artifacts {
        // Build full coordinate with version
        let coord = format!("{}:{}", key, artifact.version);

        // Build file paths using Maven cache layout
        let files = build_file_paths(key, artifact, cache_path);

        // Convert dependencies to full coordinates with versions
        let dep_coords: Vec<String> = artifact
            .dependencies
            .iter()
            .filter_map(|dep_key| {
                version_map
                    .get(dep_key.as_str())
                    .map(|version| format!("{dep_key}:{version}"))
            })
            .collect();

        dependencies.push(CoursierDependency {
            coord,
            files,
            dependencies: dep_coords,
        });
    }

    CoursierReport {
        version: COURSIER_VERSION.to_string(),
        conflict_resolution,
        dependencies,
    }
}

/// Builds file paths for an artifact using Maven cache layout.
///
/// Layout: `{cache_path}/{group/as/path}/{artifact}/{version}/{artifact}-{version}[-{classifier}].{ext}`
fn build_file_paths(
    key: &str,
    artifact: &crate::LockedArtifact,
    cache_path: &Path,
) -> Vec<(String, String)> {
    let mut files = Vec::new();

    // Parse group:artifact from key
    let parts: Vec<&str> = key.split(':').collect();
    if parts.len() != 2 {
        return files;
    }

    let group = parts[0];
    let artifact_id = parts[1];
    let version = &artifact.version;

    // Convert group to path (e.g., "org.example" -> "org/example")
    let group_path = group.replace('.', "/");

    // Determine extension from packaging
    let extension = &artifact.packaging;

    // Build the base path
    let base_path = cache_path.join(&group_path).join(artifact_id).join(version);

    // Main artifact file
    let main_filename = artifact.classifier.as_ref().map_or_else(
        || format!("{artifact_id}-{version}.{extension}"),
        |classifier| format!("{artifact_id}-{version}-{classifier}.{extension}"),
    );

    let main_path = base_path.join(&main_filename);
    let classifier_label = artifact.classifier.clone().unwrap_or_default();
    files.push((classifier_label, main_path.to_string_lossy().to_string()));

    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Conflict, LockedArtifact, LockfileMetadata};
    use std::path::PathBuf;

    fn create_test_lockfile() -> Lockfile {
        let mut artifacts = IndexMap::new();

        // Main artifact with dependencies
        let guava = LockedArtifact::new("33.0.0-jre", "abc123def456")
            .with_repository("https://repo1.maven.org/maven2/")
            .with_dependencies(vec!["com.google.guava:failureaccess".to_string()]);

        // Dependency artifact
        let failureaccess = LockedArtifact::new("1.0.2", "789ghi012jkl")
            .with_repository("https://repo1.maven.org/maven2/");

        artifacts.insert("com.google.guava:guava".to_string(), guava);
        artifacts.insert("com.google.guava:failureaccess".to_string(), failureaccess);

        Lockfile {
            version: "1".to_string(),
            format: "antlers-lock".to_string(),
            artifacts,
            repositories: vec![],
            conflicts: vec![],
            metadata: LockfileMetadata::with_generator("test"),
            extensions: IndexMap::new(),
        }
    }

    #[test]
    fn test_write_coursier_basic() {
        let lockfile = create_test_lockfile();
        let cache_path =
            PathBuf::from("/home/user/.cache/coursier/v1/https/repo1.maven.org/maven2");
        let json = write_coursier_with_cache_path(&lockfile, &cache_path).unwrap();

        assert!(json.contains("\"version\": \"0.0.1\""));
        assert!(json.contains("\"coord\": \"com.google.guava:guava:33.0.0-jre\""));
        assert!(json.contains("\"coord\": \"com.google.guava:failureaccess:1.0.2\""));
    }

    #[test]
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    fn test_write_coursier_structure() {
        let lockfile = create_test_lockfile();
        let cache_path = PathBuf::from("/cache");
        let json = write_coursier_with_cache_path(&lockfile, &cache_path).unwrap();

        // Parse back to verify structure
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Check version
        assert_eq!(parsed.get("version").unwrap().as_str().unwrap(), "0.0.1");

        // Check dependencies is an array
        let deps = parsed.get("dependencies").unwrap().as_array().unwrap();
        assert_eq!(deps.len(), 2);

        // Find guava dependency
        let guava = deps
            .iter()
            .find(|d| {
                d.get("coord")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .contains("guava:guava")
            })
            .unwrap();

        // Check coord includes version
        assert_eq!(
            guava.get("coord").unwrap().as_str().unwrap(),
            "com.google.guava:guava:33.0.0-jre"
        );

        // Check files is an array of tuples
        let files = guava.get("files").unwrap().as_array().unwrap();
        assert!(!files.is_empty());

        // Check first file tuple
        let first_file = files[0].as_array().unwrap();
        assert_eq!(first_file.len(), 2);
        assert_eq!(first_file[0].as_str().unwrap(), ""); // empty classifier for main jar
        assert!(first_file[1].as_str().unwrap().ends_with(".jar"));

        // Check dependencies includes version
        let dep_list = guava.get("dependencies").unwrap().as_array().unwrap();
        assert_eq!(dep_list.len(), 1);
        assert_eq!(
            dep_list[0].as_str().unwrap(),
            "com.google.guava:failureaccess:1.0.2"
        );
    }

    #[test]
    fn test_write_coursier_file_paths() {
        let lockfile = create_test_lockfile();
        let cache_path = PathBuf::from("/cache/maven");
        let json = write_coursier_with_cache_path(&lockfile, &cache_path).unwrap();

        // Check file path follows Maven layout
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let deps = parsed.get("dependencies").unwrap().as_array().unwrap();

        let guava = deps
            .iter()
            .find(|d| {
                d.get("coord")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .contains("guava:guava")
            })
            .unwrap();

        let files = guava.get("files").unwrap().as_array().unwrap();
        let path = files[0].as_array().unwrap()[1].as_str().unwrap();

        // Path should follow Maven layout: {cache}/com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar
        assert!(path.contains("/com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar"));
    }

    #[test]
    fn test_write_coursier_with_classifier() {
        let mut lockfile = Lockfile::new();

        // Add artifact with classifier
        let sources = LockedArtifact::new("1.0.0", "abc123").with_classifier("sources");

        lockfile.insert("org.example:lib".to_string(), sources);

        let cache_path = PathBuf::from("/cache");
        let json = write_coursier_with_cache_path(&lockfile, &cache_path).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let deps = parsed.get("dependencies").unwrap().as_array().unwrap();
        let lib = &deps[0];

        let files = lib.get("files").unwrap().as_array().unwrap();
        let first_file = files[0].as_array().unwrap();

        // Classifier should be in the tuple
        assert_eq!(first_file[0].as_str().unwrap(), "sources");
        // Path should include classifier
        assert!(first_file[1].as_str().unwrap().contains("-sources.jar"));
    }

    #[test]
    fn test_write_coursier_with_conflicts() {
        let mut lockfile = create_test_lockfile();

        // Add a conflict
        lockfile.add_conflict(Conflict::new(
            "com.example:lib",
            vec!["1.0".to_string(), "2.0".to_string()],
            "2.0",
            "highest-wins",
        ));

        let cache_path = PathBuf::from("/cache");
        let json = write_coursier_with_cache_path(&lockfile, &cache_path).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let conflicts = parsed
            .get("conflict_resolution")
            .unwrap()
            .as_object()
            .unwrap();

        // Only the non-selected version should be in conflict_resolution
        assert!(conflicts.contains_key("com.example:lib:1.0"));
        assert_eq!(
            conflicts
                .get("com.example:lib:1.0")
                .unwrap()
                .as_str()
                .unwrap(),
            "com.example:lib:2.0"
        );
        // The selected version should not have an entry pointing to itself
        assert!(!conflicts.contains_key("com.example:lib:2.0"));
    }

    #[test]
    fn test_write_coursier_compact() {
        let lockfile = create_test_lockfile();
        let cache_path = PathBuf::from("/cache");
        let json = write_coursier_compact(&lockfile, &cache_path).unwrap();

        // Compact format should not have newlines
        assert!(!json.contains('\n'));
        assert!(json.contains("\"version\":\"0.0.1\""));
    }

    #[test]
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    fn test_write_coursier_aar_packaging() {
        let mut lockfile = Lockfile::new();

        // Add Android AAR artifact
        let aar = LockedArtifact::new("1.0.0", "abc123").with_packaging("aar");

        lockfile.insert("com.android:lib".to_string(), aar);

        let cache_path = PathBuf::from("/cache");
        let json = write_coursier_with_cache_path(&lockfile, &cache_path).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let deps = parsed.get("dependencies").unwrap().as_array().unwrap();
        let lib = &deps[0];

        let files = lib.get("files").unwrap().as_array().unwrap();
        let path = files[0].as_array().unwrap()[1].as_str().unwrap();

        // Path should have .aar extension
        assert!(path.ends_with(".aar"));
    }

    #[test]
    fn test_write_coursier_empty_lockfile() {
        let lockfile = Lockfile::new();
        let cache_path = PathBuf::from("/cache");
        let json = write_coursier_with_cache_path(&lockfile, &cache_path).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.get("version").unwrap().as_str().unwrap(), "0.0.1");
        assert!(
            parsed
                .get("dependencies")
                .unwrap()
                .as_array()
                .unwrap()
                .is_empty()
        );
        // conflict_resolution should be omitted when empty
        assert!(parsed.get("conflict_resolution").is_none());
    }
}
