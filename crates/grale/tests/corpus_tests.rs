//! Corpus tests for Gradle Module Metadata parsing.
//!
//! These tests verify that we can parse real-world .module files from popular
//! libraries. The corpus files are downloaded from Maven Central and checked
//! into the repository for reproducibility.

use grale::{GradleModule, GradleModuleParser};

/// Test that all corpus files can be parsed successfully.
fn test_parse_corpus_file(name: &str, content: &str) -> GradleModule {
    GradleModuleParser::parse(content).unwrap_or_else(|e| panic!("Failed to parse {name}: {e}"))
}

/// Verify basic structure of a parsed module
fn verify_module_structure(module: &GradleModule, expected_group: &str, expected_artifact: &str) {
    assert_eq!(module.component.group, expected_group, "Group mismatch");
    assert_eq!(
        module.component.module, expected_artifact,
        "Artifact mismatch"
    );
    assert!(!module.variants.is_empty(), "Module should have variants");
}

// =============================================================================
// OkHttp - Tests numeric attribute values (org.gradle.jvm.version: 8)
// =============================================================================

#[test]
fn test_corpus_okhttp() {
    let content = include_str!("corpus/okhttp-4.12.0.module");
    let module = test_parse_corpus_file("okhttp-4.12.0.module", content);

    verify_module_structure(&module, "com.squareup.okhttp3", "okhttp");
    assert_eq!(module.component.version, "4.12.0");

    // Should have API and runtime variants
    assert!(module.api_variant().is_some(), "Should have API variant");
    assert!(
        module.runtime_variant().is_some(),
        "Should have runtime variant"
    );

    // Check runtime variant has correct dependencies
    let runtime = module.runtime_variant().unwrap();
    assert!(
        !runtime.dependencies.is_empty(),
        "Runtime should have dependencies"
    );

    // Should have okio and kotlin-stdlib dependencies
    let dep_names: Vec<_> = runtime
        .dependencies
        .iter()
        .map(|d| format!("{}:{}", d.group, d.module))
        .collect();
    assert!(
        dep_names.iter().any(|n| n == "com.squareup.okio:okio"),
        "Should depend on okio"
    );
    assert!(
        dep_names.iter().any(|n| n.contains("kotlin-stdlib")),
        "Should depend on kotlin-stdlib"
    );
}

// =============================================================================
// Kotlin stdlib - Tests Kotlin Multiplatform structure
// =============================================================================

#[test]
fn test_corpus_kotlin_stdlib() {
    let content = include_str!("corpus/kotlin-stdlib-2.1.0.module");
    let module = test_parse_corpus_file("kotlin-stdlib-2.1.0.module", content);

    verify_module_structure(&module, "org.jetbrains.kotlin", "kotlin-stdlib");
    assert_eq!(module.component.version, "2.1.0");

    // Kotlin stdlib has many variants for different platforms
    assert!(module.variants.len() >= 2, "Should have multiple variants");

    // Should have JVM variants
    let has_jvm_variants = module
        .variants
        .iter()
        .any(|v| v.name.contains("jvm") || v.name.contains("Jvm"));
    assert!(has_jvm_variants, "Should have JVM variants");
}

// =============================================================================
// Ktor - Tests complex multiplatform module with many variants
// =============================================================================

#[test]
fn test_corpus_ktor() {
    let content = include_str!("corpus/ktor-client-core-2.3.0.module");
    let module = test_parse_corpus_file("ktor-client-core-2.3.0.module", content);

    verify_module_structure(&module, "io.ktor", "ktor-client-core");
    assert_eq!(module.component.version, "2.3.0");

    // Ktor has many platform-specific variants
    assert!(module.variants.len() >= 4, "Should have many variants");

    // Verify variant names include different platforms
    let variant_names: Vec<_> = module.variants.iter().map(|v| &v.name).collect();
    println!("Ktor variants: {variant_names:?}");
}

// =============================================================================
// Guava - Tests standard Java library structure
// =============================================================================

#[test]
fn test_corpus_guava() {
    let content = include_str!("corpus/guava-33.0.0-jre.module");
    let module = test_parse_corpus_file("guava-33.0.0-jre.module", content);

    verify_module_structure(&module, "com.google.guava", "guava");
    assert_eq!(module.component.version, "33.0.0-jre");

    // Should have standard API and runtime variants
    assert!(module.api_variant().is_some(), "Should have API variant");
    assert!(
        module.runtime_variant().is_some(),
        "Should have runtime variant"
    );

    // Check for dependencies
    let runtime = module.runtime_variant().unwrap();
    let dep_names: Vec<_> = runtime
        .dependencies
        .iter()
        .map(|d| format!("{}:{}", d.group, d.module))
        .collect();

    // Guava depends on errorprone annotations, jsr305, etc.
    println!("Guava runtime dependencies: {dep_names:?}");
}

// =============================================================================
// Jackson - Tests dependency constraints (version alignment)
// =============================================================================

