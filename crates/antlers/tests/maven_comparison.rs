//! Integration tests comparing antler resolution with Maven.
//!
//! These tests verify that antler produces the same resolution as Maven,
//! which uses "nearest wins" conflict resolution by default.
//!
//! Run with: `cargo test --test maven_comparison -- --ignored`
//! (These are ignored by default as they require network access and mvn)

use std::collections::HashSet;
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

/// Create a minimal pom.xml for resolving a single artifact.
fn create_pom_xml(group_id: &str, artifact_id: &str, version: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>
    <groupId>test</groupId>
    <artifactId>test</artifactId>
    <version>1.0.0</version>
    <dependencies>
        <dependency>
            <groupId>{group_id}</groupId>
            <artifactId>{artifact_id}</artifactId>
            <version>{version}</version>
        </dependency>
    </dependencies>
</project>
"#
    )
}

/// Parse maven dependency:list output into a set of artifact coordinates.
fn parse_maven_output(output: &str) -> HashSet<String> {
    output
        .lines()
        .filter(|line| line.contains(":jar:") || line.contains(":pom:"))
        .filter_map(|line| {
            // Maven format: [INFO]    group:artifact:jar:version:scope -- module ...
            let trimmed = line.trim().trim_start_matches("[INFO]").trim();

            // Remove the "-- module ..." suffix if present
            let coord_part = trimmed.split(" -- ").next().unwrap_or(trimmed);

            let parts: Vec<&str> = coord_part.split(':').collect();
            if parts.len() >= 4 {
                // group:artifact:packaging:version[:scope]
                Some(format!("{}:{}:{}", parts[0], parts[1], parts[3]))
            } else {
                None
            }
        })
        .collect()
}

/// Try to find `JAVA_HOME` automatically on macOS.
fn find_java_home() -> Option<String> {
    // Check if JAVA_HOME is already set
    if let Ok(java_home) = std::env::var("JAVA_HOME")
        && !java_home.is_empty()
    {
        return Some(java_home);
    }

    // Try macOS java_home utility
    if let Ok(output) = Command::new("/usr/libexec/java_home").output()
        && output.status.success()
    {
        let java_home = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !java_home.is_empty() {
            return Some(java_home);
        }
    }

    None
}

/// Run maven dependency:list and return the set of resolved artifacts.
/// Returns None if Maven is not available or `JAVA_HOME` is not set.
fn maven_resolve(coordinate: &str) -> Option<HashSet<String>> {
    let parts: Vec<&str> = coordinate.split(':').collect();
    assert!(parts.len() == 3, "Invalid coordinate format: {coordinate}");
    let (group_id, artifact_id, version) = (parts[0], parts[1], parts[2]);

    // Find JAVA_HOME
    let Some(java_home) = find_java_home() else {
        eprintln!("Warning: JAVA_HOME not found, skipping Maven test");
        return None;
    };

    // Create a temporary directory with pom.xml
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let pom_path = temp_dir.path().join("pom.xml");
    let mut pom_file = std::fs::File::create(&pom_path).expect("Failed to create pom.xml");
    pom_file
        .write_all(create_pom_xml(group_id, artifact_id, version).as_bytes())
        .expect("Failed to write pom.xml");

    let output = Command::new("mvn")
        .env("JAVA_HOME", &java_home)
        .args([
            "dependency:list",
            "-DincludeScope=runtime",
            "-DoutputAbsoluteArtifactFilename=false",
        ])
        .current_dir(temp_dir.path())
        .output();

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Warning: Failed to execute maven: {e}");
            return None;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        eprintln!("Maven stderr: {stderr}");
        eprintln!(
            "Warning: Maven failed with exit code: {:?}",
            output.status.code()
        );
        return None;
    }

    let mut result = parse_maven_output(&stdout);
    // Add the root artifact itself
    result.insert(coordinate.to_string());
    Some(result)
}

