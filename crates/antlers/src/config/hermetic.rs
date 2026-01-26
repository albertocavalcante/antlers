//! Hermeticity configuration for build system integration.
//!
//! This module defines hermeticity levels that control how strictly
//! antler isolates itself from the environment.

/// Hermeticity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HermeticLevel {
    /// No hermeticity guarantees.
    ///
    /// - May read from environment
    /// - May use automatic cache paths
    /// - May access network
    /// - Output ordering not guaranteed
    Disabled,

    /// Reproducible output only.
    ///
    /// - Output is deterministically ordered
    /// - Same inputs produce same outputs
    /// - May still access network and environment
    Reproducible,

    /// Strict hermeticity for sandboxed builds.
    ///
    /// - No environment access (unless explicit)
    /// - No network access (offline only)
    /// - Explicit cache path required
    /// - Deterministic output
    /// - Suitable for Buck2/Bazel
    Strict,
}

/// Hermeticity configuration.
#[derive(Debug, Clone)]
pub struct HermeticConfig {
    /// Hermeticity level.
    pub level: HermeticLevel,

    /// Whether to sort output deterministically.
    pub deterministic_order: bool,

    /// Whether to fail on non-hermetic operations.
    pub fail_on_violation: bool,

    /// Whether to log non-hermetic operations.
    pub log_violations: bool,
}

impl Default for HermeticConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

impl HermeticConfig {
    /// Creates a disabled hermeticity configuration.
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            level: HermeticLevel::Disabled,
            deterministic_order: false,
            fail_on_violation: false,
            log_violations: false,
        }
    }

    /// Creates a reproducible configuration.
    ///
    /// Ensures deterministic output but allows network/environment access.
    #[must_use]
    pub const fn reproducible() -> Self {
        Self {
            level: HermeticLevel::Reproducible,
            deterministic_order: true,
            fail_on_violation: false,
            log_violations: true,
        }
    }

    /// Creates a strict hermetic configuration.
    ///
    /// Required for Buck2/Bazel integration.
    #[must_use]
    pub const fn strict() -> Self {
        Self {
            level: HermeticLevel::Strict,
            deterministic_order: true,
            fail_on_violation: true,
            log_violations: true,
        }
    }

    /// Returns whether output should be deterministically ordered.
    #[must_use]
    pub const fn should_sort_output(&self) -> bool {
        self.deterministic_order
    }

    /// Returns whether non-hermetic operations should fail.
    #[must_use]
    pub const fn should_fail_on_violation(&self) -> bool {
        self.fail_on_violation
    }

    /// Returns whether this is a hermetic configuration.
    #[must_use]
    pub const fn is_hermetic(&self) -> bool {
        matches!(self.level, HermeticLevel::Strict)
    }

    /// Returns whether this is at least reproducible.
    #[must_use]
    pub const fn is_reproducible(&self) -> bool {
        matches!(
            self.level,
            HermeticLevel::Strict | HermeticLevel::Reproducible
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled() {
        let config = HermeticConfig::disabled();
        assert_eq!(config.level, HermeticLevel::Disabled);
        assert!(!config.is_hermetic());
        assert!(!config.is_reproducible());
    }

    #[test]
    fn test_reproducible() {
        let config = HermeticConfig::reproducible();
        assert_eq!(config.level, HermeticLevel::Reproducible);
        assert!(!config.is_hermetic());
        assert!(config.is_reproducible());
        assert!(config.should_sort_output());
    }

    #[test]
    fn test_strict() {
        let config = HermeticConfig::strict();
        assert_eq!(config.level, HermeticLevel::Strict);
        assert!(config.is_hermetic());
        assert!(config.is_reproducible());
        assert!(config.should_fail_on_violation());
    }
}
