//! Parallel fetching infrastructure for high-performance resolution.
//!
//! This module provides concurrent fetching of POMs and checksums
//! with configurable parallelism limits.

use std::collections::HashMap;
use std::sync::Arc;

use futures::stream::{self, StreamExt};
use tokio::sync::{Mutex, Semaphore};

use dendro::{Checksums, ProjectFetcher};
use gather::{ChecksumAlgo, Fetcher, RepositoryList};
use gav::Artifact;
use pomace::{Pom, PomParser};

use crate::config::ParallelismConfig;
use crate::fetcher::{PomFetchError, PomProject};

/// A parallel-aware POM fetcher with caching.
///
/// This fetcher:
/// - Fetches POMs concurrently with configurable limits
/// - Caches fetched POMs in memory to avoid duplicate fetches
/// - Fetches checksums in parallel
pub struct ParallelPomFetcher {
    fetcher: Arc<Fetcher>,
    config: ParallelismConfig,
    /// Semaphore for limiting concurrent POM fetches.
    pom_semaphore: Arc<Semaphore>,
    /// Semaphore for limiting concurrent checksum fetches.
    checksum_semaphore: Arc<Semaphore>,
    /// In-memory cache of fetched POMs (keyed by coordinate).
    pom_cache: Arc<Mutex<HashMap<String, Arc<Pom>>>>,
}

impl ParallelPomFetcher {
    /// Creates a new parallel fetcher with the given configuration.
    pub fn new(repositories: RepositoryList, config: ParallelismConfig) -> Self {
        Self {
            fetcher: Arc::new(Fetcher::new(repositories)),
            pom_semaphore: Arc::new(Semaphore::new(config.max_concurrent_fetches)),
            checksum_semaphore: Arc::new(Semaphore::new(config.max_concurrent_checksums)),
            pom_cache: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Creates a new parallel fetcher with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(
            RepositoryList::with_defaults(),
            ParallelismConfig::default(),
        )
    }

    /// Returns a reference to the underlying fetcher.
    pub fn fetcher(&self) -> &Fetcher {
        &self.fetcher
    }

    /// Fetches a POM with parent resolution, using the cache.
    async fn fetch_pom_cached(&self, artifact: &Artifact) -> Result<Arc<Pom>, PomFetchError> {
        let key = artifact.coordinate();

        // Check cache first
        {
            let cache = self.pom_cache.lock().await;
            if let Some(pom) = cache.get(&key) {
                return Ok(Arc::clone(pom));
            }
        }

        // Acquire semaphore permit for concurrent fetch limiting
        let _permit = self
            .pom_semaphore
            .acquire()
            .await
            .map_err(|_| PomFetchError::new("Semaphore closed"))?;

        // Double-check cache after acquiring permit
        {
            let cache = self.pom_cache.lock().await;
            if let Some(pom) = cache.get(&key) {
                return Ok(Arc::clone(pom));
            }
        }

        // Fetch the POM
        let pom = self.fetch_pom_with_parents(artifact, 0).await?;
        let pom = Arc::new(pom);

        // Store in cache
        {
            let mut cache = self.pom_cache.lock().await;
            cache.insert(key, Arc::clone(&pom));
        }

        Ok(pom)
    }

    /// Recursively fetches and resolves parent POMs.
    fn fetch_pom_with_parents<'a>(
        &'a self,
        artifact: &'a Artifact,
        depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Pom, PomFetchError>> + Send + 'a>>
    {
        Box::pin(async move {
            const MAX_PARENT_DEPTH: usize = 10;

            if depth > MAX_PARENT_DEPTH {
                return Err(PomFetchError::new(format!(
                    "Maximum parent depth ({MAX_PARENT_DEPTH}) exceeded for {artifact}"
                )));
            }

            // Fetch the POM content
            let pom_content = self.fetcher.fetch_pom(artifact).await.map_err(|e| {
                PomFetchError::new(format!("Failed to fetch POM for {artifact}: {e}"))
            })?;

            // Parse the POM
            let mut pom = PomParser::parse(&pom_content).map_err(|e| {
                PomFetchError::new(format!("Failed to parse POM for {artifact}: {e}"))
            })?;

            // If there's a parent, fetch it recursively
            if let Some(ref parent) = pom.parent {
                let parent_artifact =
                    Artifact::new(&parent.group_id, &parent.artifact_id, &parent.version)
                        .with_extension("pom");

                match self
                    .fetch_pom_with_parents(&parent_artifact, depth + 1)
                    .await
                {
                    Ok(parent_pom) => {
                        pom.resolved_parent = Some(Box::new(parent_pom));
                    }
                    Err(e) => {
                        tracing::warn!("Failed to resolve parent POM for {artifact}: {e}");
                    }
                }
            }

            Ok(pom)
        })
    }

    /// Fetches checksums for multiple artifacts in parallel.
    pub async fn fetch_checksums_batch(
        &self,
        artifacts: &[Artifact],
    ) -> Vec<(Artifact, Checksums)> {
        stream::iter(artifacts)
            .map(|artifact| async {
                let checksums = self.fetch_checksums_single(artifact).await;
                (artifact.clone(), checksums)
            })
            .buffer_unordered(self.config.max_concurrent_checksums)
            .collect()
            .await
    }

    /// Fetches checksums for a single artifact.
    async fn fetch_checksums_single(&self, artifact: &Artifact) -> Checksums {
        let Ok(_permit) = self.checksum_semaphore.acquire().await else {
            return Checksums::none();
        };

        let mut checksums = Checksums::none();

        // Fetch SHA1 and SHA256 in parallel
        let (sha1_result, sha256_result) = tokio::join!(
            self.fetcher.fetch_checksum(artifact, ChecksumAlgo::Sha1),
            self.fetcher.fetch_checksum(artifact, ChecksumAlgo::Sha256),
        );

        if let Ok(Some(sha1)) = sha1_result {
            checksums = checksums.with_sha1(sha1);
        }

        if let Ok(Some(sha256)) = sha256_result {
            checksums = checksums.with_sha256(sha256);
        }

        if let Some(repo) = self.fetcher.repositories().iter().next() {
            checksums = checksums.with_repository(&repo.name);
        }

        checksums
    }

    /// Clears the in-memory POM cache.
    pub async fn clear_cache(&self) {
        let mut cache = self.pom_cache.lock().await;
        cache.clear();
    }

    /// Returns the number of cached POMs.
    pub async fn cache_size(&self) -> usize {
        let cache = self.pom_cache.lock().await;
        cache.len()
    }
}

impl ProjectFetcher for ParallelPomFetcher {
    type Project = PomProject;
    type Error = PomFetchError;

