//! Universal lockfile format for JVM, Kotlin KMP, and Android dependencies.
//!
//! This crate provides a lockfile format that is:
//!
//! - **Forward/backward compatible**: Unknown fields are preserved via `#[serde(flatten)]`
//! - **Interoperable**: Can read/write `rules_jvm_external` V1/V2 formats
//! - **Deterministic**: Sorted maps, consistent ordering for reproducible builds
//! - **Secure**: SHA-256 checksums only (SHA-1/MD5 are ignored as insecure)
//!
//! # Design Rationale
//!
//! This format is a superset of `rules_jvm_external` V2, adding:
//! - Per-artifact repository URL (V2 only has reverse mapping)
//! - Future KMP fields (`targets`, `kotlin_version`)
//! - Richer conflict metadata (requested versions, strategy)
//! - Cleaner structure (no `__HASH` prefixes)
//!
//! # Example
//!
//! ```
//! use antlers_lock::{Lockfile, LockedArtifact, Repository};
//!
//! // Create a new lockfile
//! let mut lockfile = Lockfile::new();
//!
//! // Add an artifact
//! lockfile.insert(
//!     "com.google.guava:guava",
//!     LockedArtifact::new("33.0.0-jre", "abc123def456")
//!         .with_repository("https://repo1.maven.org/maven2/")
//!         .with_dependencies(vec!["com.google.guava:failureaccess".to_string()]),
//! );
//!
//! // Add a repository
//! lockfile.add_repository(Repository::maven("https://repo1.maven.org/maven2/"));
//!
//! // Write to JSON
//! let json = lockfile.write().unwrap();
//!
//! // Read back
//! let parsed = Lockfile::read(&json).unwrap();
//! assert_eq!(lockfile.len(), parsed.len());
//! ```
//!
//! # Reading Different Formats
//!
//! The lockfile reader auto-detects the format:
//!
//! ```ignore
//! use antlers_lock::Lockfile;
//!
//! // Reads any supported format (antlers-lock, V1, V2)
//! let lockfile = Lockfile::read_file("install.json")?;
//!
//! // Check what we got
//! for (key, artifact) in lockfile.artifacts() {
//!     println!("{}: {} (sha256: {})", key, artifact.version, artifact.sha256);
//! }
//! ```
//!
//! # Writing for Bazel Compatibility
//!
//! To write a lockfile in `rules_jvm_external` V2 format:
//!
//! ```ignore
//! use antlers_lock::Lockfile;
//!
//! let lockfile = Lockfile::new();
//! // ... populate lockfile ...
//!
//! // Write in V2 format for Bazel
//! lockfile.write_v2_file("maven_install.json")?;
//! ```
//!
//! # Format Structure
//!
//! Our native format is JSON:
//!
//! ```json
//! {
//!   "version": "1",
//!   "format": "antlers-lock",
//!   "artifacts": {
//!     "com.google.guava:guava": {
//!       "version": "33.0.0-jre",
//!       "sha256": "...",
//!       "repository": "https://repo1.maven.org/maven2/",
//!       "dependencies": ["com.google.guava:failureaccess"]
//!     }
//!   },
//!   "repositories": [
//!     { "url": "https://repo1.maven.org/maven2/" }
//!   ],
//!   "metadata": {
//!     "generated_by": "antler 0.1.0",
//!     "generated_at": "2024-01-01T00:00:00Z"
//!   }
//! }
//! ```

mod artifact;
mod error;
mod format;
mod hash;
mod lockfile;
mod metadata;
pub mod reader;
pub mod writer;

pub use artifact::LockedArtifact;
pub use error::{Error, Result};
pub use format::LockFormat;
pub use hash::{compute_artifacts_hash, compute_input_hash};
pub use lockfile::{Conflict, LOCKFILE_FORMAT, LOCKFILE_VERSION, Lockfile, Repository};
pub use metadata::LockfileMetadata;
// Re-export legacy types for backwards compatibility
#[allow(deprecated)]
#[deprecated(since = "0.2.0", note = "Use LockFormat instead")]
pub use reader::LockfileFormat;
#[allow(deprecated)]
#[deprecated(since = "0.2.0", note = "Use LockFormat::detect instead")]
pub use reader::detect_format;

// Re-export useful types from jvm-resolver for convenience
pub use dendro::{Resolution, ResolvedArtifact, VersionConflict};

