//! Gradle Module Metadata main structures.
//!
//! This module contains the main [`GradleModule`] struct that represents
//! a parsed `.module` file.

use serde::{Deserialize, Serialize};

use crate::attributes::{AttributeMatcher, keys, values};
use crate::{Attributes, Variant};

/// A parsed Gradle Module Metadata file.
///
/// Gradle Module Metadata (GMM) is a JSON format that describes a module's
/// variants, dependencies, and artifacts. It provides richer information
/// than Maven POMs, including variant-aware dependency resolution.
///
/// # Example
///
/// ```ignore
/// use gradle_module_metadata::GradleModuleParser;
///
/// let json = r#"{
///     "formatVersion": "1.1",
///     "component": {
///         "group": "org.example",
///         "module": "library",
///         "version": "1.0.0"
///     },
///     "variants": []
/// }"#;
///
/// let module = GradleModuleParser::parse(json).unwrap();
/// assert_eq!(module.component.group, "org.example");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradleModule {
    /// The format version of the metadata (e.g., "1.1").
    pub format_version: String,

    /// The component (module) being described.
    pub component: Component,

    /// Information about what created this metadata.
    #[serde(default)]
    pub created_by: Option<CreatedBy>,

    /// The variants of this module.
    #[serde(default)]
    pub variants: Vec<Variant>,
}

impl GradleModule {
    /// Selects the best variant matching the given matchers.
    ///
    /// Returns the variant with the highest combined score from all matchers.
    /// Returns `None` if no variant matches all matchers.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use gradle_module_metadata::{GradleModule, RuntimeMatcher, LibraryMatcher};
    ///
    /// let matchers: Vec<&dyn AttributeMatcher> = vec![&RuntimeMatcher, &LibraryMatcher];
    /// if let Some(variant) = module.select_variant(&matchers) {
    ///     println!("Selected variant: {}", variant.name);
    /// }
    /// ```
    #[must_use]
    pub fn select_variant(&self, matchers: &[&dyn AttributeMatcher]) -> Option<&Variant> {
        let mut best: Option<(&Variant, u32)> = None;

        for variant in &self.variants {
            // Skip redirects
            if variant.is_redirect() {
                continue;
            }

            let mut total_score = 0u32;
            let mut all_match = true;

            for matcher in matchers {
                if let Some(score) = matcher.matches(&variant.attributes) {
                    total_score += score;
                } else {
                    all_match = false;
                    break;
                }
            }

            if all_match && best.as_ref().is_none_or(|(_, s)| total_score > *s) {
                best = Some((variant, total_score));
            }
        }

        best.map(|(v, _)| v)
    }

    /// Gets the API variant (java-api usage).
    ///
    /// This is typically used for compile-time dependencies.
    #[must_use]
    pub fn api_variant(&self) -> Option<&Variant> {
        self.variants
            .iter()
            .find(|v| v.attributes.get(keys::USAGE) == Some(values::JAVA_API) && !v.is_redirect())
    }

    /// Gets the runtime variant (java-runtime usage).
    ///
    /// This is typically used for runtime dependencies.
    #[must_use]
    pub fn runtime_variant(&self) -> Option<&Variant> {
        self.variants.iter().find(|v| {
            v.attributes.get(keys::USAGE) == Some(values::JAVA_RUNTIME) && !v.is_redirect()
        })
    }

    /// Gets a variant by name.
    #[must_use]
    pub fn variant_by_name(&self, name: &str) -> Option<&Variant> {
        self.variants.iter().find(|v| v.name == name)
    }

    /// Gets the coordinates as "group:module:version".
    #[must_use]
    pub fn coordinates(&self) -> String {
        format!(
            "{}:{}:{}",
            self.component.group, self.component.module, self.component.version
        )
    }

    /// Returns all variants that match the given usage attribute.
    #[must_use]
    pub fn variants_for_usage(&self, usage: &str) -> Vec<&Variant> {
        self.variants
            .iter()
            .filter(|v| v.attributes.get(keys::USAGE) == Some(usage) && !v.is_redirect())
            .collect()
    }

    /// Returns true if this module has any variants with files.
    #[must_use]
    pub fn has_artifacts(&self) -> bool {
        self.variants.iter().any(|v| !v.files.is_empty())
    }

