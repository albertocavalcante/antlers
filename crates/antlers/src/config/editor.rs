//! Comment-preserving TOML config editor.
//!
//! This module provides functionality to modify `antlers.toml` files while
//! preserving comments and formatting. Uses `toml_edit` for AST manipulation.
//!
//! # Example
//!
//! ```no_run
//! use antlers::config::ConfigEditor;
//!
//! let mut editor = ConfigEditor::open("antlers.toml").unwrap();
//! editor.add_dependency("com.google.guava:guava", "33.0.0-jre");
//! editor.save().unwrap();
//! ```

use std::path::{Path, PathBuf};

use gather::Ecosystem;
use toml_edit::{Array, DocumentMut, Formatted, InlineTable, Item, Table, Value};

use super::{FormatError, TomlFormatter};

/// Editor for `antlers.toml` files that preserves comments.
pub struct ConfigEditor {
    /// The parsed TOML document.
    doc: DocumentMut,
    /// Path to the file (if opened from disk).
    path: Option<PathBuf>,
    /// Whether the document has been modified.
    modified: bool,
}

impl ConfigEditor {
    /// Opens an existing `antlers.toml` file for editing.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, EditorError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        let doc: DocumentMut = content.parse()?;

        Ok(Self {
            doc,
            path: Some(path.to_path_buf()),
            modified: false,
        })
    }

    /// Creates a new editor from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns an error if the TOML cannot be parsed.
    pub fn parse(content: &str) -> Result<Self, EditorError> {
        let doc: DocumentMut = content.parse()?;

        Ok(Self {
            doc,
            path: None,
            modified: false,
        })
    }

    /// Creates a new, empty configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            doc: DocumentMut::new(),
            path: None,
            modified: false,
        }
    }

    /// Returns whether the document has been modified.
    #[must_use]
    pub const fn is_modified(&self) -> bool {
        self.modified
    }

    /// Saves the configuration to the original file.
    ///
    /// # Errors
    ///
    /// Returns an error if no path is set or the file cannot be written.
    pub fn save(&self) -> Result<(), EditorError> {
        let path = self.path.as_ref().ok_or(EditorError::NoPath)?;
        self.save_to(path)
    }

    /// Saves the configuration to a specific file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save_to(&self, path: impl AsRef<Path>) -> Result<(), EditorError> {
        std::fs::write(path.as_ref(), self.doc.to_string())?;
        Ok(())
    }

    /// Saves with canonical formatting applied.
    ///
    /// # Errors
    ///
    /// Returns an error if formatting or saving fails.
    pub fn save_formatted(&self, path: impl AsRef<Path>) -> Result<(), EditorError> {
        let content = self.doc.to_string();
        let toml_fmt = TomlFormatter::new();
        let output = toml_fmt.format(&content).map_err(EditorError::Format)?;
        std::fs::write(path.as_ref(), output)?;
        Ok(())
    }

    /// Sets the project name.
    pub fn set_project_name(&mut self, name: &str) {
        self.ensure_table("project");
        if let Some(Item::Table(project)) = self.doc.get_mut("project") {
            project["name"] = value_string(name);
        }
        self.modified = true;
    }

    /// Sets the project version.
    pub fn set_project_version(&mut self, version: &str) {
        self.ensure_table("project");
        if let Some(Item::Table(project)) = self.doc.get_mut("project") {
            project["version"] = value_string(version);
        }
        self.modified = true;
    }

    /// Adds a simple dependency with just a version.
    pub fn add_dependency(&mut self, coordinate: &str, version: &str) {
        self.ensure_table("dependencies");
        if let Some(Item::Table(deps)) = self.doc.get_mut("dependencies") {
            deps[coordinate] = value_string(version);
        }
        self.modified = true;
    }

    /// Adds a detailed dependency with options.
    pub fn add_dependency_detailed(
        &mut self,
        coordinate: &str,
        version: &str,
        scope: Option<&str>,
        classifier: Option<&str>,
    ) {
        self.ensure_table("dependencies");
        if let Some(Item::Table(deps)) = self.doc.get_mut("dependencies") {
            let mut detail = InlineTable::new();
            detail.insert(
                "version",
                Value::String(Formatted::new(version.to_string())),
            );

            if let Some(s) = scope {
                detail.insert("scope", Value::String(Formatted::new(s.to_string())));
            }
            if let Some(c) = classifier {
                detail.insert("classifier", Value::String(Formatted::new(c.to_string())));
            }

            deps[coordinate] = Item::Value(Value::InlineTable(detail));
        }
        self.modified = true;
    }

    /// Adds a dev dependency.
    pub fn add_dev_dependency(&mut self, coordinate: &str, version: &str) {
        self.ensure_table("dev-dependencies");
        if let Some(Item::Table(deps)) = self.doc.get_mut("dev-dependencies") {
            deps[coordinate] = value_string(version);
        }
        self.modified = true;
    }

    /// Removes a dependency.
    ///
    /// Returns `true` if the dependency was found and removed.
    pub fn remove_dependency(&mut self, coordinate: &str) -> bool {
        if let Some(Item::Table(deps)) = self.doc.get_mut("dependencies")
            && deps.remove(coordinate).is_some()
        {
            self.modified = true;
            return true;
        }
        false
    }

    /// Removes a dev dependency.
    ///
    /// Returns `true` if the dependency was found and removed.
    pub fn remove_dev_dependency(&mut self, coordinate: &str) -> bool {
        if let Some(Item::Table(deps)) = self.doc.get_mut("dev-dependencies")
            && deps.remove(coordinate).is_some()
        {
            self.modified = true;
            return true;
        }
        false
    }

    /// Adds a repository.
    pub fn add_repository(&mut self, id: &str, name: &str, url: &str, ecosystem: Ecosystem) {
        self.ensure_array_of_tables("repositories");

        let mut repo = Table::new();
        repo["id"] = value_string(id);
        repo["name"] = value_string(name);
        repo["url"] = value_string(url);

        if ecosystem != Ecosystem::Maven {
            repo["ecosystem"] = value_string(ecosystem.as_str());
        }

        if let Some(Item::ArrayOfTables(repos)) = self.doc.get_mut("repositories") {
            repos.push(repo);
        }
        self.modified = true;
    }

    /// Adds a repository with bearer token authentication.
    pub fn add_repository_with_bearer(
        &mut self,
        id: &str,
        name: &str,
        url: &str,
        token_env_var: &str,
    ) {
        self.ensure_array_of_tables("repositories");

        let mut repo = Table::new();
        repo["id"] = value_string(id);
        repo["name"] = value_string(name);
        repo["url"] = value_string(url);

        // Create credentials inline table
        let mut creds = Table::new();
        creds["type"] = value_string("bearer");

        let mut token = InlineTable::new();
        token.insert(
            "env",
            Value::String(Formatted::new(token_env_var.to_string())),
        );
        creds["token"] = Item::Value(Value::InlineTable(token));

        repo["credentials"] = Item::Table(creds);

        if let Some(Item::ArrayOfTables(repos)) = self.doc.get_mut("repositories") {
            repos.push(repo);
        }
        self.modified = true;
    }

    /// Removes a repository by ID.
    ///
    /// Returns `true` if the repository was found and removed.
    pub fn remove_repository(&mut self, id: &str) -> bool {
        if let Some(Item::ArrayOfTables(repos)) = self.doc.get_mut("repositories") {
            let initial_len = repos.len();
            repos.retain(|repo| {
                repo.get("id")
                    .and_then(|item| item.as_str())
                    .is_some_and(|repo_id| repo_id != id)
            });
            if repos.len() != initial_len {
                self.modified = true;
                return true;
            }
        }
        false
    }

    /// Adds a global exclusion.
    pub fn add_exclusion(&mut self, coordinate: &str, version_pattern: &str) {
        self.ensure_table("exclusions");
        if let Some(Item::Table(exclusions)) = self.doc.get_mut("exclusions") {
            exclusions[coordinate] = value_string(version_pattern);
        }
        self.modified = true;
    }

    /// Removes an exclusion.
    pub fn remove_exclusion(&mut self, coordinate: &str) -> bool {
        if let Some(Item::Table(exclusions)) = self.doc.get_mut("exclusions")
            && exclusions.remove(coordinate).is_some()
        {
            self.modified = true;
            return true;
        }
        false
    }

    /// Adds a constraint (BOM).
    pub fn add_constraint(&mut self, coordinate: &str, version: &str, constraint_type: &str) {
        self.ensure_table("constraints");
        if let Some(Item::Table(constraints)) = self.doc.get_mut("constraints") {
            let mut detail = InlineTable::new();
            detail.insert(
                "version",
                Value::String(Formatted::new(version.to_string())),
            );
            detail.insert(
                "type",
                Value::String(Formatted::new(constraint_type.to_string())),
            );

            constraints[coordinate] = Item::Value(Value::InlineTable(detail));
        }
        self.modified = true;
    }

    /// Sets the conflict resolution strategy.
    pub fn set_conflict_strategy(&mut self, strategy: &str) {
        self.ensure_table("resolver");
        if let Some(Item::Table(resolver)) = self.doc.get_mut("resolver") {
            resolver["conflict-strategy"] = value_string(strategy);
        }
        self.modified = true;
    }

    /// Sets whether transitive resolution is enabled.
    pub fn set_transitive(&mut self, enabled: bool) {
        self.ensure_table("resolver");
        if let Some(Item::Table(resolver)) = self.doc.get_mut("resolver") {
            resolver["transitive"] = Item::Value(Value::Boolean(Formatted::new(enabled)));
        }
        self.modified = true;
    }

    /// Sets the cache path.
    pub fn set_cache_path(&mut self, path: &str) {
        self.ensure_table("cache");
        if let Some(Item::Table(cache)) = self.doc.get_mut("cache") {
            cache["path"] = value_string(path);
        }
        self.modified = true;
    }

    /// Adds an allowed environment variable.
    pub fn add_allowed_env(&mut self, var: &str) {
        self.ensure_table("env");
        if let Some(Item::Table(env)) = self.doc.get_mut("env") {
            if let Some(Item::Value(Value::Array(arr))) = env.get_mut("allow") {
                // Check if already present
                let exists = arr.iter().any(|v| v.as_str().is_some_and(|s| s == var));
                if !exists {
                    arr.push(Value::String(Formatted::new(var.to_string())));
                }
            } else {
                let mut arr = Array::new();
                arr.push(Value::String(Formatted::new(var.to_string())));
                env["allow"] = Item::Value(Value::Array(arr));
            }
        }
        self.modified = true;
    }

    /// Ensures a table exists at the given key.
    fn ensure_table(&mut self, key: &str) {
        if self.doc.get(key).is_none() {
            self.doc[key] = Item::Table(Table::new());
        }
    }

    /// Ensures an array of tables exists at the given key.
    fn ensure_array_of_tables(&mut self, key: &str) {
        if self.doc.get(key).is_none() {
            self.doc[key] = Item::ArrayOfTables(toml_edit::ArrayOfTables::new());
        }
    }

    /// Gets all dependency coordinates.
    #[must_use]
    pub fn list_dependencies(&self) -> Vec<String> {
        self.doc
            .get("dependencies")
            .and_then(|item| item.as_table())
            .map(|table| table.iter().map(|(k, _)| k.to_string()).collect())
            .unwrap_or_default()
    }

    /// Gets all repository IDs.
    #[must_use]
    pub fn list_repositories(&self) -> Vec<&str> {
        self.doc
            .get("repositories")
            .and_then(|item| item.as_array_of_tables())
            .map(|repos| {
                repos
                    .iter()
                    .filter_map(|r| r.get("id").and_then(|v| v.as_str()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Default for ConfigEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ConfigEditor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.doc)
    }
}

/// Creates a string Item value.
fn value_string(s: &str) -> Item {
    Item::Value(Value::String(Formatted::new(s.to_string())))
}

/// Editor errors.
#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    /// TOML parse error.
    #[error("failed to parse TOML: {0}")]
    Parse(#[from] toml_edit::TomlError),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// No path set for save.
    #[error("no file path set; use save_to() instead")]
    NoPath,

    /// Formatting error.
    #[error("formatting error: {0}")]
    Format(#[source] FormatError),
}

#[cfg(test)]
#[allow(clippy::needless_raw_string_hashes)]
mod tests {
    use super::*;

    #[test]
    fn test_new_editor() {
        let editor = ConfigEditor::new();
        assert!(!editor.is_modified());
        assert!(editor.to_string().is_empty());
    }

    #[test]
    fn test_set_project() {
        let mut editor = ConfigEditor::new();
        editor.set_project_name("my-project");
        editor.set_project_version("1.0.0");

        assert!(editor.is_modified());
        let output = editor.to_string();
        assert!(output.contains("[project]"));
        assert!(output.contains("name = \"my-project\""));
        assert!(output.contains("version = \"1.0.0\""));
    }

    #[test]
    fn test_add_dependency() {
        let mut editor = ConfigEditor::new();
        editor.add_dependency("com.google.guava:guava", "33.0.0-jre");

        let output = editor.to_string();
        assert!(output.contains("[dependencies]"));
        assert!(output.contains("\"com.google.guava:guava\" = \"33.0.0-jre\""));
    }

    #[test]
    fn test_add_dependency_detailed() {
        let mut editor = ConfigEditor::new();
        editor.add_dependency_detailed("junit:junit", "4.13.2", Some("test"), None);

        let output = editor.to_string();
        assert!(output.contains("[dependencies]"));
        assert!(output.contains("junit:junit"));
        assert!(output.contains("version = \"4.13.2\""));
        assert!(output.contains("scope = \"test\""));
    }

    #[test]
    fn test_remove_dependency() {
        let toml = r#"
[dependencies]
"com.google.guava:guava" = "33.0.0-jre"
"junit:junit" = "4.13.2"
"#;
        let mut editor = ConfigEditor::parse(toml).unwrap();
        assert!(editor.remove_dependency("junit:junit"));
        assert!(!editor.remove_dependency("not:exists"));

        let output = editor.to_string();
        assert!(!output.contains("junit"));
        assert!(output.contains("guava"));
    }

    #[test]
    fn test_add_repository() {
        let mut editor = ConfigEditor::new();
        editor.add_repository(
            "central",
            "Maven Central",
            "https://repo1.maven.org/maven2/",
            Ecosystem::Maven,
        );

        let output = editor.to_string();
        assert!(output.contains("[[repositories]]"));
        assert!(output.contains("id = \"central\""));
        assert!(output.contains("name = \"Maven Central\""));
        assert!(!output.contains("ecosystem")); // Should skip default Maven
    }

    #[test]
    fn test_add_repository_with_ecosystem() {
        let mut editor = ConfigEditor::new();
        editor.add_repository(
            "npmjs",
            "npm Registry",
            "https://registry.npmjs.org",
            Ecosystem::Npm,
        );

        let output = editor.to_string();
        assert!(output.contains("ecosystem = \"npm\""));
    }

    #[test]
    fn test_add_repository_with_bearer() {
        let mut editor = ConfigEditor::new();
        editor.add_repository_with_bearer(
            "github",
            "GitHub Packages",
            "https://maven.pkg.github.com/owner/repo",
            "GITHUB_TOKEN",
        );

        let output = editor.to_string();
        assert!(output.contains("[repositories.credentials]"));
        assert!(output.contains("type = \"bearer\""));
        assert!(output.contains("GITHUB_TOKEN"));
    }

    #[test]
    fn test_remove_repository() {
        let toml = r#"
[[repositories]]
id = "central"
url = "https://repo1.maven.org/maven2/"

[[repositories]]
id = "google"
url = "https://maven.google.com/"
"#;
        let mut editor = ConfigEditor::parse(toml).unwrap();
        assert!(editor.remove_repository("central"));
        assert!(!editor.remove_repository("notexists"));

        let output = editor.to_string();
        assert!(!output.contains("central"));
        assert!(output.contains("google"));
    }

    #[test]
    fn test_add_exclusion() {
        let mut editor = ConfigEditor::new();
        editor.add_exclusion("commons-logging:commons-logging", "*");

        let output = editor.to_string();
        assert!(output.contains("[exclusions]"));
        assert!(output.contains("\"commons-logging:commons-logging\" = \"*\""));
    }

    #[test]
    fn test_add_constraint() {
        let mut editor = ConfigEditor::new();
        editor.add_constraint("com.fasterxml.jackson:jackson-bom", "2.16.0", "bom");

        let output = editor.to_string();
        assert!(output.contains("[constraints]"));
        assert!(output.contains("jackson-bom"));
        assert!(output.contains("version = \"2.16.0\""));
        assert!(output.contains("type = \"bom\""));
    }

    #[test]
    fn test_resolver_settings() {
        let mut editor = ConfigEditor::new();
        editor.set_conflict_strategy("strict");
        editor.set_transitive(false);

        let output = editor.to_string();
        assert!(output.contains("[resolver]"));
        assert!(output.contains("conflict-strategy = \"strict\""));
        assert!(output.contains("transitive = false"));
    }

    #[test]
    fn test_cache_path() {
        let mut editor = ConfigEditor::new();
        editor.set_cache_path(".antlers/cache");

        let output = editor.to_string();
        assert!(output.contains("[cache]"));
        assert!(output.contains("path = \".antlers/cache\""));
    }

    #[test]
    fn test_add_allowed_env() {
        let mut editor = ConfigEditor::new();
        editor.add_allowed_env("GITHUB_TOKEN");
        editor.add_allowed_env("HTTPS_PROXY");
        editor.add_allowed_env("GITHUB_TOKEN"); // Duplicate should be ignored

        let output = editor.to_string();
        assert!(output.contains("[env]"));
        assert!(output.contains("allow"));
        // Should only have one GITHUB_TOKEN
        assert_eq!(output.matches("GITHUB_TOKEN").count(), 1);
    }

    #[test]
    fn test_list_dependencies() {
        let toml = r#"
[dependencies]
"com.google.guava:guava" = "33.0.0-jre"
"junit:junit" = "4.13.2"
"#;
        let editor = ConfigEditor::parse(toml).unwrap();
        let deps = editor.list_dependencies();

        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"com.google.guava:guava".to_string()));
        assert!(deps.contains(&"junit:junit".to_string()));
    }

    #[test]
    fn test_list_repositories() {
        let toml = r#"
[[repositories]]
id = "central"
url = "https://repo1.maven.org/maven2/"

[[repositories]]
id = "google"
url = "https://maven.google.com/"
"#;
        let editor = ConfigEditor::parse(toml).unwrap();
        let repos = editor.list_repositories();

        assert_eq!(repos.len(), 2);
        assert!(repos.contains(&"central"));
        assert!(repos.contains(&"google"));
    }

    #[test]
    fn test_preserves_comments() {
        let toml = r#"# Project configuration
[project]
name = "test"

# Dependencies section
[dependencies]
# Core dependency
"com.google.guava:guava" = "33.0.0-jre"
"#;
        let mut editor = ConfigEditor::parse(toml).unwrap();
        editor.add_dependency("junit:junit", "4.13.2");

        let output = editor.to_string();
        assert!(output.contains("# Project configuration"));
        assert!(output.contains("# Dependencies section"));
        assert!(output.contains("# Core dependency"));
    }
}
