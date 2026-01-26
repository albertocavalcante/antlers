//! Version handling with ranges and constraints.
//!
//! This module provides types for parsing and comparing Maven version strings,
//! as well as version constraints (ranges, exact versions, latest).

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// A parsed Maven version.
///
/// Maven versions follow a complex comparison algorithm. This implementation
/// handles the most common patterns including numeric versions with qualifiers.
///
/// # Examples
///
/// ```
/// use jvm_artifact::Version;
///
/// let v1 = Version::parse("1.2.3").unwrap();
/// let v2 = Version::parse("1.2.4").unwrap();
/// assert!(v1 < v2);
///
/// let v3 = Version::parse("1.0-SNAPSHOT").unwrap();
/// let v4 = Version::parse("1.0").unwrap();
/// assert!(v3 < v4); // SNAPSHOT is less than release
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    /// The original version string.
    pub raw: String,
    /// Parsed numeric components (major, minor, patch, etc.).
    components: Vec<u64>,
    /// Optional qualifier (e.g., "SNAPSHOT", "alpha", "beta", "RC1").
    qualifier: Option<String>,
}

impl Version {
    /// Parses a version string into a `Version`.
    ///
    /// # Errors
    ///
    /// Returns an error if the version string is empty.
    pub fn parse(s: &str) -> Result<Self> {
        if s.is_empty() {
            return Err(Error::InvalidVersion("version cannot be empty".to_string()));
        }

        let raw = s.to_string();
        let (numeric_part, qualifier) = Self::split_qualifier(s);
        let components = Self::parse_components(numeric_part);

        Ok(Self {
            raw,
            components,
            qualifier,
        })
    }

    /// Creates a new version from raw string without validation.
    ///
    /// This is useful when you have a version string that you trust to be valid.
    #[must_use]
    pub fn new(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let (numeric_part, qualifier) = Self::split_qualifier(&raw);
        let components = Self::parse_components(numeric_part);

        Self {
            raw,
            components,
            qualifier,
        }
    }

    /// Returns the raw version string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// Returns the numeric components of the version.
    #[must_use]
    pub fn components(&self) -> &[u64] {
        &self.components
    }

    /// Returns the qualifier if present.
    #[must_use]
    pub fn qualifier(&self) -> Option<&str> {
        self.qualifier.as_deref()
    }

    /// Returns true if this is a snapshot version.
    #[must_use]
    pub fn is_snapshot(&self) -> bool {
        self.raw.contains("SNAPSHOT")
    }

    /// Splits a version string into numeric part and optional qualifier.
    fn split_qualifier(s: &str) -> (&str, Option<String>) {
        // Check for hyphen separator (e.g., "1.0-SNAPSHOT", "1.0-alpha")
        if let Some(idx) = s.find('-') {
            let (numeric, qual) = s.split_at(idx);
            return (numeric, Some(qual[1..].to_string()));
        }

        // Check for dot-separated qualifier (e.g., "1.0.Final", "1.0.RC1")
        // Look for first non-numeric segment after initial numeric parts
        let parts: Vec<&str> = s.split('.').collect();
        for (i, part) in parts.iter().enumerate() {
            if !part.chars().all(|c| c.is_ascii_digit()) {
                let numeric = parts[..i].join(".");
                let qualifier = parts[i..].join(".");
                if numeric.is_empty() {
                    return (s, None);
                }
                return (
                    // Return a slice of the original string for the numeric part
                    &s[..numeric.len()],
                    Some(qualifier),
                );
            }
        }

        (s, None)
    }

    /// Parses numeric components from a version string.
    fn parse_components(s: &str) -> Vec<u64> {
        s.split('.')
            .filter_map(|part| part.parse::<u64>().ok())
            .collect()
    }