/// Creates a lockfile from a resolution result.
///
/// This converts the output of dependency resolution into a lockfile
/// that can be persisted and used for reproducible builds.
///
/// # Example
///
/// ```ignore
/// use antlers_lock::from_resolution;
/// use dendro::Resolution;
///
/// let resolution: Resolution = resolver.resolve(&artifact).await?;
/// let lockfile = from_resolution(&resolution);
/// lockfile.write_file("deps.lock.json")?;
/// ```
#[must_use]
pub fn from_resolution(resolution: &Resolution) -> Lockfile {
    let mut lockfile = Lockfile::new();

    // Add root artifact
    lockfile
        .metadata
        .add_root_artifact(resolution.root.coordinate());

    // Convert resolved artifacts
    for resolved in &resolution.artifacts {
        let key = resolved.coordinates().to_string();

        let mut artifact = LockedArtifact::new(
            resolved.artifact.version.as_str(),
            resolved.sha256.clone().unwrap_or_default(),
        );

        // Set repository if available
        if let Some(ref repo) = resolved.repository {
            artifact.repository = Some(repo.clone());
            lockfile.add_repository(Repository::maven(repo));
        }

        // Set packaging from extension
        artifact.packaging = resolved.artifact.extension.as_str().to_string();

        // Set classifier if present
        if let Some(ref classifier) = resolved.artifact.classifier {
            artifact.classifier = Some(classifier.as_str().to_string());
        }

        lockfile.insert(key, artifact);
    }

    // Convert conflicts
    for conflict in &resolution.conflicts {
        lockfile.add_conflict(Conflict::new(
            &conflict.artifact,
            conflict.versions.clone(),
            &conflict.selected,
            &conflict.strategy,
        ));
    }

    // Update hashes
    lockfile.update_hashes();

    // Sort for deterministic output
    lockfile.sort_artifacts();

    lockfile
}

/// Converts a lockfile back to a resolution result.
///
/// This is useful for using a lockfile to skip the resolution phase
/// and directly proceed to fetching artifacts.
///
/// Note: The returned resolution has a placeholder root artifact.
#[must_use]
pub fn to_resolution(lockfile: &Lockfile) -> Resolution {
    use gav::Artifact;

    // Create a placeholder root if we have root_artifacts in metadata
    let root = lockfile
        .metadata
        .root_artifacts
        .first()
        .and_then(|root_coord| Artifact::parse(root_coord).ok())
        .unwrap_or_else(|| Artifact::new("", "", ""));

    let mut resolution = Resolution::new(root);

    // Convert artifacts
    for (key, locked) in &lockfile.artifacts {
        // Parse key as coordinates
        let parts: Vec<&str> = key.split(':').collect();
        if parts.len() != 2 {
            continue;
        }

        let artifact = Artifact::new(parts[0], parts[1], &locked.version);

        let mut resolved = ResolvedArtifact::new(artifact);

        if !locked.sha256.is_empty() {
            resolved = resolved.with_sha256(&locked.sha256);
        }

        if let Some(ref repo) = locked.repository {
            resolved = resolved.with_repository(repo);
        }

        resolution.add_artifact(resolved);
    }

    // Convert conflicts
    for conflict in &lockfile.conflicts {
        resolution.add_conflict(VersionConflict::new(
            &conflict.artifact,
            conflict.requested.clone(),
            &conflict.selected,
            &conflict.strategy,
        ));
    }

    resolution
}

#[cfg(test)]
mod tests {
    use super::*;
    use gav::Artifact;

    #[test]
    fn test_from_resolution() {
        let root = Artifact::new("com.example", "root", "1.0.0");
        let mut resolution = Resolution::new(root);

        resolution.add_artifact(
            ResolvedArtifact::new(Artifact::new("com.example", "dep", "2.0.0"))
                .with_sha256("abc123")
                .with_repository("https://repo1.maven.org/maven2/"),
        );

        let lockfile = from_resolution(&resolution);

        assert_eq!(lockfile.len(), 1);
        assert!(lockfile.contains("com.example:dep"));

        let dep = lockfile.get("com.example:dep").unwrap();
        assert_eq!(dep.version, "2.0.0");
        assert_eq!(dep.sha256, "abc123");
    }

    #[test]
    fn test_to_resolution() {
        let mut lockfile = Lockfile::new();
        lockfile
            .metadata
            .add_root_artifact("com.example:root:1.0.0");

        lockfile.insert(
            "com.example:dep",
            LockedArtifact::new("2.0.0", "abc123")
                .with_repository("https://repo1.maven.org/maven2/"),
        );

        let resolution = to_resolution(&lockfile);

        assert_eq!(resolution.artifacts.len(), 1);
        assert_eq!(resolution.artifacts[0].artifact.version.as_str(), "2.0.0");
        assert_eq!(resolution.artifacts[0].sha256, Some("abc123".to_string()));
    }

    #[test]
    fn test_roundtrip_via_resolution() {
        let root = Artifact::new("com.example", "root", "1.0.0");
        let mut original = Resolution::new(root);

        original.add_artifact(
            ResolvedArtifact::new(Artifact::new("com.example", "dep", "2.0.0"))
                .with_sha256("abc123")
                .with_repository("https://repo1.maven.org/maven2/"),
        );

        original.add_conflict(VersionConflict::new(
            "com.example:lib",
            vec!["1.0".to_string(), "2.0".to_string()],
            "2.0",
            "highest-wins",
        ));

        // Convert to lockfile and back
        let lockfile = from_resolution(&original);
        let restored = to_resolution(&lockfile);

        assert_eq!(original.artifacts.len(), restored.artifacts.len());
        assert_eq!(original.conflicts.len(), restored.conflicts.len());
    }
}
