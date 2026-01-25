//! Maven repository definitions.

use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{Error, Result};

/// A Maven repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    /// Repository identifier.
    pub id: String,
    /// Repository name.
    pub name: String,
    /// Base URL for the repository.
    pub url: Url,
    /// Whether this repository requires authentication.
    #[serde(default)]
    pub authenticated: bool,
}

impl Repository {
    /// Create a new repository with the given ID and URL.
    pub fn new(id: impl Into<String>, name: impl Into<String>, url: &str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            url: Url::parse(url).expect("invalid repository URL"),
            authenticated: false,
        }
    }

    /// Maven Central repository.
    pub fn maven_central() -> Self {
        Self::new(
            "central",
            "Maven Central",
            "https://repo1.maven.org/maven2/",
        )
    }

    /// Google's Maven repository (for Android dependencies).
    pub fn google() -> Self {
        Self::new("google", "Google Maven", "https://maven.google.com/")
    }

    /// Gradle Plugin Portal.
    pub fn gradle_plugin_portal() -> Self {
        Self::new(
            "gradle-plugins",
            "Gradle Plugin Portal",
            "https://plugins.gradle.org/m2/",
        )
    }

    /// `JCenter` (read-only, deprecated but still available).
    pub fn jcenter() -> Self {
        Self::new("jcenter", "JCenter", "https://jcenter.bintray.com/")
    }

    /// Sonatype OSS Snapshots.
    pub fn sonatype_snapshots() -> Self {
        Self::new(
            "sonatype-snapshots",
            "Sonatype Snapshots",
            "https://oss.sonatype.org/content/repositories/snapshots/",
        )
    }

    /// Sonatype OSS Releases.
    pub fn sonatype_releases() -> Self {
        Self::new(
            "sonatype-releases",
            "Sonatype Releases",
            "https://oss.sonatype.org/content/repositories/releases/",
        )
    }

    /// Get the URL for an artifact in this repository.
    ///
    /// Returns an error if the path cannot be joined to the base URL.
    pub fn artifact_url(&self, path: &str) -> Result<Url> {
        self.url.join(path).map_err(Error::InvalidUrl)
    }

    /// Create a custom repository.
    ///
    /// # Errors
    ///
    /// Returns an error if the URL is invalid.
    pub fn try_custom(id: impl Into<String>, name: impl Into<String>, url: &str) -> Result<Self> {
        Ok(Self {
            id: id.into(),
            name: name.into(),
            url: Url::parse(url)?,
            authenticated: false,
        })
    }

    /// Create a custom repository (panics on invalid URL).
    ///
    /// # Panics
    ///
    /// Panics if the URL is invalid. Use `try_custom` for fallible construction.
    pub fn custom(id: impl Into<String>, name: impl Into<String>, url: &str) -> Self {
        Self::try_custom(id, name, url).expect("invalid repository URL")
    }
}

impl Default for Repository {
    fn default() -> Self {
        Self::maven_central()
    }
}

/// A list of repositories to search for artifacts.
#[derive(Debug, Clone, Default)]
pub struct RepositoryList {
    repositories: Vec<Repository>,
}

impl RepositoryList {
    /// Create an empty repository list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a repository list with default repositories (Maven Central + Google).
    pub fn with_defaults() -> Self {
        Self {
            repositories: vec![Repository::maven_central(), Repository::google()],
        }
    }

    /// Add a repository to the list.
    pub fn add(&mut self, repo: Repository) {
        self.repositories.push(repo);
    }

    /// Add a repository and return self for chaining.
    pub fn with(mut self, repo: Repository) -> Self {
        self.add(repo);
        self
    }

    /// Get an iterator over the repositories.
    pub fn iter(&self) -> impl Iterator<Item = &Repository> {
        self.repositories.iter()
    }

    /// Check if the list is empty.
    pub const fn is_empty(&self) -> bool {
        self.repositories.is_empty()
    }

    /// Get the number of repositories.
    pub const fn len(&self) -> usize {
        self.repositories.len()
    }
}

impl FromIterator<Repository> for RepositoryList {
    fn from_iter<I: IntoIterator<Item = Repository>>(iter: I) -> Self {
        Self {
            repositories: iter.into_iter().collect(),
        }
    }
}