    /// Gets the qualifier precedence for comparison.
    /// Lower number = older/less stable.
    #[allow(clippy::option_if_let_else)] // match is clearer for this complex logic
    fn qualifier_precedence(qualifier: Option<&str>) -> i32 {
        match qualifier {
            Some(q) => {
                let q_lower = q.to_lowercase();
                if q_lower.contains("snapshot") {
                    -10
                } else if q_lower.starts_with("alpha") || q_lower.starts_with('a') {
                    -5
                } else if q_lower.starts_with("beta") || q_lower.starts_with('b') {
                    -4
                } else if q_lower.starts_with("milestone") || q_lower.starts_with('m') {
                    -3
                } else if q_lower.starts_with("rc") || q_lower.starts_with("cr") {
                    -2
                } else if q_lower == "final"
                    || q_lower == "ga"
                    || q_lower == "release"
                    || q_lower == "rel"
                {
                    0
                } else {
                    // Unknown qualifiers are treated as less than release
                    -1
                }
            }
            None => 0, // No qualifier = release
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Version {}

impl Hash for Version {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the normalized form to ensure equal versions have equal hashes
        self.components.hash(state);
        self.qualifier
            .as_ref()
            .map(|q| q.to_lowercase())
            .hash(state);
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare numeric components
        let max_len = self.components.len().max(other.components.len());
        for i in 0..max_len {
            let a = self.components.get(i).unwrap_or(&0);
            let b = other.components.get(i).unwrap_or(&0);
            match a.cmp(b) {
                Ordering::Equal => {}
                other => return other,
            }
        }

        // Components are equal, compare qualifiers
        let a_prec = Self::qualifier_precedence(self.qualifier.as_deref());
        let b_prec = Self::qualifier_precedence(other.qualifier.as_deref());
        a_prec.cmp(&b_prec)
    }
}

/// A version constraint specifying which versions are acceptable.
///
/// # Examples
///
/// ```
/// use jvm_artifact::{Version, VersionConstraint};
///
/// // Exact version
/// let exact = VersionConstraint::parse("1.2.3").unwrap();
///
/// // Version range
/// let range = VersionConstraint::parse("[1.0,2.0)").unwrap();
///
/// // Latest
/// let latest = VersionConstraint::parse("LATEST").unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionConstraint {
    /// An exact version match.
    Exact(Version),
    /// A version interval (range).
    Interval(VersionInterval),
    /// Latest version of a particular kind.
    Latest(LatestKind),
    /// A union of multiple constraints (any must match).
    Union(Vec<VersionConstraint>),
}

impl VersionConstraint {
    /// Parses a version constraint string.
    ///
    /// Supports:
    /// - Exact versions: `1.2.3`
    /// - Ranges: `[1.0,2.0)`, `(,1.0]`, `[1.0,)`
    /// - Latest: `LATEST`, `RELEASE`, `latest.integration`, `latest.release`
    /// - Multiple ranges: `[1.0,1.5],[2.0,2.5]`
    ///
    /// # Errors
    ///
    /// Returns an error if the constraint cannot be parsed.
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();

        if s.is_empty() {
            return Err(Error::InvalidConstraint(
                "constraint cannot be empty".to_string(),
            ));
        }

        // Check for latest
        match s.to_lowercase().as_str() {
            "latest" | "latest.integration" => return Ok(Self::Latest(LatestKind::Integration)),
            "release" | "latest.release" => return Ok(Self::Latest(LatestKind::Release)),
            _ => {}
        }

        // Check for union of ranges (comma-separated ranges)
        if s.contains("],[") || s.contains("),(") || s.contains("),[") || s.contains("],(") {
            return Self::parse_union(s);
        }

        // Check for range
        if s.starts_with('[') || s.starts_with('(') {
            return Self::parse_interval(s);
        }

        // Exact version
        let version = Version::parse(s)?;
        Ok(Self::Exact(version))
    }

