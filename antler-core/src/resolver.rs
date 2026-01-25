//! Dependency resolution logic.
//!
//! This module implements Maven-style dependency resolution with support for:
//! - Transitive dependency resolution
//! - Parent POM resolution and inheritance
//! - Property substitution
//! - Dependency exclusions
//! - Version conflict resolution (nearest-wins strategy)
//! - POM caching

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, trace, warn};

use crate::artifact::Artifact;
use crate::error::{Error, Result};
use crate::pom::{Exclusion, Pom};
use crate::repository::{Repository, RepositoryList};

/// Configuration for the resolver.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct ResolverConfig {
    /// Whether to resolve transitive dependencies.
    pub transitive: bool,
    /// Whether to include optional dependencies.
    pub include_optional: bool,
    /// Scopes to include (empty means all non-test/provided).
    pub include_scopes: HashSet<String>,
    /// Whether to fetch sources JARs.
    pub fetch_sources: bool,
    /// Whether to fetch javadoc JARs.
    pub fetch_javadoc: bool,
    /// Maximum depth for parent POM resolution (prevents infinite loops).
    pub max_parent_depth: usize,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            transitive: true,
            include_optional: false,
            include_scopes: HashSet::new(),
            fetch_sources: false,
            fetch_javadoc: false,
            max_parent_depth: 10,
        }
    }
}

/// Cache for fetched POMs to avoid redundant network requests.
type PomCache = Arc<RwLock<HashMap<String, Pom>>>;

/// The dependency resolver.
pub struct Resolver {
    repositories: RepositoryList,
    config: ResolverConfig,
    client: reqwest::Client,
    pom_cache: PomCache,
}

