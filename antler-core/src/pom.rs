//! POM (Project Object Model) parsing.
//!
//! This module handles parsing Maven POM files to extract dependency information,
//! including parent POM resolution and property substitution.

use std::collections::HashMap;

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

    /// Resolved parent POM (not parsed from XML, populated during resolution).
    #[serde(skip)]
    pub resolved_parent: Option<Box<Pom>>,
}

/// Parent POM reference.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parent {
    /// Parent group ID.
    pub group_id: String,
    /// Parent artifact ID.
    pub artifact_id: String,
    /// Parent version.
    pub version: String,
    /// Relative path to parent POM (optional, used for local resolution).
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
///
/// Properties in Maven POMs are arbitrary key-value pairs where each XML element
/// name is the key and the text content is the value.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Properties {
    /// The property key-value pairs.
    pub values: HashMap<String, String>,
}

impl<'de> serde::Deserialize<'de> for Properties {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Try to deserialize as a map of string values
        // quick-xml serializes arbitrary elements as nested $text values
        use serde::de::{MapAccess, Visitor};

        struct PropertiesVisitor;

        impl<'de> Visitor<'de> for PropertiesVisitor {
            type Value = Properties;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a properties map")
            }

            fn visit_map<M>(self, mut access: M) -> std::result::Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut values = HashMap::new();

                while let Some(key) = access.next_key::<String>()? {
                    // quick-xml deserializes element content as { "$text": "value" }
                    // or sometimes as a plain string
                    let value: serde_json::Value = access.next_value()?;
                    let text = match value {
                        serde_json::Value::String(s) => s,
                        serde_json::Value::Object(map) => map
                            .get("$text")
                            .or_else(|| map.get("$value"))
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .unwrap_or_default(),
                        _ => String::new(),
                    };
                    if !text.is_empty() {
                        values.insert(key, text);
                    }
                }

                Ok(Properties { values })
            }

            fn visit_unit<E>(self) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Properties::default())
            }
        }

        deserializer.deserialize_map(PropertiesVisitor)
    }
}

/// Dependency management section.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyManagement {
    /// Managed dependencies.
    #[serde(default)]
    pub dependencies: Option<Dependencies>,
}

/// Dependencies wrapper.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dependencies {
    /// List of dependencies.
    #[serde(default, rename = "dependency")]
    pub dependencies: Vec<Dependency>,
}

/// A single dependency declaration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    /// Dependency group ID.
    pub group_id: String,
    /// Dependency artifact ID.
    pub artifact_id: String,
    /// Dependency version (may be managed by parent or BOM).
    #[serde(default)]
    pub version: Option<String>,
    /// Dependency scope (compile, runtime, test, provided, system).
    #[serde(default)]
    pub scope: Option<String>,
    /// Whether this dependency is optional.
    #[serde(default)]
    pub optional: Option<bool>,
    /// Optional classifier (e.g., "sources", "javadoc").
    #[serde(default)]
    pub classifier: Option<String>,
    /// Dependency type/packaging (defaults to "jar").
    #[serde(default, rename = "type")]
    pub dep_type: Option<String>,
    /// Exclusions for transitive dependencies.
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

    /// Get the dependency key (groupId:artifactId) for matching in dependency management.
    pub fn key(&self) -> String {
        format!("{}:{}", self.group_id, self.artifact_id)
    }
}

/// Exclusions wrapper.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Exclusions {
    /// List of exclusions.
    #[serde(default, rename = "exclusion")]
    pub exclusions: Vec<Exclusion>,
}

