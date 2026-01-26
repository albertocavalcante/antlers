//! Dependency declarations.
//!
//! This module provides the [`Dependency`] type for representing dependency
//! declarations in Maven POMs and Gradle build files.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::artifact::{Classifier, Coordinates, Extension};
use crate::exclusions::Exclusions;
use crate::version::VersionConstraint;

/// A dependency declaration.
///
/// This represents a dependency as declared in a POM or build file,
/// before resolution. It includes the coordinates, version constraint,
/// scope, and exclusions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    /// The artifact coordinates (group + artifact).
    pub coordinates: Coordinates,
    /// The version constraint (may be absent if managed).
    pub version: Option<VersionConstraint>,
    /// The dependency scope.
    pub scope: Scope,
    /// Whether this dependency is optional.
    pub optional: bool,
    /// Optional classifier (e.g., "sources", "tests").
    pub classifier: Option<Classifier>,
    /// The dependency type/packaging (e.g., "jar", "pom").
    pub dep_type: Option<Extension>,
    /// Exclusions to apply to transitive dependencies.
    pub exclusions: Exclusions,
}

impl Dependency {
    /// Creates a new dependency with default values.
    #[must_use]
    pub fn new(coordinates: Coordinates) -> Self {
        Self {
            coordinates,
            version: None,
            scope: Scope::default(),
            optional: false,
            classifier: None,
            dep_type: None,
            exclusions: Exclusions::default(),
        }
    }

    /// Creates a new dependency with version.
    #[must_use]
    pub fn with_version(mut self, version: VersionConstraint) -> Self {
        self.version = Some(version);
        self
    }

    /// Creates a new dependency with scope.
    #[must_use]
    pub const fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    /// Marks this dependency as optional.
    #[must_use]
    pub const fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Creates a new dependency with classifier.
    #[must_use]
    pub fn with_classifier(mut self, classifier: impl Into<Classifier>) -> Self {
        self.classifier = Some(classifier.into());
        self
    }

    /// Creates a new dependency with type.
    #[must_use]
    pub fn with_type(mut self, dep_type: impl Into<Extension>) -> Self {
        self.dep_type = Some(dep_type.into());
        self
    }

    /// Creates a new dependency with exclusions.
    #[must_use]
    pub fn with_exclusions(mut self, exclusions: Exclusions) -> Self {
        self.exclusions = exclusions;
        self
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

    /// Returns true if this dependency should be included in the given scope context.
    ///
    /// For example, a `test` dependency is included when resolving for test,
    /// but not when resolving for compile.
    #[must_use]
    pub const fn is_applicable_for_scope(&self, context: Scope) -> bool {
        match context {
            Scope::Compile => matches!(self.scope, Scope::Compile),
            Scope::Runtime => matches!(self.scope, Scope::Compile | Scope::Runtime),
            Scope::Test => {
                matches!(
                    self.scope,
                    Scope::Compile | Scope::Runtime | Scope::Test | Scope::Provided
                )
            }
            Scope::Provided => matches!(self.scope, Scope::Compile | Scope::Provided),
            Scope::System => matches!(self.scope, Scope::System),
            Scope::Import => false, // Import is only for dependency management
        }
    }

    /// Returns true if this dependency is transitive (not provided or test).
    #[must_use]
    pub const fn is_transitive(&self) -> bool {
        !self.optional
            && !matches!(
                self.scope,
                Scope::Test | Scope::Provided | Scope::System | Scope::Import
            )
    }
}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.coordinates)?;
        if let Some(ref version) = self.version {
            write!(f, ":{version}")?;
        }
        if self.scope != Scope::Compile {
            write!(f, " ({})", self.scope)?;
        }
        if self.optional {
            write!(f, " [optional]")?;
        }
        Ok(())
    }
}

/// The scope of a dependency.
///
/// The scope determines when the dependency is available on the classpath
/// and whether it's included in transitive resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Scope {
    /// Compile scope - available at compile time and runtime.
    /// Transitive.
    #[default]
    Compile,

    /// Runtime scope - available at runtime only.
    /// Transitive.
    Runtime,

    /// Test scope - available at test compile and runtime only.
    /// Not transitive.
    Test,

    /// Provided scope - available at compile time, expected to be provided at runtime.
    /// Not transitive.
    Provided,

    /// System scope - similar to provided, but path to JAR is specified.
    /// Not transitive.
    System,

    /// Import scope - only used in dependency management for BOM imports.
    /// Not a real dependency scope.
    Import,
}