/// Run antler resolve with nearest-wins (Maven-compatible) and return the set of resolved artifacts.
async fn antler_resolve_maven_compat(coordinate: &str) -> HashSet<String> {
    use antlers::{Antlers, Artifact};

    let artifact = Artifact::parse(coordinate).expect("Invalid coordinate");
    let resolution = Antlers::with_defaults()
        .nearest_wins() // Use Maven's conflict resolution strategy
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

    errors
}

// ============================================================================
// Test Cases
// ============================================================================

#[tokio::test]
#[ignore = "requires network and mvn"]
async fn test_maven_simple_artifact() {
    let coordinate = "org.slf4j:slf4j-api:2.0.9";

    let Some(maven) = maven_resolve(coordinate) else {
        println!("Skipping test: Maven not available");
        return;
    };
    let antler = antler_resolve_maven_compat(coordinate).await;

    println!("Maven resolved {} artifacts:", maven.len());
    for artifact in &maven {
        println!("  {artifact}");
    }

    println!("\nAntler resolved {} artifacts:", antler.len());
    for artifact in &antler {
        println!("  {artifact}");
    }

    let errors = compare_resolutions(&maven, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}

#[tokio::test]
#[ignore = "requires network and mvn"]
async fn test_maven_jackson_databind() {
    let coordinate = "com.fasterxml.jackson.core:jackson-databind:2.15.3";

    let Some(maven) = maven_resolve(coordinate) else {
        println!("Skipping test: Maven not available");
        return;
    };
    let antler = antler_resolve_maven_compat(coordinate).await;

    println!("Maven resolved {} artifacts:", maven.len());
    for artifact in &maven {
        println!("  {artifact}");
    }

    println!("\nAntler resolved {} artifacts:", antler.len());
    for artifact in &antler {
        println!("  {artifact}");
    }

    let errors = compare_resolutions(&maven, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}

#[tokio::test]
#[ignore = "requires network and mvn"]
async fn test_maven_guava() {
    let coordinate = "com.google.guava:guava:32.1.3-jre";

    let Some(maven) = maven_resolve(coordinate) else {
        println!("Skipping test: Maven not available");
        return;
    };
    let antler = antler_resolve_maven_compat(coordinate).await;

    println!("Maven resolved {} artifacts:", maven.len());
    for artifact in &maven {
        println!("  {artifact}");
    }

    println!("\nAntler resolved {} artifacts:", antler.len());
    for artifact in &antler {
        println!("  {artifact}");
    }

    let errors = compare_resolutions(&maven, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}

#[tokio::test]
#[ignore = "requires network and mvn"]
async fn test_maven_okhttp() {
    // Note: Maven uses nearest-wins, so this may differ from Coursier/Gradle
    let coordinate = "com.squareup.okhttp3:okhttp:4.12.0";

    let Some(maven) = maven_resolve(coordinate) else {
        println!("Skipping test: Maven not available");
        return;
    };
    let antler = antler_resolve_maven_compat(coordinate).await;

    println!("Maven resolved {} artifacts:", maven.len());
    for artifact in &maven {
        println!("  {artifact}");
    }

    println!("\nAntler resolved {} artifacts:", antler.len());
    for artifact in &antler {
        println!("  {artifact}");
    }

    let errors = compare_resolutions(&maven, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}

#[tokio::test]
#[ignore = "requires network and mvn"]
async fn test_maven_spring_boot_starter() {
    let coordinate = "org.springframework.boot:spring-boot-starter:3.2.0";

    let Some(maven) = maven_resolve(coordinate) else {
        println!("Skipping test: Maven not available");
        return;
    };
    let antler = antler_resolve_maven_compat(coordinate).await;

    println!("Maven resolved {} artifacts:", maven.len());
    for artifact in &maven {
        println!("  {artifact}");
    }

    println!("\nAntler resolved {} artifacts:", antler.len());
    for artifact in &antler {
        println!("  {artifact}");
    }

    let errors = compare_resolutions(&maven, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}
