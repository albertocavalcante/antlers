//! Centralized constants for antlers.
//!
//! This module provides shared limits and timeouts used across the crate.

use std::time::Duration;

/// Limits for dependency resolution.
pub mod limits {
    /// Maximum depth for parent POM resolution.
    pub const MAX_PARENT_DEPTH: usize = 10;

    /// Maximum depth for transitive dependency resolution.
    pub const MAX_RESOLUTION_DEPTH: usize = 50;

    /// Maximum concurrent HTTP fetches for POM/metadata files.
    pub const MAX_CONCURRENT_FETCHES: usize = 16;

    /// Maximum concurrent checksum fetches.
    pub const MAX_CONCURRENT_CHECKSUMS: usize = 32;
}

/// Network timeout configuration.
pub mod timeouts {
    use super::Duration;

    /// Default connection timeout.
    pub const CONNECT: Duration = Duration::from_secs(30);

    /// Default read timeout.
    pub const READ: Duration = Duration::from_secs(60);
}
