//! Artifact coordinates for JVM packages.
//!
//! This module provides types for representing Maven/Gradle artifact coordinates.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::version::Version;

/// Artifact coordinates without a version (groupId:artifactId).
///
/// This is useful for referencing artifacts in contexts where the version
/// is specified separately (e.g., dependency management).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Coordinates {
    /// The group ID (e.g., "org.apache.commons").
    pub group_id: String,
    /// The artifact ID (e.g., "commons-lang3").
    pub artifact_id: String,
}

impl Coordinates {
    /// Creates new coordinates.
    #[must_use]
    pub fn new(group_id: impl Into<String>, artifact_id: impl Into<String>) -> Self {
        Self {
            group_id: group_id.into(),
            artifact_id: artifact_id.into(),
        }
    }

    /// Parses coordinates from a "group:artifact" string.
    ///
    /// # Errors
    ///
    /// Returns an error if the string doesn't contain exactly one colon.
    pub fn parse(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(Error::InvalidCoordinates(format!(
                "expected 'group:artifact', got: {s}"
            )));
        }

        Ok(Self {
            group_id: parts[0].to_string(),
            artifact_id: parts[1].to_string(),
        })
    }

    /// Returns the coordinates as a "group:artifact" string.
    #[must_use]
    pub fn to_string_without_version(&self) -> String {
        format!("{}:{}", self.group_id, self.artifact_id)
    }

    /// Returns the group ID as a path (dots replaced with slashes).
    #[must_use]
    pub fn group_path(&self) -> String {
        self.group_id.replace('.', "/")
    }
}

impl fmt::Display for Coordinates {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.group_id, self.artifact_id)
    }
}

/// A classifier for an artifact (e.g., "sources", "javadoc", "tests").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Classifier(pub String);

impl Classifier {
    /// Creates a new classifier.
    #[must_use]
    pub fn new(classifier: impl Into<String>) -> Self {
        Self(classifier.into())
    }

    /// Returns the classifier string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Classifier for source code.
    pub const SOURCES: &'static str = "sources";

    /// Classifier for javadoc.
    pub const JAVADOC: &'static str = "javadoc";

    /// Classifier for test classes.
    pub const TESTS: &'static str = "tests";
}

impl fmt::Display for Classifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Classifier {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for Classifier {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// The extension/packaging type of an artifact (e.g., "jar", "pom", "war").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Extension(pub String);

impl Extension {
    /// Creates a new extension.
    #[must_use]
    pub fn new(extension: impl Into<String>) -> Self {
        Self(extension.into())
    }

    /// Returns the extension string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// JAR extension.
    pub const JAR: &'static str = "jar";

    /// POM extension.
    pub const POM: &'static str = "pom";

    /// WAR extension.
    pub const WAR: &'static str = "war";

    /// AAR extension (Android).
    pub const AAR: &'static str = "aar";
}

impl Default for Extension {
    fn default() -> Self {
        Self(Self::JAR.to_string())
    }
}

impl fmt::Display for Extension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Extension {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for Extension {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Full artifact coordinates with version, classifier, and extension.
///
/// # Examples
///
/// ```
/// use gav::Artifact;
///
/// // Parse from string
/// let artifact = Artifact::parse("org.apache.commons:commons-lang3:3.12.0").unwrap();
/// assert_eq!(artifact.coordinate(), "org.apache.commons:commons-lang3:3.12.0");
///
/// // With classifier and extension
/// let sources = Artifact::parse("com.example:lib:1.0:sources@jar").unwrap();
/// assert_eq!(sources.filename(), "lib-1.0-sources.jar");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Artifact {
    /// The coordinates (group + artifact).
    pub coordinates: Coordinates,
    /// The version.
    pub version: Version,
    /// Optional classifier (e.g., "sources", "javadoc").
    pub classifier: Option<Classifier>,
    /// The extension/packaging type (default: "jar").
    pub extension: Extension,
}

impl Artifact {
    /// Creates a new artifact with the given coordinates and version.
    #[must_use]
    pub fn new(
        group_id: impl Into<String>,
        artifact_id: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            coordinates: Coordinates::new(group_id, artifact_id),
            version: Version::new(version),
            classifier: None,
            extension: Extension::default(),
        }
    }

