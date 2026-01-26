//! HTTP fetching for Maven artifacts.
//!
//! This module provides the `Fetcher` struct for downloading artifacts from
//! Maven repositories with optional caching and checksum verification.

use std::path::Path;

use gav::Artifact;
use tracing::{debug, trace, warn};

use crate::cache::{Cache, CacheEntry};
use crate::checksum::{Checksum, ChecksumAlgo, ChecksumVerifier};
use crate::error::{Error, Result};
use crate::repository::{MavenRepository, RepositoryList};

/// Proxy configuration for HTTP requests.
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    /// Proxy URL (e.g., `http://proxy.example.com:8080`).
    pub url: String,
    /// Optional username for proxy authentication.
    pub username: Option<String>,
    /// Optional password for proxy authentication.
    pub password: Option<String>,
}

impl ProxyConfig {
    /// Creates a new proxy configuration with just a URL.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            username: None,
            password: None,
        }
    }

    /// Adds authentication credentials to the proxy.
    #[must_use]
    pub fn with_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }

    /// Creates a proxy from environment variables (`HTTP_PROXY`, `HTTPS_PROXY`).
    ///
    /// Returns `None` if no proxy environment variables are set.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        // Check HTTPS_PROXY first, then HTTP_PROXY
        std::env::var("HTTPS_PROXY")
            .or_else(|_| std::env::var("https_proxy"))
            .or_else(|_| std::env::var("HTTP_PROXY"))
            .or_else(|_| std::env::var("http_proxy"))
            .ok()
            .filter(|s| !s.is_empty())
            .map(Self::new)
    }
}

/// Fetcher for downloading Maven artifacts.
///
/// The fetcher searches through configured repositories in order until an
/// artifact is found, optionally caching results and verifying checksums.
pub struct Fetcher {
    client: reqwest::Client,
    repositories: RepositoryList,
    cache: Option<Box<dyn Cache>>,
}

impl Fetcher {
    /// Creates a new fetcher with the given repositories.
    #[must_use]
    pub fn new(repositories: RepositoryList) -> Self {
        Self {
            client: reqwest::Client::new(),
            repositories,
            cache: None,
        }
    }

    /// Creates a new fetcher with a custom proxy.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use gather::{Fetcher, ProxyConfig, RepositoryList};
    ///
    /// let proxy = ProxyConfig::new("http://proxy.example.com:8080");
    /// let fetcher = Fetcher::with_proxy(RepositoryList::with_defaults(), &proxy);
    /// ```
    #[must_use]
    pub fn with_proxy(repositories: RepositoryList, proxy: &ProxyConfig) -> Self {
        let mut builder = reqwest::Client::builder();

        if let Ok(proxy_obj) = reqwest::Proxy::all(&proxy.url) {
            let proxy_obj = if let (Some(user), Some(pass)) = (&proxy.username, &proxy.password) {
                proxy_obj.basic_auth(user, pass)
            } else {
                proxy_obj
            };
            builder = builder.proxy(proxy_obj);
            debug!("Using proxy: {}", proxy.url);
        } else {
            warn!("Invalid proxy URL: {}", proxy.url);
        }

        Self {
            client: builder.build().unwrap_or_else(|_| reqwest::Client::new()),
            repositories,
            cache: None,
        }
    }

    /// Creates a new fetcher using system proxy settings from environment variables.
    ///
    /// Reads from `HTTPS_PROXY`, `HTTP_PROXY` (and lowercase variants).
    /// Falls back to no proxy if environment variables are not set.
    #[must_use]
    pub fn with_system_proxy(repositories: RepositoryList) -> Self {
        if let Some(ref proxy) = ProxyConfig::from_env() {
            Self::with_proxy(repositories, proxy)
        } else {
            Self::new(repositories)
        }
    }

