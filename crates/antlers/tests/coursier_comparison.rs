//! Integration tests comparing antler resolution with Coursier.
//!
//! These tests verify that antler produces the same resolution as Coursier,
//! which is the reference implementation for JVM dependency resolution.
//!
//! Run with: `cargo test --test coursier_comparison -- --ignored`
//! (These are ignored by default as they require network access and coursier)

use std::collections::HashSet;
use std::process::Command;

/// Parse coursier resolve output into a set of artifact coordinates.
fn parse_coursier_output(output: &str) -> HashSet<String> {
    output
        .lines()
        .filter(|line| !line.starts_with("Downloading") && !line.starts_with("Downloaded"))
        .filter(|line| line.contains(':'))
        .map(|line| {
            // Coursier format: group:artifact:version:config
            // We want: group:artifact:version
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                format!("{}:{}:{}", parts[0], parts[1], parts[2])
            } else {
                line.to_string()
            }
        })
        .collect()
}

/// Run coursier resolve and return the set of resolved artifacts.
fn coursier_resolve(coordinate: &str) -> HashSet<String> {
    let output = Command::new("cs")
        .args(["resolve", coordinate])
        .output()
        .expect("Failed to execute coursier. Is it installed?");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Coursier writes download progress to stderr and results to stdout
    let combined = format!("{stderr}\n{stdout}");
    parse_coursier_output(&combined)
}

/// Run antler resolve and return the set of resolved artifacts.
async fn antler_resolve(coordinate: &str) -> HashSet<String> {
    use antlers::{Antlers, Artifact};

    let artifact = Artifact::parse(coordinate).expect("Invalid coordinate");
    let resolution = Antlers::with_defaults()
        .resolve(&artifact)
        .await
        .expect("Resolution failed");

    resolution
        .artifacts()
        .iter()
        .map(|a| a.artifact.coordinate())
        .collect()
}

/// Compare two resolution sets and report differences.
fn compare_resolutions(
    expected: &HashSet<String>,
    actual: &HashSet<String>,
    label: &str,
) -> Vec<String> {
    let mut errors = Vec::new();

    let missing: Vec<_> = expected.difference(actual).collect();
    let extra: Vec<_> = actual.difference(expected).collect();

    if !missing.is_empty() {
        errors.push(format!("{label}: Missing artifacts: {missing:?}"));
    }

    if !extra.is_empty() {
        errors.push(format!("{label}: Extra artifacts: {extra:?}"));
    }

    // Check for version mismatches
    let expected_ga: std::collections::HashMap<_, _> = expected
        .iter()
        .filter_map(|coord| {
            let parts: Vec<_> = coord.split(':').collect();
            if parts.len() >= 3 {
                Some((format!("{}:{}", parts[0], parts[1]), parts[2].to_string()))
            } else {
                None
            }
        })
        .collect();

    let actual_ga: std::collections::HashMap<_, _> = actual
        .iter()
        .filter_map(|coord| {
            let parts: Vec<_> = coord.split(':').collect();
            if parts.len() >= 3 {
                Some((format!("{}:{}", parts[0], parts[1]), parts[2].to_string()))
            } else {
                None
            }
        })
        .collect();

    for (ga, expected_version) in &expected_ga {
        if let Some(actual_version) = actual_ga.get(ga)
            && expected_version != actual_version
        {
            errors.push(format!(
                "{label}: Version mismatch for {ga}: expected {expected_version}, got {actual_version}"
            ));
        }
    }

    errors
}

// ============================================================================
// Test Cases
// ============================================================================

