//! Caching abstractions for artifact fetching.
//!
//! This module provides a caching layer to avoid redundant downloads.

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// A cache entry containing artifact data.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// The cached data.
    pub data: Vec<u8>,
    /// Optional checksum of the data.
    pub checksum: Option<String>,
    /// Unix timestamp when the entry was cached.
    pub timestamp: u64,
}

impl CacheEntry {
    /// Creates a new cache entry with the current timestamp.
    #[must_use]
    pub fn new(data: Vec<u8>, checksum: Option<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            data,
            checksum,
            timestamp,
        }
    }
}

/// Cache trait for storing and retrieving artifacts.
pub trait Cache: Send + Sync {
    /// Gets an entry from the cache.
    fn get(&self, key: &str) -> Option<CacheEntry>;

    /// Puts an entry into the cache.
    fn put(&self, key: &str, entry: CacheEntry);

    /// Checks if an entry exists in the cache.
    fn contains(&self, key: &str) -> bool;

    /// Removes an entry from the cache.
    fn remove(&self, key: &str);

    /// Clears all entries from the cache.
    fn clear(&self);
}

/// In-memory cache implementation.
#[derive(Debug, Default)]
pub struct MemoryCache {
    entries: RwLock<HashMap<String, CacheEntry>>,
}

impl MemoryCache {
    /// Creates a new empty memory cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Cache for MemoryCache {
    fn get(&self, key: &str) -> Option<CacheEntry> {
        let entries = self.entries.read().ok()?;
        entries.get(key).cloned()
    }

    fn put(&self, key: &str, entry: CacheEntry) {
        if let Ok(mut entries) = self.entries.write() {
            entries.insert(key.to_string(), entry);
        }
    }

    fn contains(&self, key: &str) -> bool {
        self.entries
            .read()
            .map(|entries| entries.contains_key(key))
            .unwrap_or(false)
    }

    fn remove(&self, key: &str) {
        if let Ok(mut entries) = self.entries.write() {
            entries.remove(key);
        }
    }

    fn clear(&self) {
        if let Ok(mut entries) = self.entries.write() {
            entries.clear();
        }
    }
}

/// File-system based cache implementation.
///
/// Stores cached artifacts on disk for persistence across sessions.
pub struct FileCache {
    root: PathBuf,
}

impl FileCache {
    /// Creates a new file cache at the specified root directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created.
    pub fn new(root: impl AsRef<Path>) -> std::io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// Creates a new file cache at the default location.
    ///
    /// On Unix systems, this is typically `~/.cache/jvm-fetch`.
    /// On Windows, this is `%LOCALAPPDATA%\jvm-fetch\cache`.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created or if the home directory
    /// cannot be determined.
    pub fn with_default_location() -> std::io::Result<Self> {
        let cache_dir = dirs_path();
        Self::new(cache_dir)
    }

    /// Converts a cache key to a file path.
    fn key_to_path(&self, key: &str) -> PathBuf {
        // Replace characters that might be problematic in file paths
        let safe_key = key.replace([':', '/', '\\'], "_").replace('@', "_at_");
        self.root.join(safe_key)
    }

    /// Converts a cache key to a metadata file path.
    fn key_to_meta_path(&self, key: &str) -> PathBuf {
        let mut path = self.key_to_path(key);
        let file_name = path.file_name().map_or_else(
            || "unknown.meta".to_string(),
            |n| format!("{}.meta", n.to_string_lossy()),
        );
        path.set_file_name(file_name);
        path
    }
}

/// Returns the default cache directory path.
fn dirs_path() -> PathBuf {
    // Try to use platform-specific cache directories
    if let Some(cache_dir) = dirs::cache_dir() {
        return cache_dir.join("jvm-fetch");
    }

    // Fallback to home directory
    if let Some(home) = dirs::home_dir() {
        return home.join(".cache").join("jvm-fetch");
    }

    // Last resort: current directory
    PathBuf::from(".jvm-fetch-cache")
}

