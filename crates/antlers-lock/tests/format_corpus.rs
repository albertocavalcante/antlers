//! Corpus tests for validating lockfile format generation.
//!
//! These tests ensure that the Coursier and Gradle output formats are always
//! generated correctly by comparing against known-good expected outputs.

use std::path::PathBuf;

use antlers_lock::{LockFormat, Lockfile, writer};

/// Path to the fixtures directory.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

// =============================================================================
// Helper functions
// =============================================================================

/// Reads a fixture file.
fn read_fixture(path: &str) -> String {
    std::fs::read_to_string(fixtures_dir().join(path))
        .unwrap_or_else(|e| panic!("Failed to read fixture {path}: {e}"))
}

/// Parses a lockfile from a fixture.
fn load_lockfile(path: &str) -> Lockfile {
    let content = read_fixture(path);
    Lockfile::read(&content).unwrap_or_else(|e| panic!("Failed to parse lockfile {path}: {e}"))
}

/// Compares two JSON strings semantically.
fn assert_json_eq(actual: &str, expected: &str, context: &str) {
    let actual_val: serde_json::Value = serde_json::from_str(actual)
        .unwrap_or_else(|e| panic!("Failed to parse actual JSON for {context}: {e}"));
    let expected_val: serde_json::Value = serde_json::from_str(expected)
        .unwrap_or_else(|e| panic!("Failed to parse expected JSON for {context}: {e}"));

    assert_eq!(
        actual_val, expected_val,
        "JSON mismatch for {context}:\nActual:\n{actual}\n\nExpected:\n{expected}"
    );
}

/// Compares two TOML strings by normalizing whitespace.
fn assert_toml_eq(actual: &str, expected: &str, context: &str) {
    // Normalize by trimming each line and removing empty lines for comparison
    let normalize = |s: &str| -> Vec<String> {
        s.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(ToString::to_string)
            .collect()
    };

    let actual_lines = normalize(actual);
    let expected_lines = normalize(expected);

    assert_eq!(
        actual_lines, expected_lines,
        "TOML mismatch for {context}:\nActual:\n{actual}\n\nExpected:\n{expected}"
    );
}

// =============================================================================
// Coursier format tests
// =============================================================================

mod coursier {
    use super::*;

    const CACHE_PATH: &str = "/test/cache/coursier/v1/https/repo1.maven.org/maven2";

    #[test]
    fn test_simple() {
        let lockfile = load_lockfile("input/simple.json");
        let expected = read_fixture("expected/coursier/simple.json");
        let cache_path = std::path::PathBuf::from(CACHE_PATH);

        let actual = writer::write_coursier_with_cache_path(&lockfile, &cache_path)
            .expect("Failed to write coursier format");

        assert_json_eq(&actual, &expected, "coursier/simple");
    }

    #[test]
    fn test_transitive() {
        let lockfile = load_lockfile("input/transitive.json");
        let expected = read_fixture("expected/coursier/transitive.json");
        let cache_path = std::path::PathBuf::from(CACHE_PATH);

        let actual = writer::write_coursier_with_cache_path(&lockfile, &cache_path)
            .expect("Failed to write coursier format");

        assert_json_eq(&actual, &expected, "coursier/transitive");
    }

    #[test]
    fn test_multi_repo() {
        let lockfile = load_lockfile("input/multi-repo.json");
        let expected = read_fixture("expected/coursier/multi-repo.json");
        let cache_path = std::path::PathBuf::from(CACHE_PATH);

        let actual = writer::write_coursier_with_cache_path(&lockfile, &cache_path)
            .expect("Failed to write coursier format");

        assert_json_eq(&actual, &expected, "coursier/multi-repo");
    }
}

// =============================================================================
// Gradle version catalog format tests
// =============================================================================

mod gradle {
    use super::*;

    #[test]
    fn test_simple() {
        let lockfile = load_lockfile("input/simple.json");
        let expected = read_fixture("expected/gradle/simple.toml");

        let actual =
            writer::write_gradle_catalog(&lockfile).expect("Failed to write gradle catalog format");

        assert_toml_eq(&actual, &expected, "gradle/simple");
    }

