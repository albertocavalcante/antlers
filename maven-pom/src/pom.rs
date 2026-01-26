//! Main POM structure and types.
//!
//! This module contains the core types for representing a parsed Maven POM file.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::Properties;
use crate::profile::{Profile, Profiles};

// Re-export jvm-artifact types for convenience
pub use jvm_artifact::{
    Artifact, Coordinates, Dependency as JvmDependency, Exclusion as JvmExclusion,
    Exclusions as JvmExclusions, Extension, ManagedDependency, ParentRef, Scope as JvmScope,
    Version, VersionConstraint,
};

/// A parsed Maven POM file.
///
/// This struct represents the complete Project Object Model as defined by Maven.
/// It can be parsed from XML using [`crate::PomParser`].
///
/// # Examples
///
/// ```
/// use maven_pom::PomParser;
///
/// let xml = r#"
///     <project>
///         <groupId>com.example</groupId>
///         <artifactId>my-lib</artifactId>
///         <version>1.0.0</version>
///     </project>
/// "#;
///
/// let pom = PomParser::parse(xml).unwrap();
/// assert_eq!(pom.group_id, Some("com.example".to_string()));
/// ```
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

    /// Build profiles.
    #[serde(default)]
    pub profiles: Option<Profiles>,

    /// Resolved parent POM (not parsed from XML, populated during resolution).
    #[serde(skip)]
    pub resolved_parent: Option<Box<Pom>>,
}

/// Packaging type for a Maven artifact.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Packaging {
    /// JAR file (default).
    #[default]
    Jar,
    /// POM file (parent/BOM).
    Pom,
    /// Web application archive.
    War,
    /// Enterprise application archive.
    Ear,
    /// Resource adapter archive.
    Rar,
    /// Application client archive.
    Par,
    /// Enterprise `JavaBean`.
    Ejb,
    /// Maven plugin.
    MavenPlugin,
    /// `OSGi` bundle.
    Bundle,
}

impl Packaging {
    /// Parse a packaging string into a Packaging enum.
    ///
    /// Unknown packaging types default to `Jar`.
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pom" => Self::Pom,
            "war" => Self::War,
            "ear" => Self::Ear,
            "rar" => Self::Rar,
            "par" => Self::Par,
            "ejb" => Self::Ejb,
            "maven-plugin" => Self::MavenPlugin,
            "bundle" => Self::Bundle,
            // "jar" and unknown types default to Jar
            _ => Self::Jar,
        }
    }

    /// Returns the file extension for this packaging type.
    #[must_use]
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Pom => "pom",
            Self::War => "war",
            Self::Ear => "ear",
            Self::Rar => "rar",
            Self::Par => "par",
            Self::Jar | Self::Bundle | Self::Ejb | Self::MavenPlugin => "jar",
        }
    }
}

