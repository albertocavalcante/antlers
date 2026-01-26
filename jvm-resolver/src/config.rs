//! Resolution configuration options.
//!
//! This module provides [`ResolverConfig`] for customizing dependency resolution behavior.

use std::collections::HashSet;

use jvm_artifact::Scope;

/// Configuration for the dependency resolver.
#[derive(Debug, Clone)]
pub struct ResolverConfig {
    /// Whether to resolve transitive dependencies.
    ///
    /// When false, only the direct dependencies are resolved.
    /// Default: `true`
    pub transitive: bool,

    /// Whether to include optional dependencies.
    ///
    /// Default: `false`
    pub include_optional: bool,

    /// Scopes to include in resolution.
    ///
    /// If empty, the default behavior is to include compile and runtime scopes,
    /// excluding test, provided, and system.
    ///
    /// Default: empty (use default behavior)
    pub include_scopes: HashSet<Scope>,

    /// Maximum depth for transitive dependency resolution.
    ///
    /// This prevents infinite loops in case of circular dependencies that
    /// slip through detection.
    ///
    /// Default: `100`
    pub max_depth: usize,

    /// Maximum depth for parent POM/project resolution.
    ///
    /// Limits how deep the resolver will go when resolving parent chains.
    ///
    /// Default: `10`
    pub max_parent_depth: usize,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            transitive: true,
            include_optional: false,
            include_scopes: HashSet::new(),
            max_depth: 100,
            max_parent_depth: 10,
        }
    }
}

impl ResolverConfig {
    /// Creates a new configuration with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether to resolve transitive dependencies.
    #[must_use]
    pub const fn transitive(mut self, transitive: bool) -> Self {
        self.transitive = transitive;
        self
    }

    /// Sets whether to include optional dependencies.
    #[must_use]
    pub const fn include_optional(mut self, include: bool) -> Self {
        self.include_optional = include;
        self
    }

    /// Adds a scope to include.
    #[must_use]
    pub fn include_scope(mut self, scope: Scope) -> Self {
        self.include_scopes.insert(scope);
        self
    }

    /// Sets the maximum resolution depth.
    #[must_use]
    pub const fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Sets the maximum parent resolution depth.
    #[must_use]
    pub const fn max_parent_depth(mut self, depth: usize) -> Self {
        self.max_parent_depth = depth;
        self
    }

    /// Returns true if the given scope should be included in resolution.
    ///
    /// If `include_scopes` is empty, uses the default behavior of excluding
    /// test, provided, and system scopes.
    #[must_use]
    pub fn should_include_scope(&self, scope: &Scope) -> bool {
        if self.include_scopes.is_empty() {
            // Default behavior: exclude test, provided, system
            !matches!(scope, Scope::Test | Scope::Provided | Scope::System)
        } else {
            self.include_scopes.contains(scope)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ResolverConfig::default();
        assert!(config.transitive);
        assert!(!config.include_optional);
        assert!(config.include_scopes.is_empty());
        assert_eq!(config.max_depth, 100);
        assert_eq!(config.max_parent_depth, 10);
    }

    #[test]
    fn test_default_scope_inclusion() {
        let config = ResolverConfig::default();

        // Should include
        assert!(config.should_include_scope(&Scope::Compile));
        assert!(config.should_include_scope(&Scope::Runtime));

        // Should exclude
        assert!(!config.should_include_scope(&Scope::Test));
        assert!(!config.should_include_scope(&Scope::Provided));
        assert!(!config.should_include_scope(&Scope::System));
    }

    #[test]
    fn test_explicit_scope_inclusion() {
        let config = ResolverConfig::new().include_scope(Scope::Test);

        // Only test is included when scopes are explicit
        assert!(config.should_include_scope(&Scope::Test));
        assert!(!config.should_include_scope(&Scope::Compile));
        assert!(!config.should_include_scope(&Scope::Runtime));
    }

    #[test]
    fn test_builder_pattern() {
        let config = ResolverConfig::new()
            .transitive(false)
            .include_optional(true)
            .max_depth(50)
            .max_parent_depth(5);

        assert!(!config.transitive);
        assert!(config.include_optional);
        assert_eq!(config.max_depth, 50);
        assert_eq!(config.max_parent_depth, 5);
    }
}
