//! Dependency resolution logic.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::artifact::Artifact;
use crate::error::{Error, Result};
use crate::pom::Pom;
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
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            transitive: true,
            include_optional: false,
            include_scopes: HashSet::new(),
            fetch_sources: false,
            fetch_javadoc: false,
        }
    }
}

/// The dependency resolver.
pub struct Resolver {
    repositories: RepositoryList,
    config: ResolverConfig,
    client: reqwest::Client,
}

impl Resolver {
    /// Create a new resolver with default settings.
    pub fn new() -> Self {
        Self {
            repositories: RepositoryList::with_defaults(),
            config: ResolverConfig::default(),
            client: reqwest::Client::new(),
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
    pub async fn resolve(&self, artifact: &Artifact) -> Result<Resolution> {
        info!("Resolving {}", artifact);

        let mut resolution = Resolution::new(artifact.clone());
        let mut visited = HashSet::new();
        let mut queue = vec![artifact.clone()];

        while let Some(current) = queue.pop() {
            let coord = current.coordinate();
            if visited.contains(&coord) {
                continue;
            }
            visited.insert(coord.clone());

            debug!("Processing {}", current);

            // Fetch POM
            let pom = match self.fetch_pom(&current).await {
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

                    if let Some(dep_artifact) = dep.to_artifact() {
                        let dep_coord = dep_artifact.coordinate();
                        if !visited.contains(&dep_coord) {
                            debug!("Queuing dependency: {}", dep_artifact);
                            queue.push(dep_artifact);
                        }
                    }
                }
            }
        }

        info!(
            "Resolution complete: {} artifacts",
            resolution.artifacts.len()
        );
        Ok(resolution)
    }

    /// Fetch the POM for an artifact.
    async fn fetch_pom(&self, artifact: &Artifact) -> Result<Pom> {
        let pom_path = artifact.pom_path();

        for repo in self.repositories.iter() {
            let url = match repo.artifact_url(&pom_path) {
                Ok(url) => url,
                Err(e) => {
                    debug!("Invalid URL for {} in {}: {}", pom_path, repo.id, e);
                    continue;
                }
            };
            debug!("Trying {} from {}", pom_path, repo.id);

            match self.client.get(url.as_str()).send().await {
                Ok(response) if response.status().is_success() => {
                    let xml = response.text().await?;
                    return Pom::parse(&xml);
                }
                Ok(response) => {
                    debug!("Got {} from {}", response.status(), repo.id);
                }
                Err(e) => {
                    debug!("Error fetching from {}: {}", repo.id, e);
                }
            }
        }

        Err(Error::NotFound(artifact.coordinate()))
    }

    /// Fetch checksums for an artifact.
    async fn fetch_checksums(&self, artifact: &Artifact) -> Result<Checksums> {
        let jar_path = artifact.repository_path();

        for repo in self.repositories.iter() {
            // Try to fetch SHA1
            let sha1 = if let Ok(sha1_url) = repo.artifact_url(&format!("{jar_path}.sha1")) {
                match self.client.get(sha1_url.as_str()).send().await {
                    Ok(response) if response.status().is_success() => {
                        let text = response.text().await?;
                        // SHA1 files sometimes contain extra info, extract just the hash
                        Some(text.split_whitespace().next().unwrap_or(&text).to_string())
                    }
                    _ => None,
                }
            } else {
                None
            };

            // Try to fetch SHA256
            let sha256 = if let Ok(sha256_url) = repo.artifact_url(&format!("{jar_path}.sha256")) {
                match self.client.get(sha256_url.as_str()).send().await {
                    Ok(response) if response.status().is_success() => {
                        let text = response.text().await?;
                        Some(text.split_whitespace().next().unwrap_or(&text).to_string())
                    }
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
    fn new(root: Artifact) -> Self {
        Self {
            root,
            artifacts: Vec::new(),
            conflicts: Vec::new(),
        }
    }

    /// Get the total number of resolved artifacts.
    pub fn len(&self) -> usize {
        self.artifacts.len()
    }

    /// Check if resolution is empty.
    pub fn is_empty(&self) -> bool {
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
}