impl fmt::Display for Packaging {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Jar => write!(f, "jar"),
            Self::Pom => write!(f, "pom"),
            Self::War => write!(f, "war"),
            Self::Ear => write!(f, "ear"),
            Self::Rar => write!(f, "rar"),
            Self::Par => write!(f, "par"),
            Self::Ejb => write!(f, "ejb"),
            Self::MavenPlugin => write!(f, "maven-plugin"),
            Self::Bundle => write!(f, "bundle"),
        }
    }
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
    /// Returns the Maven coordinate string for this parent.
    #[must_use]
    pub fn coordinate(&self) -> String {
        format!("{}:{}:{}", self.group_id, self.artifact_id, self.version)
    }

    /// Converts this parent reference to a jvm-artifact Artifact.
    #[must_use]
    pub fn to_artifact(&self) -> Artifact {
        Artifact::new(&self.group_id, &self.artifact_id, &self.version).with_extension("pom")
    }

    /// Converts this parent reference to a jvm-artifact `ParentRef`.
    #[must_use]
    pub fn to_parent_ref(&self) -> ParentRef {
        let parent_ref = ParentRef::from_gav(&self.group_id, &self.artifact_id, &self.version);
        match &self.relative_path {
            Some(path) => parent_ref.with_relative_path(path),
            None => parent_ref,
        }
    }

    /// Converts this parent reference to jvm-artifact Coordinates.
    #[must_use]
    pub fn to_coordinates(&self) -> Coordinates {
        Coordinates::new(&self.group_id, &self.artifact_id)
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
    #[must_use]
    pub fn should_include(&self) -> bool {
        match self.scope.as_deref() {
            Some("test" | "provided" | "system") => false,
            _ => !self.optional.unwrap_or(false),
        }
    }

    /// Check if this is a BOM import.
    #[must_use]
    pub fn is_bom_import(&self) -> bool {
        self.scope.as_deref() == Some("import") && self.dep_type.as_deref() == Some("pom")
    }

    /// Get the dependency key (groupId:artifactId) for matching in dependency management.
    #[must_use]
    pub fn key(&self) -> String {
        format!("{}:{}", self.group_id, self.artifact_id)
    }

    /// Get the dependency key with classifier (groupId:artifactId:classifier) for more precise matching.
    #[must_use]
    pub fn key_with_classifier(&self) -> String {
        match &self.classifier {
            Some(c) if !c.is_empty() => {
                format!("{}:{}:{}", self.group_id, self.artifact_id, c)
            }
            _ => self.key(),
        }
    }

    /// Returns the Maven coordinate string for this dependency.
    #[must_use]
    #[allow(clippy::option_if_let_else)] // match is clearer for format strings
    pub fn coordinate(&self) -> String {
        match &self.version {
            Some(v) => format!("{}:{}:{}", self.group_id, self.artifact_id, v),
            None => format!("{}:{}", self.group_id, self.artifact_id),
        }
    }

    /// Parse the scope string into a Scope enum.
    #[must_use]
    pub fn parsed_scope(&self) -> Scope {
        self.scope.as_deref().map(Scope::parse).unwrap_or_default()
    }

    /// Converts this dependency to jvm-artifact Coordinates.
    #[must_use]
    pub fn to_coordinates(&self) -> Coordinates {
        Coordinates::new(&self.group_id, &self.artifact_id)
    }

    /// Converts this dependency to a jvm-artifact Artifact if version is present.
    #[must_use]
    pub fn to_artifact(&self) -> Option<Artifact> {
        let version = self.version.as_ref()?;
        let mut artifact = Artifact::new(&self.group_id, &self.artifact_id, version);

        if let Some(ref classifier) = self.classifier {
            artifact = artifact.with_classifier(classifier.as_str());
        }

        if let Some(ref dep_type) = self.dep_type {
            artifact = artifact.with_extension(dep_type.as_str());
        }

        Some(artifact)
    }

    /// Converts exclusions to jvm-artifact Exclusions.
    #[must_use]
    pub fn to_jvm_exclusions(&self) -> JvmExclusions {
        self.exclusions
            .as_ref()
            .map_or_else(JvmExclusions::none, |exclusions| {
                let list: Vec<JvmExclusion> = exclusions
                    .exclusions
                    .iter()
                    .map(|e| JvmExclusion::new(&e.group_id, &e.artifact_id))
                    .collect();
                JvmExclusions::from_list(list)
            })
    }

    /// Converts this dependency to a jvm-artifact Dependency.
    ///
    /// Returns None if the conversion cannot be performed (e.g., missing version
    /// that can't be parsed as a constraint).
    #[must_use]
    pub fn to_jvm_dependency(&self) -> JvmDependency {
        let coordinates = self.to_coordinates();
        let mut dep = JvmDependency::new(coordinates);

        if let Some(constraint) = self
            .version
            .as_ref()
            .and_then(|v| VersionConstraint::parse(v).ok())
        {
            dep = dep.with_version(constraint);
        }

        dep = dep.with_scope(self.parsed_scope().to_jvm_scope());

        if self.optional.unwrap_or(false) {
            dep = dep.optional();
        }

        if let Some(ref classifier) = self.classifier {
            dep = dep.with_classifier(classifier.as_str());
        }

        if let Some(ref dep_type) = self.dep_type {
            dep = dep.with_type(dep_type.as_str());
        }

        dep = dep.with_exclusions(self.to_jvm_exclusions());

        dep
    }

    /// Converts this dependency to a `ManagedDependency` if version is present.
    #[must_use]
    pub fn to_managed_dependency(&self) -> Option<ManagedDependency> {
        let version = self.version.as_ref()?;

        // Try to parse as VersionConstraint
        let version_constraint = VersionConstraint::parse(version).ok()?;

        let managed = ManagedDependency::new(self.to_coordinates())
            .with_version(version_constraint)
            .with_scope(self.parsed_scope().to_jvm_scope())
            .with_exclusions(self.to_jvm_exclusions());

        Some(managed)
    }
}