impl Resolver {
    /// Create a new resolver with default settings.
    pub fn new() -> Self {
        Self {
            repositories: RepositoryList::with_defaults(),
            config: ResolverConfig::default(),
            client: reqwest::Client::new(),
            pom_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a repository to search.
    pub fn with_repository(mut self, repo: Repository) -> Self {
        self.repositories.add(repo);
        self
    }

    /// Set the repository list.
    pub fn with_repositories(mut self, repos: RepositoryList) -> Self {
        self.repositories = repos;
        self
    }

    /// Configure the resolver.
    pub fn with_config(mut self, config: ResolverConfig) -> Self {
        self.config = config;
        self
    }

    /// Resolve an artifact and its dependencies.
    #[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
    pub async fn resolve(&self, artifact: &Artifact) -> Result<Resolution> {
        info!("Resolving {}", artifact);

        let mut resolution = Resolution::new(artifact.clone());
        let mut visited = HashSet::new();
        // Track version selections for conflict resolution (nearest-wins)
        // Key: groupId:artifactId, Value: (version, depth)
        let mut version_selections: HashMap<String, (String, usize)> = HashMap::new();

        // Queue entries: (artifact, depth, exclusions from parent)
        let mut queue: Vec<(Artifact, usize, Vec<Exclusion>)> =
            vec![(artifact.clone(), 0, Vec::new())];

        while let Some((current, depth, exclusions)) = queue.pop() {
            let ga_key = format!("{}:{}", current.group_id, current.artifact_id);
            let coord = current.coordinate();

            // Check if this artifact is excluded
            if exclusions.iter().any(|e| e.matches(&current)) {
                debug!("Skipping excluded artifact: {}", current);
                continue;
            }

            // Check for version conflicts (nearest-wins strategy)
            if let Some((selected_version, selected_depth)) = version_selections.get(&ga_key) {
                if *selected_depth <= depth {
                    // Already have a nearer (or equal depth) version selected
                    if selected_version != &current.version {
                        debug!(
                            "Version conflict for {}: using {} (depth {}) over {} (depth {})",
                            ga_key, selected_version, selected_depth, current.version, depth
                        );
                        resolution.conflicts.push(VersionConflict {
                            artifact: ga_key.clone(),
                            versions: vec![selected_version.clone(), current.version.clone()],
                            selected: selected_version.clone(),
                        });
                    }
                    continue;
                }
                // This version is nearer, so we'll use it instead
                debug!(
                    "Replacing {} version {} (depth {}) with {} (depth {})",
                    ga_key, selected_version, selected_depth, current.version, depth
                );
            }

            // Record this version selection
            version_selections.insert(ga_key.clone(), (current.version.clone(), depth));

            if visited.contains(&coord) {
                continue;
            }
            visited.insert(coord.clone());

            debug!("Processing {} (depth {})", current, depth);

            // Fetch and resolve POM (including parent chain)
            let pom = match self.fetch_and_resolve_pom(&current, 0).await {
                Ok(pom) => pom,
                Err(e) => {
                    warn!("Failed to fetch POM for {}: {}", current, e);
                    continue;
                }
            };

            // Fetch checksums
            let checksums = self.fetch_checksums(&current).await.unwrap_or_default();

            // Add to resolution
            let resolved = ResolvedArtifact {
                artifact: current.clone(),
                sha1: checksums.sha1,
                sha256: checksums.sha256,
                repository: checksums.repository,
            };
            resolution.artifacts.push(resolved);

            // Process dependencies if transitive resolution is enabled
            if self.config.transitive {
                for dep in pom.direct_dependencies() {
                    // Skip test/provided/system scope dependencies
                    if let Some("test" | "provided" | "system") = dep.scope.as_deref() {
                        continue;
                    }

                    // Skip optional dependencies unless include_optional is enabled
                    let is_optional = dep.optional.unwrap_or(false);
                    if is_optional && !self.config.include_optional {
                        continue;
                    }

                    // Resolve the dependency version using dependency management
                    let resolved_version = pom.resolve_dependency_version(dep);

                    if let Some(version) = resolved_version {
                        // Skip if version still has unresolved properties
                        if version.contains("${") {
                            warn!(
                                "Skipping {} - unresolved version property: {}",
                                dep.key(),
                                version
                            );
                            continue;
                        }

                        let dep_artifact = Artifact::new(&dep.group_id, &dep.artifact_id, &version);
                        let dep_coord = dep_artifact.coordinate();

                        if !visited.contains(&dep_coord) {
                            // Merge exclusions: parent's exclusions + this dependency's exclusions
                            let mut merged_exclusions = exclusions.clone();
                            if let Some(ref dep_exclusions) = dep.exclusions {
                                merged_exclusions.extend(dep_exclusions.exclusions.clone());
                            }

                            trace!("Queuing dependency: {} (depth {})", dep_artifact, depth + 1);
                            queue.push((dep_artifact, depth + 1, merged_exclusions));
                        }
                    } else {
                        warn!("Skipping dependency {} - no version found", dep.key());
                    }
                }
            }
        }

        info!(
            "Resolution complete: {} artifacts, {} conflicts",
            resolution.artifacts.len(),
            resolution.conflicts.len()
        );
        Ok(resolution)
    }

    /// Fetch and resolve a POM, including its parent chain.
    ///
    /// This method uses `Box::pin` to handle async recursion for parent POM resolution.
    fn fetch_and_resolve_pom<'a>(
        &'a self,
        artifact: &'a Artifact,
        depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Pom>> + Send + 'a>> {
        Box::pin(async move {
            if depth > self.config.max_parent_depth {
                return Err(Error::ParentResolution {
                    artifact: artifact.coordinate(),
                    details: format!(
                        "exceeded maximum parent depth of {}",
                        self.config.max_parent_depth
                    ),
                });
            }

            // Check cache first
            let cache_key = artifact.coordinate();
            {
                let cache = self.pom_cache.read().await;
                if let Some(cached) = cache.get(&cache_key) {
                    trace!("Using cached POM for {}", artifact);
                    return Ok(cached.clone());
                }
            }

            // Fetch the POM
            let mut pom = self.fetch_pom(artifact).await?;

            // Resolve parent if present
            if let Some(ref parent) = pom.parent {
                let parent_artifact = parent.to_artifact();
                debug!("Resolving parent POM: {}", parent_artifact);

                match self
                    .fetch_and_resolve_pom(&parent_artifact, depth + 1)
                    .await
                {
                    Ok(parent_pom) => {
                        pom.resolved_parent = Some(Box::new(parent_pom));
                    }
                    Err(e) => {
                        warn!("Failed to resolve parent POM for {}: {}", artifact, e);
                        // Continue without parent - some dependencies work without full parent resolution
                    }
                }
            }

            // Cache the resolved POM
            {
                let mut cache = self.pom_cache.write().await;
                cache.insert(cache_key, pom.clone());
            }

            Ok(pom)
        })
    }

    /// Fetch the POM for an artifact.
    async fn fetch_pom(&self, artifact: &Artifact) -> Result<Pom> {
        let pom_path = artifact.pom_path();
        let mut searched_repos = Vec::new();

        for repo in self.repositories.iter() {
            let url = match repo.artifact_url(&pom_path) {
                Ok(url) => url,
                Err(e) => {
                    debug!("Invalid URL for {} in {}: {}", pom_path, repo.id, e);
                    continue;
                }
            };
            searched_repos.push(repo.id.clone());
            trace!("Trying {} from {}", pom_path, repo.id);

            match self.client.get(url.as_str()).send().await {
                Ok(response) if response.status().is_success() => {
                    let xml = response.text().await.map_err(|e| Error::Network {
                        context: format!("reading POM for {}", artifact.coordinate()),
                        source: e,
                    })?;
                    return Pom::parse_with_context(&xml, artifact);
                }
                Ok(response) => {
                    trace!(
                        "Got {} from {} for {}",
                        response.status(),
                        repo.id,
                        artifact
                    );
                }
                Err(e) => {
                    debug!("Error fetching {} from {}: {}", artifact, repo.id, e);
                }
            }
        }

        Err(Error::NotFound {
            artifact: artifact.coordinate(),
            repositories: searched_repos.join(", "),
        })
    }

    /// Fetch checksums for an artifact.
    async fn fetch_checksums(&self, artifact: &Artifact) -> Result<Checksums> {
        let jar_path = artifact.repository_path();

        for repo in self.repositories.iter() {
            // Try to fetch SHA1
            let sha1 = if let Ok(sha1_url) = repo.artifact_url(&format!("{jar_path}.sha1")) {
                match self.client.get(sha1_url.as_str()).send().await {
                    Ok(response) if response.status().is_success() => {
                        match response.text().await {
                            Ok(text) => {
                                // SHA1 files sometimes contain extra info, extract just the hash
                                Some(text.split_whitespace().next().unwrap_or(&text).to_string())
                            }
                            Err(_) => None,
                        }
                    }
                    _ => None,
                }
            } else {
                None
            };

            // Try to fetch SHA256
            let sha256 = if let Ok(sha256_url) = repo.artifact_url(&format!("{jar_path}.sha256")) {
                match self.client.get(sha256_url.as_str()).send().await {
                    Ok(response) if response.status().is_success() => match response.text().await {
                        Ok(text) => {
                            Some(text.split_whitespace().next().unwrap_or(&text).to_string())
                        }
                        Err(_) => None,
                    },
                    _ => None,
                }
            } else {
                None
            };

            if sha1.is_some() || sha256.is_some() {
                return Ok(Checksums {
                    sha1,
                    sha256,
                    repository: Some(repo.id.clone()),
                });
            }
        }

        Ok(Checksums::default())
    }

    /// Clear the POM cache.
    pub async fn clear_cache(&self) {
        let mut cache = self.pom_cache.write().await;
        cache.clear();
    }

    /// Get the number of cached POMs.
    pub async fn cache_size(&self) -> usize {
        let cache = self.pom_cache.read().await;
        cache.len()
    }
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Checksums for an artifact.
#[derive(Debug, Clone, Default)]
struct Checksums {
    sha1: Option<String>,
    sha256: Option<String>,
    repository: Option<String>,
}

/// A resolved artifact with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedArtifact {
    /// The artifact coordinates.
    pub artifact: Artifact,
    /// SHA1 checksum.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    /// SHA256 checksum.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// Repository where the artifact was found.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

/// The result of dependency resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    /// The root artifact that was resolved.
    pub root: Artifact,
    /// All resolved artifacts (including transitive dependencies).
    pub artifacts: Vec<ResolvedArtifact>,
    /// Version conflicts encountered during resolution.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<VersionConflict>,
}

impl Resolution {
    const fn new(root: Artifact) -> Self {
        Self {
            root,
            artifacts: Vec::new(),
            conflicts: Vec::new(),
        }
    }

