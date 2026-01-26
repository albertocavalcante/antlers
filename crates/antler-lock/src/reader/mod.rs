//! Lockfile readers for various formats.
//!
//! This module provides readers that can parse different lockfile formats:
//!
//! - [`detect`] - Auto-detect lockfile format
//! - [`v1`] - `rules_jvm_external` V1 format (legacy)
//! - [`v2`] - `rules_jvm_external` V2 format
//!
//! The main entry point is [`read`], which auto-detects the format and
//! converts to our native format.

pub mod detect;
pub mod v1;
pub mod v2;

use serde_json::Value;

use crate::Lockfile;
use crate::error::{Error, Result};

pub use detect::{LockfileFormat, detect_format};

/// Reads a lockfile from a string, auto-detecting the format.
///
/// This function:
/// 1. Parses the JSON
/// 2. Detects the format
/// 3. Converts to our native `Lockfile` structure
///
/// # Errors
///
/// Returns an error if:
/// - The JSON is invalid
/// - The format cannot be detected
/// - The lockfile is malformed
pub fn read(content: &str) -> Result<Lockfile> {
    // First, parse as generic JSON to detect format
    let json: Value = serde_json::from_str(content)?;

    let format = detect_format(&json)
        .ok_or_else(|| Error::UnknownFormat("could not detect lockfile format".to_string()))?;

    match format {
        LockfileFormat::Antler => read_antler(content),
        LockfileFormat::RulesJvmExternalV2 => v2::read_v2(content),
        LockfileFormat::RulesJvmExternalV1 => v1::read_v1(content),
    }
}

/// Reads our native antler-lock format.
fn read_antler(content: &str) -> Result<Lockfile> {
    let lockfile: Lockfile = serde_json::from_str(content)?;
    Ok(lockfile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_antler_format() {
        let content = r#"{
            "version": "1",
            "format": "antler-lock",
            "artifacts": {
                "com.example:lib": {
                    "version": "1.0.0",
                    "sha256": "abc123"
                }
            },
            "repositories": [],
            "metadata": {
                "generated_by": "test",
                "generated_at": "2024-01-01T00:00:00Z"
            }
        }"#;

        let lockfile = read(content).unwrap();
        assert_eq!(lockfile.format, "antler-lock");
        assert_eq!(lockfile.artifacts.len(), 1);
    }

    #[test]
    fn test_read_v2_format() {
        let content = r#"{
            "version": "2",
            "artifacts": {
                "com.example:lib": {
                    "version": "1.0.0",
                    "shasums": { "sha256": "abc123" }
                }
            },
            "dependencies": {},
            "repositories": {}
        }"#;

        let lockfile = read(content).unwrap();
        assert_eq!(lockfile.format, "antler-lock");
        assert_eq!(lockfile.artifacts.len(), 1);
    }

    #[test]
    fn test_read_v1_format() {
        let content = r#"{
            "dependency_tree": {
                "version": "0.1.0",
                "dependencies": [
                    {
                        "coord": "com.example:lib:1.0.0",
                        "sha256": "abc123",
                        "directDependencies": []
                    }
                ]
            }
        }"#;

        let lockfile = read(content).unwrap();
        assert_eq!(lockfile.format, "antler-lock");
        assert_eq!(lockfile.artifacts.len(), 1);
    }

    #[test]
    fn test_read_unknown_format() {
        let content = r#"{ "unknown": "format" }"#;
        let result = read(content);
        assert!(result.is_err());
    }
}