    /// Creates a new artifact with a classifier.
    #[must_use]
    pub fn with_classifier(mut self, classifier: impl Into<Classifier>) -> Self {
        self.classifier = Some(classifier.into());
        self
    }

    /// Creates a new artifact with an extension.
    #[must_use]
    pub fn with_extension(mut self, extension: impl Into<Extension>) -> Self {
        self.extension = extension.into();
        self
    }

    /// Parses an artifact from a coordinate string.
    ///
    /// Supports the following formats:
    /// - `group:artifact:version`
    /// - `group:artifact:version:classifier`
    /// - `group:artifact:version@extension`
    /// - `group:artifact:version:classifier@extension`
    ///
    /// # Errors
    ///
    /// Returns an error if the string cannot be parsed.
    #[allow(clippy::option_if_let_else)] // if-let is more readable here
    pub fn parse(s: &str) -> Result<Self> {
        // Split off extension first (if present)
        let (coord_part, extension) = if let Some(at_idx) = s.rfind('@') {
            let ext = &s[at_idx + 1..];
            (&s[..at_idx], Extension::new(ext))
        } else {
            (s, Extension::default())
        };

        // Split by colons
        let parts: Vec<&str> = coord_part.split(':').collect();

        match parts.len() {
            3 => {
                let version = Version::parse(parts[2])?;
                Ok(Self {
                    coordinates: Coordinates::new(parts[0], parts[1]),
                    version,
                    classifier: None,
                    extension,
                })
            }
            4 => {
                let version = Version::parse(parts[2])?;
                let classifier = if parts[3].is_empty() {
                    None
                } else {
                    Some(Classifier::new(parts[3]))
                };
                Ok(Self {
                    coordinates: Coordinates::new(parts[0], parts[1]),
                    version,
                    classifier,
                    extension,
                })
            }
            _ => Err(Error::InvalidCoordinates(format!(
                "expected 'group:artifact:version[:classifier][@extension]', got: {s}"
            ))),
        }
    }

    /// Returns the group ID.
    #[must_use]
    pub fn group_id(&self) -> &str {
        &self.coordinates.group_id
    }

    /// Returns the artifact ID.
    #[must_use]
    pub fn artifact_id(&self) -> &str {
        &self.coordinates.artifact_id
    }

    /// Returns the basic coordinate string (group:artifact:version).
    #[must_use]
    pub fn coordinate(&self) -> String {
        format!(
            "{}:{}:{}",
            self.coordinates.group_id, self.coordinates.artifact_id, self.version
        )
    }

    /// Returns the full coordinate string including classifier and extension.
    ///
    /// Format: `group:artifact:version[:classifier][@extension]`
    #[must_use]
    pub fn full_coordinate(&self) -> String {
        let mut result = self.coordinate();

        if let Some(ref classifier) = self.classifier {
            result.push(':');
            result.push_str(&classifier.0);
        }

        if self.extension.0 != Extension::JAR {
            result.push('@');
            result.push_str(&self.extension.0);
        }

        result
    }

    /// Returns the Maven repository path for this artifact.
    ///
    /// Example: `org/apache/commons/commons-lang3/3.12.0/commons-lang3-3.12.0.jar`
    #[must_use]
    pub fn repository_path(&self) -> String {
        format!(
            "{}/{}/{}/{}",
            self.coordinates.group_path(),
            self.coordinates.artifact_id,
            self.version,
            self.filename()
        )
    }

    /// Returns the filename for this artifact.
    ///
    /// Example: `commons-lang3-3.12.0.jar` or `commons-lang3-3.12.0-sources.jar`
    #[must_use]
    #[allow(clippy::option_if_let_else)] // match is more readable for format strings
    pub fn filename(&self) -> String {
        match &self.classifier {
            Some(classifier) => {
                format!(
                    "{}-{}-{}.{}",
                    self.coordinates.artifact_id, self.version, classifier.0, self.extension.0
                )
            }
            None => {
                format!(
                    "{}-{}.{}",
                    self.coordinates.artifact_id, self.version, self.extension.0
                )
            }
        }
    }

    /// Returns the path to the POM file for this artifact.
    ///
    /// Example: `org/apache/commons/commons-lang3/3.12.0/commons-lang3-3.12.0.pom`
    #[must_use]
    pub fn pom_path(&self) -> String {
        format!(
            "{}/{}/{}/{}-{}.pom",
            self.coordinates.group_path(),
            self.coordinates.artifact_id,
            self.version,
            self.coordinates.artifact_id,
            self.version
        )
    }

