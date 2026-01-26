//! Efficient exclusion matching for dependencies.
//!
//! This module provides the [`Exclusions`] type for efficiently checking if an
//! artifact should be excluded from resolution. It's based on Coursier's
//! `MinimizedExclusions` pattern for optimal performance.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::artifact::Artifact;

/// A single exclusion pattern.
///
/// Exclusions can match by group ID, artifact ID, or both.
/// A wildcard "*" matches any value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Exclusion {
    /// The group ID pattern (or "*" for any).
    pub group_id: String,
    /// The artifact ID pattern (or "*" for any).
    pub artifact_id: String,
}

impl Exclusion {
    /// Creates a new exclusion.
    #[must_use]
    pub fn new(group_id: impl Into<String>, artifact_id: impl Into<String>) -> Self {
        Self {
            group_id: group_id.into(),
            artifact_id: artifact_id.into(),
        }
    }

    /// Creates an exclusion that matches all artifacts in a group.
    #[must_use]
    pub fn by_group(group_id: impl Into<String>) -> Self {
        Self {
            group_id: group_id.into(),
            artifact_id: "*".to_string(),
        }
    }

    /// Creates an exclusion that matches a specific artifact in any group.
    #[must_use]
    pub fn by_artifact(artifact_id: impl Into<String>) -> Self {
        Self {
            group_id: "*".to_string(),
            artifact_id: artifact_id.into(),
        }
    }

    /// Creates an exclusion that matches all artifacts.
    #[must_use]
    pub fn all() -> Self {
        Self {
            group_id: "*".to_string(),
            artifact_id: "*".to_string(),
        }
    }

    /// Returns true if this exclusion is a wildcard for all artifacts.
    #[must_use]
    pub fn is_all(&self) -> bool {
        self.group_id == "*" && self.artifact_id == "*"
    }

    /// Returns true if this exclusion matches the given artifact.
    #[must_use]
    pub fn matches(&self, artifact: &Artifact) -> bool {
        let group_matches = self.group_id == "*" || self.group_id == artifact.coordinates.group_id;
        let artifact_matches =
            self.artifact_id == "*" || self.artifact_id == artifact.coordinates.artifact_id;
        group_matches && artifact_matches
    }
}

/// A collection of exclusions optimized for fast matching.
///
/// This type uses a "minimized" representation that categorizes exclusions
/// for efficient lookup:
/// - Exclusions that match all artifacts from a group
/// - Exclusions that match a specific artifact from any group
/// - Exact group:artifact exclusions
///
/// # Examples
///
/// ```
/// use gav::{Artifact, Exclusion, Exclusions};
///
/// let mut exclusions = Exclusions::default();
/// exclusions.add(Exclusion::new("org.slf4j", "*"));
/// exclusions.add(Exclusion::new("commons-logging", "commons-logging"));
///
/// let slf4j = Artifact::new("org.slf4j", "slf4j-api", "1.7.30");
/// assert!(exclusions.matches(&slf4j));
///
/// let guava = Artifact::new("com.google.guava", "guava", "31.0-jre");
/// assert!(!exclusions.matches(&guava));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Exclusions {
    /// No exclusions.
    #[default]
    None,
    /// Exclude all transitive dependencies.
    All,
    /// Specific exclusion patterns.
    Specific {
        /// Exclude all artifacts from these groups (`group_id` -> *).
        by_group: HashSet<String>,
        /// Exclude these artifacts from any group (* -> `artifact_id`).
        by_artifact: HashSet<String>,
        /// Exact exclusions (`group_id`, `artifact_id`).
        exact: HashSet<(String, String)>,
    },
}

impl Exclusions {
    /// Creates an empty exclusions set.
    #[must_use]
    pub const fn none() -> Self {
        Self::None
    }

    /// Creates an exclusions set that excludes all artifacts.
    #[must_use]
    pub const fn all() -> Self {
        Self::All
    }

    /// Creates exclusions from a list of exclusion patterns.
    #[must_use]
    pub fn from_list(exclusions: Vec<Exclusion>) -> Self {
        if exclusions.is_empty() {
            return Self::None;
        }

        let mut result = Self::None;
        for exclusion in exclusions {
            result.add(exclusion);
        }
        result
    }

    /// Returns true if this exclusions set is empty (no exclusions).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns true if this excludes all artifacts.
    #[must_use]
    pub const fn excludes_all(&self) -> bool {
        matches!(self, Self::All)
    }

    /// Returns true if the given artifact should be excluded.
    #[must_use]
    pub fn matches(&self, artifact: &Artifact) -> bool {
        match self {
            Self::None => false,
            Self::All => true,
            Self::Specific {
                by_group,
                by_artifact,
                exact,
            } => {
                by_group.contains(&artifact.coordinates.group_id)
                    || by_artifact.contains(&artifact.coordinates.artifact_id)
                    || exact.contains(&(
                        artifact.coordinates.group_id.clone(),
                        artifact.coordinates.artifact_id.clone(),
                    ))
            }
        }
    }

    /// Adds an exclusion to this set.
    pub fn add(&mut self, exclusion: Exclusion) {
        if exclusion.is_all() {
            *self = Self::All;
            return;
        }

        if matches!(self, Self::All) {
            return; // Already excluding everything
        }

        // Ensure we have a Specific variant
        if matches!(self, Self::None) {
            *self = Self::Specific {
                by_group: HashSet::new(),
                by_artifact: HashSet::new(),
                exact: HashSet::new(),
            };
        }

        if let Self::Specific {
            by_group,
            by_artifact,
            exact,
        } = self
        {
            if exclusion.artifact_id == "*" {
                // Exclude all from group
                by_group.insert(exclusion.group_id);
            } else if exclusion.group_id == "*" {
                // Exclude artifact from any group
                by_artifact.insert(exclusion.artifact_id);
            } else {
                // Exact exclusion
                exact.insert((exclusion.group_id, exclusion.artifact_id));
            }
        }
    }

