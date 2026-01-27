//! Lockfile format detection.
//!
//! This module provides automatic detection of lockfile formats
//! based on their JSON structure.
//!
//! # Deprecation Notice
//!
//! The [`LockfileFormat`] enum in this module is deprecated.
//! Use [`crate::LockFormat`] instead, which provides a unified enum
//! for all supported lockfile and output formats.

use serde_json::Value;

use crate::LockFormat;

/// Detected lockfile format.
///
/// # Deprecation
///
/// This enum is deprecated. Use [`LockFormat`] instead, which provides
/// a more comprehensive set of formats and additional methods.
///
/// # Migration
///
/// ```ignore
/// // Old code:
/// use antlers_lock::LockfileFormat;
/// let format = detect_format(&json);
/// if format == Some(LockfileFormat::Antler) { ... }
///
/// // New code:
/// use antlers_lock::LockFormat;
/// let format = LockFormat::detect(content);
/// if format == Some(LockFormat::Antlers) { ... }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[deprecated(since = "0.2.0", note = "Use LockFormat instead")]
pub enum LockfileFormat {
    /// Our native antlers-lock format.
    Antler,
    /// `rules_jvm_external` V2 format.
    RulesJvmExternalV2,
    /// `rules_jvm_external` V1 format (legacy).
    RulesJvmExternalV1,
}

#[allow(deprecated)]
impl LockfileFormat {
    /// Returns the format name for display.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Antler => "antlers-lock",
            Self::RulesJvmExternalV2 => "rules_jvm_external-v2",
            Self::RulesJvmExternalV1 => "rules_jvm_external-v1",
        }
    }

    /// Converts this deprecated format to the new [`LockFormat`].
    #[must_use]
    pub const fn to_lock_format(&self) -> LockFormat {
        match self {
            Self::Antler => LockFormat::Antlers,
            Self::RulesJvmExternalV2 => LockFormat::RulesJvmV2,
            Self::RulesJvmExternalV1 => LockFormat::RulesJvmV1,
        }
    }
}

#[allow(deprecated)]
impl From<LockfileFormat> for LockFormat {
    fn from(format: LockfileFormat) -> Self {
        format.to_lock_format()
    }
}

/// Detects the lockfile format from parsed JSON.
///
/// # Deprecation
///
/// This function is deprecated. Use [`LockFormat::detect`] instead,
/// which can detect from raw content strings.
///
/// Detection logic:
/// 1. If `format == "antlers-lock"` → Antler format
/// 2. If `version == "2"` and has `artifacts` map → `rules_jvm_external` V2
/// 3. If `dependency_tree.version == "0.1.0"` → `rules_jvm_external` V1
/// 4. Otherwise → Unknown
#[deprecated(since = "0.2.0", note = "Use LockFormat::detect instead")]
#[allow(deprecated)]
pub fn detect_format(json: &Value) -> Option<LockfileFormat> {
    // Check for antlers-lock format
    if json.get("format").and_then(Value::as_str) == Some("antlers-lock") {
        return Some(LockfileFormat::Antler);
    }

    // Check for rules_jvm_external V2
    if json.get("version").and_then(Value::as_str) == Some("2") && json.get("artifacts").is_some() {
        return Some(LockfileFormat::RulesJvmExternalV2);
    }

    // Check for rules_jvm_external V1 (has dependency_tree with version 0.1.0)
    if json
        .get("dependency_tree")
        .and_then(|dt| dt.get("version"))
        .and_then(Value::as_str)
        == Some("0.1.0")
    {
        return Some(LockfileFormat::RulesJvmExternalV1);
    }

    None
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_detect_antler_format() {
        let json = json!({
            "version": "1",
            "format": "antlers-lock",
            "artifacts": {}
        });

        assert_eq!(detect_format(&json), Some(LockfileFormat::Antler));
    }

    #[test]
    fn test_detect_v2_format() {
        let json = json!({
            "version": "2",
            "artifacts": {
                "com.example:lib": {}
            }
        });

        assert_eq!(
            detect_format(&json),
            Some(LockfileFormat::RulesJvmExternalV2)
        );
    }

    #[test]
    fn test_detect_v1_format() {
        let json = json!({
            "dependency_tree": {
                "version": "0.1.0",
                "dependencies": []
            }
        });

        assert_eq!(
            detect_format(&json),
            Some(LockfileFormat::RulesJvmExternalV1)
        );
    }

    #[test]
    fn test_detect_unknown_format() {
        let json = json!({
            "unknown": "format"
        });

        assert_eq!(detect_format(&json), None);
    }

    #[test]
    fn test_format_names() {
        assert_eq!(LockfileFormat::Antler.name(), "antlers-lock");
        assert_eq!(
            LockfileFormat::RulesJvmExternalV2.name(),
            "rules_jvm_external-v2"
        );
        assert_eq!(
            LockfileFormat::RulesJvmExternalV1.name(),
            "rules_jvm_external-v1"
        );
    }
}
