//! Lockfile format detection.
//!
//! This module provides automatic detection of lockfile formats
//! based on their JSON structure.

use serde_json::Value;

/// Detected lockfile format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockfileFormat {
    /// Our native antler-lock format.
    Antler,
    /// `rules_jvm_external` V2 format.
    RulesJvmExternalV2,
    /// `rules_jvm_external` V1 format (legacy).
    RulesJvmExternalV1,
}

impl LockfileFormat {
    /// Returns the format name for display.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Antler => "antler-lock",
            Self::RulesJvmExternalV2 => "rules_jvm_external-v2",
            Self::RulesJvmExternalV1 => "rules_jvm_external-v1",
        }
    }
}

/// Detects the lockfile format from parsed JSON.
///
/// Detection logic:
/// 1. If `format == "antler-lock"` → Antler format
/// 2. If `version == "2"` and has `artifacts` map → `rules_jvm_external` V2
/// 3. If `dependency_tree.version == "0.1.0"` → `rules_jvm_external` V1
/// 4. Otherwise → Unknown
pub fn detect_format(json: &Value) -> Option<LockfileFormat> {
    // Check for antler-lock format
    if json.get("format").and_then(Value::as_str) == Some("antler-lock") {
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
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_detect_antler_format() {
        let json = json!({
            "version": "1",
            "format": "antler-lock",
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
        assert_eq!(LockfileFormat::Antler.name(), "antler-lock");
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