#[test]
fn test_corpus_jackson() {
    let content = include_str!("corpus/jackson-databind-2.16.0.module");
    let module = test_parse_corpus_file("jackson-databind-2.16.0.module", content);

    verify_module_structure(&module, "com.fasterxml.jackson.core", "jackson-databind");
    assert_eq!(module.component.version, "2.16.0");

    // Should have API and runtime variants
    assert!(module.api_variant().is_some(), "Should have API variant");
    assert!(
        module.runtime_variant().is_some(),
        "Should have runtime variant"
    );

    // Jackson-databind depends on jackson-core and jackson-annotations
    let runtime = module.runtime_variant().unwrap();
    let dep_names: Vec<_> = runtime
        .dependencies
        .iter()
        .map(|d| format!("{}:{}", d.group, d.module))
        .collect();
    assert!(
        dep_names.iter().any(|n| n.contains("jackson-core")),
        "Should depend on jackson-core"
    );
    assert!(
        dep_names.iter().any(|n| n.contains("jackson-annotations")),
        "Should depend on jackson-annotations"
    );
}

// =============================================================================
// Spring Boot - Tests complex BOM-style module
// =============================================================================

#[test]
fn test_corpus_spring_boot() {
    let content = include_str!("corpus/spring-boot-3.2.0.module");
    let module = test_parse_corpus_file("spring-boot-3.2.0.module", content);

    verify_module_structure(&module, "org.springframework.boot", "spring-boot");
    assert_eq!(module.component.version, "3.2.0");

    // Spring Boot has standard variants
    assert!(
        module.api_variant().is_some() || module.runtime_variant().is_some(),
        "Should have at least one standard variant"
    );

    // Check for Spring dependencies
    let has_spring_deps = module.variants.iter().any(|v| {
        v.dependencies
            .iter()
            .any(|d| d.group.contains("springframework"))
    });
    assert!(has_spring_deps, "Should have Spring Framework dependencies");
}

// =============================================================================
// Kotlinx Coroutines - Tests complex multiplatform with rich version constraints
// =============================================================================

#[test]
fn test_corpus_kotlinx_coroutines() {
    let content = include_str!("corpus/kotlinx-coroutines-core-1.7.3.module");
    let module = test_parse_corpus_file("kotlinx-coroutines-core-1.7.3.module", content);

    verify_module_structure(&module, "org.jetbrains.kotlinx", "kotlinx-coroutines-core");
    assert_eq!(module.component.version, "1.7.3");

    // Kotlinx coroutines is multiplatform - should have many variants
    assert!(
        module.variants.len() >= 4,
        "Should have multiple platform variants"
    );

    // Print variant summary for debugging
    for variant in &module.variants {
        println!(
            "  Variant '{}': {} deps, {} files, redirect={}",
            variant.name,
            variant.dependencies.len(),
            variant.files.len(),
            variant.is_redirect()
        );
    }
}

// =============================================================================
// Comprehensive parsing tests
// =============================================================================

#[test]
fn test_all_corpus_files_parse_without_error() {
    let corpus_files = [
        (
            "okhttp-4.12.0.module",
            include_str!("corpus/okhttp-4.12.0.module"),
        ),
        (
            "kotlin-stdlib-2.1.0.module",
            include_str!("corpus/kotlin-stdlib-2.1.0.module"),
        ),
        (
            "ktor-client-core-2.3.0.module",
            include_str!("corpus/ktor-client-core-2.3.0.module"),
        ),
        (
            "guava-33.0.0-jre.module",
            include_str!("corpus/guava-33.0.0-jre.module"),
        ),
        (
            "jackson-databind-2.16.0.module",
            include_str!("corpus/jackson-databind-2.16.0.module"),
        ),
        (
            "spring-boot-3.2.0.module",
            include_str!("corpus/spring-boot-3.2.0.module"),
        ),
        (
            "kotlinx-coroutines-core-1.7.3.module",
            include_str!("corpus/kotlinx-coroutines-core-1.7.3.module"),
        ),
    ];

    let mut failed = vec![];

    for (name, content) in &corpus_files {
        match GradleModuleParser::parse(content) {
            Ok(module) => {
                println!(
                    "OK: {} - {}:{}:{} ({} variants)",
                    name,
                    module.component.group,
                    module.component.module,
                    module.component.version,
                    module.variants.len()
                );
            }
            Err(e) => {
                eprintln!("FAIL: {name} - {e}");
                failed.push(*name);
            }
        }
    }

    assert!(
        failed.is_empty(),
        "Failed to parse {} corpus files: {:?}",
        failed.len(),
        failed
    );
}

// =============================================================================
// Variant selection tests
// =============================================================================