#[tokio::test]
#[ignore = "requires network and coursier"]
async fn test_simple_artifact_no_deps() {
    // slf4j-api has no dependencies
    let coordinate = "org.slf4j:slf4j-api:2.0.9";

    let coursier = coursier_resolve(coordinate);
    let antler = antler_resolve(coordinate).await;

    let errors = compare_resolutions(&coursier, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}

#[tokio::test]
#[ignore = "requires network and coursier"]
async fn test_okhttp_transitive_deps() {
    // OkHttp has multiple levels of transitive dependencies
    // This is the case that currently fails
    let coordinate = "com.squareup.okhttp3:okhttp:4.12.0";

    let coursier = coursier_resolve(coordinate);
    let antler = antler_resolve(coordinate).await;

    println!("Coursier resolved {} artifacts:", coursier.len());
    for artifact in &coursier {
        println!("  {artifact}");
    }

    println!("\nAntler resolved {} artifacts:", antler.len());
    for artifact in &antler {
        println!("  {artifact}");
    }

    let errors = compare_resolutions(&coursier, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}

#[tokio::test]
#[ignore = "requires network and coursier"]
async fn test_kotlin_stdlib_transitive() {
    // Kotlin stdlib has transitive deps including kotlin-stdlib-common
    let coordinate = "org.jetbrains.kotlin:kotlin-stdlib:1.9.10";

    let coursier = coursier_resolve(coordinate);
    let antler = antler_resolve(coordinate).await;

    let errors = compare_resolutions(&coursier, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}

#[tokio::test]
#[ignore = "requires network and coursier"]
async fn test_guava_complex_deps() {
    // Guava has many dependencies with version conflicts
    let coordinate = "com.google.guava:guava:32.1.3-jre";

    let coursier = coursier_resolve(coordinate);
    let antler = antler_resolve(coordinate).await;

    let errors = compare_resolutions(&coursier, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}

#[tokio::test]
#[ignore = "requires network and coursier"]
async fn test_jackson_databind() {
    // Jackson has multiple modules
    let coordinate = "com.fasterxml.jackson.core:jackson-databind:2.15.3";

    let coursier = coursier_resolve(coordinate);
    let antler = antler_resolve(coordinate).await;

    let errors = compare_resolutions(&coursier, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}

#[tokio::test]
#[ignore = "requires network and coursier"]
async fn test_spring_boot_starter() {
    // Spring Boot has deep dependency trees with BOMs
    let coordinate = "org.springframework.boot:spring-boot-starter:3.2.0";

    let coursier = coursier_resolve(coordinate);
    let antler = antler_resolve(coordinate).await;

    let errors = compare_resolutions(&coursier, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}

// ============================================================================
// Version Conflict Resolution Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires network and coursier"]
async fn test_version_conflict_highest_wins() {
    // OkHttp requests kotlin-stdlib-jdk8:1.8.21, but okio-jvm requests 1.9.10
    // Coursier uses highest-wins, so 1.9.10 should be selected
    let coordinate = "com.squareup.okhttp3:okhttp:4.12.0";

    let coursier = coursier_resolve(coordinate);
    let antler = antler_resolve(coordinate).await;

    // Specifically check kotlin-stdlib-jdk8 version
    let coursier_kotlin: Vec<_> = coursier
        .iter()
        .filter(|c| c.contains("kotlin-stdlib-jdk8"))
        .collect();
    let antler_kotlin: Vec<_> = antler
        .iter()
        .filter(|c| c.contains("kotlin-stdlib-jdk8"))
        .collect();

    assert_eq!(
        coursier_kotlin.len(),
        1,
        "Expected exactly one kotlin-stdlib-jdk8 in coursier"
    );
    assert_eq!(
        antler_kotlin.len(),
        1,
        "Expected exactly one kotlin-stdlib-jdk8 in antler"
    );

    // Coursier should have selected 1.9.10 (highest wins)
    assert!(
        coursier_kotlin[0].contains("1.9.10"),
        "Coursier should select kotlin-stdlib-jdk8:1.9.10, got: {}",
        coursier_kotlin[0]
    );

    // Antler should also select 1.9.10 (when using highest-wins strategy)
    assert!(
        antler_kotlin[0].contains("1.9.10"),
        "Antler should select kotlin-stdlib-jdk8:1.9.10, got: {}",
        antler_kotlin[0]
    );
}

// ============================================================================
// Multiplatform Artifact Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires network and coursier"]
async fn test_okio_multiplatform() {
    // Okio is a Kotlin Multiplatform library
    // The POM for okio:3.6.0 redirects to okio-jvm:3.6.0
    let coordinate = "com.squareup.okio:okio:3.6.0";

    let coursier = coursier_resolve(coordinate);
    let antler = antler_resolve(coordinate).await;

    // Should include okio-jvm
    assert!(
        coursier.iter().any(|c| c.contains("okio-jvm")),
        "Coursier should include okio-jvm"
    );
    assert!(
        antler.iter().any(|c| c.contains("okio-jvm")),
        "Antler should include okio-jvm"
    );

    let errors = compare_resolutions(&coursier, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}
