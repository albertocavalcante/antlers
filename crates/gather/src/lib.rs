//! Artifact fetching, caching, and checksum verification.
//!
//! This crate provides infrastructure for fetching JVM artifacts from Maven repositories.
//!
//! # Features
//!
//! - [`MavenRepository`] - Maven-layout repository definitions (Maven Central, Google, etc.)
//! - [`RepositoryList`] - Collection of repositories to search
//! - [`Fetcher`] - HTTP fetching with multi-repository search
//! - [`Cache`] trait - Caching abstraction ([`MemoryCache`], [`FileCache`])
//! - [`ChecksumVerifier`] - SHA1/SHA256/SHA512/MD5 verification
//!
//! # Example
//!
//! ```no_run
//! use gav::Artifact;
//! use gather::{Fetcher, RepositoryList, MemoryCache};
//!
//! # async fn example() -> gather::Result<()> {
//! // Create a fetcher with default repositories
//! let fetcher = Fetcher::with_defaults()
//!     .with_cache(MemoryCache::new());
//!
//! // Fetch an artifact
//! let artifact = Artifact::parse("org.apache.commons:commons-lang3:3.12.0")?;
//! let (data, checksum) = fetcher.fetch_verified(&artifact).await?;
//!
//! println!("Downloaded {} bytes", data.len());
//! if let Some(cs) = checksum {
//!     println!("Verified with {}: {}", cs.algo, cs.value);
//! }
//! # Ok(())
//! # }
//! ```

mod auth;
mod cache;
mod checksum;
mod error;
mod fetch;
mod repository;

pub use auth::{Credentials, Netrc, StringOrEnvRef};
pub use cache::{Cache, CacheEntry, FileCache, MemoryCache};
pub use checksum::{Checksum, ChecksumAlgo, ChecksumVerifier};
pub use error::{Error, Result};
pub use fetch::{Fetcher, ProxyConfig};
pub use repository::{Ecosystem, MavenRepository, Repository, RepositoryList};
