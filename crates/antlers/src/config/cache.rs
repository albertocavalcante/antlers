//! Cache configuration for hermetic builds.
//!
//! Provides content-addressable caching with support for:
//! - Read-only mode (for hermetic builds)
//! - Read-write mode (for interactive use)
//! - Disabled mode (no caching)
//! - Custom cache paths

use std::path::{Path, PathBuf};

/// Cache operation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheMode {
    /// Automatically determine cache location.
    /// Uses `~/.cache/antler` on Unix, `%LOCALAPPDATA%\antler` on Windows.
    /// **Not hermetic** - reads from environment/filesystem.
    Auto,

    /// Use a specific cache directory.
    /// Can be read-write or read-only.
    Explicit,

    /// Caching is disabled.
    /// All fetches go to network (or fail in offline mode).
    Disabled,
}

/// Cache configuration.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Cache mode.
    pub mode: CacheMode,

    /// Cache directory path (when mode is Explicit).
    pub path: Option<PathBuf>,

    /// Whether the cache is read-only.
    /// In read-only mode, cache hits are used but misses are not written.
    pub read_only: bool,

    /// Whether to verify checksums on cache reads.
    pub verify_on_read: bool,

    /// Maximum cache size in bytes (0 = unlimited).
    pub max_size: u64,

    /// Cache entry TTL in seconds (0 = no expiry).
    /// **Warning**: Using TTL breaks hermeticity.
    pub ttl_seconds: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            mode: CacheMode::Auto,
            path: None,
            read_only: false,
            verify_on_read: true,
            max_size: 0,
            ttl_seconds: 0,
        }
    }
}

impl CacheConfig {
    /// Creates a disabled cache configuration.
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            mode: CacheMode::Disabled,
            path: None,
            read_only: false,
            verify_on_read: false,
            max_size: 0,
            ttl_seconds: 0,
        }
    }

    /// Creates a read-only cache at the specified path.
    ///
    /// This is the recommended mode for hermetic builds:
    /// - Cache hits are used
    /// - Cache misses fail (in offline mode) or fetch without writing
    #[must_use]
    pub fn read_only(path: impl AsRef<Path>) -> Self {
        Self {
            mode: CacheMode::Explicit,
            path: Some(path.as_ref().to_path_buf()),
            read_only: true,
            verify_on_read: true,
            max_size: 0,
            ttl_seconds: 0,
        }
    }

    /// Creates a read-write cache at the specified path.
    #[must_use]
    pub fn read_write(path: impl AsRef<Path>) -> Self {
        Self {
            mode: CacheMode::Explicit,
            path: Some(path.as_ref().to_path_buf()),
            read_only: false,
            verify_on_read: true,
            max_size: 0,
            ttl_seconds: 0,
        }
    }

    /// Creates a cache using XDG/platform conventions.
    ///
    /// - Unix: `$XDG_CACHE_HOME/antler` or `~/.cache/antler`
    /// - macOS: `~/Library/Caches/antler`
    /// - Windows: `%LOCALAPPDATA%\antler\cache`
    ///
    /// **Warning**: This reads from environment, not hermetic.
    #[must_use]
    pub const fn platform_default() -> Self {
        Self {
            mode: CacheMode::Auto,
            path: None, // Resolved at runtime
            read_only: false,
            verify_on_read: true,
            max_size: 0,
            ttl_seconds: 0,
        }
    }

    /// Returns the effective cache path.
    ///
    /// For `Auto` mode, this resolves the platform-specific default.
    /// For `Explicit` mode, returns the configured path.
    /// For `Disabled` mode, returns `None`.
    #[must_use]
    pub fn effective_path(&self) -> Option<PathBuf> {
        match self.mode {
            CacheMode::Disabled => None,
            CacheMode::Explicit => self.path.clone(),
            CacheMode::Auto => Self::resolve_auto_path(),
        }
    }

    /// Resolves the automatic cache path based on platform conventions.
    fn resolve_auto_path() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            dirs::home_dir().map(|h| h.join("Library/Caches/antler"))
        }

        #[cfg(target_os = "windows")]
        {
            dirs::cache_dir().map(|c| c.join("antler"))
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            std::env::var("XDG_CACHE_HOME")
                .ok()
                .map(PathBuf::from)
                .or_else(|| dirs::home_dir().map(|h| h.join(".cache")))
                .map(|p| p.join("antler"))
        }
    }

    /// Sets the maximum cache size in bytes.
    #[must_use]
    pub const fn with_max_size(mut self, bytes: u64) -> Self {
        self.max_size = bytes;
        self
    }

    /// Enables or disables checksum verification on cache reads.
    #[must_use]
    pub const fn with_verify_on_read(mut self, verify: bool) -> Self {
        self.verify_on_read = verify;
        self
    }

    /// Returns whether caching is enabled.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        !matches!(self.mode, CacheMode::Disabled)
    }

    /// Returns whether the cache is hermetic (explicit path, no TTL).
    #[must_use]
    pub const fn is_hermetic(&self) -> bool {
        matches!(self.mode, CacheMode::Explicit | CacheMode::Disabled) && self.ttl_seconds == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled_cache() {
        let config = CacheConfig::disabled();
        assert_eq!(config.mode, CacheMode::Disabled);
        assert!(!config.is_enabled());
        assert!(config.is_hermetic());
    }

    #[test]
    fn test_read_only_cache() {
        let config = CacheConfig::read_only("/tmp/cache");
        assert_eq!(config.mode, CacheMode::Explicit);
        assert!(config.read_only);
        assert!(config.is_enabled());
        assert!(config.is_hermetic());
    }

    #[test]
    fn test_auto_cache_not_hermetic() {
        let config = CacheConfig::platform_default();
        assert_eq!(config.mode, CacheMode::Auto);
        assert!(!config.is_hermetic());
    }

    #[test]
    fn test_effective_path_explicit() {
        let config = CacheConfig::read_write("/custom/cache");
        assert_eq!(
            config.effective_path(),
            Some(PathBuf::from("/custom/cache"))
        );
    }

    #[test]
    fn test_effective_path_disabled() {
        let config = CacheConfig::disabled();
        assert_eq!(config.effective_path(), None);
    }
}