    /// Returns the union of this exclusions set with another.
    ///
    /// The result will exclude artifacts that are excluded by either set.
    #[must_use]
    #[allow(clippy::similar_names)] // bg1/ba1/bg2/ba2 are clear in context
    pub fn join(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::None, other) => other.clone(),
            (this, Self::None) => this.clone(),
            (Self::All, _) | (_, Self::All) => Self::All,
            (
                Self::Specific {
                    by_group: bg1,
                    by_artifact: ba1,
                    exact: e1,
                },
                Self::Specific {
                    by_group: bg2,
                    by_artifact: ba2,
                    exact: e2,
                },
            ) => Self::Specific {
                by_group: bg1.union(bg2).cloned().collect(),
                by_artifact: ba1.union(ba2).cloned().collect(),
                exact: e1.union(e2).cloned().collect(),
            },
        }
    }

    /// Returns the intersection of this exclusions set with another.
    ///
    /// The result will only exclude artifacts that are excluded by both sets.
    #[must_use]
    #[allow(clippy::similar_names)] // bg1/ba1/bg2/ba2 are clear in context
    pub fn intersect(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::None, _) | (_, Self::None) => Self::None,
            (Self::All, other) => other.clone(),
            (this, Self::All) => this.clone(),
            (
                Self::Specific {
                    by_group: bg1,
                    by_artifact: ba1,
                    exact: e1,
                },
                Self::Specific {
                    by_group: bg2,
                    by_artifact: ba2,
                    exact: e2,
                },
            ) => {
                let by_group: HashSet<_> = bg1.intersection(bg2).cloned().collect();
                let by_artifact: HashSet<_> = ba1.intersection(ba2).cloned().collect();
                let exact: HashSet<_> = e1.intersection(e2).cloned().collect();

                if by_group.is_empty() && by_artifact.is_empty() && exact.is_empty() {
                    Self::None
                } else {
                    Self::Specific {
                        by_group,
                        by_artifact,
                        exact,
                    }
                }
            }
        }
    }

    /// Returns the number of exclusion patterns.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::None => 0,
            Self::All => 1,
            Self::Specific {
                by_group,
                by_artifact,
                exact,
            } => by_group.len() + by_artifact.len() + exact.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exclusion_matches() {
        let artifact = Artifact::new("org.slf4j", "slf4j-api", "1.7.30");

        // Exact match
        let exact = Exclusion::new("org.slf4j", "slf4j-api");
        assert!(exact.matches(&artifact));

        // Group wildcard
        let by_group = Exclusion::by_group("org.slf4j");
        assert!(by_group.matches(&artifact));

        // Artifact wildcard
        let by_artifact = Exclusion::by_artifact("slf4j-api");
        assert!(by_artifact.matches(&artifact));

        // Non-matching
        let other = Exclusion::new("com.google.guava", "guava");
        assert!(!other.matches(&artifact));
    }

    #[test]
    fn test_exclusions_none() {
        let exclusions = Exclusions::none();
        let artifact = Artifact::new("org.slf4j", "slf4j-api", "1.7.30");
        assert!(!exclusions.matches(&artifact));
        assert!(exclusions.is_empty());
    }

    #[test]
    fn test_exclusions_all() {
        let exclusions = Exclusions::all();
        let artifact = Artifact::new("org.slf4j", "slf4j-api", "1.7.30");
        assert!(exclusions.matches(&artifact));
        assert!(exclusions.excludes_all());
    }

    #[test]
    fn test_exclusions_specific() {
        let mut exclusions = Exclusions::default();
        exclusions.add(Exclusion::by_group("org.slf4j"));
        exclusions.add(Exclusion::new("commons-logging", "commons-logging"));

        let slf4j = Artifact::new("org.slf4j", "slf4j-api", "1.7.30");
        assert!(exclusions.matches(&slf4j));

        let log4j = Artifact::new("org.slf4j", "log4j-over-slf4j", "1.7.30");
        assert!(exclusions.matches(&log4j));

        let commons = Artifact::new("commons-logging", "commons-logging", "1.2");
        assert!(exclusions.matches(&commons));

        let guava = Artifact::new("com.google.guava", "guava", "31.0-jre");
        assert!(!exclusions.matches(&guava));
    }

    #[test]
    fn test_exclusions_join() {
        let mut e1 = Exclusions::default();
        e1.add(Exclusion::by_group("org.slf4j"));

        let mut e2 = Exclusions::default();
        e2.add(Exclusion::new("commons-logging", "commons-logging"));

        let joined = e1.join(&e2);

        let slf4j = Artifact::new("org.slf4j", "slf4j-api", "1.7.30");
        assert!(joined.matches(&slf4j));

        let commons = Artifact::new("commons-logging", "commons-logging", "1.2");
        assert!(joined.matches(&commons));
    }

    #[test]
    fn test_exclusions_from_list() {
        let exclusions = Exclusions::from_list(vec![
            Exclusion::by_group("org.slf4j"),
            Exclusion::new("commons-logging", "commons-logging"),
        ]);

        let slf4j = Artifact::new("org.slf4j", "slf4j-api", "1.7.30");
        assert!(exclusions.matches(&slf4j));
    }

    #[test]
    fn test_add_all_exclusion() {
        let mut exclusions = Exclusions::default();
        exclusions.add(Exclusion::by_group("org.slf4j"));
        exclusions.add(Exclusion::all());

        assert!(exclusions.excludes_all());
    }
}