    /// Get the total number of resolved artifacts.
    pub const fn len(&self) -> usize {
        self.artifacts.len()
    }

    /// Check if resolution is empty.
    pub const fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }

    /// Get all resolved artifacts.
    pub fn artifacts(&self) -> &[ResolvedArtifact] {
        &self.artifacts
    }

    /// Get artifacts as a map by coordinate.
    pub fn artifacts_by_coordinate(&self) -> HashMap<String, &ResolvedArtifact> {
        self.artifacts
            .iter()
            .map(|a| (a.artifact.coordinate(), a))
            .collect()
    }
}

/// A version conflict between dependencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionConflict {
    /// The artifact with conflicting versions.
    pub artifact: String,
    /// The versions that were requested.
    pub versions: Vec<String>,
    /// The version that was selected.
    pub selected: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resolver_creation() {
        let resolver = Resolver::new();
        assert!(!resolver.repositories.is_empty());
    }

    #[tokio::test]
    async fn test_resolver_cache() {
        let resolver = Resolver::new();
        assert_eq!(resolver.cache_size().await, 0);
    }
}

/// Integration tests that fetch real POMs from Maven Central.
/// These tests are ignored by default and can be run with:
/// `cargo test -- --ignored`
#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test resolving Kotlin stdlib - a real-world artifact with transitive dependencies.
    #[tokio::test]
    #[ignore = "requires network access to Maven Central"]
    async fn test_resolve_kotlin_stdlib() {
        let resolver = Resolver::new();
        let artifact = Artifact::parse("org.jetbrains.kotlin:kotlin-stdlib:2.0.0").unwrap();

        let resolution = resolver.resolve(&artifact).await.unwrap();

        // Should have at least the root artifact
        assert!(!resolution.is_empty());
        assert_eq!(resolution.root, artifact);

        // Should find the kotlin-stdlib artifact
        let coords: Vec<_> = resolution
            .artifacts
            .iter()
            .map(|a| a.artifact.coordinate())
            .collect();
        assert!(
            coords.contains(&"org.jetbrains.kotlin:kotlin-stdlib:2.0.0".to_string()),
            "Expected kotlin-stdlib in {:?}",
            coords
        );

        println!("Resolved {} artifacts for kotlin-stdlib:", resolution.len());
        for artifact in resolution.artifacts() {
            println!("  {}", artifact.artifact);
        }
    }

    /// Test resolving Groovy - tests parent POM resolution.
    #[tokio::test]
    #[ignore = "requires network access to Maven Central"]
    async fn test_resolve_groovy() {
        let resolver = Resolver::new();
        let artifact = Artifact::parse("org.apache.groovy:groovy:4.0.24").unwrap();

        let resolution = resolver.resolve(&artifact).await.unwrap();

        assert!(!resolution.is_empty());

        println!("Resolved {} artifacts for groovy:", resolution.len());
        for artifact in resolution.artifacts() {
            println!("  {}", artifact.artifact);
        }
    }

    /// Test resolving Guava - tests property substitution and complex dependencies.
    #[tokio::test]
    #[ignore = "requires network access to Maven Central"]
    async fn test_resolve_guava() {
        let resolver = Resolver::new();
        let artifact = Artifact::parse("com.google.guava:guava:33.0.0-jre").unwrap();

        let resolution = resolver.resolve(&artifact).await.unwrap();

        assert!(!resolution.is_empty());

        // Guava has dependencies like failureaccess, checker-qual, etc.
        let coords: Vec<_> = resolution
            .artifacts
            .iter()
            .map(|a| a.artifact.coordinate())
            .collect();
        println!(
            "Resolved {} artifacts for guava: {:?}",
            resolution.len(),
            coords
        );
    }

    /// Test resolving SLF4J API - a simple artifact with minimal dependencies.
    #[tokio::test]
    #[ignore = "requires network access to Maven Central"]
    async fn test_resolve_slf4j_api() {
        let resolver = Resolver::new();
        let artifact = Artifact::parse("org.slf4j:slf4j-api:2.0.9").unwrap();

        let resolution = resolver.resolve(&artifact).await.unwrap();

        // SLF4J API should have very few or no transitive dependencies
        assert!(!resolution.is_empty());
        println!("Resolved {} artifacts for slf4j-api", resolution.len());
    }

    /// Test POM caching works correctly.
    #[tokio::test]
    #[ignore = "requires network access to Maven Central"]
    async fn test_pom_caching() {
        let resolver = Resolver::new();
        let artifact = Artifact::parse("org.slf4j:slf4j-api:2.0.9").unwrap();

        // First resolution
        let _ = resolver.resolve(&artifact).await.unwrap();
        let cache_size_after_first = resolver.cache_size().await;
        assert!(
            cache_size_after_first > 0,
            "Cache should not be empty after resolution"
        );

        // Second resolution should use cache
        let _ = resolver.resolve(&artifact).await.unwrap();
        let cache_size_after_second = resolver.cache_size().await;
        assert_eq!(
            cache_size_after_first, cache_size_after_second,
            "Cache size should not change on second resolution"
        );

        // Clear cache
        resolver.clear_cache().await;
        assert_eq!(resolver.cache_size().await, 0);
    }

    /// Test that version conflicts are detected and reported.
    #[tokio::test]
    #[ignore = "requires network access to Maven Central"]
    async fn test_version_conflict_detection() {
        let resolver = Resolver::new();
        // Jackson has multiple modules that might have version conflicts in transitive deps
        let artifact =
            Artifact::parse("com.fasterxml.jackson.core:jackson-databind:2.16.0").unwrap();

        let resolution = resolver.resolve(&artifact).await.unwrap();

        assert!(!resolution.is_empty());
        println!(
            "Resolved {} artifacts, {} conflicts",
            resolution.len(),
            resolution.conflicts.len()
        );

        for conflict in &resolution.conflicts {
            println!(
                "  Conflict: {} - versions {:?}, selected {}",
                conflict.artifact, conflict.versions, conflict.selected
            );
        }
    }
}
