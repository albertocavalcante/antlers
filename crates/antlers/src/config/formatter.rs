//! TOML formatter for canonical, deterministic output.
//!
//! This module provides formatting functionality to ensure `antlers.toml` files
//! have consistent ordering and formatting, enabling idempotent `antlers fmt` operations.
//!
//! # Canonical Ordering
//!
//! Sections are ordered semantically:
//! 1. `project` (identity first)
//! 2. `repositories` (where to get artifacts)
//! 3. `dependencies` / `dev-dependencies` / `build-dependencies`
//! 4. `constraints` (BOMs)
//! 5. `exclusions`
//! 6. `resolver`
//! 7. `cache`
//! 8. `network`
//! 9. `env`
//! 10. `output`
//!
//! Within sections:
//! - Dependencies: Alphabetical by coordinate
//! - Repositories: Preserve declaration order (priority matters)
//! - Table keys: Semantic priority (`id`, `name`, `url` first), then alphabetical

use similar::{ChangeTag, TextDiff};
use toml_edit::{Array, DocumentMut, Formatted, Item, Key, Table, Value};

/// Section ordering priority (lower = earlier in file).
const SECTION_ORDER: &[&str] = &[
    "project",
    "repositories",
    "dependencies",
    "dev-dependencies",
    "build-dependencies",
    "constraints",
    "exclusions",
    "resolver",
    "cache",
    "network",
    "env",
    "output",
];

/// Key priority within repository tables.
const REPOSITORY_KEY_ORDER: &[&str] = &["id", "name", "url", "ecosystem", "credentials"];

/// Key priority within dependency detail tables.
const DEPENDENCY_KEY_ORDER: &[&str] = &[
    "version",
    "scope",
    "classifier",
    "type",
    "exclusions",
    "transitive",
];

/// Key priority within credentials tables.
const CREDENTIALS_KEY_ORDER: &[&str] = &["type", "username", "password", "token"];

/// TOML formatter for canonical output.
pub struct TomlFormatter {
    /// Whether to add blank lines between sections.
    section_spacing: bool,
    /// Maximum inline table width before switching to multi-line.
    inline_table_width: usize,
}

impl Default for TomlFormatter {
    fn default() -> Self {
        Self {
            section_spacing: true,
            inline_table_width: 60,
        }
    }
}

impl TomlFormatter {
    /// Creates a new formatter with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether to add blank lines between sections.
    #[must_use]
    pub const fn with_section_spacing(mut self, spacing: bool) -> Self {
        self.section_spacing = spacing;
        self
    }

    /// Sets the maximum width for inline tables.
    #[must_use]
    pub const fn with_inline_table_width(mut self, width: usize) -> Self {
        self.inline_table_width = width;
        self
    }

    /// Formats a TOML string with canonical ordering.
    ///
    /// # Errors
    ///
    /// Returns an error if the TOML cannot be parsed.
    pub fn format(&self, content: &str) -> Result<String, FormatError> {
        // Parse with toml crate to get ordered map structure
        let value: toml::Value = toml::from_str(content)?;

        // Reorder sections and serialize back
        let ordered = Self::reorder_toml_value(value);

        // Serialize back to string using toml (preserves indexmap order)
        let reordered_str = toml::to_string_pretty(&ordered)?;

        // Parse with toml_edit for final formatting adjustments
        let mut doc: DocumentMut = reordered_str.parse()?;

        // Format individual sections
        self.format_project(&mut doc);
        self.format_repositories(&mut doc);
        self.format_dependencies(&mut doc, "dependencies");
        self.format_dependencies(&mut doc, "dev-dependencies");
        self.format_dependencies(&mut doc, "build-dependencies");
        self.format_dependencies(&mut doc, "constraints");
        self.format_dependencies(&mut doc, "exclusions");

        Ok(doc.to_string())
    }

    /// Reorders a TOML value's top-level keys.
    fn reorder_toml_value(value: toml::Value) -> toml::Value {
        if let toml::Value::Table(table) = value {
            let priority: std::collections::HashMap<&str, usize> = SECTION_ORDER
                .iter()
                .enumerate()
                .map(|(i, k)| (*k, i))
                .collect();

            // Sort keys by priority
            let mut keys: Vec<_> = table.keys().collect();
            keys.sort_by_key(|k| *priority.get(k.as_str()).unwrap_or(&usize::MAX));

            // Build new table with sorted keys
            let mut ordered = toml::map::Map::new();
            for key in keys {
                if let Some(val) = table.get(key) {
                    ordered.insert(key.clone(), val.clone());
                }
            }

            toml::Value::Table(ordered)
        } else {
            value
        }
    }