/// Maven dependency scope.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Scope {
    /// Compile scope (default) - available in all classpaths.
    #[default]
    Compile,
    /// Provided scope - expected to be provided by the runtime environment.
    Provided,
    /// Runtime scope - not needed for compilation, but needed at runtime.
    Runtime,
    /// Test scope - only available for test compilation and execution.
    Test,
    /// System scope - similar to provided, but path must be explicit.
    System,
    /// Import scope - used with pom type dependencies in dependencyManagement.
    Import,
}

impl Scope {
    /// Parse a scope string into a Scope enum.
    ///
    /// Unknown scopes default to `Compile`.
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "provided" => Self::Provided,
            "runtime" => Self::Runtime,
            "test" => Self::Test,
            "system" => Self::System,
            "import" => Self::Import,
            // "compile" and unknown scopes default to Compile
            _ => Self::Compile,
        }
    }

    /// Returns whether this scope is transitive.
    #[must_use]
    pub const fn is_transitive(&self) -> bool {
        matches!(self, Self::Compile | Self::Runtime)
    }

    /// Converts this scope to a jvm-artifact Scope.
    #[must_use]
    pub const fn to_jvm_scope(&self) -> JvmScope {
        match self {
            Self::Compile => JvmScope::Compile,
            Self::Provided => JvmScope::Provided,
            Self::Runtime => JvmScope::Runtime,
            Self::Test => JvmScope::Test,
            Self::System => JvmScope::System,
            Self::Import => JvmScope::Import,
        }
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compile => write!(f, "compile"),
            Self::Provided => write!(f, "provided"),
            Self::Runtime => write!(f, "runtime"),
            Self::Test => write!(f, "test"),
            Self::System => write!(f, "system"),
            Self::Import => write!(f, "import"),
        }
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
    /// Check if this exclusion matches a given group and artifact.
    #[must_use]
    pub fn matches(&self, group_id: &str, artifact_id: &str) -> bool {
        let group_matches = self.group_id == "*" || self.group_id == group_id;
        let artifact_matches = self.artifact_id == "*" || self.artifact_id == artifact_id;
        group_matches && artifact_matches
    }
}

impl Pom {
    /// Get the effective group ID (falls back to parent).
    #[must_use]
    pub fn effective_group_id(&self) -> Option<&str> {
        self.group_id
            .as_deref()
            .or_else(|| self.parent.as_ref().map(|p| p.group_id.as_str()))
    }

    /// Get the effective version (falls back to parent).
    #[must_use]
    pub fn effective_version(&self) -> Option<&str> {
        self.version
            .as_deref()
            .or_else(|| self.parent.as_ref().map(|p| p.version.as_str()))
    }

    /// Get the effective packaging type.
    #[must_use]
    pub fn effective_packaging(&self) -> Packaging {
        self.packaging
            .as_deref()
            .map(Packaging::parse)
            .unwrap_or_default()
    }

    /// Get the Maven coordinate string for this POM.
    #[must_use]
    pub fn coordinate(&self) -> Option<String> {
        let group_id = self.effective_group_id()?;
        let artifact_id = self.artifact_id.as_deref()?;
        let version = self.effective_version()?;
        Some(format!("{group_id}:{artifact_id}:{version}"))
    }

