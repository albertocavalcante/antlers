//! Property substitution for Maven POM files.
//!
//! Maven POMs support property substitution using the `${property.name}` syntax.
//! This module provides types for storing and substituting properties.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Properties map for variable substitution.
///
/// Properties in Maven POMs are arbitrary key-value pairs where each XML element
/// name is the key and the text content is the value.
///
/// # Examples
///
/// ```
/// use maven_pom::Properties;
///
/// let mut props = Properties::new();
/// props.insert("kotlin.version".to_string(), "2.0.0".to_string());
///
/// assert_eq!(props.get("kotlin.version"), Some("2.0.0"));
/// assert_eq!(props.substitute("${kotlin.version}"), "2.0.0");
/// ```
#[derive(Debug, Clone, Default, Serialize)]
pub struct Properties {
    /// The property key-value pairs.
    values: HashMap<String, String>,
}

impl Properties {
    /// Creates a new empty Properties instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    /// Gets a property value by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    /// Inserts a property value.
    pub fn insert(&mut self, key: String, value: String) {
        self.values.insert(key, value);
    }

    /// Returns an iterator over the properties.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.values.iter()
    }

    /// Returns the number of properties.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns true if there are no properties.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Merges properties from another Properties instance.
    ///
    /// Properties from `other` are added, but existing properties in `self`
    /// are not overwritten. This is typically used to merge parent properties.
    pub fn merge_from(&mut self, other: &Self) {
        for (key, value) in &other.values {
            self.values
                .entry(key.clone())
                .or_insert_with(|| value.clone());
        }
    }

    /// Substitute `${property}` patterns in a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use maven_pom::Properties;
    ///
    /// let mut props = Properties::new();
    /// props.insert("version".to_string(), "1.0.0".to_string());
    ///
    /// assert_eq!(props.substitute("lib-${version}"), "lib-1.0.0");
    /// ```
    #[must_use]
    pub fn substitute(&self, s: &str) -> String {
        self.substitute_with_context(s, None, None, None, None, None)
    }

    /// Substitute properties with additional context.
    ///
    /// This handles special Maven properties:
    /// - `${project.version}` / `${pom.version}` / `${version}`
    /// - `${project.groupId}` / `${pom.groupId}`
    /// - `${project.artifactId}`
    /// - `${project.parent.version}` / `${parent.version}`
    /// - `${project.parent.groupId}` / `${parent.groupId}`
    #[must_use]
    pub fn substitute_with_context(
        &self,
        s: &str,
        project_version: Option<&str>,
        project_group: Option<&str>,
        project_artifact: Option<&str>,
        parent_version: Option<&str>,
        parent_group: Option<&str>,
    ) -> String {
        let mut result = s.to_string();

        // Substitute ${property} patterns from properties
        for (key, value) in &self.values {
            let pattern = format!("${{{key}}}");
            result = result.replace(&pattern, value);
        }

        // Handle ${project.version}, ${pom.version}, ${version}
        if let Some(version) = project_version {
            result = result.replace("${project.version}", version);
            result = result.replace("${pom.version}", version);
            #[allow(clippy::literal_string_with_formatting_args)]
            {
                result = result.replace("${version}", version);
            }
        }

        // Handle ${project.groupId}, ${pom.groupId}
        if let Some(group_id) = project_group {
            result = result.replace("${project.groupId}", group_id);
            result = result.replace("${pom.groupId}", group_id);
        }

        // Handle ${project.artifactId}
        if let Some(artifact_id) = project_artifact {
            result = result.replace("${project.artifactId}", artifact_id);
        }

        // Handle ${project.parent.version}, ${parent.version}
        if let Some(version) = parent_version {
            result = result.replace("${project.parent.version}", version);
            result = result.replace("${parent.version}", version);
        }

        // Handle ${project.parent.groupId}, ${parent.groupId}
        if let Some(group_id) = parent_group {
            result = result.replace("${project.parent.groupId}", group_id);
            result = result.replace("${parent.groupId}", group_id);
        }

        result
    }
}

impl<'de> Deserialize<'de> for Properties {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};

        struct PropertiesVisitor;

        impl<'de> Visitor<'de> for PropertiesVisitor {
            type Value = Properties;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a properties map")
            }

            fn visit_map<M>(self, mut access: M) -> std::result::Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut values = HashMap::new();

                while let Some(key) = access.next_key::<String>()? {
                    // quick-xml deserializes element content as { "$text": "value" }
                    // or sometimes as a plain string
                    let value: serde_json::Value = access.next_value()?;
                    let text = match value {
                        serde_json::Value::String(s) => s,
                        serde_json::Value::Object(map) => map
                            .get("$text")
                            .or_else(|| map.get("$value"))
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .unwrap_or_default(),
                        _ => String::new(),
                    };
                    if !text.is_empty() {
                        values.insert(key, text);
                    }
                }

                Ok(Properties { values })
            }

            fn visit_unit<E>(self) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Properties::default())
            }
        }

        deserializer.deserialize_map(PropertiesVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_substitution() {
        let mut props = Properties::new();
        props.insert("kotlin.version".to_string(), "2.0.0".to_string());
        props.insert("scala.version".to_string(), "3.4.0".to_string());

        assert_eq!(props.substitute("${kotlin.version}"), "2.0.0");
        assert_eq!(
            props.substitute("kotlin-${kotlin.version}-scala-${scala.version}"),
            "kotlin-2.0.0-scala-3.4.0"
        );
    }

    #[test]
    fn test_context_substitution() {
        let props = Properties::new();

        let result = props.substitute_with_context(
            "${project.groupId}:${project.artifactId}:${project.version}",
            Some("1.0.0"),
            Some("com.example"),
            Some("my-lib"),
            None,
            None,
        );
        assert_eq!(result, "com.example:my-lib:1.0.0");
    }

    #[test]
    fn test_parent_substitution() {
        let props = Properties::new();

        let result = props.substitute_with_context(
            "${parent.version}",
            None,
            None,
            None,
            Some("2.0.0"),
            None,
        );
        assert_eq!(result, "2.0.0");
    }

    #[test]
    fn test_merge_from() {
        let mut props = Properties::new();
        props.insert("a".to_string(), "1".to_string());

        let mut parent = Properties::new();
        parent.insert("a".to_string(), "2".to_string());
        parent.insert("b".to_string(), "3".to_string());

        props.merge_from(&parent);

        // Own property is not overwritten
        assert_eq!(props.get("a"), Some("1"));
        // Parent property is added
        assert_eq!(props.get("b"), Some("3"));
    }
}
