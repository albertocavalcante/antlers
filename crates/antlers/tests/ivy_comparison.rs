//! Integration tests comparing antler resolution with Apache Ivy.
//!
//! These tests verify that antler produces the same resolution as Apache Ivy.
//! Ivy uses "latest wins" by default, similar to "highest wins".
//!
//! Run with: `cargo test --test ivy_comparison -- --ignored`
//! (These are ignored by default as they require network access and ivy)

use std::collections::HashSet;
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

/// Create a minimal ivy.xml for resolving a single artifact.
fn create_ivy_xml(group_id: &str, artifact_id: &str, version: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ivy-module version="2.0">
    <info organisation="test" module="test" revision="1.0.0"/>
    <dependencies>
        <dependency org="{group_id}" name="{artifact_id}" rev="{version}" conf="default"/>
    </dependencies>
</ivy-module>
"#
    )
}

/// Create ivysettings.xml pointing to Maven Central.
const fn create_ivy_settings() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<ivysettings>
    <settings defaultResolver="chain"/>
    <resolvers>
        <chain name="chain">
            <ibiblio name="central" m2compatible="true" root="https://repo1.maven.org/maven2/"/>
            <ibiblio name="google" m2compatible="true" root="https://maven.google.com/"/>
        </chain>
    </resolvers>
</ivysettings>
"#
}

