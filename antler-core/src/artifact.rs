//! Artifact coordinate handling.
//!
//! Maven artifacts are identified by coordinates in the form:
//! `groupId:artifactId:version[:classifier][@extension]`

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::{Error, Result};

/// A Maven artifact coordinate.
///
/// Represents the unique identifier for a Maven artifact, consisting of:
/// - `group_id`: The organization or project (e.g., "org.jetbrains.kotlin")
/// - `artifact_id`: The artifact name (e.g., "kotlin-stdlib")
/// - `version`: The version string (e.g., "2.3.0")
/// - `classifier`: Optional classifier (e.g., "sources", "javadoc")
/// - `extension`: File extension, defaults to "jar"
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Artifact {
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classifier: Option<String>,
    #[serde(default = "default_extension")]
    pub extension: String,
}

fn default_extension() -> String {
    "jar".to_string()
}

impl Artifact {
    /// Create a new artifact with the given coordinates.
    pub fn new(
        group_id: impl Into<String>,
        artifact_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            group_id: group_id.into(),
            artifact_id: artifact_id.into(),
            version: version.into(),
            classifier: None,
            extension: "jar".to_string(),
        }
    }

    /// Set the classifier for this artifact.
    pub fn with_classifier(mut self, classifier: impl Into<String>) -> Self {
        self.classifier = Some(classifier.into());
        self
    }

    /// Set the extension for this artifact.
    pub fn with_extension(mut self, extension: impl Into<String>) -> Self {
        self.extension = extension.into();
        self
    }

    /// Parse an artifact from a coordinate string.
    ///
    /// Supported formats:
    /// - `groupId:artifactId:version`
    /// - `groupId:artifactId:version:classifier`
    /// - `groupId:artifactId:version:classifier@extension`
    /// - `groupId:artifactId:version@extension`
    pub fn parse(coord: &str) -> Result<Self> {
        // Handle extension suffix
        let (coord, extension) = if let Some(at_pos) = coord.rfind('@') {
            let ext = &coord[at_pos + 1..];
            let coord = &coord[..at_pos];
            (coord, ext.to_string())
        } else {
            (coord, "jar".to_string())
        };

        let parts: Vec<&str> = coord.split(':').collect();

        match parts.len() {
            3 => Ok(Self {
                group_id: parts[0].to_string(),
                artifact_id: parts[1].to_string(),
                version: parts[2].to_string(),
                classifier: None,
                extension,
            }),
            4 => Ok(Self {
                group_id: parts[0].to_string(),
                artifact_id: parts[1].to_string(),
                version: parts[2].to_string(),
                classifier: Some(parts[3].to_string()),
                extension,
            }),
            _ => Err(Error::InvalidCoordinates(format!(
                "expected 'groupId:artifactId:version[:classifier][@extension]', got '{coord}'"
            ))),
        }
    }

    /// Get the coordinate string without classifier or extension.
    pub fn coordinate(&self) -> String {
        format!("{}:{}:{}", self.group_id, self.artifact_id, self.version)
    }

    /// Get the full coordinate string including classifier and extension.
    pub fn full_coordinate(&self) -> String {
        let mut coord = self.coordinate();
        if let Some(ref classifier) = self.classifier {
            coord.push(':');
            coord.push_str(classifier);
        }
        if self.extension != "jar" {
            coord.push('@');
            coord.push_str(&self.extension);
        }
        coord
    }

    /// Get the path to this artifact in a Maven repository.
    ///
    /// For example: `org/jetbrains/kotlin/kotlin-stdlib/2.3.0/kotlin-stdlib-2.3.0.jar`
    pub fn repository_path(&self) -> String {
        let group_path = self.group_id.replace('.', "/");
        let filename = self.filename();
        format!(
            "{}/{}/{}/{}",
            group_path, self.artifact_id, self.version, filename
        )
    }

    /// Get the filename for this artifact.
    ///
    /// For example: `kotlin-stdlib-2.3.0.jar` or `kotlin-stdlib-2.3.0-sources.jar`
    pub fn filename(&self) -> String {
        match &self.classifier {
            Some(classifier) => format!(
                "{}-{}-{}.{}",
                self.artifact_id, self.version, classifier, self.extension
            ),
            None => format!("{}-{}.{}", self.artifact_id, self.version, self.extension),
        }
    }

    /// Get the POM path for this artifact.
    pub fn pom_path(&self) -> String {
        let group_path = self.group_id.replace('.', "/");
        format!(
            "{}/{}/{}/{}-{}.pom",
            group_path, self.artifact_id, self.version, self.artifact_id, self.version
        )
    }

    /// Create an artifact for the POM of this artifact.
    pub fn pom_artifact(&self) -> Self {
        Self {
            group_id: self.group_id.clone(),
            artifact_id: self.artifact_id.clone(),
            version: self.version.clone(),
            classifier: None,
            extension: "pom".to_string(),
        }
    }

    /// Create an artifact for the sources JAR.
    pub fn sources_artifact(&self) -> Self {
        self.clone().with_classifier("sources")
    }

    /// Create an artifact for the javadoc JAR.
    pub fn javadoc_artifact(&self) -> Self {
        self.clone().with_classifier("javadoc")
    }
}

impl fmt::Display for Artifact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.full_coordinate())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let artifact = Artifact::parse("org.jetbrains.kotlin:kotlin-stdlib:2.3.0").unwrap();
        assert_eq!(artifact.group_id, "org.jetbrains.kotlin");
        assert_eq!(artifact.artifact_id, "kotlin-stdlib");
        assert_eq!(artifact.version, "2.3.0");
        assert_eq!(artifact.classifier, None);
        assert_eq!(artifact.extension, "jar");
    }

    #[test]
    fn test_parse_with_classifier() {
        let artifact = Artifact::parse("org.jetbrains.kotlin:kotlin-stdlib:2.3.0:sources").unwrap();
        assert_eq!(artifact.classifier, Some("sources".to_string()));
    }

    #[test]
    fn test_parse_with_extension() {
        let artifact = Artifact::parse("org.jetbrains.kotlin:kotlin-stdlib:2.3.0@pom").unwrap();
        assert_eq!(artifact.extension, "pom");
    }

    #[test]
    fn test_repository_path() {
        let artifact = Artifact::parse("org.jetbrains.kotlin:kotlin-stdlib:2.3.0").unwrap();
        assert_eq!(
            artifact.repository_path(),
            "org/jetbrains/kotlin/kotlin-stdlib/2.3.0/kotlin-stdlib-2.3.0.jar"
        );
    }

    #[test]
    fn test_filename_with_classifier() {
        let artifact = Artifact::parse("org.jetbrains.kotlin:kotlin-stdlib:2.3.0:sources").unwrap();
        assert_eq!(artifact.filename(), "kotlin-stdlib-2.3.0-sources.jar");
    }
}
