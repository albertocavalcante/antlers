//! Gradle variant attributes and matchers.
//!
//! Gradle Module Metadata uses attributes to describe variants and select
//! the appropriate one for a given consumer. This module provides:
//! - [`Attributes`] - Key-value pairs describing a variant
//! - [`AttributeMatcher`] - Trait for matching attributes
//! - Built-in matchers for common attributes (usage, JVM version, etc.)

use std::collections::HashMap;
use std::fmt;

use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Gradle variant attributes (key-value pairs).
///
/// Attributes are used to describe the characteristics of a variant,
/// such as its intended usage (API vs runtime), target JVM version,
/// category, etc.
///
/// Note: GMM attribute values can be strings, numbers, or booleans in JSON.
/// This type normalizes all values to strings for consistent handling.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attributes(HashMap<String, String>);

impl Serialize for Attributes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Attributes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(AttributesVisitor)
    }
}

struct AttributesVisitor;

impl<'de> Visitor<'de> for AttributesVisitor {
    type Value = Attributes;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map of attribute key-value pairs")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut map = HashMap::with_capacity(access.size_hint().unwrap_or(0));

        while let Some((key, value)) = access.next_entry::<String, serde_json::Value>()? {
            // Convert any JSON value to a string
            let string_value = match value {
                serde_json::Value::String(s) => s,
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Null => String::new(),
                // For arrays/objects, use JSON representation
                other => other.to_string(),
            };
            map.insert(key, string_value);
        }

        Ok(Attributes(map))
    }
}

impl Attributes {
    /// Creates a new empty set of attributes.
    #[must_use]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Gets an attribute value by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }

    /// Inserts an attribute key-value pair.
    pub fn insert(&mut self, key: String, value: String) {
        self.0.insert(key, value);
    }

    /// Returns an iterator over the attributes.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.0.iter()
    }

    /// Returns true if the attributes are empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of attributes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl FromIterator<(String, String)> for Attributes {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// Common Gradle attribute keys.
pub mod keys {
    /// Usage attribute (java-api, java-runtime, etc.)
    pub const USAGE: &str = "org.gradle.usage";

    /// Category attribute (library, platform, etc.)
    pub const CATEGORY: &str = "org.gradle.category";

    /// JVM version attribute.
    pub const JVM_VERSION: &str = "org.gradle.jvm.version";

    /// Library elements attribute (jar, classes, resources).
    pub const LIBRARY_ELEMENTS: &str = "org.gradle.libraryelements";

    /// Dependency bundling attribute.
    pub const BUNDLING: &str = "org.gradle.dependency.bundling";

    /// Documentation type attribute.
    pub const DOCS_TYPE: &str = "org.gradle.docstype";

    /// Status attribute (release, integration, etc.)
    pub const STATUS: &str = "org.gradle.status";

    /// JVM environment attribute (standard-jvm, android, etc.)
    pub const JVM_ENVIRONMENT: &str = "org.gradle.jvm.environment";

    /// Kotlin platform type attribute (jvm, native, js, common, etc.)
    pub const KOTLIN_PLATFORM_TYPE: &str = "org.jetbrains.kotlin.platform.type";
}

/// Common Gradle attribute values.
pub mod values {
    // Usage values
    /// Java API usage (compile-time dependencies).
    pub const JAVA_API: &str = "java-api";
    /// Java runtime usage (runtime dependencies).
    pub const JAVA_RUNTIME: &str = "java-runtime";

    // Category values
    /// Library category.
    pub const LIBRARY: &str = "library";
    /// Platform category (BOM).
    pub const PLATFORM: &str = "platform";
    /// Enforced platform category.
    pub const ENFORCED_PLATFORM: &str = "enforced-platform";
    /// Documentation category.
    pub const DOCUMENTATION: &str = "documentation";

    // Library elements values
    /// JAR library element.
    pub const JAR: &str = "jar";
    /// Classes library element.
    pub const CLASSES: &str = "classes";
    /// Resources library element.
    pub const RESOURCES: &str = "resources";

    // Bundling values
    /// External dependencies bundling.
    pub const EXTERNAL: &str = "external";
    /// Embedded dependencies bundling.
    pub const EMBEDDED: &str = "embedded";
    /// Shadowed dependencies bundling.
    pub const SHADOWED: &str = "shadowed";
}