    /// Returns true if this is a platform/BOM module.
    #[must_use]
    pub fn is_platform(&self) -> bool {
        self.component.attributes.get(keys::CATEGORY) == Some(values::PLATFORM)
            || self.component.attributes.get(keys::CATEGORY) == Some(values::ENFORCED_PLATFORM)
    }
}

/// The component (module) being described.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    /// The group ID (e.g., "org.apache.commons").
    pub group: String,

    /// The module/artifact ID (e.g., "commons-lang3").
    pub module: String,

    /// The version (e.g., "3.12.0").
    pub version: String,

    /// Optional URL for the module.
    #[serde(default)]
    pub url: Option<String>,

    /// Attributes of the component.
    #[serde(default)]
    pub attributes: Attributes,
}

impl Component {
    /// Returns the coordinates as "group:module".
    #[must_use]
    pub fn group_module(&self) -> String {
        format!("{}:{}", self.group, self.module)
    }

    /// Returns the full coordinates as "group:module:version".
    #[must_use]
    pub fn coordinates(&self) -> String {
        format!("{}:{}:{}", self.group, self.module, self.version)
    }
}

/// Information about what created this metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreatedBy {
    /// Gradle build information.
    #[serde(default)]
    pub gradle: Option<GradleInfo>,
}

/// Gradle build information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradleInfo {
    /// The Gradle version used.
    pub version: String,

    /// Optional build ID.
    #[serde(default)]
    pub build_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::{ApiMatcher, RuntimeMatcher};

    fn create_test_module() -> GradleModule {
        let api_variant = Variant {
            name: "apiElements".to_string(),
            attributes: [
                (keys::USAGE.to_string(), values::JAVA_API.to_string()),
                (keys::CATEGORY.to_string(), values::LIBRARY.to_string()),
            ]
            .into_iter()
            .collect(),
            dependencies: vec![],
            dependency_constraints: vec![],
            files: vec![],
            capabilities: vec![],
            available_at: None,
        };

        let runtime_variant = Variant {
            name: "runtimeElements".to_string(),
            attributes: [
                (keys::USAGE.to_string(), values::JAVA_RUNTIME.to_string()),
                (keys::CATEGORY.to_string(), values::LIBRARY.to_string()),
            ]
            .into_iter()
            .collect(),
            dependencies: vec![],
            dependency_constraints: vec![],
            files: vec![],
            capabilities: vec![],
            available_at: None,
        };

        GradleModule {
            format_version: "1.1".to_string(),
            component: Component {
                group: "org.example".to_string(),
                module: "library".to_string(),
                version: "1.0.0".to_string(),
                url: None,
                attributes: Attributes::new(),
            },
            created_by: None,
            variants: vec![api_variant, runtime_variant],
        }
    }

    #[test]
    fn test_api_variant() {
        let module = create_test_module();
        let api = module.api_variant().unwrap();
        assert_eq!(api.name, "apiElements");
    }

    #[test]
    fn test_runtime_variant() {
        let module = create_test_module();
        let runtime = module.runtime_variant().unwrap();
        assert_eq!(runtime.name, "runtimeElements");
    }

    #[test]
    fn test_select_variant_with_matchers() {
        let module = create_test_module();

        let api_matcher = ApiMatcher;
        let matchers: Vec<&dyn AttributeMatcher> = vec![&api_matcher];
        let selected = module.select_variant(&matchers).unwrap();
        assert_eq!(selected.name, "apiElements");

        let runtime_matcher = RuntimeMatcher;
        let matchers: Vec<&dyn AttributeMatcher> = vec![&runtime_matcher];
        let selected = module.select_variant(&matchers).unwrap();
        assert_eq!(selected.name, "runtimeElements");
    }

    #[test]
    fn test_coordinates() {
        let module = create_test_module();
        assert_eq!(module.coordinates(), "org.example:library:1.0.0");
        assert_eq!(module.component.group_module(), "org.example:library");
    }

    #[test]
    fn test_variant_by_name() {
        let module = create_test_module();
        let variant = module.variant_by_name("apiElements").unwrap();
        assert_eq!(variant.name, "apiElements");
        assert!(module.variant_by_name("nonexistent").is_none());
    }
}
