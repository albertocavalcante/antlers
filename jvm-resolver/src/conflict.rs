//! Version conflict resolution strategies.
//!
//! This module provides the [`ConflictStrategy`] trait and several built-in
//! strategies for resolving version conflicts during dependency resolution.

use jvm_artifact::Version;
use serde::{Deserialize, Serialize};

/// The result of a version conflict resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionChoice {
    /// Keep the existing (already selected) version.
    KeepExisting,
    /// Use the new version instead.
    UseNew,
}

/// A strategy for resolving version conflicts.
///
/// When the same artifact is requested with different versions during resolution,
/// this trait determines which version to use.
pub trait ConflictStrategy: Send + Sync {
    /// Resolves a version conflict.
    ///
    /// # Arguments
    ///
    /// * `existing` - The version already selected
    /// * `existing_depth` - The depth at which the existing version was selected
    /// * `new` - The new version being requested
    /// * `new_depth` - The depth at which the new version is requested
    ///
    /// # Returns
    ///
    /// Whether to keep the existing version or use the new one.
    fn resolve(
        &self,
        existing: &Version,
        existing_depth: usize,
        new: &Version,
        new_depth: usize,
    ) -> VersionChoice;

    /// Returns the name of this strategy for logging/reporting.
    fn name(&self) -> &'static str;
}

/// Maven's default conflict resolution: nearest definition wins.
///
/// When the same artifact is found at different depths in the dependency tree,
/// the version closest to the root (lowest depth) wins. If depths are equal,
/// the first one encountered is kept.
///
/// # Example
///
/// ```text
/// A -> B:1.0
/// A -> C -> B:2.0
/// ```
///
/// With `NearestWins`, B:1.0 is selected because it's at depth 1, while B:2.0
/// is at depth 2.
#[derive(Debug, Clone, Copy, Default)]
pub struct NearestWins;

impl ConflictStrategy for NearestWins {
    fn resolve(
        &self,
        _existing: &Version,
        existing_depth: usize,
        _new: &Version,
        new_depth: usize,
    ) -> VersionChoice {
        if new_depth < existing_depth {
            VersionChoice::UseNew
        } else {
            VersionChoice::KeepExisting
        }
    }

    fn name(&self) -> &'static str {
        "nearest-wins"
    }
}

/// Always select the highest version.
///
/// This strategy always selects the highest version regardless of where
/// it appears in the dependency tree.
///
/// # Example
///
/// ```text
/// A -> B:1.0
/// A -> C -> B:2.0
/// ```
///
/// With `HighestWins`, B:2.0 is selected because 2.0 > 1.0.
#[derive(Debug, Clone, Copy, Default)]
pub struct HighestWins;

impl ConflictStrategy for HighestWins {
    fn resolve(
        &self,
        existing: &Version,
        _existing_depth: usize,
        new: &Version,
        _new_depth: usize,
    ) -> VersionChoice {
        if new > existing {
            VersionChoice::UseNew
        } else {
            VersionChoice::KeepExisting
        }
    }

    fn name(&self) -> &'static str {
        "highest-wins"
    }
}

/// Strict conflict resolution: fail on any conflict.
///
/// This strategy reports an error if any version conflict is detected.
/// It's useful for builds that require deterministic dependency versions.
///
/// Note: This strategy always returns `KeepExisting` because the actual
/// failure is handled by the resolver when it detects that the strategy
/// is `StrictFails`.
#[derive(Debug, Clone, Copy, Default)]
pub struct StrictFails;

impl ConflictStrategy for StrictFails {
    fn resolve(
        &self,
        _existing: &Version,
        _existing_depth: usize,
        _new: &Version,
        _new_depth: usize,
    ) -> VersionChoice {
        // The resolver will check if we're using StrictFails and return an error
        // before this is actually used for selection.
        VersionChoice::KeepExisting
    }

    fn name(&self) -> &'static str {
        "strict"
    }
}

/// A record of a version conflict encountered during resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionConflict {
    /// The artifact coordinates (groupId:artifactId) that had a conflict.
    pub artifact: String,
    /// The versions that were requested.
    pub versions: Vec<String>,
    /// The version that was selected.
    pub selected: String,
    /// The strategy that was used to resolve the conflict.
    pub strategy: String,
}

impl VersionConflict {
    /// Creates a new version conflict record.
    #[must_use]
    pub fn new(
        artifact: impl Into<String>,
        versions: Vec<String>,
        selected: impl Into<String>,
        strategy: impl Into<String>,
    ) -> Self {
        Self {
            artifact: artifact.into(),
            versions,
            selected: selected.into(),
            strategy: strategy.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn test_nearest_wins() {
        let strategy = NearestWins;

        // New is nearer -> use new
        assert_eq!(
            strategy.resolve(&v("1.0"), 2, &v("2.0"), 1),
            VersionChoice::UseNew
        );

        // Existing is nearer -> keep existing
        assert_eq!(
            strategy.resolve(&v("1.0"), 1, &v("2.0"), 2),
            VersionChoice::KeepExisting
        );

        // Same depth -> keep existing
        assert_eq!(
            strategy.resolve(&v("1.0"), 2, &v("2.0"), 2),
            VersionChoice::KeepExisting
        );
    }

    #[test]
    fn test_highest_wins() {
        let strategy = HighestWins;

        // New is higher -> use new
        assert_eq!(
            strategy.resolve(&v("1.0"), 1, &v("2.0"), 2),
            VersionChoice::UseNew
        );

        // Existing is higher -> keep existing
        assert_eq!(
            strategy.resolve(&v("2.0"), 2, &v("1.0"), 1),
            VersionChoice::KeepExisting
        );

        // Same version -> keep existing
        assert_eq!(
            strategy.resolve(&v("1.0"), 1, &v("1.0"), 2),
            VersionChoice::KeepExisting
        );
    }

    #[test]
    fn test_strategy_names() {
        assert_eq!(NearestWins.name(), "nearest-wins");
        assert_eq!(HighestWins.name(), "highest-wins");
        assert_eq!(StrictFails.name(), "strict");
    }

    #[test]
    fn test_version_conflict_creation() {
        let conflict = VersionConflict::new(
            "com.example:lib",
            vec!["1.0".to_string(), "2.0".to_string()],
            "2.0",
            "highest-wins",
        );

        assert_eq!(conflict.artifact, "com.example:lib");
        assert_eq!(conflict.versions, vec!["1.0", "2.0"]);
        assert_eq!(conflict.selected, "2.0");
        assert_eq!(conflict.strategy, "highest-wins");
    }
}