    /// Parses a union of version constraints.
    fn parse_union(s: &str) -> Result<Self> {
        let mut constraints = Vec::new();
        let mut depth = 0;
        let mut start = 0;

        for (i, c) in s.char_indices() {
            match c {
                '[' | '(' => depth += 1,
                ']' | ')' => {
                    depth -= 1;
                    if depth == 0 {
                        let part = &s[start..=i];
                        constraints.push(Self::parse_interval(part)?);
                        start = i + 2; // Skip the comma
                    }
                }
                _ => {}
            }
        }

        if constraints.len() == 1 {
            Ok(constraints.remove(0))
        } else {
            Ok(Self::Union(constraints))
        }
    }

    /// Parses a version interval.
    fn parse_interval(s: &str) -> Result<Self> {
        let s = s.trim();

        if s.len() < 2 {
            return Err(Error::InvalidConstraint(format!("range too short: {s}")));
        }

        let from_inclusive = s.starts_with('[');
        let to_inclusive = s.ends_with(']');

        if !s.starts_with('[') && !s.starts_with('(') {
            return Err(Error::InvalidConstraint(format!(
                "range must start with '[' or '(': {s}"
            )));
        }

        if !s.ends_with(']') && !s.ends_with(')') {
            return Err(Error::InvalidConstraint(format!(
                "range must end with ']' or ')': {s}"
            )));
        }

        let inner = &s[1..s.len() - 1];
        let parts: Vec<&str> = inner.splitn(2, ',').collect();

        if parts.len() != 2 {
            return Err(Error::InvalidConstraint(format!(
                "range must contain exactly one comma: {s}"
            )));
        }

        let from_str = parts[0].trim();
        let to_str = parts[1].trim();

        let from = if from_str.is_empty() {
            Bound::Unbounded
        } else if from_inclusive {
            Bound::Inclusive(Version::parse(from_str)?)
        } else {
            Bound::Exclusive(Version::parse(from_str)?)
        };

        let to = if to_str.is_empty() {
            Bound::Unbounded
        } else if to_inclusive {
            Bound::Inclusive(Version::parse(to_str)?)
        } else {
            Bound::Exclusive(Version::parse(to_str)?)
        };

        Ok(Self::Interval(VersionInterval { from, to }))
    }

    /// Returns true if the given version satisfies this constraint.
    #[must_use]
    pub fn matches(&self, version: &Version) -> bool {
        match self {
            Self::Exact(v) => v == version,
            Self::Interval(interval) => interval.contains(version),
            Self::Latest(_) => true, // Latest always matches (resolution determines version)
            Self::Union(constraints) => constraints.iter().any(|c| c.matches(version)),
        }
    }

    /// Returns true if this is a soft constraint that allows version negotiation.
    #[must_use]
    pub const fn is_soft(&self) -> bool {
        matches!(self, Self::Exact(_) | Self::Latest(_))
    }
}

impl fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(v) => write!(f, "{v}"),
            Self::Interval(interval) => write!(f, "{interval}"),
            Self::Latest(kind) => write!(f, "{kind}"),
            Self::Union(constraints) => {
                for (i, c) in constraints.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{c}")?;
                }
                Ok(())
            }
        }
    }
}

/// The kind of "latest" version to resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LatestKind {
    /// Latest version including snapshots (latest.integration).
    Integration,
    /// Latest release version (latest.release).
    Release,
}

impl fmt::Display for LatestKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integration => write!(f, "latest.integration"),
            Self::Release => write!(f, "latest.release"),
        }
    }
}

/// A version interval with lower and upper bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionInterval {
    /// The lower bound.
    pub from: Bound,
    /// The upper bound.
    pub to: Bound,
}

impl VersionInterval {
    /// Creates a new version interval.
    #[must_use]
    pub const fn new(from: Bound, to: Bound) -> Self {
        Self { from, to }
    }

