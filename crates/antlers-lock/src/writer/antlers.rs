//! Writer for our native antlers-lock format.

use crate::Lockfile;
use crate::error::Result;

/// Writes a lockfile to our native antlers-lock format.
///
/// The output is pretty-printed JSON with 2-space indentation.
pub fn write_antlers(lockfile: &Lockfile) -> Result<String> {
    let json = serde_json::to_string_pretty(lockfile)?;
    Ok(json)
}

/// Writes a lockfile to compact JSON (no extra whitespace).
pub fn write_antlers_compact(lockfile: &Lockfile) -> Result<String> {
    let json = serde_json::to_string(lockfile)?;
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LockedArtifact, LockfileMetadata};
    use indexmap::IndexMap;

    #[test]
    fn test_write_antlers() {
        let mut artifacts = IndexMap::new();
        artifacts.insert(
            "com.example:lib".to_string(),
            LockedArtifact::new("1.0.0", "abc123"),
        );

        let lockfile = Lockfile {
            version: "1".to_string(),
            format: "antlers-lock".to_string(),
            artifacts,
            repositories: vec![],
            conflicts: vec![],
            metadata: LockfileMetadata::with_generator("test"),
            extensions: IndexMap::new(),
        };

        let json = write_antlers(&lockfile).unwrap();

        assert!(json.contains("\"version\": \"1\""));
        assert!(json.contains("\"format\": \"antlers-lock\""));
        assert!(json.contains("\"com.example:lib\""));
        assert!(json.contains("\"sha256\": \"abc123\""));
    }

    #[test]
    fn test_write_antlers_compact() {
        let mut artifacts = IndexMap::new();
        artifacts.insert(
            "com.example:lib".to_string(),
            LockedArtifact::new("1.0.0", "abc123"),
        );

        let lockfile = Lockfile {
            version: "1".to_string(),
            format: "antlers-lock".to_string(),
            artifacts,
            repositories: vec![],
            conflicts: vec![],
            metadata: LockfileMetadata::with_generator("test"),
            extensions: IndexMap::new(),
        };

        let json = write_antlers_compact(&lockfile).unwrap();

        // Compact format should not have newlines
        assert!(!json.contains('\n'));
        assert!(json.contains("\"version\":\"1\""));
    }

    #[test]
    fn test_roundtrip() {
        let mut artifacts = IndexMap::new();
        artifacts.insert(
            "com.example:lib".to_string(),
            LockedArtifact::new("1.0.0", "abc123")
                .with_repository("https://repo1.maven.org/maven2/")
                .with_dependencies(vec!["com.example:dep".to_string()]),
        );

        let original = Lockfile {
            version: "1".to_string(),
            format: "antlers-lock".to_string(),
            artifacts,
            repositories: vec![],
            conflicts: vec![],
            metadata: LockfileMetadata::with_generator("test"),
            extensions: IndexMap::new(),
        };

        let json = write_antlers(&original).unwrap();
        let parsed: Lockfile = serde_json::from_str(&json).unwrap();

        assert_eq!(original.version, parsed.version);
        assert_eq!(original.format, parsed.format);
        assert_eq!(original.artifacts.len(), parsed.artifacts.len());

        let orig_lib = original.get("com.example:lib").unwrap();
        let parsed_lib = parsed.get("com.example:lib").unwrap();
        assert_eq!(orig_lib.version, parsed_lib.version);
        assert_eq!(orig_lib.sha256, parsed_lib.sha256);
        assert_eq!(orig_lib.repository, parsed_lib.repository);
        assert_eq!(orig_lib.dependencies, parsed_lib.dependencies);
    }
}