impl Scope {
    /// Parses a scope from a string.
    #[must_use]
    #[allow(clippy::should_implement_trait)]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "compile" | "" => Some(Self::Compile),
            "runtime" => Some(Self::Runtime),
            "test" => Some(Self::Test),
            "provided" => Some(Self::Provided),
            "system" => Some(Self::System),
            "import" => Some(Self::Import),
            _ => None,
        }
    }

    /// Returns the string representation of the scope.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Compile => "compile",
            Self::Runtime => "runtime",
            Self::Test => "test",
            Self::Provided => "provided",
            Self::System => "system",
            Self::Import => "import",
        }
    }

    /// Returns true if dependencies with this scope are transitive.
    #[must_use]
    pub const fn is_transitive(&self) -> bool {
        matches!(self, Self::Compile | Self::Runtime)
    }

    /// Returns the effective scope when a dependency of this scope
    /// depends transitively on a dependency with the given scope.
    ///
    /// For example, if A (compile) -> B (runtime) -> C (compile),
    /// then C is effectively `runtime` from A's perspective.
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub const fn resolve_transitive(&self, transitive_scope: Self) -> Option<Self> {
        match (self, transitive_scope) {
            // Compile dependencies
            (Self::Compile, Self::Compile) => Some(Self::Compile),
            (Self::Compile, Self::Runtime) => Some(Self::Runtime),
            (Self::Compile, Self::Provided | Self::Test | Self::System | Self::Import) => None,

            // Runtime dependencies
            (Self::Runtime, Self::Compile | Self::Runtime) => Some(Self::Runtime),
            (Self::Runtime, Self::Provided | Self::Test | Self::System | Self::Import) => None,

            // Other scopes don't propagate transitively
            (Self::Test | Self::Provided | Self::System | Self::Import, _) => None,
        }
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::Version;

    #[test]
    fn test_dependency_new() {
        let dep = Dependency::new(Coordinates::new("org.apache.commons", "commons-lang3"));
        assert_eq!(dep.group_id(), "org.apache.commons");
        assert_eq!(dep.artifact_id(), "commons-lang3");
        assert_eq!(dep.scope, Scope::Compile);
        assert!(!dep.optional);
    }

    #[test]
    fn test_dependency_builder() {
        let dep = Dependency::new(Coordinates::new("junit", "junit"))
            .with_version(VersionConstraint::Exact(Version::new("4.13.2")))
            .with_scope(Scope::Test)
            .optional();

        assert_eq!(dep.scope, Scope::Test);
        assert!(dep.optional);
        assert!(dep.version.is_some());
    }

    #[test]
    fn test_scope_parse() {
        assert_eq!(Scope::parse("compile"), Some(Scope::Compile));
        assert_eq!(Scope::parse("runtime"), Some(Scope::Runtime));
        assert_eq!(Scope::parse("test"), Some(Scope::Test));
        assert_eq!(Scope::parse("provided"), Some(Scope::Provided));
        assert_eq!(Scope::parse("system"), Some(Scope::System));
        assert_eq!(Scope::parse("import"), Some(Scope::Import));
        assert_eq!(Scope::parse(""), Some(Scope::Compile));
        assert_eq!(Scope::parse("invalid"), None);
    }

    #[test]
    fn test_scope_transitivity() {
        assert!(Scope::Compile.is_transitive());
        assert!(Scope::Runtime.is_transitive());
        assert!(!Scope::Test.is_transitive());
        assert!(!Scope::Provided.is_transitive());
    }

    #[test]
    fn test_scope_resolve_transitive() {
        // compile -> compile = compile
        assert_eq!(
            Scope::Compile.resolve_transitive(Scope::Compile),
            Some(Scope::Compile)
        );

        // compile -> runtime = runtime
        assert_eq!(
            Scope::Compile.resolve_transitive(Scope::Runtime),
            Some(Scope::Runtime)
        );

        // compile -> test = nothing
        assert_eq!(Scope::Compile.resolve_transitive(Scope::Test), None);

        // runtime -> compile = runtime
        assert_eq!(
            Scope::Runtime.resolve_transitive(Scope::Compile),
            Some(Scope::Runtime)
        );

        // test -> anything = nothing
        assert_eq!(Scope::Test.resolve_transitive(Scope::Compile), None);
    }

    #[test]
    fn test_dependency_is_applicable_for_scope() {
        let compile_dep = Dependency::new(Coordinates::new("a", "b")).with_scope(Scope::Compile);
        let test_dep = Dependency::new(Coordinates::new("c", "d")).with_scope(Scope::Test);
        let provided_dep = Dependency::new(Coordinates::new("e", "f")).with_scope(Scope::Provided);

        // Compile dependencies available in compile and test contexts
        assert!(compile_dep.is_applicable_for_scope(Scope::Compile));
        assert!(compile_dep.is_applicable_for_scope(Scope::Runtime));
        assert!(compile_dep.is_applicable_for_scope(Scope::Test));

        // Test dependencies only in test context
        assert!(!test_dep.is_applicable_for_scope(Scope::Compile));
        assert!(test_dep.is_applicable_for_scope(Scope::Test));

        // Provided available in compile context
        assert!(!provided_dep.is_applicable_for_scope(Scope::Compile));
        assert!(provided_dep.is_applicable_for_scope(Scope::Provided));
        assert!(provided_dep.is_applicable_for_scope(Scope::Test));
    }
}
