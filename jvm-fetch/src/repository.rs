//! Maven repository definitions.
//!
//! This module provides types for representing Maven repositories and collections
//! of repositories to search for artifacts.

use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{Error, Result};

/// A Maven repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MavenRepository {
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

impl MavenRepository {
    /// Creates a new repository with the given ID and URL.
    ///
    /// # Panics
    ///
    /// Panics if the URL is invalid. Use `try_new` for fallible construction.
    #[must_use]
    pub fn new(id: impl Into<String>, name: impl Into<String>, url: &str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            url: Url::parse(url).expect("invalid repository URL"),
            authenticated: false,
        }
    }

    /// Creates a new repository with the given ID and URL.
    ///
    /// # Errors
    ///
    /// Returns an error if the URL is invalid.
    pub fn try_new(id: impl Into<String>, name: impl Into<String>, url: &str) -> Result<Self> {
        Ok(Self {
            id: id.into(),
            name: name.into(),
            url: Url::parse(url)?,
            authenticated: false,
        })
    }

    /// Maven Central repository.
    #[must_use]
    pub fn maven_central() -> Self {
        Self::new(
            "central",
            "Maven Central",
            "https://repo1.maven.org/maven2/",
        )
    }

    /// Google's Maven repository (for Android dependencies).
    #[must_use]
    pub fn google() -> Self {
        Self::new("google", "Google Maven", "https://maven.google.com/")
    }

    /// Gradle Plugin Portal.
    #[must_use]
    pub fn gradle_plugin_portal() -> Self {
        Self::new(
            "gradle-plugins",
            "Gradle Plugin Portal",
            "https://plugins.gradle.org/m2/",
        )
    }

    /// `JCenter` (read-only, deprecated but still available).
    #[must_use]
    pub fn jcenter() -> Self {
        Self::new("jcenter", "JCenter", "https://jcenter.bintray.com/")
    }

    /// Sonatype OSS Snapshots.
    #[must_use]
    pub fn sonatype_snapshots() -> Self {
        Self::new(
            "sonatype-snapshots",
            "Sonatype Snapshots",
            "https://oss.sonatype.org/content/repositories/snapshots/",
        )
    }

    /// Sonatype OSS Releases.
    #[must_use]
    pub fn sonatype_releases() -> Self {
        Self::new(
            "sonatype-releases",
            "Sonatype Releases",
            "https://oss.sonatype.org/content/repositories/releases/",
        )
    }

    /// Creates a custom repository.
    ///
    /// # Panics
    ///
    /// Panics if the URL is invalid. Use `try_custom` for fallible construction.
    #[must_use]
    pub fn custom(id: impl Into<String>, name: impl Into<String>, url: &str) -> Self {
        Self::new(id, name, url)
    }

    /// Creates a custom repository.
    ///
    /// # Errors
    ///
    /// Returns an error if the URL is invalid.
    pub fn try_custom(id: impl Into<String>, name: impl Into<String>, url: &str) -> Result<Self> {
        Self::try_new(id, name, url)
    }

    /// Gets the URL for an artifact path in this repository.
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be joined to the base URL.
    pub fn artifact_url(&self, path: &str) -> Result<Url> {
        self.url.join(path).map_err(Error::InvalidUrl)
    }
}

impl Default for MavenRepository {
    fn default() -> Self {
        Self::maven_central()
    }
}

/// Repository trait for abstraction over different repository types.
pub trait Repository: Send + Sync {
    /// Returns the repository identifier.
    fn id(&self) -> &str;

    /// Returns the repository name.
    fn name(&self) -> &str;

    /// Gets the URL for an artifact path.
    ///
    /// # Errors
    ///
    /// Returns an error if the URL cannot be constructed.
    fn artifact_url(&self, path: &str) -> Result<Url>;
}

impl Repository for MavenRepository {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn artifact_url(&self, path: &str) -> Result<Url> {
        Self::artifact_url(self, path)
    }
}

/// A collection of repositories to search for artifacts.
#[derive(Debug, Clone, Default)]
pub struct RepositoryList {
    repositories: Vec<MavenRepository>,
}

impl RepositoryList {
    /// Creates an empty repository list.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a repository list with default repositories (Maven Central + Google).
    #[must_use]
    pub fn with_defaults() -> Self {
        Self {
            repositories: vec![MavenRepository::maven_central(), MavenRepository::google()],
        }
    }

    /// Adds a repository to the list.
    pub fn add(&mut self, repo: MavenRepository) {
        self.repositories.push(repo);
    }

    /// Adds a repository and returns self for chaining.
    #[must_use]
    pub fn with(mut self, repo: MavenRepository) -> Self {
        self.add(repo);
        self
    }

    /// Returns an iterator over the repositories.
    pub fn iter(&self) -> impl Iterator<Item = &MavenRepository> {
        self.repositories.iter()
    }

    /// Returns true if the list is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.repositories.is_empty()
    }

    /// Returns the number of repositories.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.repositories.len()
    }

    /// Returns the repository names as a comma-separated string.
    #[must_use]
    pub fn names(&self) -> String {
        self.repositories
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl FromIterator<MavenRepository> for RepositoryList {
    fn from_iter<I: IntoIterator<Item = MavenRepository>>(iter: I) -> Self {
        Self {
            repositories: iter.into_iter().collect(),
        }
    }
}

impl<'a> IntoIterator for &'a RepositoryList {
    type Item = &'a MavenRepository;
    type IntoIter = std::slice::Iter<'a, MavenRepository>;

    fn into_iter(self) -> Self::IntoIter {
        self.repositories.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maven_central() {
        let repo = MavenRepository::maven_central();
        assert_eq!(repo.id, "central");
        assert_eq!(repo.url.as_str(), "https://repo1.maven.org/maven2/");
    }

    #[test]
    fn test_artifact_url() {
        let repo = MavenRepository::maven_central();
        let url = repo
            .artifact_url("org/apache/commons/commons-lang3/3.12.0/commons-lang3-3.12.0.jar")
            .unwrap();
        assert_eq!(
            url.as_str(),
            "https://repo1.maven.org/maven2/org/apache/commons/commons-lang3/3.12.0/commons-lang3-3.12.0.jar"
        );
    }

    #[test]
    fn test_repository_list_defaults() {
        let list = RepositoryList::with_defaults();
        assert_eq!(list.len(), 2);
        assert!(!list.is_empty());
    }

    #[test]
    fn test_repository_list_with() {
        let list = RepositoryList::new()
            .with(MavenRepository::maven_central())
            .with(MavenRepository::google());
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_repository_names() {
        let list = RepositoryList::with_defaults();
        let names = list.names();
        assert!(names.contains("Maven Central"));
        assert!(names.contains("Google Maven"));
    }
}
