//! Repository registry with well-known repository presets.
//!
//! This module provides a catalog of well-known package repositories across
//! different ecosystems (Maven, npm, `PyPI`, etc.). Use presets to quickly
//! configure common repositories without remembering URLs.
//!
//! # Example
//!
//! ```
//! use gather::{RepositoryRegistry, Ecosystem};
//!
//! // Get a preset by name
//! let jenkins = RepositoryRegistry::get("jenkins").unwrap();
//! println!("Jenkins URL: {}", jenkins.url);
//!
//! // List all Maven presets
//! for preset in RepositoryRegistry::by_ecosystem(Ecosystem::Maven) {
//!     println!("{}: {}", preset.id, preset.description);
//! }
//! ```

use std::fmt::Write;

use crate::{Ecosystem, MavenRepository};

/// A well-known repository preset.
///
/// Presets provide pre-configured repository settings for common package
/// registries, making it easy to add repositories without looking up URLs.
#[derive(Debug, Clone, Copy)]
pub struct RepositoryPreset {
    /// Short identifier (e.g., "jenkins", "gradle-plugins").
    pub id: &'static str,
    /// Human-readable name.
    pub name: &'static str,
    /// Repository base URL.
    pub url: &'static str,
    /// The ecosystem this repository serves.
    pub ecosystem: Ecosystem,
    /// Brief description of what this repository contains.
    pub description: &'static str,
}

impl RepositoryPreset {
    /// Creates a [`MavenRepository`] from this preset.
    #[must_use]
    pub fn to_repository(&self) -> MavenRepository {
        MavenRepository::new(self.id, self.name, self.url).with_ecosystem(self.ecosystem)
    }
}

