//! POM (Project Object Model) parsing.
//!
//! This module handles parsing Maven POM files to extract dependency information.

use serde::{Deserialize, Serialize};

use crate::artifact::Artifact;

/// A parsed POM file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pom {
    /// Model version (usually "4.0.0").
    #[serde(default)]
    pub model_version: Option<String>,

    /// Parent POM reference.
    #[serde(default)]
    pub parent: Option<Parent>,

    /// Group ID.
    #[serde(default)]
    pub group_id: Option<String>,

    /// Artifact ID.
    #[serde(default)]
    pub artifact_id: Option<String>,

    /// Version.
    #[serde(default)]
    pub version: Option<String>,

    /// Packaging type (jar, pom, war, etc.).
    #[serde(default)]
    pub packaging: Option<String>,

    /// Project name.
    #[serde(default)]
    pub name: Option<String>,

    /// Project description.
    #[serde(default)]
    pub description: Option<String>,

    /// Project URL.
    #[serde(default)]
    pub url: Option<String>,

    /// Properties for variable substitution.
    #[serde(default)]
    pub properties: Properties,

    /// Dependency management section.
    #[serde(default)]
    pub dependency_management: Option<DependencyManagement>,

    /// Direct dependencies.
    #[serde(default)]
    pub dependencies: Option<Dependencies>,
}

/// Parent POM reference.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parent {
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
    #[serde(default)]
    pub relative_path: Option<String>,
}

impl Parent {
    /// Convert to an Artifact.
    pub fn to_artifact(&self) -> Artifact {
        Artifact::new(&self.group_id, &self.artifact_id, &self.version).with_extension("pom")
    }
}

/// Properties map for variable substitution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Properties {
    #[serde(flatten)]
    pub values: std::collections::HashMap<String, String>,
}

/// Dependency management section.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyManagement {
    #[serde(default)]
    pub dependencies: Option<Dependencies>,
}

/// Dependencies wrapper.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dependencies {
    #[serde(default, rename = "dependency")]
    pub dependencies: Vec<Dependency>,
}

/// A single dependency declaration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    pub group_id: String,
    pub artifact_id: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub optional: Option<bool>,
    #[serde(default)]
    pub classifier: Option<String>,
    #[serde(default, rename = "type")]
    pub dep_type: Option<String>,
    #[serde(default)]
    pub exclusions: Option<Exclusions>,
}

impl Dependency {
    /// Check if this dependency should be included in resolution.
    ///
    /// Excludes test, provided, and system scope dependencies by default.
    pub fn should_include(&self) -> bool {
        match self.scope.as_deref() {
            Some("test" | "provided" | "system") => false,
            _ => !self.optional.unwrap_or(false),
        }
    }

    /// Convert to an Artifact, if version is present.
    pub fn to_artifact(&self) -> Option<Artifact> {
        let version = self.version.as_ref()?;
        let mut artifact = Artifact::new(&self.group_id, &self.artifact_id, version);

        if let Some(ref classifier) = self.classifier {
            artifact = artifact.with_classifier(classifier);
        }

        if let Some(ref dep_type) = self.dep_type {
            artifact = artifact.with_extension(dep_type);
        }

        Some(artifact)
    }

    /// Check if this is a BOM import.
    pub fn is_bom_import(&self) -> bool {
        self.scope.as_deref() == Some("import") && self.dep_type.as_deref() == Some("pom")
    }
}

/// Exclusions wrapper.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Exclusions {
    #[serde(default, rename = "exclusion")]
    pub exclusions: Vec<Exclusion>,
}

/// A single exclusion.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Exclusion {
    pub group_id: String,
    pub artifact_id: String,
}

impl Exclusion {
    /// Check if this exclusion matches an artifact.
    pub fn matches(&self, artifact: &Artifact) -> bool {
        let group_matches = self.group_id == "*" || self.group_id == artifact.group_id;
        let artifact_matches = self.artifact_id == "*" || self.artifact_id == artifact.artifact_id;
        group_matches && artifact_matches
    }
}

impl Pom {
    /// Parse a POM from XML content.
    pub fn parse(xml: &str) -> crate::Result<Self> {
        // Remove XML declaration and namespace for easier parsing
        let xml = xml
            .lines()
            .filter(|line| !line.trim().starts_with("<?xml"))
            .collect::<Vec<_>>()
            .join("\n");

        // Remove namespace declarations
        let xml = xml
            .replace("xmlns=\"http://maven.apache.org/POM/4.0.0\"", "")
            .replace("xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"", "")
            .replace(
                r#"xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd""#,
                "",
            );

        quick_xml::de::from_str(&xml).map_err(|e| crate::Error::PomParse(e.to_string()))
    }

    /// Get the effective group ID (falls back to parent).
    pub fn effective_group_id(&self) -> Option<&str> {
        self.group_id
            .as_deref()
            .or_else(|| self.parent.as_ref().map(|p| p.group_id.as_str()))
    }

    /// Get the effective version (falls back to parent).
    pub fn effective_version(&self) -> Option<&str> {
        self.version
            .as_deref()
            .or_else(|| self.parent.as_ref().map(|p| p.version.as_str()))
    }

    /// Get all direct dependencies.
    pub fn direct_dependencies(&self) -> Vec<&Dependency> {
        self.dependencies
            .as_ref()
            .map(|d| d.dependencies.iter().collect())
            .unwrap_or_default()
    }

    /// Get managed dependencies (from dependencyManagement).
    pub fn managed_dependencies(&self) -> Vec<&Dependency> {
        self.dependency_management
            .as_ref()
            .and_then(|dm| dm.dependencies.as_ref())
            .map(|d| d.dependencies.iter().collect())
            .unwrap_or_default()
    }

    /// Substitute properties in a string.
    pub fn substitute_properties(&self, s: &str) -> String {
        let mut result = s.to_string();

        // Substitute ${property} patterns
        for (key, value) in &self.properties.values {
            let pattern = format!("${{{key}}}");
            result = result.replace(&pattern, value);
        }

        // Handle ${project.version} and similar
        if let Some(ref version) = self.version {
            result = result.replace("${project.version}", version);
            result = result.replace("${pom.version}", version);
        }

        if let Some(ref group_id) = self.group_id {
            result = result.replace("${project.groupId}", group_id);
            result = result.replace("${pom.groupId}", group_id);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_pom() {
        let xml = r"
            <project>
                <modelVersion>4.0.0</modelVersion>
                <groupId>com.example</groupId>
                <artifactId>my-lib</artifactId>
                <version>1.0.0</version>
            </project>
        ";

        let pom = Pom::parse(xml).unwrap();
        assert_eq!(pom.group_id, Some("com.example".to_string()));
        assert_eq!(pom.artifact_id, Some("my-lib".to_string()));
        assert_eq!(pom.version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_dependency_should_include() {
        let dep = Dependency {
            group_id: "com.example".to_string(),
            artifact_id: "lib".to_string(),
            version: Some("1.0".to_string()),
            scope: Some("test".to_string()),
            ..Default::default()
        };
        assert!(!dep.should_include());

        let dep = Dependency {
            group_id: "com.example".to_string(),
            artifact_id: "lib".to_string(),
            version: Some("1.0".to_string()),
            scope: Some("compile".to_string()),
            ..Default::default()
        };
        assert!(dep.should_include());
    }
}
