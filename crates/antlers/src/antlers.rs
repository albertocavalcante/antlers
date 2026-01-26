//! High-level builder API for JVM dependency resolution.
//!
//! This module provides [`Antlers`], the main entry point for resolving
//! JVM dependencies using a fluent builder pattern.

use std::path::Path;

use dendro::{HighestWins, NearestWins, Resolution, Resolver, ResolverConfig};
use gather::{Fetcher, MavenRepository, RepositoryList};
use gav::Artifact;

use crate::Result;
use crate::fetcher::PomFetcher;

/// High-level API for JVM dependency resolution.
///
/// `Antlers` provides a fluent builder API for configuring and executing
/// dependency resolution. It combines all the lower-level crates into
/// an ergonomic interface.
///
/// # Example
///
/// ```no_run
/// use antlers::{Antlers, Artifact};
///
/// # async fn example() -> antlers::Result<()> {
/// let resolution = Antlers::new()
///     .with_maven_central()
///     .resolve(&Artifact::parse("org.slf4j:slf4j-api:2.0.9")?)
///     .await?;
///
/// for artifact in resolution.artifacts() {
///     println!("{}", artifact.artifact);
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Configuration
///
/// Use the builder methods to configure the resolver:
///
/// ```no_run
/// use antlers::{Antlers, ResolverConfig};
///
/// let antler = Antlers::new()
///     .with_maven_central()
///     .with_google()
///     .transitive(true)
///     .include_optional(false);
/// ```
pub struct Antlers {
    repositories: RepositoryList,
    config: ResolverConfig,
    use_highest_wins: bool,
}

impl Default for Antlers {
    fn default() -> Self {
        Self::new()
    }
}

impl Antlers {
    /// Creates a new `Antlers` instance with default settings.
    ///
    /// By default, no repositories are configured. Use [`with_maven_central`](Self::with_maven_central),
    /// [`with_google`](Self::with_google), or [`with_repository`](Self::with_repository) to add repositories.
    ///
    /// By default, uses "highest wins" conflict resolution (like Coursier and Gradle).
    #[must_use]
    pub fn new() -> Self {
        Self {
            repositories: RepositoryList::new(),
            config: ResolverConfig::default(),
            use_highest_wins: true, // Match Coursier/Gradle behavior
        }
    }

    /// Creates a new `Antlers` instance with default repositories (Maven Central + Google).
    ///
    /// This is a convenience method equivalent to:
    /// ```no_run
    /// # use antlers::Antlers;
    /// Antlers::new()
    ///     .with_maven_central()
    ///     .with_google();
    /// ```
    #[must_use]
    pub fn with_defaults() -> Self {
        Self {
            repositories: RepositoryList::with_defaults(),
            config: ResolverConfig::default(),
            use_highest_wins: true, // Match Coursier/Gradle behavior
        }
    }

    /// Uses "highest wins" conflict resolution strategy.
    ///
    /// When version conflicts occur, the highest version is selected.
    /// This matches Coursier and Gradle behavior.
    #[must_use]
    pub const fn highest_wins(mut self) -> Self {
        self.use_highest_wins = true;
        self
    }

    /// Uses "nearest wins" conflict resolution strategy.
    ///
    /// When version conflicts occur, the version closest to the root is selected.
    /// This matches Maven's default behavior.
    #[must_use]
    pub const fn nearest_wins(mut self) -> Self {
        self.use_highest_wins = false;
        self
    }

    /// Adds Maven Central repository.
    ///
    /// Maven Central is the primary repository for most Java/JVM artifacts.
    #[must_use]
    pub fn with_maven_central(mut self) -> Self {
        self.repositories.add(MavenRepository::maven_central());
        self
    }

    /// Adds Google's Maven repository.
    ///
    /// Google's repository contains Android artifacts and related libraries.
    #[must_use]
    pub fn with_google(mut self) -> Self {
        self.repositories.add(MavenRepository::google());
        self
    }

    /// Adds a custom repository.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use antlers::{Antlers, MavenRepository};
    ///
    /// let antler = Antlers::new()
    ///     .with_repository(MavenRepository::new("my-repo", "My Repository", "https://repo.example.com/maven2"));
    /// ```
    #[must_use]
    pub fn with_repository(mut self, repo: MavenRepository) -> Self {
        self.repositories.add(repo);
        self
    }