/// Static catalog of well-known repositories.
///
/// Add new repositories here as they become commonly needed.
const PRESETS: &[RepositoryPreset] = &[
    // ==========================================================================
    // Maven / JVM Ecosystem
    // ==========================================================================
    RepositoryPreset {
        id: "central",
        name: "Maven Central",
        url: "https://repo1.maven.org/maven2/",
        ecosystem: Ecosystem::Maven,
        description: "The default Maven repository for open-source Java artifacts",
    },
    RepositoryPreset {
        id: "google",
        name: "Google Maven",
        url: "https://maven.google.com/",
        ecosystem: Ecosystem::Maven,
        description: "Google's Maven repository for Android and Google libraries",
    },
    RepositoryPreset {
        id: "gradle-plugins",
        name: "Gradle Plugin Portal",
        url: "https://plugins.gradle.org/m2/",
        ecosystem: Ecosystem::Maven,
        description: "Gradle plugins published via the Plugin Portal",
    },
    RepositoryPreset {
        id: "jcenter",
        name: "JCenter",
        url: "https://jcenter.bintray.com/",
        ecosystem: Ecosystem::Maven,
        description: "JCenter (read-only, deprecated but still available)",
    },
    RepositoryPreset {
        id: "sonatype-snapshots",
        name: "Sonatype Snapshots",
        url: "https://oss.sonatype.org/content/repositories/snapshots/",
        ecosystem: Ecosystem::Maven,
        description: "Sonatype OSS snapshot repository for pre-release artifacts",
    },
    RepositoryPreset {
        id: "sonatype-releases",
        name: "Sonatype Releases",
        url: "https://oss.sonatype.org/content/repositories/releases/",
        ecosystem: Ecosystem::Maven,
        description: "Sonatype OSS releases repository",
    },
    RepositoryPreset {
        id: "sonatype-s01-snapshots",
        name: "Sonatype S01 Snapshots",
        url: "https://s01.oss.sonatype.org/content/repositories/snapshots/",
        ecosystem: Ecosystem::Maven,
        description: "Sonatype S01 snapshot repository (newer projects)",
    },
    RepositoryPreset {
        id: "jenkins",
        name: "Jenkins",
        url: "https://repo.jenkins-ci.org/public/",
        ecosystem: Ecosystem::Maven,
        description: "Jenkins CI plugins and libraries",
    },
    RepositoryPreset {
        id: "jitpack",
        name: "JitPack",
        url: "https://jitpack.io/",
        ecosystem: Ecosystem::Maven,
        description: "Build and publish JVM libraries directly from GitHub",
    },
    RepositoryPreset {
        id: "spring-milestones",
        name: "Spring Milestones",
        url: "https://repo.spring.io/milestone/",
        ecosystem: Ecosystem::Maven,
        description: "Spring milestone and release candidate builds",
    },
    RepositoryPreset {
        id: "spring-snapshots",
        name: "Spring Snapshots",
        url: "https://repo.spring.io/snapshot/",
        ecosystem: Ecosystem::Maven,
        description: "Spring snapshot builds",
    },
    RepositoryPreset {
        id: "atlassian",
        name: "Atlassian",
        url: "https://packages.atlassian.com/maven-public/",
        ecosystem: Ecosystem::Maven,
        description: "Atlassian public Maven repository (Jira, Confluence SDKs)",
    },
    RepositoryPreset {
        id: "redhat-ga",
        name: "Red Hat GA",
        url: "https://maven.repository.redhat.com/ga/",
        ecosystem: Ecosystem::Maven,
        description: "Red Hat General Availability repository",
    },
    RepositoryPreset {
        id: "clojars",
        name: "Clojars",
        url: "https://repo.clojars.org/",
        ecosystem: Ecosystem::Maven,
        description: "Community repository for Clojure libraries",
    },
    RepositoryPreset {
        id: "jboss",
        name: "JBoss",
        url: "https://repository.jboss.org/nexus/content/groups/public/",
        ecosystem: Ecosystem::Maven,
        description: "JBoss community repository",
    },
    RepositoryPreset {
        id: "apache-snapshots",
        name: "Apache Snapshots",
        url: "https://repository.apache.org/content/repositories/snapshots/",
        ecosystem: Ecosystem::Maven,
        description: "Apache project snapshot builds",
    },
    RepositoryPreset {
        id: "hortonworks",
        name: "Hortonworks",
        url: "https://repo.hortonworks.com/content/repositories/releases/",
        ecosystem: Ecosystem::Maven,
        description: "Hortonworks Data Platform artifacts",
    },
    RepositoryPreset {
        id: "confluent",
        name: "Confluent",
        url: "https://packages.confluent.io/maven/",
        ecosystem: Ecosystem::Maven,
        description: "Confluent platform (Kafka ecosystem) artifacts",
    },
    RepositoryPreset {
        id: "kotlin-dev",
        name: "Kotlin Dev",
        url: "https://maven.pkg.jetbrains.space/kotlin/p/kotlin/dev/",
        ecosystem: Ecosystem::Maven,
        description: "Kotlin development builds from JetBrains",
    },
    RepositoryPreset {
        id: "compose-dev",
        name: "Compose Dev",
        url: "https://maven.pkg.jetbrains.space/public/p/compose/dev/",
        ecosystem: Ecosystem::Maven,
        description: "Compose Multiplatform development builds",
    },
    // ==========================================================================
    // npm Ecosystem (for future use)
    // ==========================================================================
    RepositoryPreset {
        id: "npmjs",
        name: "npm Registry",
        url: "https://registry.npmjs.org/",
        ecosystem: Ecosystem::Npm,
        description: "The default npm registry for JavaScript packages",
    },
    RepositoryPreset {
        id: "yarn",
        name: "Yarn Registry",
        url: "https://registry.yarnpkg.com/",
        ecosystem: Ecosystem::Npm,
        description: "Yarn's npm registry mirror",
    },
    // ==========================================================================
    // PyPI Ecosystem (for future use)
    // ==========================================================================
    RepositoryPreset {
        id: "pypi",
        name: "PyPI",
        url: "https://pypi.org/simple/",
        ecosystem: Ecosystem::Pypi,
        description: "The Python Package Index",
    },
    RepositoryPreset {
        id: "testpypi",
        name: "TestPyPI",
        url: "https://test.pypi.org/simple/",
        ecosystem: Ecosystem::Pypi,
        description: "Test Python Package Index for pre-release testing",
    },
    // ==========================================================================
    // NuGet Ecosystem (for future use)
    // ==========================================================================
    RepositoryPreset {
        id: "nuget",
        name: "NuGet Gallery",
        url: "https://api.nuget.org/v3/index.json",
        ecosystem: Ecosystem::Nuget,
        description: "The default NuGet package registry for .NET",
    },
];

/// Registry of well-known repository presets.
///
/// Provides static methods to look up and list repository presets
/// across different package ecosystems.
pub struct RepositoryRegistry;

