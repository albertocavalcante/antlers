//! Configuration and hermeticity controls for antler.
//!
//! This module provides configuration types that support hermetic builds
//! for integration with build systems like Buck2 and Bazel.
//!
//! # Hermeticity
//!
//! By default, antler operates in a **non-hermetic** mode suitable for
//! interactive use. For build system integration, use [`HermeticConfig`]
//! to ensure reproducible, sandboxed execution.
//!
//! ```no_run
//! use antlers::config::{AntlerConfig, HermeticConfig, CacheConfig};
//!
//! // Fully hermetic - no network, no env vars, explicit cache only
//! let config = AntlerConfig::hermetic()
//!     .with_cache(CacheConfig::read_only("/sandbox/cache"))
//!     .with_allowed_env_vars(&["https_proxy"]); // explicit allowlist
//! ```

mod cache;
mod env;
mod hermetic;

pub use cache::{CacheConfig, CacheMode};
pub use env::{EnvConfig, EnvVar};
pub use hermetic::{HermeticConfig, HermeticLevel};

use std::time::Duration;

/// Main configuration for antler.
///
/// Controls caching, network access, environment isolation, and other
/// behaviors critical for hermetic builds.
#[derive(Debug, Clone)]
pub struct AntlerConfig {
    /// Cache configuration.
    pub cache: CacheConfig,

    /// Environment variable access control.
    pub env: EnvConfig,

    /// Hermeticity settings.
    pub hermetic: HermeticConfig,

    /// Network configuration.
    pub network: NetworkConfig,

    /// Parallelism settings.
    pub parallelism: ParallelismConfig,
}

impl Default for AntlerConfig {
    fn default() -> Self {
        Self::interactive()
    }
}

impl AntlerConfig {
    /// Creates a configuration for interactive (non-hermetic) use.
    ///
    /// This is the default mode, suitable for CLI usage. It:
    /// - Uses `~/.cache/antler` for caching
    /// - Allows network access
    /// - Inherits common environment variables
    #[must_use]
    pub fn interactive() -> Self {
        Self {
            cache: CacheConfig::default(),
            env: EnvConfig::default(),
            hermetic: HermeticConfig::disabled(),
            network: NetworkConfig::default(),
            parallelism: ParallelismConfig::default(),
        }
    }

    /// Creates a fully hermetic configuration for build systems.
    ///
    /// This mode:
    /// - Disables all network access (offline only)
    /// - Blocks all environment variable access
    /// - Requires explicit cache path
    /// - Produces deterministic output
    #[must_use]
    pub fn hermetic() -> Self {
        Self {
            cache: CacheConfig::disabled(),
            env: EnvConfig::blocked(),
            hermetic: HermeticConfig::strict(),
            network: NetworkConfig::offline(),
            parallelism: ParallelismConfig::default(),
        }
    }

    /// Creates a configuration for CI/CD environments.
    ///
    /// This mode:
    /// - Allows network access for fetching
    /// - Uses explicit cache path
    /// - Blocks most environment variables
    /// - Produces deterministic output
    #[must_use]
    pub fn ci() -> Self {
        Self {
            cache: CacheConfig::default(),
            env: EnvConfig::ci_defaults(),
            hermetic: HermeticConfig::reproducible(),
            network: NetworkConfig::default(),
            parallelism: ParallelismConfig::default(),
        }
    }

    /// Sets the cache configuration.
    #[must_use]
    pub fn with_cache(mut self, cache: CacheConfig) -> Self {
        self.cache = cache;
        self
    }

    /// Sets the environment configuration.
    #[must_use]
    pub fn with_env(mut self, env: EnvConfig) -> Self {
        self.env = env;
        self
    }

    /// Adds allowed environment variables.
    #[must_use]
    pub fn with_allowed_env_vars(mut self, vars: &[&str]) -> Self {
        for var in vars {
            self.env.allow(var);
        }
        self
    }

    /// Sets the network configuration.
    #[must_use]
    pub const fn with_network(mut self, network: NetworkConfig) -> Self {
        self.network = network;
        self
    }