    /// Returns true if this interval contains the given version.
    #[must_use]
    pub fn contains(&self, version: &Version) -> bool {
        let from_ok = match &self.from {
            Bound::Unbounded => true,
            Bound::Inclusive(v) => version >= v,
            Bound::Exclusive(v) => version > v,
        };

        let to_ok = match &self.to {
            Bound::Unbounded => true,
            Bound::Inclusive(v) => version <= v,
            Bound::Exclusive(v) => version < v,
        };

        from_ok && to_ok
    }
}

impl fmt::Display for VersionInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.from {
            Bound::Unbounded | Bound::Exclusive(_) => write!(f, "(")?,
            Bound::Inclusive(_) => write!(f, "[")?,
        }

        match &self.from {
            Bound::Unbounded => {}
            Bound::Inclusive(v) | Bound::Exclusive(v) => write!(f, "{v}")?,
        }

        write!(f, ",")?;

        match &self.to {
            Bound::Unbounded => {}
            Bound::Inclusive(v) | Bound::Exclusive(v) => write!(f, "{v}")?,
        }

        match &self.to {
            Bound::Unbounded | Bound::Exclusive(_) => write!(f, ")")?,
            Bound::Inclusive(_) => write!(f, "]")?,
        }

        Ok(())
    }
}

/// A bound of a version interval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Bound {
    /// The version is included in the interval.
    Inclusive(Version),
    /// The version is excluded from the interval.
    Exclusive(Version),
    /// No bound (unbounded).
    Unbounded,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parse() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.components(), &[1, 2, 3]);
        assert_eq!(v.qualifier(), None);
    }

    #[test]
    fn test_version_parse_with_qualifier() {
        let v = Version::parse("1.0-SNAPSHOT").unwrap();
        assert_eq!(v.components(), &[1, 0]);
        assert_eq!(v.qualifier(), Some("SNAPSHOT"));
        assert!(v.is_snapshot());
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::parse("1.0").unwrap();
        let v2 = Version::parse("1.1").unwrap();
        let v3 = Version::parse("1.0-SNAPSHOT").unwrap();
        let v4 = Version::parse("1.0-alpha").unwrap();
        let v5 = Version::parse("1.0-beta").unwrap();
        let v6 = Version::parse("1.0-RC1").unwrap();

        assert!(v3 < v4); // SNAPSHOT < alpha
        assert!(v4 < v5); // alpha < beta
        assert!(v5 < v6); // beta < RC
        assert!(v6 < v1); // RC < release
        assert!(v1 < v2); // 1.0 < 1.1
    }

    #[test]
    fn test_version_constraint_exact() {
        let c = VersionConstraint::parse("1.2.3").unwrap();
        assert!(matches!(c, VersionConstraint::Exact(_)));

        let v = Version::parse("1.2.3").unwrap();
        assert!(c.matches(&v));

        let v2 = Version::parse("1.2.4").unwrap();
        assert!(!c.matches(&v2));
    }

    #[test]
    fn test_version_constraint_range() {
        let c = VersionConstraint::parse("[1.0,2.0)").unwrap();

        assert!(c.matches(&Version::parse("1.0").unwrap()));
        assert!(c.matches(&Version::parse("1.5").unwrap()));
        assert!(!c.matches(&Version::parse("0.9").unwrap()));
        assert!(!c.matches(&Version::parse("2.0").unwrap()));
    }

    #[test]
    fn test_version_constraint_open_range() {
        let c = VersionConstraint::parse("[1.0,)").unwrap();

        assert!(c.matches(&Version::parse("1.0").unwrap()));
        assert!(c.matches(&Version::parse("99.0").unwrap()));
        assert!(!c.matches(&Version::parse("0.9").unwrap()));
    }

    #[test]
    fn test_version_constraint_latest() {
        let c1 = VersionConstraint::parse("LATEST").unwrap();
        assert!(matches!(
            c1,
            VersionConstraint::Latest(LatestKind::Integration)
        ));

        let c2 = VersionConstraint::parse("latest.release").unwrap();
        assert!(matches!(c2, VersionConstraint::Latest(LatestKind::Release)));
    }
}