/// Trait for matching attributes to select variants.
///
/// Attribute matchers are used to select the most appropriate variant
/// for a given consumer. Each matcher returns a score indicating how
/// well the variant matches (higher is better), or `None` if it doesn't match.
pub trait AttributeMatcher: Send + Sync {
    /// Returns a match score if the attributes match, or `None` if they don't.
    ///
    /// Higher scores indicate better matches. A score of 0 means compatible
    /// but not preferred. `None` means incompatible.
    fn matches(&self, attrs: &Attributes) -> Option<u32>;

    /// Returns the name of this matcher for debugging.
    fn name(&self) -> &'static str;
}

/// Matcher for `org.gradle.usage = java-api`.
///
/// Matches variants intended for compilation (compile-time dependencies).
#[derive(Debug, Clone, Default)]
pub struct ApiMatcher;

impl AttributeMatcher for ApiMatcher {
    fn matches(&self, attrs: &Attributes) -> Option<u32> {
        match attrs.get(keys::USAGE) {
            Some(values::JAVA_API) => Some(100),
            Some(values::JAVA_RUNTIME) => Some(50), // Runtime can substitute for API
            Some(_) => None,
            None => Some(0), // No usage attribute, compatible but not preferred
        }
    }

    fn name(&self) -> &'static str {
        "ApiMatcher"
    }
}

/// Matcher for `org.gradle.usage = java-runtime`.
///
/// Matches variants intended for runtime (runtime dependencies).
#[derive(Debug, Clone, Default)]
pub struct RuntimeMatcher;

impl AttributeMatcher for RuntimeMatcher {
    fn matches(&self, attrs: &Attributes) -> Option<u32> {
        match attrs.get(keys::USAGE) {
            Some(values::JAVA_RUNTIME) => Some(100),
            Some(_) => None, // Other usages (including API) cannot substitute for runtime
            None => Some(0), // No usage attribute, compatible but not preferred
        }
    }

    fn name(&self) -> &'static str {
        "RuntimeMatcher"
    }
}

/// Matcher for `org.gradle.jvm.version >= N`.
///
/// Matches variants that are compatible with the specified JVM version.
/// A variant with JVM version <= requested is compatible.
#[derive(Debug, Clone)]
pub struct JvmVersionMatcher(pub u8);

impl JvmVersionMatcher {
    /// Creates a new JVM version matcher for the specified version.
    #[must_use]
    pub const fn new(version: u8) -> Self {
        Self(version)
    }
}

impl AttributeMatcher for JvmVersionMatcher {
    fn matches(&self, attrs: &Attributes) -> Option<u32> {
        attrs
            .get(keys::JVM_VERSION)
            .map_or(Some(50), |version_str| {
                version_str.parse::<u8>().ok().and_then(|variant_version| {
                    if variant_version <= self.0 {
                        // Prefer variants closer to the requested version
                        Some(100 - u32::from(self.0 - variant_version))
                    } else {
                        // Variant requires newer JVM than we have
                        None
                    }
                })
            })
    }

    fn name(&self) -> &'static str {
        "JvmVersionMatcher"
    }
}

/// Matcher for `org.gradle.category = library`.
///
/// Matches library variants (as opposed to platforms or documentation).
#[derive(Debug, Clone, Default)]
pub struct LibraryMatcher;

impl AttributeMatcher for LibraryMatcher {
    fn matches(&self, attrs: &Attributes) -> Option<u32> {
        match attrs.get(keys::CATEGORY) {
            Some(values::LIBRARY) => Some(100),
            Some(_) => None,  // Not a library
            None => Some(50), // No category, assume library
        }
    }

    fn name(&self) -> &'static str {
        "LibraryMatcher"
    }
}

/// Matcher for `org.gradle.libraryelements = jar`.
///
/// Matches variants that produce JAR files.
#[derive(Debug, Clone, Default)]
pub struct JarElementsMatcher;

impl AttributeMatcher for JarElementsMatcher {
    #[allow(clippy::match_same_arms)] // CLASSES and None have different semantic meanings
    fn matches(&self, attrs: &Attributes) -> Option<u32> {
        match attrs.get(keys::LIBRARY_ELEMENTS) {
            Some(values::JAR) => Some(100),
            Some(values::CLASSES) => Some(50), // Classes are acceptable
            Some(_) => None,
            None => Some(50), // No elements attribute, probably compatible
        }
    }

    fn name(&self) -> &'static str {
        "JarElementsMatcher"
    }
}

/// Composite matcher that requires all sub-matchers to match.
///
/// The final score is the sum of all sub-matcher scores.
#[derive(Default)]
pub struct AllOf(Vec<Box<dyn AttributeMatcher>>);