    /// Enables offline mode (no network access).
    #[must_use]
    pub fn offline(mut self) -> Self {
        self.network = NetworkConfig::offline();
        self
    }

    /// Sets the parallelism configuration.
    #[must_use]
    pub const fn with_parallelism(mut self, parallelism: ParallelismConfig) -> Self {
        self.parallelism = parallelism;
        self
    }

    /// Validates the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid (e.g., hermetic mode
    /// with network enabled).
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Hermetic mode requires offline
        if self.hermetic.level == HermeticLevel::Strict && self.network.enabled {
            return Err(ConfigError::InvalidCombination(
                "Strict hermetic mode requires offline network".to_string(),
            ));
        }

        // Hermetic mode requires explicit cache or disabled cache
        if self.hermetic.level == HermeticLevel::Strict && self.cache.mode == CacheMode::Auto {
            return Err(ConfigError::InvalidCombination(
                "Strict hermetic mode requires explicit cache path or disabled cache".to_string(),
            ));
        }

        Ok(())
    }
}

/// Network configuration.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Whether network access is enabled.
    pub enabled: bool,

    /// Connection timeout.
    pub connect_timeout: Duration,

    /// Read timeout.
    pub read_timeout: Duration,

    /// Maximum concurrent connections.
    pub max_connections: usize,

    /// Retry configuration.
    pub retries: RetryConfig,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            connect_timeout: Duration::from_secs(30),
            read_timeout: Duration::from_secs(60),
            max_connections: 16,
            retries: RetryConfig::default(),
        }
    }
}

impl NetworkConfig {
    /// Creates an offline configuration (no network access).
    #[must_use]
    pub fn offline() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Sets the connection timeout.
    #[must_use]
    pub const fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Sets the maximum concurrent connections.
    #[must_use]
    pub const fn with_max_connections(mut self, max: usize) -> Self {
        self.max_connections = max;
        self
    }
}

/// Retry configuration for network requests.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retries.
    pub max_retries: u32,

    /// Initial backoff duration.
    pub initial_backoff: Duration,

    /// Maximum backoff duration.
    pub max_backoff: Duration,

    /// Backoff multiplier.
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            multiplier: 2.0,
        }
    }
}

/// Parallelism configuration.
#[derive(Debug, Clone)]
pub struct ParallelismConfig {
    /// Maximum concurrent POM fetches.
    pub max_concurrent_fetches: usize,

    /// Maximum concurrent checksum fetches.
    pub max_concurrent_checksums: usize,

    /// Whether to fetch checksums in parallel with resolution.
    pub parallel_checksums: bool,
}

impl Default for ParallelismConfig {
    fn default() -> Self {
        Self {
            max_concurrent_fetches: 16,
            max_concurrent_checksums: 32,
            parallel_checksums: true,
        }
    }
}

impl ParallelismConfig {
    /// Creates a sequential (non-parallel) configuration.
    #[must_use]
    pub const fn sequential() -> Self {
        Self {
            max_concurrent_fetches: 1,
            max_concurrent_checksums: 1,
            parallel_checksums: false,
        }
    }
}

/// Configuration error.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConfigError {
    /// Invalid configuration combination.
    #[error("Invalid configuration: {0}")]
    InvalidCombination(String),

    /// Invalid path.
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Missing required configuration.
    #[error("Missing required configuration: {0}")]
    MissingRequired(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interactive_config() {
        let config = AntlerConfig::interactive();
        assert!(config.network.enabled);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_hermetic_config() {
        let config = AntlerConfig::hermetic();
        assert!(!config.network.enabled);
        assert_eq!(config.hermetic.level, HermeticLevel::Strict);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_hermetic_with_network() {
        let config = AntlerConfig::hermetic().with_network(NetworkConfig::default());
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_ci_config() {
        let config = AntlerConfig::ci();
        assert!(config.network.enabled);
        assert_eq!(config.hermetic.level, HermeticLevel::Reproducible);
    }
}