// We need the `dirs` crate, but since it's not in the dependencies,
// let's implement a simple fallback
mod dirs {
    use std::path::PathBuf;

    pub fn cache_dir() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            home_dir().map(|h| h.join("Library/Caches"))
        }
        #[cfg(target_os = "linux")]
        {
            std::env::var("XDG_CACHE_HOME")
                .ok()
                .map(PathBuf::from)
                .or_else(|| home_dir().map(|h| h.join(".cache")))
        }
        #[cfg(target_os = "windows")]
        {
            std::env::var("LOCALAPPDATA").ok().map(PathBuf::from)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            home_dir().map(|h| h.join(".cache"))
        }
    }

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var("HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
    }
}

impl Cache for FileCache {
    fn get(&self, key: &str) -> Option<CacheEntry> {
        let path = self.key_to_path(key);
        let meta_path = self.key_to_meta_path(key);

        // Read data
        let mut data = Vec::new();
        let mut file = fs::File::open(&path).ok()?;
        file.read_to_end(&mut data).ok()?;

        // Read metadata (optional)
        let (checksum, timestamp) = if let Ok(mut meta_file) = fs::File::open(&meta_path) {
            let mut meta_content = String::new();
            meta_file.read_to_string(&mut meta_content).ok()?;

            let mut checksum = None;
            let mut timestamp = 0u64;

            for line in meta_content.lines() {
                if let Some(value) = line.strip_prefix("checksum=") {
                    checksum = Some(value.to_string());
                } else if let Some(value) = line.strip_prefix("timestamp=") {
                    timestamp = value.parse().unwrap_or(0);
                }
            }

            (checksum, timestamp)
        } else {
            // If no metadata, use file modification time
            let metadata = fs::metadata(&path).ok()?;
            let timestamp = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs());
            (None, timestamp)
        };

        Some(CacheEntry {
            data,
            checksum,
            timestamp,
        })
    }

    fn put(&self, key: &str, entry: CacheEntry) {
        let path = self.key_to_path(key);
        let meta_path = self.key_to_meta_path(key);

        // Write data
        if let Ok(mut file) = fs::File::create(&path) {
            let _ = file.write_all(&entry.data);
        }

        // Write metadata
        if let Ok(mut meta_file) = fs::File::create(&meta_path) {
            use std::fmt::Write as _;
            let mut meta_content = format!("timestamp={}\n", entry.timestamp);
            if let Some(ref checksum) = entry.checksum {
                let _ = writeln!(meta_content, "checksum={checksum}");
            }
            let _ = meta_file.write_all(meta_content.as_bytes());
        }
    }

    fn contains(&self, key: &str) -> bool {
        self.key_to_path(key).exists()
    }

    fn remove(&self, key: &str) {
        let path = self.key_to_path(key);
        let meta_path = self.key_to_meta_path(key);
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(meta_path);
    }

    fn clear(&self) {
        if let Ok(entries) = fs::read_dir(&self.root) {
            for entry in entries.flatten() {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

impl std::fmt::Debug for FileCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileCache")
            .field("root", &self.root)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_cache() {
        let cache = MemoryCache::new();

        assert!(!cache.contains("test"));

        let entry = CacheEntry::new(b"hello".to_vec(), Some("abc123".to_string()));
        cache.put("test", entry);

        assert!(cache.contains("test"));

        let retrieved = cache.get("test").unwrap();
        assert_eq!(retrieved.data, b"hello");
        assert_eq!(retrieved.checksum, Some("abc123".to_string()));

        cache.remove("test");
        assert!(!cache.contains("test"));
    }

    #[test]
    fn test_cache_entry_new() {
        let entry = CacheEntry::new(b"data".to_vec(), None);
        assert_eq!(entry.data, b"data");
        assert!(entry.checksum.is_none());
        assert!(entry.timestamp > 0);
    }
}