impl AllOf {
    /// Creates a new composite matcher.
    #[must_use]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Adds a matcher to this composite.
    #[must_use]
    pub fn with(mut self, matcher: impl AttributeMatcher + 'static) -> Self {
        self.0.push(Box::new(matcher));
        self
    }

    /// Creates a matcher for API usage with library category.
    #[must_use]
    pub fn api() -> Self {
        Self::new()
            .with(ApiMatcher)
            .with(LibraryMatcher)
            .with(JarElementsMatcher)
    }

    /// Creates a matcher for runtime usage with library category.
    #[must_use]
    pub fn runtime() -> Self {
        Self::new()
            .with(RuntimeMatcher)
            .with(LibraryMatcher)
            .with(JarElementsMatcher)
    }

    /// Creates a matcher for a specific JVM version with runtime usage.
    #[must_use]
    pub fn runtime_for_jvm(version: u8) -> Self {
        Self::new()
            .with(RuntimeMatcher)
            .with(LibraryMatcher)
            .with(JarElementsMatcher)
            .with(JvmVersionMatcher(version))
    }
}

impl AttributeMatcher for AllOf {
    fn matches(&self, attrs: &Attributes) -> Option<u32> {
        let mut total_score = 0u32;
        for matcher in &self.0 {
            match matcher.matches(attrs) {
                Some(score) => total_score += score,
                None => return None,
            }
        }
        Some(total_score)
    }

    fn name(&self) -> &'static str {
        "AllOf"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attributes_basic() {
        let mut attrs = Attributes::new();
        assert!(attrs.is_empty());

        attrs.insert("key".to_string(), "value".to_string());
        assert_eq!(attrs.get("key"), Some("value"));
        assert_eq!(attrs.get("nonexistent"), None);
        assert_eq!(attrs.len(), 1);
    }

    #[test]
    fn test_api_matcher() {
        let matcher = ApiMatcher;

        let mut api_attrs = Attributes::new();
        api_attrs.insert(keys::USAGE.to_string(), values::JAVA_API.to_string());
        assert_eq!(matcher.matches(&api_attrs), Some(100));

        let mut runtime_attrs = Attributes::new();
        runtime_attrs.insert(keys::USAGE.to_string(), values::JAVA_RUNTIME.to_string());
        assert_eq!(matcher.matches(&runtime_attrs), Some(50));

        let empty_attrs = Attributes::new();
        assert_eq!(matcher.matches(&empty_attrs), Some(0));
    }

    #[test]
    fn test_runtime_matcher() {
        let matcher = RuntimeMatcher;

        let mut runtime_attrs = Attributes::new();
        runtime_attrs.insert(keys::USAGE.to_string(), values::JAVA_RUNTIME.to_string());
        assert_eq!(matcher.matches(&runtime_attrs), Some(100));

        let mut api_attrs = Attributes::new();
        api_attrs.insert(keys::USAGE.to_string(), values::JAVA_API.to_string());
        assert_eq!(matcher.matches(&api_attrs), None);
    }

    #[test]
    fn test_jvm_version_matcher() {
        let matcher = JvmVersionMatcher(11);

        let mut jvm8_attrs = Attributes::new();
        jvm8_attrs.insert(keys::JVM_VERSION.to_string(), "8".to_string());
        assert!(matcher.matches(&jvm8_attrs).is_some());

        let mut jvm11_attrs = Attributes::new();
        jvm11_attrs.insert(keys::JVM_VERSION.to_string(), "11".to_string());
        assert_eq!(matcher.matches(&jvm11_attrs), Some(100));

        let mut jvm17_attrs = Attributes::new();
        jvm17_attrs.insert(keys::JVM_VERSION.to_string(), "17".to_string());
        assert_eq!(matcher.matches(&jvm17_attrs), None);
    }

    #[test]
    fn test_all_of_matcher() {
        let matcher = AllOf::runtime();

        let mut good_attrs = Attributes::new();
        good_attrs.insert(keys::USAGE.to_string(), values::JAVA_RUNTIME.to_string());
        good_attrs.insert(keys::CATEGORY.to_string(), values::LIBRARY.to_string());
        good_attrs.insert(keys::LIBRARY_ELEMENTS.to_string(), values::JAR.to_string());
        assert!(matcher.matches(&good_attrs).is_some());

        let mut bad_attrs = Attributes::new();
        bad_attrs.insert(keys::USAGE.to_string(), values::JAVA_API.to_string());
        bad_attrs.insert(keys::CATEGORY.to_string(), values::PLATFORM.to_string());
        assert_eq!(matcher.matches(&bad_attrs), None);
    }
}