    /// Creates a new artifact for the POM of this artifact.
    #[must_use]
    pub fn pom_artifact(&self) -> Self {
        Self {
            coordinates: self.coordinates.clone(),
            version: self.version.clone(),
            classifier: None,
            extension: Extension::new(Extension::POM),
        }
    }

    /// Creates an artifact for the sources of this artifact.
    #[must_use]
    pub fn sources_artifact(&self) -> Self {
        Self {
            coordinates: self.coordinates.clone(),
            version: self.version.clone(),
            classifier: Some(Classifier::new(Classifier::SOURCES)),
            extension: Extension::new(Extension::JAR),
        }
    }

    /// Creates an artifact for the javadoc of this artifact.
    #[must_use]
    pub fn javadoc_artifact(&self) -> Self {
        Self {
            coordinates: self.coordinates.clone(),
            version: self.version.clone(),
            classifier: Some(Classifier::new(Classifier::JAVADOC)),
            extension: Extension::new(Extension::JAR),
        }
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
        let artifact = Artifact::parse("org.apache.commons:commons-lang3:3.12.0").unwrap();
        assert_eq!(artifact.group_id(), "org.apache.commons");
        assert_eq!(artifact.artifact_id(), "commons-lang3");
        assert_eq!(artifact.version.as_str(), "3.12.0");
        assert!(artifact.classifier.is_none());
        assert_eq!(artifact.extension.as_str(), "jar");
    }

    #[test]
    fn test_parse_with_classifier() {
        let artifact = Artifact::parse("com.example:lib:1.0:sources").unwrap();
        assert_eq!(artifact.classifier.as_ref().unwrap().as_str(), "sources");
    }

    #[test]
    fn test_parse_with_extension() {
        let artifact = Artifact::parse("com.example:lib:1.0@pom").unwrap();
        assert_eq!(artifact.extension.as_str(), "pom");
    }

    #[test]
    fn test_parse_with_classifier_and_extension() {
        let artifact = Artifact::parse("com.example:lib:1.0:sources@jar").unwrap();
        assert_eq!(artifact.classifier.as_ref().unwrap().as_str(), "sources");
        assert_eq!(artifact.extension.as_str(), "jar");
    }

    #[test]
    fn test_coordinate() {
        let artifact = Artifact::new("org.apache.commons", "commons-lang3", "3.12.0");
        assert_eq!(
            artifact.coordinate(),
            "org.apache.commons:commons-lang3:3.12.0"
        );
    }

    #[test]
    fn test_full_coordinate() {
        let artifact = Artifact::new("com.example", "lib", "1.0")
            .with_classifier("sources")
            .with_extension("jar");
        assert_eq!(artifact.full_coordinate(), "com.example:lib:1.0:sources");

        let pom = Artifact::new("com.example", "lib", "1.0").with_extension("pom");
        assert_eq!(pom.full_coordinate(), "com.example:lib:1.0@pom");
    }

    #[test]
    fn test_repository_path() {
        let artifact = Artifact::new("org.apache.commons", "commons-lang3", "3.12.0");
        assert_eq!(
            artifact.repository_path(),
            "org/apache/commons/commons-lang3/3.12.0/commons-lang3-3.12.0.jar"
        );
    }

    #[test]
    fn test_filename() {
        let artifact = Artifact::new("com.example", "lib", "1.0");
        assert_eq!(artifact.filename(), "lib-1.0.jar");

        let sources = artifact.sources_artifact();
        assert_eq!(sources.filename(), "lib-1.0-sources.jar");
    }

    #[test]
    fn test_pom_path() {
        let artifact = Artifact::new("org.apache.commons", "commons-lang3", "3.12.0");
        assert_eq!(
            artifact.pom_path(),
            "org/apache/commons/commons-lang3/3.12.0/commons-lang3-3.12.0.pom"
        );
    }

    #[test]
    fn test_coordinates_parse() {
        let coords = Coordinates::parse("org.apache.commons:commons-lang3").unwrap();
        assert_eq!(coords.group_id, "org.apache.commons");
        assert_eq!(coords.artifact_id, "commons-lang3");
    }
}