    async fn fetch(&self, artifact: &Artifact) -> Result<Self::Project, Self::Error> {
        let pom = self.fetch_pom_cached(artifact).await?;
        // Clone the Pom from Arc for PomProject
        PomProject::new((*pom).clone())
            .map_err(|e| PomFetchError::new(format!("Invalid POM for {artifact}: {e}")))
    }

    async fn fetch_checksums(&self, artifact: &Artifact) -> Checksums {
        self.fetch_checksums_single(artifact).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_fetcher_creation() {
        let fetcher = ParallelPomFetcher::with_defaults();
        assert_eq!(fetcher.fetcher().repositories().len(), 2);
    }

    #[test]
    fn test_parallelism_config() {
        let config = ParallelismConfig {
            max_concurrent_fetches: 8,
            max_concurrent_checksums: 16,
            parallel_checksums: true,
        };
        let fetcher = ParallelPomFetcher::new(RepositoryList::with_defaults(), config);
        // Verify semaphores have correct permits
        assert_eq!(fetcher.pom_semaphore.available_permits(), 8);
        assert_eq!(fetcher.checksum_semaphore.available_permits(), 16);
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let fetcher = ParallelPomFetcher::with_defaults();
        assert_eq!(fetcher.cache_size().await, 0);

        // Cache operations are tested in integration tests
        fetcher.clear_cache().await;
        assert_eq!(fetcher.cache_size().await, 0);
    }
}
