//! Checksum computation and verification.
//!
//! This module provides utilities for computing and verifying checksums
//! using various algorithms commonly used in Maven repositories.

use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};

/// Supported checksum algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChecksumAlgo {
    /// SHA-1 (160-bit).
    Sha1,
    /// SHA-256 (256-bit).
    Sha256,
    /// SHA-512 (512-bit).
    Sha512,
    /// MD5 (128-bit, deprecated but still used by some repositories).
    Md5,
}

impl ChecksumAlgo {
    /// Returns the file extension for this algorithm.
    #[must_use]
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Sha1 => "sha1",
            Self::Sha256 => "sha256",
            Self::Sha512 => "sha512",
            Self::Md5 => "md5",
        }
    }

    /// Returns all supported algorithms in order of preference.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::Sha256, Self::Sha512, Self::Sha1, Self::Md5]
    }

    /// Returns the recommended algorithms (excludes MD5).
    #[must_use]
    pub const fn recommended() -> &'static [Self] {
        &[Self::Sha256, Self::Sha512, Self::Sha1]
    }
}

impl std::fmt::Display for ChecksumAlgo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.extension())
    }
}

/// A checksum value with its algorithm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checksum {
    /// The algorithm used.
    pub algo: ChecksumAlgo,
    /// The hex-encoded checksum value.
    pub value: String,
}

impl Checksum {
    /// Creates a new checksum.
    #[must_use]
    pub fn new(algo: ChecksumAlgo, value: impl Into<String>) -> Self {
        Self {
            algo,
            value: value.into(),
        }
    }
}

impl std::fmt::Display for Checksum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.algo, self.value)
    }
}

/// Utility for computing and verifying checksums.
pub struct ChecksumVerifier;

impl ChecksumVerifier {
    /// Computes the checksum of the given data.
    #[must_use]
    pub fn compute(data: &[u8], algo: ChecksumAlgo) -> String {
        match algo {
            ChecksumAlgo::Sha1 => {
                let mut hasher = Sha1::new();
                hasher.update(data);
                hex::encode(hasher.finalize())
            }
            ChecksumAlgo::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(data);
                hex::encode(hasher.finalize())
            }
            ChecksumAlgo::Sha512 => {
                let mut hasher = Sha512::new();
                hasher.update(data);
                hex::encode(hasher.finalize())
            }
            ChecksumAlgo::Md5 => {
                // MD5 is deprecated but still supported for compatibility
                let digest = md5::compute(data);
                hex::encode(digest.0)
            }
        }
    }

    /// Verifies that the data matches the expected checksum.
    #[must_use]
    pub fn verify(data: &[u8], expected: &Checksum) -> bool {
        let actual = Self::compute(data, expected.algo);
        // Normalize both values to lowercase for comparison
        actual.to_lowercase() == expected.value.to_lowercase()
    }

    /// Computes a checksum and returns it as a `Checksum` struct.
    #[must_use]
    pub fn compute_checksum(data: &[u8], algo: ChecksumAlgo) -> Checksum {
        Checksum::new(algo, Self::compute(data, algo))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_sha1() {
        let data = b"hello world";
        let hash = ChecksumVerifier::compute(data, ChecksumAlgo::Sha1);
        assert_eq!(hash, "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed");
    }

    #[test]
    fn test_compute_sha256() {
        let data = b"hello world";
        let hash = ChecksumVerifier::compute(data, ChecksumAlgo::Sha256);
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_verify() {
        let data = b"hello world";
        let checksum = Checksum::new(
            ChecksumAlgo::Sha256,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
        );
        assert!(ChecksumVerifier::verify(data, &checksum));
    }

    #[test]
    fn test_verify_case_insensitive() {
        let data = b"hello world";
        let checksum = Checksum::new(
            ChecksumAlgo::Sha256,
            "B94D27B9934D3E08A52E52D7DA7DABFAC484EFE37A5380EE9088F7ACE2EFCDE9",
        );
        assert!(ChecksumVerifier::verify(data, &checksum));
    }

    #[test]
    fn test_algo_extension() {
        assert_eq!(ChecksumAlgo::Sha1.extension(), "sha1");
        assert_eq!(ChecksumAlgo::Sha256.extension(), "sha256");
        assert_eq!(ChecksumAlgo::Sha512.extension(), "sha512");
        assert_eq!(ChecksumAlgo::Md5.extension(), "md5");
    }
}