    /// Get all direct dependencies.
    #[must_use]
    pub fn direct_dependencies(&self) -> Vec<&Dependency> {
        self.dependencies
            .as_ref()
            .map(|d| d.dependencies.iter().collect())
            .unwrap_or_default()
    }

    /// Get managed dependencies (from dependencyManagement).
    #[must_use]
    pub fn managed_dependencies(&self) -> Vec<&Dependency> {
        self.dependency_management
            .as_ref()
            .and_then(|dm| dm.dependencies.as_ref())
            .map(|d| d.dependencies.iter().collect())
            .unwrap_or_default()
    }

    /// Get all managed dependencies including from parent POMs.
    #[must_use]
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
    #[must_use]
    pub fn all_properties(&self) -> HashMap<&str, &str> {
        let mut props = HashMap::new();

        // First, add parent's properties (these can be overridden)
        if let Some(ref parent) = self.resolved_parent {
            for (key, value) in parent.all_properties() {
                props.insert(key, value);
            }
        }

        // Then add this POM's properties (override parent's)
        for (key, value) in self.properties.iter() {
            props.insert(key.as_str(), value.as_str());
        }

        props
    }

    /// Get profiles defined in this POM.
    #[must_use]
    pub fn profiles(&self) -> Vec<&Profile> {
        self.profiles
            .as_ref()
            .map(|p| p.profiles.iter().collect())
            .unwrap_or_default()
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
    #[must_use]
    pub fn substitute_properties(&self, s: &str) -> String {
        // Build a merged properties with all properties from parents
        let mut merged = Properties::new();

        // Add all properties
        for (key, value) in self.all_properties() {
            merged.insert(key.to_string(), value.to_string());
        }

        merged.substitute_with_context(
            s,
            self.effective_version(),
            self.effective_group_id(),
            self.artifact_id.as_deref(),
            self.parent.as_ref().map(|p| p.version.as_str()),
            self.parent.as_ref().map(|p| p.group_id.as_str()),
        )
    }

    /// Apply dependency management to resolve version for a dependency.
    ///
    /// If the dependency doesn't have a version, looks it up in dependency management.
    #[must_use]
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
        if let Some(version) = managed.get(&dep.key()).and_then(|m| m.version.as_ref()) {
            return Some(self.substitute_properties(version));
        }

        // Return original version with properties substituted if we have one
        dep.version.as_ref().map(|v| self.substitute_properties(v))
    }

    /// Get BOM imports from dependency management.
    #[must_use]
    pub fn bom_imports(&self) -> Vec<&Dependency> {
        self.managed_dependencies()
            .into_iter()
            .filter(|d| d.is_bom_import())
            .collect()
    }

    // --- JVM-Artifact Conversion Methods ---

    /// Creates a jvm-artifact Artifact for this POM.
    ///
    /// Returns None if the POM doesn't have complete coordinates (group ID,
    /// artifact ID, and version).
    #[must_use]
    pub fn to_artifact(&self) -> Option<Artifact> {
        let group_id = self.effective_group_id()?;
        let artifact_id = self.artifact_id.as_deref()?;
        let version = self.effective_version()?;

        let mut artifact = Artifact::new(group_id, artifact_id, version);

        // Set extension based on packaging
        let packaging = self.effective_packaging();
        if packaging != Packaging::Jar {
            artifact = artifact.with_extension(packaging.extension());
        }

        Some(artifact)
    }

    /// Creates jvm-artifact Coordinates for this POM.
    ///
    /// Returns None if the POM doesn't have group ID and artifact ID.
    #[must_use]
    pub fn to_coordinates(&self) -> Option<Coordinates> {
        let group_id = self.effective_group_id()?;
        let artifact_id = self.artifact_id.as_deref()?;
        Some(Coordinates::new(group_id, artifact_id))
    }

    /// Returns the parent as a jvm-artifact `ParentRef`.
    #[must_use]
    pub fn to_parent_ref(&self) -> Option<ParentRef> {
        self.parent.as_ref().map(Parent::to_parent_ref)
    }