    /// Sets the resolver configuration.
    ///
    /// This replaces the entire configuration. For individual settings,
    /// use the specific builder methods like [`transitive`](Self::transitive).
    #[must_use]
    pub fn with_config(mut self, config: ResolverConfig) -> Self {
        self.config = config;
        self
    }

    /// Enables or disables transitive dependency resolution.
    ///
    /// When enabled (default), the resolver will resolve not just the requested
    /// artifact but also all its dependencies, and their dependencies, etc.
    ///
    /// When disabled, only the directly requested artifact is resolved.
    #[must_use]
    pub const fn transitive(mut self, enabled: bool) -> Self {
        self.config.transitive = enabled;
        self
    }

    /// Enables or disables inclusion of optional dependencies.
    ///
    /// By default, optional dependencies are not included in the resolution.
    #[must_use]
    pub const fn include_optional(mut self, enabled: bool) -> Self {
        self.config.include_optional = enabled;
        self
    }

    /// Resolves an artifact and its dependencies.
    ///
    /// This method fetches the POM for the requested artifact, parses its
    /// dependencies, and optionally resolves transitive dependencies.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The artifact cannot be found in any configured repository
    /// - The POM cannot be parsed
    /// - A network error occurs
    ///
    /// # Example
    ///
    /// ```no_run
    /// use antlers::{Antlers, Artifact};
    ///
    /// # async fn example() -> antlers::Result<()> {
    /// let resolution = Antlers::with_defaults()
    ///     .resolve(&Artifact::parse("com.google.guava:guava:32.1.3-jre")?)
    ///     .await?;
    ///
    /// println!("Resolved {} artifacts", resolution.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn resolve(&self, artifact: &Artifact) -> Result<Resolution> {
        // Create the POM fetcher
        let fetcher = PomFetcher::new(self.repositories.clone());

        // Use the proper dendro Resolver for transitive resolution
        let resolution = if self.use_highest_wins {
            let resolver = Resolver::new(fetcher)
                .with_config(self.config.clone())
                .with_strategy(HighestWins);
            resolver.resolve(artifact).await?
        } else {
            let resolver = Resolver::new(fetcher)
                .with_config(self.config.clone())
                .with_strategy(NearestWins);
            resolver.resolve(artifact).await?
        };

        Ok(resolution)
    }

    /// Fetches an artifact to a local path with checksum verification.
    ///
    /// Downloads the artifact JAR/file to the specified destination path,
    /// verifying the checksum if available.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The artifact cannot be found
    /// - Checksum verification fails
    /// - The file cannot be written
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use antlers::{Antlers, Artifact};
    ///
    /// # async fn example() -> antlers::Result<()> {
    /// Antlers::with_defaults()
    ///     .fetch(
    ///         &Artifact::parse("org.slf4j:slf4j-api:2.0.9")?,
    ///         Path::new("./libs/slf4j-api-2.0.9.jar"),
    ///     )
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch(&self, artifact: &Artifact, dest: &Path) -> Result<()> {
        let fetcher = Fetcher::new(self.repositories.clone());
        fetcher.download_verified(artifact, dest).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_antler_new() {
        let antler = Antlers::new();
        assert!(antler.repositories.is_empty());
        assert!(antler.config.transitive);
        assert!(antler.use_highest_wins);
    }

    #[test]
    fn test_antler_with_defaults() {
        let antler = Antlers::with_defaults();
        assert_eq!(antler.repositories.len(), 2);
        assert!(antler.use_highest_wins);
    }

    #[test]
    fn test_antler_builder() {
        let antler = Antlers::new()
            .with_maven_central()
            .with_google()
            .transitive(false)
            .include_optional(true)
            .nearest_wins();

        assert_eq!(antler.repositories.len(), 2);
        assert!(!antler.config.transitive);
        assert!(antler.config.include_optional);
        assert!(!antler.use_highest_wins);
    }

    #[test]
    fn test_antler_default_trait() {
        let antler = Antlers::default();
        assert!(antler.repositories.is_empty());
    }

    #[test]
    fn test_conflict_strategy_selection() {
        let antler = Antlers::new().highest_wins();
        assert!(antler.use_highest_wins);

        let antler = Antlers::new().nearest_wins();
        assert!(!antler.use_highest_wins);
    }
}