    /// Formats a file in place.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, parsed, or written.
    pub fn format_file(&self, path: impl AsRef<std::path::Path>) -> Result<(), FormatError> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let formatted = self.format(&content)?;
        std::fs::write(path.as_ref(), formatted)?;
        Ok(())
    }

    /// Returns the diff between original and formatted content.
    #[must_use]
    pub fn diff(&self, original: &str, formatted: &str) -> String {
        let diff = TextDiff::from_lines(original, formatted);
        let mut output = String::new();

        for change in diff.iter_all_changes() {
            let prefix = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };
            output.push_str(prefix);
            output.push_str(change.value());
        }

        output
    }

    /// Checks if the content is already formatted.
    ///
    /// # Errors
    ///
    /// Returns an error if the content cannot be parsed.
    pub fn check(&self, content: &str) -> Result<bool, FormatError> {
        let formatted = self.format(content)?;
        Ok(content == formatted)
    }

    /// Formats the project section.
    fn format_project(&self, doc: &mut DocumentMut) {
        if let Some(Item::Table(table)) = doc.get_mut("project") {
            self.sort_table_keys(table, &["name", "version", "description"]);
        }
    }

    /// Formats the repositories array.
    fn format_repositories(&self, doc: &mut DocumentMut) {
        if let Some(Item::ArrayOfTables(array)) = doc.get_mut("repositories") {
            for table in array.iter_mut() {
                self.sort_table_keys(table, REPOSITORY_KEY_ORDER);

                // Format credentials sub-table
                if let Some(Item::Table(creds)) = table.get_mut("credentials") {
                    self.sort_table_keys(creds, CREDENTIALS_KEY_ORDER);
                }
            }
        }
    }

    /// Formats a dependencies-style section (alphabetical keys).
    fn format_dependencies(&self, doc: &mut DocumentMut, section: &str) {
        if let Some(Item::Table(table)) = doc.get_mut(section) {
            // Get keys and sort alphabetically
            let mut keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
            keys.sort();

            // Extract items
            let mut items: Vec<(String, Item)> = Vec::new();
            for key in &keys {
                if let Some(item) = table.remove(key) {
                    items.push((key.clone(), item));
                }
            }

            // Re-insert in sorted order
            for (key, item) in items {
                table.insert(&key, item);
            }

            // Format detail tables
            for (_key, item) in table.iter_mut() {
                if let Item::Table(detail) = item {
                    self.sort_table_keys(detail, DEPENDENCY_KEY_ORDER);
                }
            }
        }
    }

    /// Sorts table keys according to priority list.
    #[allow(clippy::unused_self)]
    fn sort_table_keys(&self, table: &mut Table, priority: &[&str]) {
        let mut ordered_items: Vec<(Key, Item)> = Vec::new();

        // First, extract prioritized keys in order
        for key in priority {
            if let Some((orig_key, item)) = table.remove_entry(key) {
                ordered_items.push((orig_key, item));
            }
        }

        // Then extract remaining keys alphabetically
        let remaining: Vec<_> = table.iter().map(|(k, _)| k.to_string()).collect();
        let mut remaining_sorted = remaining;
        remaining_sorted.sort();

        for key in remaining_sorted {
            if let Some((orig_key, item)) = table.remove_entry(&key) {
                ordered_items.push((orig_key, item));
            }
        }

        // Re-insert in order
        for (key, item) in ordered_items {
            table.insert_formatted(&key, item);
        }
    }
}

// Helper functions for building TOML values programmatically.
// These are currently unused but may be useful for future features.