    /// Converts direct dependencies to jvm-artifact Dependencies.
    ///
    /// This performs property substitution and returns dependencies that can
    /// be used with the jvm-artifact resolver.
    #[must_use]
    pub fn jvm_dependencies(&self) -> Vec<JvmDependency> {
        self.direct_dependencies()
            .iter()
            .map(|dep| {
                let mut jvm_dep = dep.to_jvm_dependency();

                // Try to resolve version from dependency management
                // Note: clippy suggests let_chains but that's unstable
                #[allow(clippy::collapsible_if)]
                if jvm_dep.version.is_none() {
                    if let Some(constraint) = self
                        .resolve_dependency_version(dep)
                        .and_then(|v| VersionConstraint::parse(&v).ok())
                    {
                        jvm_dep = jvm_dep.with_version(constraint);
                    }
                }

                jvm_dep
            })
            .collect()
    }

    /// Converts managed dependencies to jvm-artifact `ManagedDependency` list.
    ///
    /// This performs property substitution on versions.
    #[must_use]
    pub fn jvm_managed_dependencies(&self) -> Vec<ManagedDependency> {
        self.managed_dependencies()
            .iter()
            .filter_map(|dep| {
                // First try to get the version with substitution
                let version = dep
                    .version
                    .as_ref()
                    .map(|v| self.substitute_properties(v))?;

                // Try to parse as VersionConstraint
                let version_constraint = VersionConstraint::parse(&version).ok()?;

                let managed = ManagedDependency::new(dep.to_coordinates())
                    .with_version(version_constraint)
                    .with_scope(dep.parsed_scope().to_jvm_scope())
                    .with_exclusions(dep.to_jvm_exclusions());

                Some(managed)
            })
            .collect()
    }

    /// Returns properties as a Vec of key-value pairs.
    ///
    /// This is useful for implementing the Project trait.
    #[must_use]
    pub fn properties_as_vec(&self) -> Vec<(String, String)> {
        self.all_properties()
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let exclusion = Exclusion {
            group_id: "com.example".to_string(),
            artifact_id: "lib".to_string(),
        };
        assert!(exclusion.matches("com.example", "lib"));
        assert!(!exclusion.matches("org.other", "lib"));

        let wildcard = Exclusion {
            group_id: "*".to_string(),
            artifact_id: "*".to_string(),
        };
        assert!(wildcard.matches("com.example", "lib"));
        assert!(wildcard.matches("org.other", "something"));
    }

    #[test]
    fn test_packaging_from_str() {
        assert_eq!(Packaging::parse("jar"), Packaging::Jar);
        assert_eq!(Packaging::parse("pom"), Packaging::Pom);
        assert_eq!(Packaging::parse("war"), Packaging::War);
        assert_eq!(Packaging::parse("maven-plugin"), Packaging::MavenPlugin);
        assert_eq!(Packaging::parse("unknown"), Packaging::Jar);
    }

    #[test]
    fn test_scope_from_str() {
        assert_eq!(Scope::parse("compile"), Scope::Compile);
        assert_eq!(Scope::parse("test"), Scope::Test);
        assert_eq!(Scope::parse("provided"), Scope::Provided);
        assert_eq!(Scope::parse("import"), Scope::Import);
        assert_eq!(Scope::parse("unknown"), Scope::Compile);
    }