/// A single exclusion.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Exclusion {
    /// Group ID pattern to exclude (supports "*" wildcard).
    pub group_id: String,
    /// Artifact ID pattern to exclude (supports "*" wildcard).
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
            )
            .replace(
                r#"xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 https://maven.apache.org/xsd/maven-4.0.0.xsd""#,
                "",
            );

        quick_xml::de::from_str(&xml).map_err(|e| crate::Error::PomParse {
            artifact: String::new(),
            details: e.to_string(),
        })
    }

    /// Parse a POM from XML content with artifact context for better error messages.
    pub fn parse_with_context(xml: &str, artifact: &Artifact) -> crate::Result<Self> {
        Self::parse(xml).map_err(|e| {
            if let crate::Error::PomParse { details, .. } = e {
                crate::Error::PomParse {
                    artifact: artifact.coordinate(),
                    details,
                }
            } else {
                e
            }
        })
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

    /// Get all managed dependencies including from parent POMs.
    pub fn all_managed_dependencies(&self) -> HashMap<String, &Dependency> {
        let mut managed = HashMap::new();

        // First, add parent's managed dependencies (these can be overridden)
        if let Some(ref parent) = self.resolved_parent {
            for (key, dep) in parent.all_managed_dependencies() {
                managed.insert(key, dep);
            }
        }

        // Then add this POM's managed dependencies (override parent's)
        for dep in self.managed_dependencies() {
            managed.insert(dep.key(), dep);
        }

        managed
    }

    /// Get all properties including from parent POMs.
    pub fn all_properties(&self) -> HashMap<&str, &str> {
        let mut props = HashMap::new();

        // First, add parent's properties (these can be overridden)
        if let Some(ref parent) = self.resolved_parent {
            for (key, value) in parent.all_properties() {
                props.insert(key, value);
            }
        }

        // Then add this POM's properties (override parent's)
        for (key, value) in &self.properties.values {
            props.insert(key.as_str(), value.as_str());
        }

        props
    }

    /// Substitute properties in a string.
    ///
    /// Handles the following patterns:
    /// - `${property.name}` - custom properties
    /// - `${project.version}` / `${pom.version}` - project version
    /// - `${project.groupId}` / `${pom.groupId}` - project group ID
    /// - `${project.artifactId}` - project artifact ID
    /// - `${project.parent.version}` / `${parent.version}` - parent version
    /// - `${project.parent.groupId}` - parent group ID
    pub fn substitute_properties(&self, s: &str) -> String {
        let mut result = s.to_string();
        let all_props = self.all_properties();

        // Substitute ${property} patterns from all properties
        for (key, value) in &all_props {
            let pattern = format!("${{{key}}}");
            result = result.replace(&pattern, value);
        }

        // Handle ${project.version} and ${pom.version}
        if let Some(version) = self.effective_version() {
            result = result.replace("${project.version}", version);
            result = result.replace("${pom.version}", version);
            // Also handle bare ${version} which is less common but used
            #[allow(clippy::literal_string_with_formatting_args)]
            {
                result = result.replace("${version}", version);
            }
        }

        // Handle ${project.groupId} and ${pom.groupId}
        if let Some(group_id) = self.effective_group_id() {
            result = result.replace("${project.groupId}", group_id);
            result = result.replace("${pom.groupId}", group_id);
        }

        // Handle ${project.artifactId}
        if let Some(ref artifact_id) = self.artifact_id {
            result = result.replace("${project.artifactId}", artifact_id);
        }

        // Handle ${project.parent.version} and ${parent.version}
        if let Some(ref parent) = self.parent {
            result = result.replace("${project.parent.version}", &parent.version);
            result = result.replace("${parent.version}", &parent.version);
            result = result.replace("${project.parent.groupId}", &parent.group_id);
            result = result.replace("${parent.groupId}", &parent.group_id);
        }

        result
    }

    /// Apply dependency management to resolve version for a dependency.
    ///
    /// If the dependency doesn't have a version, looks it up in dependency management.
    pub fn resolve_dependency_version(&self, dep: &Dependency) -> Option<String> {
        // If dependency already has a version, substitute properties and return
        if let Some(ref version) = dep.version {
            let resolved = self.substitute_properties(version);
            // If the version still contains unresolved properties, try dependency management
            if !resolved.contains("${") {
                return Some(resolved);
            }
        }

        // Look up in dependency management
        let managed = self.all_managed_dependencies();
        if let Some(managed_dep) = managed.get(&dep.key())
            && let Some(ref version) = managed_dep.version
        {
            return Some(self.substitute_properties(version));
        }

        // Return original version with properties substituted if we have one
        dep.version.as_ref().map(|v| self.substitute_properties(v))
    }

    /// Get BOM imports from dependency management.
    pub fn bom_imports(&self) -> Vec<&Dependency> {
        self.managed_dependencies()
            .into_iter()
            .filter(|d| d.is_bom_import())
            .collect()
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
    fn test_parse_pom_with_parent() {
        let xml = r"
            <project>
                <modelVersion>4.0.0</modelVersion>
                <parent>
                    <groupId>com.example</groupId>
                    <artifactId>parent</artifactId>
                    <version>1.0.0</version>
                </parent>
                <artifactId>my-lib</artifactId>
            </project>
        ";

        let pom = Pom::parse(xml).unwrap();
        assert!(pom.parent.is_some());
        let parent = pom.parent.as_ref().unwrap();
        assert_eq!(parent.group_id, "com.example");
        assert_eq!(parent.artifact_id, "parent");
        assert_eq!(parent.version, "1.0.0");
        assert_eq!(pom.effective_group_id(), Some("com.example"));
    }

    #[test]
    fn test_property_substitution() {
        // Note: quick-xml doesn't support #[serde(flatten)] for HashMap in XML,
        // so we construct the POM manually for this test
        let mut properties = HashMap::new();
        properties.insert("kotlin.version".to_string(), "2.0.0".to_string());

        let pom = Pom {
            group_id: Some("com.example".to_string()),
            artifact_id: Some("my-lib".to_string()),
            version: Some("1.0.0".to_string()),
            properties: Properties { values: properties },
            ..Default::default()
        };

        assert_eq!(pom.substitute_properties("${kotlin.version}"), "2.0.0");
        assert_eq!(pom.substitute_properties("${project.version}"), "1.0.0");
        assert_eq!(
            pom.substitute_properties("${project.groupId}"),
            "com.example"
        );
    }

    #[test]
    fn test_parent_version_substitution() {
        let xml = r"
            <project>
                <modelVersion>4.0.0</modelVersion>
                <parent>
                    <groupId>com.example</groupId>
                    <artifactId>parent</artifactId>
                    <version>2.0.0</version>
                </parent>
                <artifactId>my-lib</artifactId>
            </project>
        ";

        let pom = Pom::parse(xml).unwrap();
        assert_eq!(pom.substitute_properties("${parent.version}"), "2.0.0");
        assert_eq!(
            pom.substitute_properties("${project.parent.version}"),
            "2.0.0"
        );
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

    #[test]
    fn test_bom_import_detection() {
        let dep = Dependency {
            group_id: "com.example".to_string(),
            artifact_id: "bom".to_string(),
            version: Some("1.0".to_string()),
            scope: Some("import".to_string()),
            dep_type: Some("pom".to_string()),
            ..Default::default()
        };
        assert!(dep.is_bom_import());
    }

    #[test]
    fn test_exclusion_matching() {
        let artifact = Artifact::new("com.example", "lib", "1.0.0");

        let exclusion = Exclusion {
            group_id: "com.example".to_string(),
            artifact_id: "lib".to_string(),
        };
        assert!(exclusion.matches(&artifact));

        let wildcard = Exclusion {
            group_id: "*".to_string(),
            artifact_id: "*".to_string(),
        };
        assert!(wildcard.matches(&artifact));

        let no_match = Exclusion {
            group_id: "org.other".to_string(),
            artifact_id: "lib".to_string(),
        };
        assert!(!no_match.matches(&artifact));
    }
}
