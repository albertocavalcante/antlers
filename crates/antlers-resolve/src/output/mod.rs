//! Output formatters for antlers-resolve.

use antlers_lock::{LockedArtifact, Lockfile};
use gav::{Artifact, Coordinates};

use crate::{Error, Result};

pub mod bazel;
pub mod buck;
pub mod json;
pub mod starlark;

const MAVEN_CENTRAL: &str = "https://repo1.maven.org/maven2/";

pub(crate) fn coordinates_from_key(key: &str) -> Result<Coordinates> {
    Coordinates::parse(key).map_err(|_| Error::InvalidCoordinates(key.to_string()))
}

pub(crate) fn artifact_from_locked(coords: &Coordinates, locked: &LockedArtifact) -> Artifact {
    let mut artifact = Artifact::new(&coords.group_id, &coords.artifact_id, &locked.version);
    if let Some(classifier) = locked.classifier.as_deref() {
        artifact = artifact.with_classifier(classifier);
    }
    if !locked.packaging.is_empty() {
        artifact = artifact.with_extension(locked.packaging.as_str());
    }
    artifact
}

pub(crate) fn checksum_or_none(locked: &LockedArtifact) -> Option<&str> {
    let checksum = locked.sha256.trim();
    if checksum.is_empty() {
        None
    } else {
        Some(checksum)
    }
}

pub(crate) fn repository_url<'a>(lockfile: &'a Lockfile, locked: &'a LockedArtifact) -> &'a str {
    locked
        .repository
        .as_deref()
        .or_else(|| lockfile.repositories.first().map(|repo| repo.url.as_str()))
        .unwrap_or(MAVEN_CENTRAL)
}

pub(crate) fn artifact_download_url(
    lockfile: &Lockfile,
    artifact: &Artifact,
    locked: &LockedArtifact,
) -> String {
    let repo_base = repository_url(lockfile, locked).trim_end_matches('/');
    format!("{repo_base}/{}", artifact.repository_path())
}

pub(crate) fn escape_starlark(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