    #[test]
    fn test_property_substitution() {
        let mut props = Properties::new();
        props.insert("kotlin.version".to_string(), "2.0.0".to_string());

        let pom = Pom {
            group_id: Some("com.example".to_string()),
            artifact_id: Some("my-lib".to_string()),
            version: Some("1.0.0".to_string()),
            properties: props,
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
    fn test_effective_group_id_from_parent() {
        let pom = Pom {
            artifact_id: Some("my-lib".to_string()),
            parent: Some(Parent {
                group_id: "com.example".to_string(),
                artifact_id: "parent".to_string(),
                version: "1.0.0".to_string(),
                relative_path: None,
            }),
            ..Default::default()
        };

        assert_eq!(pom.effective_group_id(), Some("com.example"));
    }

    // --- JVM-Artifact Conversion Tests ---

    #[test]
    fn test_pom_to_artifact() {
        let pom = Pom {
            group_id: Some("com.example".to_string()),
            artifact_id: Some("my-lib".to_string()),
            version: Some("1.0.0".to_string()),
            ..Default::default()
        };

        let artifact = pom.to_artifact().unwrap();
        assert_eq!(artifact.group_id(), "com.example");
        assert_eq!(artifact.artifact_id(), "my-lib");
        assert_eq!(artifact.version.as_str(), "1.0.0");
    }

    #[test]
    fn test_parent_to_parent_ref() {
        let parent = Parent {
            group_id: "com.example".to_string(),
            artifact_id: "parent".to_string(),
            version: "1.0.0".to_string(),
            relative_path: Some("../pom.xml".to_string()),
        };

        let parent_ref = parent.to_parent_ref();
        assert_eq!(parent_ref.group_id(), "com.example");
        assert_eq!(parent_ref.artifact_id(), "parent");
        assert_eq!(parent_ref.version.as_str(), "1.0.0");
        assert_eq!(parent_ref.relative_path, Some("../pom.xml".to_string()));
    }

    #[test]
    fn test_dependency_to_artifact() {
        let dep = Dependency {
            group_id: "org.slf4j".to_string(),
            artifact_id: "slf4j-api".to_string(),
            version: Some("2.0.0".to_string()),
            classifier: Some("sources".to_string()),
            ..Default::default()
        };

        let artifact = dep.to_artifact().unwrap();
        assert_eq!(artifact.group_id(), "org.slf4j");
        assert_eq!(artifact.artifact_id(), "slf4j-api");
        assert_eq!(artifact.version.as_str(), "2.0.0");
        assert_eq!(artifact.classifier.as_ref().unwrap().as_str(), "sources");
    }

    #[test]
    fn test_dependency_to_jvm_dependency() {
        let dep = Dependency {
            group_id: "org.slf4j".to_string(),
            artifact_id: "slf4j-api".to_string(),
            version: Some("2.0.0".to_string()),
            scope: Some("compile".to_string()),
            optional: Some(true),
            ..Default::default()
        };

        let jvm_dep = dep.to_jvm_dependency();
        assert_eq!(jvm_dep.group_id(), "org.slf4j");
        assert_eq!(jvm_dep.artifact_id(), "slf4j-api");
        assert!(jvm_dep.version.is_some());
        assert_eq!(jvm_dep.scope, JvmScope::Compile);
        assert!(jvm_dep.optional);
    }

    #[test]
    fn test_scope_to_jvm_scope() {
        assert_eq!(Scope::Compile.to_jvm_scope(), JvmScope::Compile);
        assert_eq!(Scope::Test.to_jvm_scope(), JvmScope::Test);
        assert_eq!(Scope::Runtime.to_jvm_scope(), JvmScope::Runtime);
        assert_eq!(Scope::Provided.to_jvm_scope(), JvmScope::Provided);
        assert_eq!(Scope::System.to_jvm_scope(), JvmScope::System);
        assert_eq!(Scope::Import.to_jvm_scope(), JvmScope::Import);
    }

    #[test]
    fn test_pom_jvm_dependencies() {
        let pom = Pom {
            group_id: Some("com.example".to_string()),
            artifact_id: Some("my-lib".to_string()),
            version: Some("1.0.0".to_string()),
            dependencies: Some(Dependencies {
                dependencies: vec![Dependency {
                    group_id: "org.slf4j".to_string(),
                    artifact_id: "slf4j-api".to_string(),
                    version: Some("2.0.0".to_string()),
                    ..Default::default()
                }],
            }),
            ..Default::default()
        };

        let jvm_deps = pom.jvm_dependencies();
        assert_eq!(jvm_deps.len(), 1);
        assert_eq!(jvm_deps[0].group_id(), "org.slf4j");
    }
}