    #[test]
    fn test_transitive() {
        let lockfile = load_lockfile("input/transitive.json");
        let expected = read_fixture("expected/gradle/transitive.toml");

        let actual =
            writer::write_gradle_catalog(&lockfile).expect("Failed to write gradle catalog format");

        assert_toml_eq(&actual, &expected, "gradle/transitive");
    }

    #[test]
    fn test_multi_repo() {
        let lockfile = load_lockfile("input/multi-repo.json");
        let expected = read_fixture("expected/gradle/multi-repo.toml");

        let actual =
            writer::write_gradle_catalog(&lockfile).expect("Failed to write gradle catalog format");

        assert_toml_eq(&actual, &expected, "gradle/multi-repo");
    }
}

// =============================================================================
// Format detection tests
// =============================================================================

mod detect {
    use super::*;

    #[test]
    fn test_detect_antlers_format() {
        let content = read_fixture("input/simple.json");
        let format = LockFormat::detect(&content);
        assert_eq!(format, Some(LockFormat::Antlers));
    }

    #[test]
    fn test_detect_coursier_format() {
        let content = read_fixture("expected/coursier/simple.json");
        let format = LockFormat::detect(&content);
        assert_eq!(format, Some(LockFormat::Coursier));
    }

    #[test]
    fn test_detect_gradle_catalog_format() {
        let content = read_fixture("expected/gradle/simple.toml");
        let format = LockFormat::detect(&content);
        assert_eq!(format, Some(LockFormat::GradleCatalog));
    }

    #[test]
    fn test_detect_rules_jvm_v2() {
        let content = r#"{"version": "2", "artifacts": {"com.example:lib": {"sha256": "abc"}}}"#;
        let format = LockFormat::detect(content);
        assert_eq!(format, Some(LockFormat::RulesJvmV2));
    }

    #[test]
    fn test_detect_rules_jvm_v1() {
        let content = r#"{"dependency_tree": {"version": "0.1.0", "dependencies": []}}"#;
        let format = LockFormat::detect(content);
        assert_eq!(format, Some(LockFormat::RulesJvmV1));
    }

    #[test]
    fn test_detect_starlark() {
        let content = r#"
load("@rules_jvm_external//:defs.bzl", "maven_install")

def setup_deps():
    maven_install(
        artifacts = ["com.google.guava:guava:33.0.0-jre"],
    )
"#;
        let format = LockFormat::detect(content);
        assert_eq!(format, Some(LockFormat::Starlark));
    }

    #[test]
    fn test_detect_bazel_build() {
        let content = r#"
java_library(
    name = "app",
    deps = ["@maven//:com_google_guava_guava"],
)
"#;
        let format = LockFormat::detect(content);
        assert_eq!(format, Some(LockFormat::BazelBuild));
    }

    #[test]
    fn test_detect_unknown() {
        let content = "This is just some random text that is not a lockfile.";
        let format = LockFormat::detect(content);
        assert_eq!(format, None);
    }
}

// =============================================================================
// Roundtrip tests (input -> write -> detect)
// =============================================================================

mod roundtrip {
    use super::*;

    #[test]
    fn test_antlers_roundtrip() {
        let lockfile = load_lockfile("input/simple.json");
        let written = lockfile.write().expect("Failed to write");
        let parsed = Lockfile::read(&written).expect("Failed to parse");

        assert_eq!(lockfile.len(), parsed.len());
        assert_eq!(
            lockfile.get("com.google.guava:guava").unwrap().version,
            parsed.get("com.google.guava:guava").unwrap().version
        );
    }

    #[test]
    fn test_transitive_deps_preserved() {
        let lockfile = load_lockfile("input/transitive.json");
        let written = lockfile.write().expect("Failed to write");
        let parsed = Lockfile::read(&written).expect("Failed to parse");

        let kotlin = parsed.get("org.jetbrains.kotlin:kotlin-stdlib").unwrap();
        assert!(
            kotlin
                .dependencies
                .contains(&"org.jetbrains:annotations".to_string())
        );
    }

    #[test]
    fn test_conflicts_preserved() {
        let lockfile = load_lockfile("input/transitive.json");
        let written = lockfile.write().expect("Failed to write");
        let parsed = Lockfile::read(&written).expect("Failed to parse");

        assert_eq!(lockfile.conflicts.len(), parsed.conflicts.len());
        assert_eq!(lockfile.conflicts[0].artifact, parsed.conflicts[0].artifact);
    }
}
