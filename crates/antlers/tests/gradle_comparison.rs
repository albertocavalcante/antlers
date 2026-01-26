//! Integration tests comparing antler resolution with Gradle.
//!
//! These tests verify that antler produces the same resolution as Gradle,
//! which uses "highest wins" conflict resolution (like Coursier).
//!
//! Run with: `cargo test --test gradle_comparison -- --ignored`
//! (These are ignored by default as they require network access and gradle)

use std::collections::HashSet;
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

/// Create a minimal build.gradle for resolving a single artifact.
fn create_build_gradle(group_id: &str, artifact_id: &str, version: &str) -> String {
    format!(
        r#"plugins {{
    id 'java'
}}

repositories {{
    mavenCentral()
    google()
}}

dependencies {{
    implementation '{group_id}:{artifact_id}:{version}'
}}

task listDependencies {{
    doLast {{
        configurations.runtimeClasspath.resolvedConfiguration.resolvedArtifacts.each {{ artifact ->
            def id = artifact.moduleVersion.id
            println "${{id.group}}:${{id.name}}:${{id.version}}"
        }}
    }}
}}
"#
    )
}

/// Parse gradle listDependencies output into a set of artifact coordinates.
fn parse_gradle_output(output: &str) -> HashSet<String> {
    output
        .lines()
        .filter(|line| line.contains(':') && !line.starts_with('>') && !line.contains("BUILD"))
        .filter(|line| {
            // Filter out Gradle task output lines
            !line.starts_with("Task")
                && !line.starts_with("Starting")
                && !line.starts_with("Settings")
                && !line.starts_with("Gradle")
                && !line.contains("CONFIGURING")
                && !line.contains("EXECUTING")
        })
        .map(|line| line.trim().to_string())
        .filter(|line| {
            // Should match pattern: group:artifact:version
            let parts: Vec<_> = line.split(':').collect();
            parts.len() == 3 && !parts[0].is_empty()
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

/// Run gradle listDependencies and return the set of resolved artifacts.
/// Returns None if Gradle is not available or `JAVA_HOME` is not set.
fn gradle_resolve(coordinate: &str) -> Option<HashSet<String>> {
    let parts: Vec<&str> = coordinate.split(':').collect();
    assert!(parts.len() == 3, "Invalid coordinate format: {coordinate}");
    let (group_id, artifact_id, version) = (parts[0], parts[1], parts[2]);

    // Find JAVA_HOME
    let Some(java_home) = find_java_home() else {
        eprintln!("Warning: JAVA_HOME not found, skipping Gradle test");
        return None;
    };

    // Create a temporary directory with build.gradle
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let build_path = temp_dir.path().join("build.gradle");
    let mut build_file = std::fs::File::create(&build_path).expect("Failed to create build.gradle");
    build_file
        .write_all(create_build_gradle(group_id, artifact_id, version).as_bytes())
        .expect("Failed to write build.gradle");

    // Also create settings.gradle to avoid warnings
    let settings_path = temp_dir.path().join("settings.gradle");
    std::fs::File::create(&settings_path).expect("Failed to create settings.gradle");

    let output = Command::new("gradle")
        .env("JAVA_HOME", &java_home)
        .args(["listDependencies", "--quiet", "--no-daemon"])
        .current_dir(temp_dir.path())
        .output();

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Warning: Failed to execute gradle: {e}");
            return None;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        eprintln!("Gradle stdout: {stdout}");
        eprintln!("Gradle stderr: {stderr}");
        eprintln!(
            "Warning: Gradle failed with exit code: {:?}",
            output.status.code()
        );
        return None;
    }

    let mut result = parse_gradle_output(&stdout);
    // Add the root artifact itself
    result.insert(coordinate.to_string());
    Some(result)
}

/// Run antler resolve with highest-wins (Gradle-compatible) and return the set of resolved artifacts.
async fn antler_resolve_gradle_compat(coordinate: &str) -> HashSet<String> {
    use antlers::{Antlers, Artifact};

    let artifact = Artifact::parse(coordinate).expect("Invalid coordinate");
    let resolution = Antlers::with_defaults()
        .highest_wins() // Use Gradle's conflict resolution strategy
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
#[ignore = "requires network and gradle"]
async fn test_gradle_simple_artifact() {
    let coordinate = "org.slf4j:slf4j-api:2.0.9";

    let Some(gradle) = gradle_resolve(coordinate) else {
        println!("Skipping test: Gradle not available");
        return;
    };
    let antler = antler_resolve_gradle_compat(coordinate).await;

    println!("Gradle resolved {} artifacts:", gradle.len());
    for artifact in &gradle {
        println!("  {artifact}");
    }

    println!("\nAntler resolved {} artifacts:", antler.len());
    for artifact in &antler {
        println!("  {artifact}");
    }

    let errors = compare_resolutions(&gradle, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}

#[tokio::test]
#[ignore = "requires network and gradle"]
async fn test_gradle_jackson_databind() {
    let coordinate = "com.fasterxml.jackson.core:jackson-databind:2.15.3";

    let Some(gradle) = gradle_resolve(coordinate) else {
        println!("Skipping test: Gradle not available");
        return;
    };
    let antler = antler_resolve_gradle_compat(coordinate).await;

    println!("Gradle resolved {} artifacts:", gradle.len());
    for artifact in &gradle {
        println!("  {artifact}");
    }

    println!("\nAntler resolved {} artifacts:", antler.len());
    for artifact in &antler {
        println!("  {artifact}");
    }

    let errors = compare_resolutions(&gradle, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}

#[tokio::test]
#[ignore = "requires network and gradle"]
async fn test_gradle_guava() {
    let coordinate = "com.google.guava:guava:32.1.3-jre";

    let Some(gradle) = gradle_resolve(coordinate) else {
        println!("Skipping test: Gradle not available");
        return;
    };
    let antler = antler_resolve_gradle_compat(coordinate).await;

    println!("Gradle resolved {} artifacts:", gradle.len());
    for artifact in &gradle {
        println!("  {artifact}");
    }

    println!("\nAntler resolved {} artifacts:", antler.len());
    for artifact in &antler {
        println!("  {artifact}");
    }

    let errors = compare_resolutions(&gradle, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}

#[tokio::test]
#[ignore = "requires network and gradle"]
async fn test_gradle_okhttp() {
    let coordinate = "com.squareup.okhttp3:okhttp:4.12.0";

    let Some(gradle) = gradle_resolve(coordinate) else {
        println!("Skipping test: Gradle not available");
        return;
    };
    let antler = antler_resolve_gradle_compat(coordinate).await;

    println!("Gradle resolved {} artifacts:", gradle.len());
    for artifact in &gradle {
        println!("  {artifact}");
    }

    println!("\nAntler resolved {} artifacts:", antler.len());
    for artifact in &antler {
        println!("  {artifact}");
    }

    let errors = compare_resolutions(&gradle, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}

#[tokio::test]
#[ignore = "requires network and gradle"]
async fn test_gradle_kotlin_stdlib() {
    let coordinate = "org.jetbrains.kotlin:kotlin-stdlib:1.9.10";

    let Some(gradle) = gradle_resolve(coordinate) else {
        println!("Skipping test: Gradle not available");
        return;
    };
    let antler = antler_resolve_gradle_compat(coordinate).await;

    println!("Gradle resolved {} artifacts:", gradle.len());
    for artifact in &gradle {
        println!("  {artifact}");
    }

    println!("\nAntler resolved {} artifacts:", antler.len());
    for artifact in &antler {
        println!("  {artifact}");
    }

    let errors = compare_resolutions(&gradle, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}

#[tokio::test]
#[ignore = "requires network and gradle"]
async fn test_gradle_spring_boot_starter() {
    let coordinate = "org.springframework.boot:spring-boot-starter:3.2.0";

    let Some(gradle) = gradle_resolve(coordinate) else {
        println!("Skipping test: Gradle not available");
        return;
    };
    let antler = antler_resolve_gradle_compat(coordinate).await;

    println!("Gradle resolved {} artifacts:", gradle.len());
    for artifact in &gradle {
        println!("  {artifact}");
    }

    println!("\nAntler resolved {} artifacts:", antler.len());
    for artifact in &antler {
        println!("  {artifact}");
    }

    let errors = compare_resolutions(&gradle, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}