impl RepositoryRegistry {
    /// Returns all available presets.
    #[must_use]
    pub const fn all() -> &'static [RepositoryPreset] {
        PRESETS
    }

    /// Gets a preset by ID.
    ///
    /// Returns `None` if no preset with the given ID exists.
    #[must_use]
    pub fn get(id: &str) -> Option<&'static RepositoryPreset> {
        PRESETS.iter().find(|p| p.id == id)
    }

    /// Returns presets filtered by ecosystem.
    pub fn by_ecosystem(ecosystem: Ecosystem) -> impl Iterator<Item = &'static RepositoryPreset> {
        PRESETS.iter().filter(move |p| p.ecosystem == ecosystem)
    }

    /// Returns preset IDs filtered by ecosystem.
    pub fn ids_by_ecosystem(ecosystem: Ecosystem) -> impl Iterator<Item = &'static str> {
        Self::by_ecosystem(ecosystem).map(|p| p.id)
    }

    /// Returns all preset IDs.
    pub fn ids() -> impl Iterator<Item = &'static str> {
        PRESETS.iter().map(|p| p.id)
    }

    /// Returns the default presets for Maven/JVM development.
    ///
    /// Includes Maven Central and Google Maven.
    #[must_use]
    pub fn maven_defaults() -> Vec<&'static RepositoryPreset> {
        vec![
            Self::get("central").expect("central preset exists"),
            Self::get("google").expect("google preset exists"),
        ]
    }

    /// Checks if a preset ID exists.
    #[must_use]
    pub fn exists(id: &str) -> bool {
        Self::get(id).is_some()
    }

    /// Formats a list of available presets for display.
    #[must_use]
    pub fn format_list(ecosystem: Option<Ecosystem>) -> String {
        let mut output = String::new();

        let presets: Vec<_> = ecosystem.map_or_else(
            || PRESETS.iter().collect(),
            |eco| Self::by_ecosystem(eco).collect(),
        );

        // Group by ecosystem
        let mut current_ecosystem: Option<Ecosystem> = None;

        for preset in presets {
            if current_ecosystem != Some(preset.ecosystem) {
                if current_ecosystem.is_some() {
                    output.push('\n');
                }
                let _ = writeln!(output, "{}:", preset.ecosystem.as_str().to_uppercase());
                current_ecosystem = Some(preset.ecosystem);
            }

            let _ = writeln!(output, "  {:20} {}", preset.id, preset.description);
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_preset() {
        let central = RepositoryRegistry::get("central");
        assert!(central.is_some());
        let central = central.unwrap();
        assert_eq!(central.id, "central");
        assert_eq!(central.name, "Maven Central");
        assert!(central.url.contains("repo1.maven.org"));
    }

    #[test]
    fn test_get_nonexistent() {
        assert!(RepositoryRegistry::get("nonexistent").is_none());
    }

    #[test]
    fn test_by_ecosystem() {
        let maven_presets: Vec<_> = RepositoryRegistry::by_ecosystem(Ecosystem::Maven).collect();
        assert!(!maven_presets.is_empty());
        assert!(
            maven_presets
                .iter()
                .all(|p| p.ecosystem == Ecosystem::Maven)
        );
    }

    #[test]
    fn test_to_repository() {
        let preset = RepositoryRegistry::get("jenkins").unwrap();
        let repo = preset.to_repository();
        assert_eq!(repo.id, "jenkins");
        assert_eq!(repo.name, "Jenkins");
        assert_eq!(repo.ecosystem, Ecosystem::Maven);
    }

    #[test]
    fn test_maven_defaults() {
        let defaults = RepositoryRegistry::maven_defaults();
        assert_eq!(defaults.len(), 2);
        assert!(defaults.iter().any(|p| p.id == "central"));
        assert!(defaults.iter().any(|p| p.id == "google"));
    }

    #[test]
    fn test_exists() {
        assert!(RepositoryRegistry::exists("central"));
        assert!(RepositoryRegistry::exists("jenkins"));
        assert!(!RepositoryRegistry::exists("fake-repo"));
    }

    #[test]
    fn test_ids() {
        let ids: Vec<_> = RepositoryRegistry::ids().collect();
        assert!(ids.contains(&"central"));
        assert!(ids.contains(&"gradle-plugins"));
        assert!(ids.contains(&"jenkins"));
    }

    #[test]
    fn test_format_list() {
        let output = RepositoryRegistry::format_list(Some(Ecosystem::Maven));
        assert!(output.contains("MAVEN:"));
        assert!(output.contains("central"));
        assert!(output.contains("jenkins"));
        assert!(!output.contains("NPM:"));
    }

    #[test]
    fn test_format_list_all() {
        let output = RepositoryRegistry::format_list(None);
        assert!(output.contains("MAVEN:"));
        assert!(output.contains("NPM:"));
        assert!(output.contains("PYPI:"));
    }
}