    /// Creates a new fetcher with default repositories (Maven Central + Google).
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(RepositoryList::with_defaults())
    }

    /// Adds a cache to the fetcher.
    #[must_use]
    pub fn with_cache(mut self, cache: impl Cache + 'static) -> Self {
        self.cache = Some(Box::new(cache));
        self
    }

    /// Adds a repository to search.
    #[must_use]
    pub fn with_repository(mut self, repo: MavenRepository) -> Self {
        self.repositories.add(repo);
        self
    }

    /// Returns a reference to the configured repositories.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const due to reqwest::Client
    pub fn repositories(&self) -> &RepositoryList {
        &self.repositories
    }

    /// Fetches the content of an artifact.
    ///
    /// Searches through repositories in order until the artifact is found.
    ///
    /// # Errors
    ///
    /// Returns an error if the artifact cannot be found in any repository
    /// or if a network error occurs.
    pub async fn fetch(&self, artifact: &Artifact) -> Result<Vec<u8>> {
        let path = artifact.repository_path();
        self.fetch_path(&path, &artifact.coordinate()).await
    }

    /// Fetches the POM content for an artifact as a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the POM cannot be found or fetched.
    pub async fn fetch_pom(&self, artifact: &Artifact) -> Result<String> {
        let path = artifact.pom_path();
        let bytes = self
            .fetch_path(&path, &format!("{} POM", artifact.coordinate()))
            .await?;
        String::from_utf8(bytes).map_err(|e| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("POM is not valid UTF-8: {e}"),
            ))
        })
    }

    /// Fetches the Gradle Module Metadata (.module) for an artifact.
    ///
    /// Returns `None` if the .module file doesn't exist (404), otherwise returns
    /// the content as a string.
    ///
    /// # Errors
    ///
    /// Returns an error if a network error occurs (other than 404).
    pub async fn fetch_module(&self, artifact: &Artifact) -> Result<Option<String>> {
        let path = artifact.module_path();

        for repo in self.repositories.iter() {
            match self.try_fetch_from_repo(repo, &path).await {
                Ok(bytes) => {
                    let content = String::from_utf8(bytes).map_err(|e| {
                        Error::Io(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Module metadata is not valid UTF-8: {e}"),
                        ))
                    })?;
                    return Ok(Some(content));
                }
                Err(Error::Network { .. }) => {
                    // Try next repository
                    trace!("No .module file in {}: {}", repo.name, path);
                }
                Err(e) => return Err(e),
            }
        }

        // Not found in any repository
        Ok(None)
    }

    /// Fetches a checksum file for an artifact.
    ///
    /// Returns `None` if the checksum file doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if a network error occurs (other than 404).
    pub async fn fetch_checksum(
        &self,
        artifact: &Artifact,
        algo: ChecksumAlgo,
    ) -> Result<Option<String>> {
        let path = format!("{}.{}", artifact.repository_path(), algo.extension());

        for repo in self.repositories.iter() {
            match self.try_fetch_from_repo(repo, &path).await {
                Ok(bytes) => {
                    let content = String::from_utf8_lossy(&bytes);
                    // Checksum files often contain just the hash, or hash + filename
                    let checksum = content.split_whitespace().next().unwrap_or("").to_string();
                    return Ok(Some(checksum));
                }
                Err(Error::Network { .. }) => {}
                Err(e) => return Err(e),
            }
        }

        Ok(None)
    }

    /// Fetches an artifact and verifies its checksum.
    ///
    /// Tries SHA-256 first, then falls back to SHA-1.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The artifact cannot be found
    /// - The checksum verification fails
    /// - A network error occurs
    pub async fn fetch_verified(&self, artifact: &Artifact) -> Result<(Vec<u8>, Option<Checksum>)> {
        let data = self.fetch(artifact).await?;

        // Try to fetch and verify checksum (prefer SHA-256)
        for algo in ChecksumAlgo::recommended() {
            if let Ok(Some(expected)) = self.fetch_checksum(artifact, *algo).await {
                let checksum = Checksum::new(*algo, expected.clone());

                if ChecksumVerifier::verify(&data, &checksum) {
                    debug!(
                        "Verified {} checksum for {}",
                        algo.extension(),
                        artifact.coordinate()
                    );
                    return Ok((data, Some(checksum)));
                }

                let actual = ChecksumVerifier::compute(&data, *algo);
                return Err(Error::ChecksumMismatch {
                    artifact: artifact.coordinate(),
                    expected,
                    actual,
                });
            }
        }

        // No checksum available
        warn!("No checksum available for {}", artifact.coordinate());
        Ok((data, None))
    }

    /// Downloads an artifact to a file.
    ///
    /// # Errors
    ///
    /// Returns an error if the artifact cannot be fetched or the file cannot be written.
    pub async fn download(&self, artifact: &Artifact, dest: &Path) -> Result<()> {
        let data = self.fetch(artifact).await?;

        // Create parent directories if needed
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(dest, &data)?;
        debug!("Downloaded {} to {}", artifact.coordinate(), dest.display());

        Ok(())
    }

    /// Downloads an artifact to a file with checksum verification.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The artifact cannot be fetched
    /// - Checksum verification fails
    /// - The file cannot be written
    pub async fn download_verified(
        &self,
        artifact: &Artifact,
        dest: &Path,
    ) -> Result<Option<Checksum>> {
        let (data, checksum) = self.fetch_verified(artifact).await?;

        // Create parent directories if needed
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(dest, &data)?;
        debug!(
            "Downloaded and verified {} to {}",
            artifact.coordinate(),
            dest.display()
        );

        Ok(checksum)
    }

    /// Fetches a path from repositories.
    async fn fetch_path(&self, path: &str, description: &str) -> Result<Vec<u8>> {
        // Check cache first
        if let Some(entry) = self.cache.as_ref().and_then(|c| c.get(path)) {
            trace!("Cache hit for {}", path);
            return Ok(entry.data);
        }

        // Try each repository
        for repo in self.repositories.iter() {
            match self.try_fetch_from_repo(repo, path).await {
                Ok(data) => {
                    // Cache the result
                    if let Some(ref cache) = self.cache {
                        cache.put(path, CacheEntry::new(data.clone(), None));
                    }
                    return Ok(data);
                }
                Err(Error::Network { .. }) => {
                    trace!("Not found in {}: {}", repo.name, path);
                }
                Err(e) => return Err(e),
            }
        }

        Err(Error::NotFound {
            artifact: description.to_string(),
            repositories: self.repositories.names(),
        })
    }

    /// Tries to fetch a path from a specific repository.
    async fn try_fetch_from_repo(&self, repo: &MavenRepository, path: &str) -> Result<Vec<u8>> {
        let url = repo.artifact_url(path)?;
        trace!("Trying {}", url);

        let mut request = self.client.get(url.clone());

        // Apply authentication if configured
        if let Some(ref creds) = repo.credentials
            && let Some(host) = repo.host()
            && let Some(auth_header) = creds.authorization_header(host)
        {
            trace!("Applying authentication for {}", host);
            request = request.header("Authorization", auth_header);
        }

        let response = request.send().await.map_err(|e| Error::Network {
            context: url.to_string(),
            source: e,
        })?;

        if !response.status().is_success() {
            return Err(Error::Network {
                context: format!("{} returned HTTP {}", url, response.status()),
                source: response.error_for_status().unwrap_err(),
            });
        }

        let bytes = response.bytes().await.map_err(|e| Error::Network {
            context: format!("reading response from {url}"),
            source: e,
        })?;

        Ok(bytes.to_vec())
    }
}

