//! Unified lockfile and output format definitions.
//!
//! This module provides a centralized [`LockFormat`] enum that represents all
//! supported lockfile formats for reading and writing dependencies.
//!
//! # Format Categories
//!
//! - **Native**: Our own `antlers-lock` format
//! - **Bazel ecosystem**: `rules_jvm_external` V1/V2, BUILD files, Starlark
//! - **Coursier ecosystem**: Coursier JSON format
//! - **Gradle ecosystem**: `gradle.lockfile`, version catalogs
//!
//! # Example
//!
//! ```
//! use antlers_lock::LockFormat;
//! use std::path::Path;
//!
//! // Detect from file extension
//! let format = LockFormat::from_extension("json");
//! assert_eq!(format, Some(LockFormat::Antlers));
//!
//! // Detect from path
//! let format = LockFormat::from_path(Path::new("maven_install.json"));
//! assert_eq!(format, Some(LockFormat::RulesJvmV2));
//!
//! // Check capabilities
//! assert!(LockFormat::Antlers.can_read());
//! assert!(LockFormat::Antlers.can_write());
//! assert!(!LockFormat::BazelBuild.can_read()); // write-only
//! ```

use std::path::Path;

use serde_json::Value;

/// Unified lockfile and output format enum.
///
/// This enum represents all lockfile formats that antlers can read from or write to.
/// Some formats are read-only (we can parse them), some are write-only (we can generate
/// them), and some support both operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockFormat {
    // =========================================================================
    // Native format
    // =========================================================================
    /// Our native antlers-lock JSON format.
    ///
    /// This is a superset of `rules_jvm_external` V2 with additional fields
    /// for KMP support, richer conflict metadata, and per-artifact repository URLs.
    ///
    /// **Capabilities**: read, write
    Antlers,

    // =========================================================================
    // Bazel ecosystem (rules_jvm_external)
    // =========================================================================
    /// `rules_jvm_external` V1 format (legacy).
    ///
    /// This is the older format with `dependency_tree.version == "0.1.0"`.
    /// We can read this format but prefer writing V2.
    ///
    /// **Capabilities**: read-only
    RulesJvmV1,

    /// `rules_jvm_external` V2 format.
    ///
    /// The current `rules_jvm_external` format with `version == "2"`.
    /// This is the recommended format for Bazel compatibility.
    ///
    /// **Capabilities**: read, write
    RulesJvmV2,

    // =========================================================================
    // Coursier ecosystem
    // =========================================================================
    /// Coursier JSON lockfile format.
    ///
    /// Used by Coursier and tools built on it (e.g., `cs resolve --json-output-file`).
    ///
    /// **Capabilities**: read, write
    Coursier,

    // =========================================================================
    // Gradle ecosystem
    // =========================================================================
    /// Gradle lockfile format (`gradle.lockfile`).
    ///
    /// The standard Gradle dependency locking format introduced in Gradle 4.8.
    /// This is a simple text format with one dependency per line.
    ///
    /// **Capabilities**: read, write
    GradleLockfile,

    /// Gradle version catalog format (`libs.versions.toml`).
    ///
    /// TOML-based dependency catalog introduced in Gradle 7.0.
    /// Defines versions, libraries, bundles, and plugins in a declarative format.
    ///
    /// **Capabilities**: read, write
    GradleCatalog,

    // =========================================================================
    // Build system outputs (write-only)
    // =========================================================================
    /// Bazel BUILD file output.
    ///
    /// Generates `BUILD` or `BUILD.bazel` files with `java_library` or
    /// `jvm_import` rules for resolved dependencies.
    ///
    /// **Capabilities**: write-only
    BazelBuild,

    /// Buck2 BUCK file output.
    ///
    /// Generates `BUCK` files with `prebuilt_jar` or `java_library` rules.
    ///
    /// **Capabilities**: write-only
    Buck2,

    /// Generic Starlark output (`.bzl` files).
    ///
    /// Generates Starlark files that can be loaded into BUILD files.
    /// Useful for generating repository rules or macros.
    ///
    /// **Capabilities**: write-only
    Starlark,
}