/// Creates a formatted inline table value.
#[allow(dead_code)]
pub fn inline_table<I>(items: I) -> Value
where
    I: IntoIterator<Item = (&'static str, Value)>,
{
    let mut table = toml_edit::InlineTable::new();
    for (key, value) in items {
        table.insert(key, value);
    }
    Value::InlineTable(table)
}

/// Creates a formatted string value.
#[allow(dead_code)]
pub fn string(s: impl AsRef<str>) -> Value {
    Value::String(Formatted::new(s.as_ref().to_string()))
}

/// Creates a formatted array value.
#[allow(dead_code)]
pub fn array<I>(items: I) -> Value
where
    I: IntoIterator<Item = Value>,
{
    let mut arr = Array::new();
    for item in items {
        arr.push(item);
    }
    Value::Array(arr)
}

/// Formatting errors.
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    /// TOML parse error.
    #[error("failed to parse TOML: {0}")]
    Parse(#[from] toml_edit::TomlError),

    /// TOML deserialization error.
    #[error("failed to parse TOML: {0}")]
    TomlDe(#[from] toml::de::Error),

    /// TOML serialization error.
    #[error("failed to serialize TOML: {0}")]
    TomlSer(#[from] toml::ser::Error),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
#[allow(clippy::needless_raw_string_hashes)]
mod tests {
    use super::*;

    #[test]
    fn test_section_reordering() {
        let input = r#"
[cache]
path = "/cache"

[dependencies]
"junit:junit" = "4.13.2"

[project]
name = "test"
"#;

        let fmt = TomlFormatter::new();
        let output = fmt.format(input).unwrap();

        // project should come before dependencies, which comes before cache
        let project_pos = output.find("[project]").unwrap();
        let deps_pos = output.find("[dependencies]").unwrap();
        let cache_pos = output.find("[cache]").unwrap();

        assert!(
            project_pos < deps_pos,
            "project should come before dependencies"
        );
        assert!(
            deps_pos < cache_pos,
            "dependencies should come before cache"
        );
    }

    #[test]
    fn test_dependency_alphabetization() {
        let input = r#"
[dependencies]
"org.apache.commons:commons-lang3" = "3.12.0"
"com.google.guava:guava" = "33.0.0-jre"
"junit:junit" = "4.13.2"
"#;

        let fmt = TomlFormatter::new();
        let output = fmt.format(input).unwrap();

        let guava_pos = output.find("com.google.guava").unwrap();
        let junit_pos = output.find("junit:junit").unwrap();
        let commons_pos = output.find("org.apache.commons").unwrap();

        assert!(guava_pos < junit_pos);
        assert!(junit_pos < commons_pos);
    }

    #[test]
    fn test_check_formatted() {
        let well_formatted = r#"[project]
name = "test"
version = "1.0.0"

[dependencies]
"com.google.guava:guava" = "33.0.0-jre"
"junit:junit" = "4.13.2"
"#;

        let fmt = TomlFormatter::new();
        assert!(fmt.check(well_formatted).unwrap());
    }

    #[test]
    fn test_check_unformatted() {
        let unformatted = r#"[dependencies]
"junit:junit" = "4.13.2"
"com.google.guava:guava" = "33.0.0-jre"

[project]
name = "test"
"#;

        let fmt = TomlFormatter::new();
        assert!(!fmt.check(unformatted).unwrap());
    }

    #[test]
    fn test_diff() {
        let original = "a\nb\nc\n";
        let modified = "a\nx\nc\n";

        let fmt = TomlFormatter::new();
        let diff = fmt.diff(original, modified);

        assert!(diff.contains("-b"));
        assert!(diff.contains("+x"));
    }

    #[test]
    fn test_repository_key_order() {
        let input = r#"
[[repositories]]
url = "https://example.com"
ecosystem = "maven"
id = "test"
name = "Test Repo"
"#;

        let fmt = TomlFormatter::new();
        let output = fmt.format(input).unwrap();

        let id_pos = output.find("id =").unwrap();
        let name_pos = output.find("name =").unwrap();
        let url_pos = output.find("url =").unwrap();
        let eco_pos = output.find("ecosystem =").unwrap();

        assert!(id_pos < name_pos);
        assert!(name_pos < url_pos);
        assert!(url_pos < eco_pos);
    }

    #[test]
    fn test_idempotent() {
        let input = r#"
[cache]
path = "/cache"

[project]
name = "test"
version = "1.0.0"

[dependencies]
"org.apache:commons" = "1.0"
"com.google:guava" = "2.0"
"#;

        let fmt = TomlFormatter::new();
        let first = fmt.format(input).unwrap();
        let second = fmt.format(&first).unwrap();

        assert_eq!(first, second, "Formatting should be idempotent");
    }

    #[test]
    fn test_inline_table_helper() {
        let table = inline_table([("version", string("1.0.0")), ("scope", string("test"))]);

        if let Value::InlineTable(t) = table {
            assert!(t.contains_key("version"));
            assert!(t.contains_key("scope"));
        } else {
            panic!("Expected inline table");
        }
    }
}