impl std::fmt::Debug for Fetcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fetcher")
            .field("repositories", &self.repositories)
            .field("has_cache", &self.cache.is_some())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::MemoryCache;

    #[test]
    fn test_fetcher_new() {
        let fetcher = Fetcher::with_defaults();
        assert_eq!(fetcher.repositories().len(), 2);
    }

    #[test]
    fn test_fetcher_with_cache() {
        let fetcher = Fetcher::with_defaults().with_cache(MemoryCache::new());
        assert!(fetcher.cache.is_some());
    }

    #[test]
    fn test_fetcher_with_repository() {
        let fetcher =
            Fetcher::new(RepositoryList::new()).with_repository(MavenRepository::maven_central());
        assert_eq!(fetcher.repositories().len(), 1);
    }

    #[test]
    fn test_proxy_config_new() {
        let proxy = ProxyConfig::new("http://proxy.example.com:8080");
        assert_eq!(proxy.url, "http://proxy.example.com:8080");
        assert!(proxy.username.is_none());
        assert!(proxy.password.is_none());
    }

    #[test]
    fn test_proxy_config_with_auth() {
        let proxy = ProxyConfig::new("http://proxy.example.com:8080").with_auth("user", "pass");
        assert_eq!(proxy.url, "http://proxy.example.com:8080");
        assert_eq!(proxy.username.as_deref(), Some("user"));
        assert_eq!(proxy.password.as_deref(), Some("pass"));
    }

    #[test]
    fn test_fetcher_with_proxy() {
        let proxy = ProxyConfig::new("http://localhost:8080");
        let fetcher = Fetcher::with_proxy(RepositoryList::with_defaults(), &proxy);
        assert_eq!(fetcher.repositories().len(), 2);
    }

    #[test]
    fn test_fetcher_with_proxy_and_auth() {
        let proxy = ProxyConfig::new("http://localhost:8080").with_auth("proxyuser", "proxypass");
        let fetcher = Fetcher::with_proxy(RepositoryList::with_defaults(), &proxy);
        assert_eq!(fetcher.repositories().len(), 2);
    }
}