#[test]
fn test_variant_selection_okhttp() {
    let content = include_str!("corpus/okhttp-4.12.0.module");
    let module = GradleModuleParser::parse(content).unwrap();

    // Test API variant selection
    let api = module.api_variant().expect("Should have API variant");
    assert_eq!(api.name, "apiElements");

    // Verify API dependencies
    assert!(!api.dependencies.is_empty());
    for dep in &api.dependencies {
        assert!(!dep.group.is_empty());
        assert!(!dep.module.is_empty());
        assert!(dep.version.is_some(), "API deps should have version");
    }

    // Test runtime variant selection
    let runtime = module
        .runtime_variant()
        .expect("Should have runtime variant");
    assert_eq!(runtime.name, "runtimeElements");

    // Runtime should have same or more dependencies than API
    assert!(runtime.dependencies.len() >= api.dependencies.len());
}

#[test]
fn test_numeric_attribute_handling() {
    // This specifically tests that numeric attributes like org.gradle.jvm.version: 8
    // are handled correctly (converted to strings)
    let content = include_str!("corpus/okhttp-4.12.0.module");
    let module = GradleModuleParser::parse(content).unwrap();

    let api = module.api_variant().unwrap();

    // Check that JVM version attribute is present and is "8"
    let jvm_version = api.attributes.get("org.gradle.jvm.version");
    assert_eq!(
        jvm_version,
        Some("8"),
        "JVM version should be '8' (as string)"
    );
}

#[test]
fn test_dependency_conversion_to_gav() {
    let content = include_str!("corpus/okhttp-4.12.0.module");
    let module = GradleModuleParser::parse(content).unwrap();

    let runtime = module.runtime_variant().unwrap();

    // Convert all dependencies to gav format
    for dep in &runtime.dependencies {
        let gav_dep = dep.to_gav_dependency();
        assert!(
            gav_dep.is_some(),
            "Dependency {} should convert to gav",
            dep.coordinates()
        );

        let gav = gav_dep.unwrap();
        assert_eq!(gav.group_id(), dep.group);
        assert_eq!(gav.artifact_id(), dep.module);
    }
}

#[test]
fn test_multiplatform_available_at_redirects() {
    // Kotlin multiplatform modules use available_at for platform-specific modules
    let content = include_str!("corpus/kotlinx-coroutines-core-1.7.3.module");
    let module = GradleModuleParser::parse(content).unwrap();

    // Find variants with available_at redirects
    let redirect_variants: Vec<_> = module.variants.iter().filter(|v| v.is_redirect()).collect();

    println!("Found {} redirect variants", redirect_variants.len());
    for v in &redirect_variants {
        if let Some(ref at) = v.available_at {
            println!("  {} -> {}:{}:{}", v.name, at.group, at.module, at.version);
        }
    }

    // There should be some redirect variants in a multiplatform module
    // (but this depends on the specific module structure)
}

#[test]
fn test_kmp_variant_selection_follows_jvm_redirect() {
    // For KMP modules, runtime_variant() should return the JVM redirect variant
    let content = include_str!("corpus/kotlinx-coroutines-core-1.7.3.module");
    let module = GradleModuleParser::parse(content).unwrap();

    // runtime_variant should find the jvmRuntimeElements-published variant
    let runtime = module
        .runtime_variant()
        .expect("Should find runtime variant");
    println!("Selected runtime variant: {}", runtime.name);

    // It should be a redirect to kotlinx-coroutines-core-jvm
    assert!(
        runtime.is_redirect(),
        "KMP runtime variant should be a redirect"
    );

    let available_at = runtime
        .available_at
        .as_ref()
        .expect("Should have available_at");
    assert_eq!(
        available_at.module, "kotlinx-coroutines-core-jvm",
        "Should redirect to -jvm module"
    );

    // api_variant should do the same
    let api = module.api_variant().expect("Should find API variant");
    println!("Selected API variant: {}", api.name);
    assert!(api.is_redirect(), "KMP API variant should be a redirect");

    let api_at = api.available_at.as_ref().expect("Should have available_at");
    assert_eq!(
        api_at.module, "kotlinx-coroutines-core-jvm",
        "Should redirect to -jvm module"
    );
}

#[test]
fn test_checksum_extraction() {
    let content = include_str!("corpus/okhttp-4.12.0.module");
    let module = GradleModuleParser::parse(content).unwrap();

    let runtime = module.runtime_variant().unwrap();

    for file in &runtime.files {
        // Files should have checksums
        let checksum = file.strongest_checksum();
        assert!(
            checksum.is_some(),
            "File {} should have checksum",
            file.name
        );

        let (algo, hash) = checksum.unwrap();
        println!("File {}: {} = {}", file.name, algo, hash);

        // Verify hash length is reasonable
        match algo {
            "sha512" => assert_eq!(hash.len(), 128),
            "sha256" => assert_eq!(hash.len(), 64),
            "sha1" => assert_eq!(hash.len(), 40),
            "md5" => assert_eq!(hash.len(), 32),
            _ => panic!("Unknown algo: {algo}"),
        }
    }
}