/// Parse ivy resolve output into a set of artifact coordinates.
fn parse_ivy_output(output: &str, cache_dir: &std::path::Path) -> HashSet<String> {
    // Ivy doesn't have a nice output format, so we'll look at the cache
    // The cache structure is: cache/org/module/revision/
    let mut result = HashSet::new();

    if let Ok(entries) = std::fs::read_dir(cache_dir) {
        for org_entry in entries.flatten() {
            if !org_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let org = org_entry.file_name().to_string_lossy().to_string();

            if let Ok(modules) = std::fs::read_dir(org_entry.path()) {
                for module_entry in modules.flatten() {
                    if !module_entry
                        .file_type()
                        .map(|t| t.is_dir())
                        .unwrap_or(false)
                    {
                        continue;
                    }
                    let module = module_entry.file_name().to_string_lossy().to_string();

                    if let Ok(revisions) = std::fs::read_dir(module_entry.path()) {
                        for rev_entry in revisions.flatten() {
                            if !rev_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                                continue;
                            }
                            let rev = rev_entry.file_name().to_string_lossy().to_string();

                            // Check if there's actually an artifact (jar or pom)
                            if let Ok(files) = std::fs::read_dir(rev_entry.path()) {
                                let has_artifact = files.flatten().any(|f| {
                                    let path = f.path();
                                    path.extension().is_some_and(|ext| {
                                        ext.eq_ignore_ascii_case("jar")
                                            || ext.eq_ignore_ascii_case("pom")
                                    })
                                });
                                if has_artifact {
                                    result.insert(format!("{org}:{module}:{rev}"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Also parse the output for downloaded artifacts
    for line in output.lines() {
        // Look for lines like: downloading https://repo1.maven.org/maven2/org/slf4j/slf4j-api/2.0.9/slf4j-api-2.0.9.jar
        if line.contains("downloading")
            && line.contains(".jar")
            && let Some(url) = line.split_whitespace().last()
        {
            // Parse URL: .../org/slf4j/slf4j-api/2.0.9/slf4j-api-2.0.9.jar
            let parts: Vec<&str> = url.split('/').collect();
            if parts.len() >= 4 {
                // Find the artifact name pattern: artifact-version.jar
                let version = parts.get(parts.len() - 2).unwrap_or(&"");
                let artifact = parts.get(parts.len() - 3).unwrap_or(&"");

                // Find org (everything before artifact)
                if let Some(artifact_pos) = parts.iter().position(|&p| p == *artifact)
                    && artifact_pos >= 4
                {
                    // Skip protocol and domain
                    let org_parts = &parts[3..artifact_pos];
                    if !org_parts.is_empty() {
                        let org = org_parts.join(".");
                        result.insert(format!("{org}:{artifact}:{version}"));
                    }
                }
            }
        }
    }

    result
}

/// Run ivy resolve and return the set of resolved artifacts.
fn ivy_resolve(coordinate: &str) -> HashSet<String> {
    let parts: Vec<&str> = coordinate.split(':').collect();
    assert!(parts.len() == 3, "Invalid coordinate format: {coordinate}");
    let (group_id, artifact_id, version) = (parts[0], parts[1], parts[2]);

    // Create a temporary directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create ivy.xml
    let ivy_path = temp_dir.path().join("ivy.xml");
    let mut ivy_file = std::fs::File::create(&ivy_path).expect("Failed to create ivy.xml");
    ivy_file
        .write_all(create_ivy_xml(group_id, artifact_id, version).as_bytes())
        .expect("Failed to write ivy.xml");

    // Create ivysettings.xml
    let settings_path = temp_dir.path().join("ivysettings.xml");
    let mut settings_file =
        std::fs::File::create(&settings_path).expect("Failed to create ivysettings.xml");
    settings_file
        .write_all(create_ivy_settings().as_bytes())
        .expect("Failed to write ivysettings.xml");

    // Create cache directory
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir(&cache_dir).expect("Failed to create cache dir");

    // Run Ivy - try different possible command names
    let ivy_commands = ["ivy", "apache-ivy", "ant"];
    let mut output = None;

    for cmd in &ivy_commands {
        let result = if *cmd == "ant" {
            // For ant, we'd need a build.xml - skip for now
            continue;
        } else {
            Command::new(cmd)
                .args([
                    "-ivy",
                    ivy_path.to_str().unwrap(),
                    "-settings",
                    settings_path.to_str().unwrap(),
                    "-cache",
                    cache_dir.to_str().unwrap(),
                    "-retrieve",
                    temp_dir
                        .path()
                        .join("lib/[artifact]-[revision].[ext]")
                        .to_str()
                        .unwrap(),
                ])
                .current_dir(temp_dir.path())
                .output()
        };

        if let Ok(res) = result {
            output = Some(res);
            break;
        }
    }

    let output = output.expect("Failed to execute ivy. Is it installed?");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        eprintln!("Ivy stdout: {stdout}");
        eprintln!("Ivy stderr: {stderr}");
        // Ivy might not be installed, return empty set with a warning
        eprintln!("Warning: Ivy command failed, test will be skipped");
        return HashSet::new();
    }

    let combined = format!("{stdout}\n{stderr}");
    let mut result = parse_ivy_output(&combined, &cache_dir);
    // Add the root artifact itself
    result.insert(coordinate.to_string());
    result
}

/// Run antler resolve and return the set of resolved artifacts.
async fn antler_resolve(coordinate: &str) -> HashSet<String> {
    use antlers::{Antlers, Artifact};

    let artifact = Artifact::parse(coordinate).expect("Invalid coordinate");
    let resolution = Antlers::with_defaults()
        .highest_wins() // Ivy uses "latest wins" which is similar
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

    // Skip if Ivy returned empty (probably not installed)
    if expected.is_empty() {
        return errors;
    }

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
#[ignore = "requires network and ivy"]
async fn test_ivy_simple_artifact() {
    let coordinate = "org.slf4j:slf4j-api:2.0.9";

    let ivy = ivy_resolve(coordinate);
    if ivy.is_empty() {
        println!("Skipping test: Ivy not installed or failed");
        return;
    }

    let antler = antler_resolve(coordinate).await;

    println!("Ivy resolved {} artifacts:", ivy.len());
    for artifact in &ivy {
        println!("  {artifact}");
    }

    println!("\nAntler resolved {} artifacts:", antler.len());
    for artifact in &antler {
        println!("  {artifact}");
    }

    let errors = compare_resolutions(&ivy, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}

#[tokio::test]
#[ignore = "requires network and ivy"]
async fn test_ivy_jackson_databind() {
    let coordinate = "com.fasterxml.jackson.core:jackson-databind:2.15.3";

    let ivy = ivy_resolve(coordinate);
    if ivy.is_empty() {
        println!("Skipping test: Ivy not installed or failed");
        return;
    }

    let antler = antler_resolve(coordinate).await;

    println!("Ivy resolved {} artifacts:", ivy.len());
    for artifact in &ivy {
        println!("  {artifact}");
    }

    println!("\nAntler resolved {} artifacts:", antler.len());
    for artifact in &antler {
        println!("  {artifact}");
    }

    let errors = compare_resolutions(&ivy, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}

#[tokio::test]
#[ignore = "requires network and ivy"]
async fn test_ivy_guava() {
    let coordinate = "com.google.guava:guava:32.1.3-jre";

    let ivy = ivy_resolve(coordinate);
    if ivy.is_empty() {
        println!("Skipping test: Ivy not installed or failed");
        return;
    }

    let antler = antler_resolve(coordinate).await;

    println!("Ivy resolved {} artifacts:", ivy.len());
    for artifact in &ivy {
        println!("  {artifact}");
    }

    println!("\nAntler resolved {} artifacts:", antler.len());
    for artifact in &antler {
        println!("  {artifact}");
    }

    let errors = compare_resolutions(&ivy, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}

#[tokio::test]
#[ignore = "requires network and ivy"]
async fn test_ivy_okhttp() {
    let coordinate = "com.squareup.okhttp3:okhttp:4.12.0";

    let ivy = ivy_resolve(coordinate);
    if ivy.is_empty() {
        println!("Skipping test: Ivy not installed or failed");
        return;
    }

    let antler = antler_resolve(coordinate).await;

    println!("Ivy resolved {} artifacts:", ivy.len());
    for artifact in &ivy {
        println!("  {artifact}");
    }

    println!("\nAntler resolved {} artifacts:", antler.len());
    for artifact in &antler {
        println!("  {artifact}");
    }

    let errors = compare_resolutions(&ivy, &antler, coordinate);
    assert!(
        errors.is_empty(),
        "Resolution differences:\n{}",
        errors.join("\n")
    );
}
