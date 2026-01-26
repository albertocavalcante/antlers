//! Maven repository definitions.
//!
//! This module provides types for representing Maven repositories and collections
//! of repositories to search for artifacts.

use serde::{Deserialize, Serialize};
use url::Url;

use crate::auth::Credentials;
use crate::error::{Error, Result};

/// The artifact ecosystem that a repository serves.
///
/// While antlers primarily focuses on Maven/JVM artifacts, repositories may serve
/// different ecosystems. This enum allows explicit declaration of what type of
/// artifacts a repository provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ecosystem {
    /// Maven/JVM artifacts (JAR, POM, AAR, etc.)
    #[default]
    Maven,
    /// npm packages (for polyglot projects)
    Npm,
    /// Python packages (for polyglot projects)
    Pypi,
    /// `NuGet` packages (for polyglot projects)
    Nuget,
}

impl Ecosystem {
    /// Returns the ecosystem name as a string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Maven => "maven",
            Self::Npm => "npm",
            Self::Pypi => "pypi",
            Self::Nuget => "nuget",
        }
    }
}

impl std::fmt::Display for Ecosystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A Maven repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MavenRepository {
    /// Repository identifier.
    pub id: String,
    /// Repository name.
    pub name: String,
    /// Base URL for the repository.
    pub url: Url,
    /// The ecosystem this repository serves.
    #[serde(default, skip_serializing_if = "is_default_ecosystem")]
    pub ecosystem: Ecosystem,
    /// Credentials for authentication (if required).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credentials: Option<Credentials>,
}

// Required signature for serde's skip_serializing_if
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_default_ecosystem(e: &Ecosystem) -> bool {
    *e == Ecosystem::Maven
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
            ecosystem: Ecosystem::default(),
            credentials: None,
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
            ecosystem: Ecosystem::default(),
            credentials: None,
        })
    }

    /// Sets the ecosystem for this repository.
    #[must_use]
    pub const fn with_ecosystem(mut self, ecosystem: Ecosystem) -> Self {
        self.ecosystem = ecosystem;
        self
    }

    /// Adds HTTP Basic authentication credentials.
    ///
    /// # Example
    ///
    /// ```
    /// use gather::MavenRepository;
    ///
    /// let repo = MavenRepository::new("artifactory", "My Artifactory", "https://repo.example.com/maven")
    ///     .with_basic_auth("user", "token");
    /// ```
    #[must_use]
    pub fn with_basic_auth(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.credentials = Some(Credentials::basic(username, password));
        self
    }

    /// Adds Bearer token authentication.
    ///
    /// Common for GitHub Packages, GitLab, and other OAuth2-based registries.
    ///
    /// # Example
    ///
    /// ```
    /// use gather::MavenRepository;
    ///
    /// let repo = MavenRepository::new("github", "GitHub Packages", "https://maven.pkg.github.com/owner/repo")
    ///     .with_bearer_token("ghp_xxxxxxxxxxxx");
    /// ```
    #[must_use]
    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.credentials = Some(Credentials::bearer(token));
        self
    }

    /// Uses credentials from `~/.netrc` file.
    ///
    /// The netrc file is parsed once and cached. Credentials are looked up
    /// by the repository's hostname.
    ///
    /// # Example
    ///
    /// ```
    /// use gather::MavenRepository;
    ///
    /// let repo = MavenRepository::new("private", "Private Repo", "https://maven.example.com/releases")
    ///     .with_netrc();
    /// ```
    #[must_use]
    pub fn with_netrc(mut self) -> Self {
        self.credentials = Some(Credentials::netrc());
        self
    }

    /// Returns true if this repository has credentials configured.
    #[must_use]
    pub const fn has_credentials(&self) -> bool {
        self.credentials.is_some()
    }

    /// Returns the host portion of the repository URL.
    #[must_use]
    pub fn host(&self) -> Option<&str> {
        self.url.host_str()
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

    #[test]
    fn test_with_basic_auth() {
        let repo = MavenRepository::new("test", "Test", "https://example.com/maven")
            .with_basic_auth("user", "pass");

        assert!(repo.has_credentials());
        assert!(repo.credentials.is_some());

        if let Some(Credentials::Basic { username, password }) = &repo.credentials {
            assert_eq!(username, "user");
            assert_eq!(password, "pass");
        } else {
            panic!("Expected Basic credentials");
        }
    }

    #[test]
    fn test_with_bearer_token() {
        let repo = MavenRepository::new(
            "github",
            "GitHub",
            "https://maven.pkg.github.com/owner/repo",
        )
        .with_bearer_token("ghp_token123");

        assert!(repo.has_credentials());

        if let Some(Credentials::Bearer { token }) = &repo.credentials {
            assert_eq!(token, "ghp_token123");
        } else {
            panic!("Expected Bearer credentials");
        }
    }

    #[test]
    fn test_with_netrc() {
        let repo =
            MavenRepository::new("private", "Private", "https://maven.example.com").with_netrc();

        assert!(repo.has_credentials());
        assert!(matches!(repo.credentials, Some(Credentials::Netrc)));
    }

    #[test]
    fn test_has_credentials_false() {
        let repo = MavenRepository::maven_central();
        assert!(!repo.has_credentials());
        assert!(repo.credentials.is_none());
    }

    #[test]
    fn test_host() {
        let repo = MavenRepository::new("test", "Test", "https://maven.example.com:8443/repo");
        assert_eq!(repo.host(), Some("maven.example.com"));
    }

    #[test]
    fn test_host_maven_central() {
        let repo = MavenRepository::maven_central();
        assert_eq!(repo.host(), Some("repo1.maven.org"));
    }

    #[test]
    fn test_try_new_invalid_url() {
        let result = MavenRepository::try_new("test", "Test", "not a url");
        assert!(result.is_err());
    }

    #[test]
    fn test_credentials_override() {
        // Verify that setting credentials twice replaces the first
        let repo = MavenRepository::new("test", "Test", "https://example.com")
            .with_basic_auth("user1", "pass1")
            .with_bearer_token("token123");

        // Should now have bearer, not basic
        assert!(matches!(repo.credentials, Some(Credentials::Bearer { .. })));
    }

    #[test]
    fn test_ecosystem_default() {
        assert_eq!(Ecosystem::default(), Ecosystem::Maven);
    }

    #[test]
    fn test_ecosystem_as_str() {
        assert_eq!(Ecosystem::Maven.as_str(), "maven");
        assert_eq!(Ecosystem::Npm.as_str(), "npm");
        assert_eq!(Ecosystem::Pypi.as_str(), "pypi");
        assert_eq!(Ecosystem::Nuget.as_str(), "nuget");
    }

    #[test]
    fn test_ecosystem_display() {
        assert_eq!(format!("{}", Ecosystem::Maven), "maven");
        assert_eq!(format!("{}", Ecosystem::Npm), "npm");
    }

    #[test]
    fn test_ecosystem_serde() {
        assert_eq!(
            serde_json::to_string(&Ecosystem::Maven).unwrap(),
            r#""maven""#
        );
        assert_eq!(
            serde_json::from_str::<Ecosystem>(r#""npm""#).unwrap(),
            Ecosystem::Npm
        );
    }

    #[test]
    fn test_with_ecosystem() {
        let repo = MavenRepository::new("npmjs", "npmjs", "https://registry.npmjs.org")
            .with_ecosystem(Ecosystem::Npm);
        assert_eq!(repo.ecosystem, Ecosystem::Npm);
    }

    #[test]
    fn test_repository_default_ecosystem() {
        let repo = MavenRepository::maven_central();
        assert_eq!(repo.ecosystem, Ecosystem::Maven);
    }
}
