//! Maven profile support.
//!
//! Maven profiles allow conditional configuration that can be activated based on
//! various conditions like JDK version, OS, system properties, or file presence.

use serde::{Deserialize, Serialize};

use crate::Properties;
use crate::pom::{Dependencies, DependencyManagement};

/// A Maven profile.
///
/// Profiles allow conditional configuration of the build. They can contain
/// properties, dependencies, dependency management, and other configuration
/// that is only applied when the profile is active.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    /// The profile ID (name).
    #[serde(default)]
    pub id: String,

    /// Activation conditions for this profile.
    #[serde(default)]
    pub activation: Option<Activation>,

    /// Properties defined in this profile.
    #[serde(default)]
    pub properties: Properties,

    /// Dependencies declared in this profile.
    #[serde(default)]
    pub dependencies: Option<Dependencies>,

    /// Dependency management in this profile.
    #[serde(default)]
    pub dependency_management: Option<DependencyManagement>,
}

/// Profile activation conditions.
///
/// A profile can be activated by various conditions. If multiple conditions
/// are specified, all must match for the profile to be activated.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activation {
    /// If true, this profile is active by default.
    #[serde(default)]
    pub active_by_default: bool,

    /// JDK version pattern for activation.
    ///
    /// Examples: "1.8", "[1.8,)", "!1.6"
    #[serde(default)]
    pub jdk: Option<String>,

    /// Operating system conditions.
    #[serde(default)]
    pub os: Option<OsActivation>,

    /// System property condition.
    #[serde(default)]
    pub property: Option<PropertyActivation>,

    /// File existence condition.
    #[serde(default)]
    pub file: Option<FileActivation>,
}

/// Operating system activation conditions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OsActivation {
    /// OS name pattern.
    #[serde(default)]
    pub name: Option<String>,

    /// OS family (e.g., "windows", "unix", "mac").
    #[serde(default)]
    pub family: Option<String>,

    /// OS architecture (e.g., "x86", "amd64").
    #[serde(default)]
    pub arch: Option<String>,

    /// OS version pattern.
    #[serde(default)]
    pub version: Option<String>,
}

/// System property activation condition.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PropertyActivation {
    /// Property name to check.
    #[serde(default)]
    pub name: Option<String>,

    /// Expected property value (if omitted, just checks presence).
    #[serde(default)]
    pub value: Option<String>,
}

/// File-based activation condition.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileActivation {
    /// Path to a file that must exist.
    #[serde(default)]
    pub exists: Option<String>,

    /// Path to a file that must not exist.
    #[serde(default)]
    pub missing: Option<String>,
}

/// Wrapper for profiles list in POM.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Profiles {
    /// List of profiles.
    #[serde(default, rename = "profile")]
    pub profiles: Vec<Profile>,
}

impl Activation {
    /// Returns true if this activation is active by default with no other conditions.
    #[must_use]
    pub const fn is_default_only(&self) -> bool {
        self.active_by_default
            && self.jdk.is_none()
            && self.os.is_none()
            && self.property.is_none()
            && self.file.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_activation() {
        let activation = Activation {
            active_by_default: true,
            ..Default::default()
        };
        assert!(activation.is_default_only());
    }

    #[test]
    fn test_conditional_activation() {
        let activation = Activation {
            active_by_default: true,
            jdk: Some("1.8".to_string()),
            ..Default::default()
        };
        assert!(!activation.is_default_only());
    }
}