impl LockFormat {
    /// Detects the format from a file extension.
    ///
    /// # Examples
    ///
    /// ```
    /// use antlers_lock::LockFormat;
    ///
    /// assert_eq!(LockFormat::from_extension("json"), Some(LockFormat::Antlers));
    /// assert_eq!(LockFormat::from_extension("lockfile"), Some(LockFormat::GradleLockfile));
    /// assert_eq!(LockFormat::from_extension("bzl"), Some(LockFormat::Starlark));
    /// assert_eq!(LockFormat::from_extension("unknown"), None);
    /// ```
    #[must_use]
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            // JSON formats default to our native format
            // (specific file names are handled by from_path)
            "json" => Some(Self::Antlers),
            // Gradle lockfile
            "lockfile" => Some(Self::GradleLockfile),
            // TOML could be version catalog
            "toml" => Some(Self::GradleCatalog),
            // Starlark files
            "bzl" => Some(Self::Starlark),
            // Bazel BUILD files (no extension but handled by from_path)
            "bazel" => Some(Self::BazelBuild),
            _ => None,
        }
    }

    /// Detects the format from a file path.
    ///
    /// This uses both the file name and extension to determine the format.
    /// File names take precedence over extensions for disambiguation.
    ///
    /// # Examples
    ///
    /// ```
    /// use antlers_lock::LockFormat;
    /// use std::path::Path;
    ///
    /// // Specific file names
    /// assert_eq!(
    ///     LockFormat::from_path(Path::new("maven_install.json")),
    ///     Some(LockFormat::RulesJvmV2)
    /// );
    /// assert_eq!(
    ///     LockFormat::from_path(Path::new("deps.lock.json")),
    ///     Some(LockFormat::Antlers)
    /// );
    /// assert_eq!(
    ///     LockFormat::from_path(Path::new("BUILD.bazel")),
    ///     Some(LockFormat::BazelBuild)
    /// );
    /// ```
    #[must_use]
    pub fn from_path(path: &Path) -> Option<Self> {
        let file_name = path.file_name()?.to_str()?;
        let file_name_lower = file_name.to_lowercase();

        // Check specific file names first
        match file_name_lower.as_str() {
            // Antlers native format
            "deps.lock.json" | "antlers.lock.json" => return Some(Self::Antlers),

            // rules_jvm_external (V1 vs V2 determined by content)
            "install.json" | "maven_install.json" | "pinned_maven_install.json" => {
                return Some(Self::RulesJvmV2);
            }

            // Coursier
            name if name.ends_with(".coursier.json") => return Some(Self::Coursier),

            // Gradle lockfile
            "gradle.lockfile" | "buildscript-gradle.lockfile" => {
                return Some(Self::GradleLockfile);
            }

            // Gradle version catalog
            "libs.versions.toml" | "gradle.versions.toml" => return Some(Self::GradleCatalog),

            // Bazel BUILD files
            "build" | "build.bazel" => return Some(Self::BazelBuild),

            // Buck2
            "buck" => return Some(Self::Buck2),

            _ => {}
        }

        // Check for .lock.json suffix (our format)
        if file_name_lower.ends_with(".lock.json") {
            return Some(Self::Antlers);
        }

        // Check for .coursier.json suffix
        if file_name_lower.ends_with(".coursier.json") {
            return Some(Self::Coursier);
        }

        // Fall back to extension-based detection
        let ext = path.extension()?.to_str()?;
        Self::from_extension(ext)
    }

    /// Detects the format from file content.
    ///
    /// This performs content-based detection for formats that can't be
    /// reliably identified by file name alone (e.g., distinguishing
    /// `rules_jvm_external` V1 from V2).
    ///
    /// # Examples
    ///
    /// ```
    /// use antlers_lock::LockFormat;
    ///
    /// let v2_content = r#"{"version": "2", "artifacts": {}}"#;
    /// assert_eq!(LockFormat::detect(v2_content), Some(LockFormat::RulesJvmV2));
    ///
    /// let v1_content = r#"{"dependency_tree": {"version": "0.1.0"}}"#;
    /// assert_eq!(LockFormat::detect(v1_content), Some(LockFormat::RulesJvmV1));
    /// ```
    #[must_use]
    pub fn detect(content: &str) -> Option<Self> {
        // Try parsing as JSON first
        if let Ok(json) = serde_json::from_str::<Value>(content) {
            return Self::detect_from_json(&json);
        }

        // Try detecting TOML-based formats
        if content.contains("[versions]") || content.contains("[libraries]") {
            return Some(Self::GradleCatalog);
        }

        // Try detecting Gradle lockfile format (text-based)
        // Format: "group:artifact:version=classifier" per line
        let lines: Vec<&str> = content.lines().collect();
        if lines.iter().any(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with('#')
                && trimmed.contains(':')
                && trimmed.contains('=')
        }) {
            // Check for Gradle lockfile header comment
            if lines.iter().any(|line| {
                line.contains("This is a Gradle generated file") || line.contains("gradle.lockfile")
            }) {
                return Some(Self::GradleLockfile);
            }
        }

        // Check for Buck2 specific patterns
        if content.contains("prebuilt_jar(") || content.contains("remote_file(") {
            return Some(Self::Buck2);
        }

        // Try detecting Starlark/BUILD files
        // Check for common Bazel/Starlark patterns
        let has_load = content.contains("load(");
        let has_def = content.contains("def ");
        let has_rule = content.contains("rule(");
        let has_java_library = content.contains("java_library(");
        let has_jvm_import = content.contains("jvm_import(");
        let has_maven_install = content.contains("maven_install(");
        let has_java_binary = content.contains("java_binary(");
        let has_kt_library = content.contains("kt_jvm_library(");

        // Starlark files (.bzl) typically have load() and def
        if has_load && has_def {
            return Some(Self::Starlark);
        }

        // BUILD files have rule calls like java_library, jvm_import, etc.
        if has_java_library || has_jvm_import || has_java_binary || has_kt_library {
            return Some(Self::BazelBuild);
        }

        // If it has load() with maven_install, it's likely a .bzl file
        if has_load && has_maven_install {
            return Some(Self::Starlark);
        }

        // Generic Starlark detection
        if has_def || has_rule {
            return Some(Self::Starlark);
        }

        None
    }

    /// Detects the format from parsed JSON.
    fn detect_from_json(json: &Value) -> Option<Self> {
        // Check for antlers-lock format
        if json.get("format").and_then(Value::as_str) == Some("antlers-lock") {
            return Some(Self::Antlers);
        }

        // Check for rules_jvm_external V2
        if json.get("version").and_then(Value::as_str) == Some("2")
            && json.get("artifacts").is_some()
        {
            return Some(Self::RulesJvmV2);
        }

        // Check for rules_jvm_external V1 (has dependency_tree with version 0.1.0)
        if json
            .get("dependency_tree")
            .and_then(|dt| dt.get("version"))
            .and_then(Value::as_str)
            == Some("0.1.0")
        {
            return Some(Self::RulesJvmV1);
        }

        // Check for Coursier format
        // Coursier JSON typically has "dependencies" array with "coord" fields
        if json.get("dependencies").is_some()
            && json
                .get("dependencies")
                .and_then(Value::as_array)
                .is_some_and(|deps| {
                    deps.iter()
                        .any(|d| d.get("coord").is_some() || d.get("module").is_some())
                })
        {
            return Some(Self::Coursier);
        }

        None
    }

    /// Returns whether this format can be read/parsed.
    ///
    /// # Examples
    ///
    /// ```
    /// use antlers_lock::LockFormat;
    ///
    /// assert!(LockFormat::Antlers.can_read());
    /// assert!(LockFormat::RulesJvmV1.can_read());
    /// assert!(!LockFormat::BazelBuild.can_read()); // write-only
    /// ```
    #[must_use]
    pub const fn can_read(&self) -> bool {
        matches!(
            self,
            Self::Antlers
                | Self::RulesJvmV1
                | Self::RulesJvmV2
                | Self::Coursier
                | Self::GradleLockfile
                | Self::GradleCatalog
        )
    }

    /// Returns whether this format can be written/generated.
    ///
    /// # Examples
    ///
    /// ```
    /// use antlers_lock::LockFormat;
    ///
    /// assert!(LockFormat::Antlers.can_write());
    /// assert!(LockFormat::BazelBuild.can_write());
    /// assert!(!LockFormat::RulesJvmV1.can_write()); // read-only (legacy)
    /// ```
    #[must_use]
    pub const fn can_write(&self) -> bool {
        matches!(
            self,
            Self::Antlers
                | Self::RulesJvmV2
                | Self::Coursier
                | Self::GradleLockfile
                | Self::GradleCatalog
                | Self::BazelBuild
                | Self::Buck2
                | Self::Starlark
        )
    }

    /// Returns the canonical file extension for this format.
    ///
    /// Note: Some formats like `BazelBuild` don't have a traditional extension.
    ///
    /// # Examples
    ///
    /// ```
    /// use antlers_lock::LockFormat;
    ///
    /// assert_eq!(LockFormat::Antlers.extension(), "lock.json");
    /// assert_eq!(LockFormat::GradleLockfile.extension(), "lockfile");
    /// assert_eq!(LockFormat::Starlark.extension(), "bzl");
    /// ```
    #[must_use]
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Antlers => "lock.json",
            Self::RulesJvmV1 | Self::RulesJvmV2 => "json",
            Self::Coursier => "coursier.json",
            Self::GradleLockfile => "lockfile",
            Self::GradleCatalog => "toml",
            Self::BazelBuild => "bazel",
            Self::Buck2 => "",
            Self::Starlark => "bzl",
        }
    }

    /// Returns a human-readable name for this format.
    ///
    /// # Examples
    ///
    /// ```
    /// use antlers_lock::LockFormat;
    ///
    /// assert_eq!(LockFormat::Antlers.name(), "antlers-lock");
    /// assert_eq!(LockFormat::RulesJvmV2.name(), "rules_jvm_external v2");
    /// assert_eq!(LockFormat::GradleCatalog.name(), "Gradle version catalog");
    /// ```
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Antlers => "antlers-lock",
            Self::RulesJvmV1 => "rules_jvm_external v1",
            Self::RulesJvmV2 => "rules_jvm_external v2",
            Self::Coursier => "Coursier",
            Self::GradleLockfile => "Gradle lockfile",
            Self::GradleCatalog => "Gradle version catalog",
            Self::BazelBuild => "Bazel BUILD",
            Self::Buck2 => "Buck2",
            Self::Starlark => "Starlark",
        }
    }

    /// Returns a short identifier for this format (useful for CLI flags).
    ///
    /// # Examples
    ///
    /// ```
    /// use antlers_lock::LockFormat;
    ///
    /// assert_eq!(LockFormat::Antlers.id(), "antlers");
    /// assert_eq!(LockFormat::RulesJvmV2.id(), "rules-jvm-v2");
    /// assert_eq!(LockFormat::GradleCatalog.id(), "gradle-catalog");
    /// ```
    #[must_use]
    pub const fn id(&self) -> &'static str {
        match self {
            Self::Antlers => "antlers",
            Self::RulesJvmV1 => "rules-jvm-v1",
            Self::RulesJvmV2 => "rules-jvm-v2",
            Self::Coursier => "coursier",
            Self::GradleLockfile => "gradle-lockfile",
            Self::GradleCatalog => "gradle-catalog",
            Self::BazelBuild => "bazel-build",
            Self::Buck2 => "buck2",
            Self::Starlark => "starlark",
        }
    }

    /// Parses a format from its ID string.
    ///
    /// # Examples
    ///
    /// ```
    /// use antlers_lock::LockFormat;
    ///
    /// assert_eq!(LockFormat::from_id("antlers"), Some(LockFormat::Antlers));
    /// assert_eq!(LockFormat::from_id("rules-jvm-v2"), Some(LockFormat::RulesJvmV2));
    /// assert_eq!(LockFormat::from_id("unknown"), None);
    /// ```
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        match id.to_lowercase().as_str() {
            "antlers" | "antlers-lock" => Some(Self::Antlers),
            "rules-jvm-v1" | "rje-v1" | "v1" => Some(Self::RulesJvmV1),
            "rules-jvm-v2" | "rje-v2" | "v2" | "rules-jvm" | "rje" => Some(Self::RulesJvmV2),
            "coursier" => Some(Self::Coursier),
            "gradle-lockfile" | "gradle" => Some(Self::GradleLockfile),
            "gradle-catalog" | "version-catalog" | "toml-catalog" => Some(Self::GradleCatalog),
            "bazel-build" | "bazel" | "build" => Some(Self::BazelBuild),
            "buck2" | "buck" => Some(Self::Buck2),
            "starlark" | "bzl" => Some(Self::Starlark),
            _ => None,
        }
    }

    /// Returns all supported formats.
    ///
    /// # Examples
    ///
    /// ```
    /// use antlers_lock::LockFormat;
    ///
    /// let formats = LockFormat::all();
    /// assert!(formats.contains(&LockFormat::Antlers));
    /// assert_eq!(formats.len(), 9);
    /// ```
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Antlers,
            Self::RulesJvmV1,
            Self::RulesJvmV2,
            Self::Coursier,
            Self::GradleLockfile,
            Self::GradleCatalog,
            Self::BazelBuild,
            Self::Buck2,
            Self::Starlark,
        ]
    }

    /// Returns all formats that support reading.
    ///
    /// # Examples
    ///
    /// ```
    /// use antlers_lock::LockFormat;
    ///
    /// let readable = LockFormat::readable();
    /// assert!(readable.iter().all(|f| f.can_read()));
    /// ```
    #[must_use]
    pub fn readable() -> Vec<Self> {
        Self::all().iter().copied().filter(Self::can_read).collect()
    }

    /// Returns all formats that support writing.
    ///
    /// # Examples
    ///
    /// ```
    /// use antlers_lock::LockFormat;
    ///
    /// let writable = LockFormat::writable();
    /// assert!(writable.iter().all(|f| f.can_write()));
    /// ```
    #[must_use]
    pub fn writable() -> Vec<Self> {
        Self::all()
            .iter()
            .copied()
            .filter(Self::can_write)
            .collect()
    }
}

impl std::fmt::Display for LockFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Extension detection tests
    // =========================================================================

    #[test]
    fn test_from_extension_json() {
        assert_eq!(
            LockFormat::from_extension("json"),
            Some(LockFormat::Antlers)
        );
        assert_eq!(
            LockFormat::from_extension("JSON"),
            Some(LockFormat::Antlers)
        );
    }

    #[test]
    fn test_from_extension_lockfile() {
        assert_eq!(
            LockFormat::from_extension("lockfile"),
            Some(LockFormat::GradleLockfile)
        );
    }

    #[test]
    fn test_from_extension_toml() {
        assert_eq!(
            LockFormat::from_extension("toml"),
            Some(LockFormat::GradleCatalog)
        );
    }

    #[test]
    fn test_from_extension_bzl() {
        assert_eq!(
            LockFormat::from_extension("bzl"),
            Some(LockFormat::Starlark)
        );
    }

    #[test]
    fn test_from_extension_bazel() {
        assert_eq!(
            LockFormat::from_extension("bazel"),
            Some(LockFormat::BazelBuild)
        );
    }

    #[test]
    fn test_from_extension_unknown() {
        assert_eq!(LockFormat::from_extension("txt"), None);
        assert_eq!(LockFormat::from_extension("xml"), None);
        assert_eq!(LockFormat::from_extension(""), None);
    }

    // =========================================================================
    // Path detection tests
    // =========================================================================

    #[test]
    fn test_from_path_antlers() {
        assert_eq!(
            LockFormat::from_path(Path::new("deps.lock.json")),
            Some(LockFormat::Antlers)
        );
        assert_eq!(
            LockFormat::from_path(Path::new("antlers.lock.json")),
            Some(LockFormat::Antlers)
        );
        assert_eq!(
            LockFormat::from_path(Path::new("my-project.lock.json")),
            Some(LockFormat::Antlers)
        );
    }

    #[test]
    fn test_from_path_rules_jvm() {
        assert_eq!(
            LockFormat::from_path(Path::new("maven_install.json")),
            Some(LockFormat::RulesJvmV2)
        );
        assert_eq!(
            LockFormat::from_path(Path::new("install.json")),
            Some(LockFormat::RulesJvmV2)
        );
        assert_eq!(
            LockFormat::from_path(Path::new("pinned_maven_install.json")),
            Some(LockFormat::RulesJvmV2)
        );
    }

    #[test]
    fn test_from_path_coursier() {
        assert_eq!(
            LockFormat::from_path(Path::new("deps.coursier.json")),
            Some(LockFormat::Coursier)
        );
        assert_eq!(
            LockFormat::from_path(Path::new("project.coursier.json")),
            Some(LockFormat::Coursier)
        );
    }

    #[test]
    fn test_from_path_gradle() {
        assert_eq!(
            LockFormat::from_path(Path::new("gradle.lockfile")),
            Some(LockFormat::GradleLockfile)
        );
        assert_eq!(
            LockFormat::from_path(Path::new("buildscript-gradle.lockfile")),
            Some(LockFormat::GradleLockfile)
        );
        assert_eq!(
            LockFormat::from_path(Path::new("libs.versions.toml")),
            Some(LockFormat::GradleCatalog)
        );
        assert_eq!(
            LockFormat::from_path(Path::new("gradle.versions.toml")),
            Some(LockFormat::GradleCatalog)
        );
    }

    #[test]
    fn test_from_path_bazel() {
        assert_eq!(
            LockFormat::from_path(Path::new("BUILD")),
            Some(LockFormat::BazelBuild)
        );
        assert_eq!(
            LockFormat::from_path(Path::new("BUILD.bazel")),
            Some(LockFormat::BazelBuild)
        );
    }

    #[test]
    fn test_from_path_buck() {
        assert_eq!(
            LockFormat::from_path(Path::new("BUCK")),
            Some(LockFormat::Buck2)
        );
    }

    #[test]
    fn test_from_path_starlark() {
        assert_eq!(
            LockFormat::from_path(Path::new("deps.bzl")),
            Some(LockFormat::Starlark)
        );
    }

    #[test]
    fn test_from_path_with_directory() {
        assert_eq!(
            LockFormat::from_path(Path::new("/some/path/maven_install.json")),
            Some(LockFormat::RulesJvmV2)
        );
        assert_eq!(
            LockFormat::from_path(Path::new("./gradle/libs.versions.toml")),
            Some(LockFormat::GradleCatalog)
        );
    }

    // =========================================================================
    // Content detection tests
    // =========================================================================

    #[test]
    fn test_detect_antlers() {
        let content = r#"{"version": "1", "format": "antlers-lock", "artifacts": {}}"#;
        assert_eq!(LockFormat::detect(content), Some(LockFormat::Antlers));
    }

    #[test]
    fn test_detect_rules_jvm_v2() {
        let content = r#"{"version": "2", "artifacts": {"com.example:lib": {}}}"#;
        assert_eq!(LockFormat::detect(content), Some(LockFormat::RulesJvmV2));
    }

    #[test]
    fn test_detect_rules_jvm_v1() {
        let content = r#"{"dependency_tree": {"version": "0.1.0", "dependencies": []}}"#;
        assert_eq!(LockFormat::detect(content), Some(LockFormat::RulesJvmV1));
    }

    #[test]
    fn test_detect_coursier() {
        let content =
            r#"{"dependencies": [{"coord": "com.example:lib:1.0", "file": "lib-1.0.jar"}]}"#;
        assert_eq!(LockFormat::detect(content), Some(LockFormat::Coursier));
    }

    #[test]
    fn test_detect_gradle_catalog() {
        let content = r#"
[versions]
guava = "33.0.0-jre"

[libraries]
guava = { group = "com.google.guava", name = "guava", version.ref = "guava" }
"#;
        assert_eq!(LockFormat::detect(content), Some(LockFormat::GradleCatalog));
    }

    #[test]
    fn test_detect_gradle_lockfile() {
        let content = r"# This is a Gradle generated file for dependency locking.
# Manual edits can break the build and are not advised.
# This file is expected to be part of source control.
com.google.guava:guava:33.0.0-jre=compileClasspath,runtimeClasspath
";
        assert_eq!(
            LockFormat::detect(content),
            Some(LockFormat::GradleLockfile)
        );
    }

    #[test]
    fn test_detect_bazel_build() {
        let content = r#"
java_library(
    name = "guava",
    deps = ["@maven//:com_google_guava_guava"],
)
"#;
        assert_eq!(LockFormat::detect(content), Some(LockFormat::BazelBuild));
    }

    #[test]
    fn test_detect_starlark() {
        let content = r#"
load("@rules_jvm_external//:defs.bzl", "maven_install")

def deps():
    maven_install(...)
"#;
        assert_eq!(LockFormat::detect(content), Some(LockFormat::Starlark));
    }

    #[test]
    fn test_detect_buck2() {
        let content = r#"
prebuilt_jar(
    name = "guava",
    binary_jar = "guava-33.0.0-jre.jar",
)
"#;
        assert_eq!(LockFormat::detect(content), Some(LockFormat::Buck2));
    }

    #[test]
    fn test_detect_unknown() {
        assert_eq!(LockFormat::detect("random text"), None);
        assert_eq!(LockFormat::detect("{}"), None);
        assert_eq!(LockFormat::detect("<xml/>"), None);
    }

    // =========================================================================
    // Capability tests
    // =========================================================================

    #[test]
    fn test_can_read() {
        // Readable formats
        assert!(LockFormat::Antlers.can_read());
        assert!(LockFormat::RulesJvmV1.can_read());
        assert!(LockFormat::RulesJvmV2.can_read());
        assert!(LockFormat::Coursier.can_read());
        assert!(LockFormat::GradleLockfile.can_read());
        assert!(LockFormat::GradleCatalog.can_read());

        // Write-only formats
        assert!(!LockFormat::BazelBuild.can_read());
        assert!(!LockFormat::Buck2.can_read());
        assert!(!LockFormat::Starlark.can_read());
    }

    #[test]
    fn test_can_write() {
        // Writable formats
        assert!(LockFormat::Antlers.can_write());
        assert!(LockFormat::RulesJvmV2.can_write());
        assert!(LockFormat::Coursier.can_write());
        assert!(LockFormat::GradleLockfile.can_write());
        assert!(LockFormat::GradleCatalog.can_write());
        assert!(LockFormat::BazelBuild.can_write());
        assert!(LockFormat::Buck2.can_write());
        assert!(LockFormat::Starlark.can_write());

        // Read-only formats (legacy)
        assert!(!LockFormat::RulesJvmV1.can_write());
    }

    // =========================================================================
    // Extension and name tests
    // =========================================================================

    #[test]
    fn test_extension() {
        assert_eq!(LockFormat::Antlers.extension(), "lock.json");
        assert_eq!(LockFormat::RulesJvmV1.extension(), "json");
        assert_eq!(LockFormat::RulesJvmV2.extension(), "json");
        assert_eq!(LockFormat::Coursier.extension(), "coursier.json");
        assert_eq!(LockFormat::GradleLockfile.extension(), "lockfile");
        assert_eq!(LockFormat::GradleCatalog.extension(), "toml");
        assert_eq!(LockFormat::BazelBuild.extension(), "bazel");
        assert_eq!(LockFormat::Buck2.extension(), "");
        assert_eq!(LockFormat::Starlark.extension(), "bzl");
    }

    #[test]
    fn test_name() {
        assert_eq!(LockFormat::Antlers.name(), "antlers-lock");
        assert_eq!(LockFormat::RulesJvmV1.name(), "rules_jvm_external v1");
        assert_eq!(LockFormat::RulesJvmV2.name(), "rules_jvm_external v2");
        assert_eq!(LockFormat::Coursier.name(), "Coursier");
        assert_eq!(LockFormat::GradleLockfile.name(), "Gradle lockfile");
        assert_eq!(LockFormat::GradleCatalog.name(), "Gradle version catalog");
        assert_eq!(LockFormat::BazelBuild.name(), "Bazel BUILD");
        assert_eq!(LockFormat::Buck2.name(), "Buck2");
        assert_eq!(LockFormat::Starlark.name(), "Starlark");
    }

    #[test]
    fn test_id() {
        assert_eq!(LockFormat::Antlers.id(), "antlers");
        assert_eq!(LockFormat::RulesJvmV1.id(), "rules-jvm-v1");
        assert_eq!(LockFormat::RulesJvmV2.id(), "rules-jvm-v2");
        assert_eq!(LockFormat::Coursier.id(), "coursier");
        assert_eq!(LockFormat::GradleLockfile.id(), "gradle-lockfile");
        assert_eq!(LockFormat::GradleCatalog.id(), "gradle-catalog");
        assert_eq!(LockFormat::BazelBuild.id(), "bazel-build");
        assert_eq!(LockFormat::Buck2.id(), "buck2");
        assert_eq!(LockFormat::Starlark.id(), "starlark");
    }

    #[test]
    fn test_from_id() {
        // Standard IDs
        assert_eq!(LockFormat::from_id("antlers"), Some(LockFormat::Antlers));
        assert_eq!(
            LockFormat::from_id("rules-jvm-v1"),
            Some(LockFormat::RulesJvmV1)
        );
        assert_eq!(
            LockFormat::from_id("rules-jvm-v2"),
            Some(LockFormat::RulesJvmV2)
        );
        assert_eq!(LockFormat::from_id("coursier"), Some(LockFormat::Coursier));

        // Aliases
        assert_eq!(
            LockFormat::from_id("antlers-lock"),
            Some(LockFormat::Antlers)
        );
        assert_eq!(LockFormat::from_id("rje-v2"), Some(LockFormat::RulesJvmV2));
        assert_eq!(LockFormat::from_id("v2"), Some(LockFormat::RulesJvmV2));
        assert_eq!(
            LockFormat::from_id("gradle"),
            Some(LockFormat::GradleLockfile)
        );
        assert_eq!(LockFormat::from_id("buck"), Some(LockFormat::Buck2));

        // Case insensitive
        assert_eq!(LockFormat::from_id("ANTLERS"), Some(LockFormat::Antlers));

        // Unknown
        assert_eq!(LockFormat::from_id("unknown"), None);
    }

    // =========================================================================
    // Collection tests
    // =========================================================================

    #[test]
    fn test_all() {
        let all = LockFormat::all();
        assert_eq!(all.len(), 9);
        assert!(all.contains(&LockFormat::Antlers));
        assert!(all.contains(&LockFormat::RulesJvmV1));
        assert!(all.contains(&LockFormat::RulesJvmV2));
        assert!(all.contains(&LockFormat::Coursier));
        assert!(all.contains(&LockFormat::GradleLockfile));
        assert!(all.contains(&LockFormat::GradleCatalog));
        assert!(all.contains(&LockFormat::BazelBuild));
        assert!(all.contains(&LockFormat::Buck2));
        assert!(all.contains(&LockFormat::Starlark));
    }

    #[test]
    fn test_readable() {
        let readable = LockFormat::readable();
        assert_eq!(readable.len(), 6);
        assert!(readable.iter().all(LockFormat::can_read));
        assert!(!readable.contains(&LockFormat::BazelBuild));
        assert!(!readable.contains(&LockFormat::Buck2));
        assert!(!readable.contains(&LockFormat::Starlark));
    }

    #[test]
    fn test_writable() {
        let writable = LockFormat::writable();
        assert_eq!(writable.len(), 8);
        assert!(writable.iter().all(LockFormat::can_write));
        assert!(!writable.contains(&LockFormat::RulesJvmV1));
    }

    // =========================================================================
    // Display test
    // =========================================================================

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", LockFormat::Antlers), "antlers-lock");
        assert_eq!(
            format!("{}", LockFormat::RulesJvmV2),
            "rules_jvm_external v2"
        );
    }
}
