//! Gradle Module Metadata JSON parsing.
//!
//! This module provides the [`GradleModuleParser`] for parsing `.module` files.

use crate::{Error, GradleModule, Result};

/// Parser for Gradle Module Metadata files.
///
/// Gradle Module Metadata is a JSON format with the extension `.module`.
/// This parser handles format versions 1.0 and 1.1.
///
/// # Example
///
/// ```
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
/// assert_eq!(module.component.module, "library");
/// assert_eq!(module.component.version, "1.0.0");
/// ```
pub struct GradleModuleParser;

impl GradleModuleParser {
    /// Parses a Gradle Module Metadata JSON string.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The JSON is malformed
    /// - The format version is unsupported
    /// - Required fields are missing
    pub fn parse(json: &str) -> Result<GradleModule> {
        let module: GradleModule = serde_json::from_str(json)?;

        // Validate format version
        Self::validate_format_version(&module.format_version)?;

        Ok(module)
    }

    /// Parses from a byte slice (useful for reading from files).
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails.
    pub fn parse_bytes(bytes: &[u8]) -> Result<GradleModule> {
        let module: GradleModule = serde_json::from_slice(bytes)?;

        Self::validate_format_version(&module.format_version)?;

        Ok(module)
    }

    /// Validates the format version.
    fn validate_format_version(version: &str) -> Result<()> {
        match version {
            "1.0" | "1.1" => Ok(()),
            other => Err(Error::UnsupportedVersion(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal() {
        let json = r#"{
            "formatVersion": "1.1",
            "component": {
                "group": "org.example",
                "module": "library",
                "version": "1.0.0"
            }
        }"#;

        let module = GradleModuleParser::parse(json).unwrap();
        assert_eq!(module.format_version, "1.1");
        assert_eq!(module.component.group, "org.example");
        assert_eq!(module.component.module, "library");
        assert_eq!(module.component.version, "1.0.0");
        assert!(module.variants.is_empty());
    }

    #[test]
    fn test_parse_with_variants() {
        let json = r#"{
            "formatVersion": "1.1",
            "component": {
                "group": "org.example",
                "module": "library",
                "version": "1.0.0"
            },
            "variants": [
                {
                    "name": "apiElements",
                    "attributes": {
                        "org.gradle.usage": "java-api",
                        "org.gradle.category": "library"
                    },
                    "dependencies": [
                        {
                            "group": "com.google.guava",
                            "module": "guava",
                            "version": {
                                "requires": "31.1-jre"
                            }
                        }
                    ],
                    "files": [
                        {
                            "name": "library-1.0.0.jar",
                            "url": "library-1.0.0.jar",
                            "size": 12345,
                            "sha256": "abc123def456"
                        }
                    ]
                },
                {
                    "name": "runtimeElements",
                    "attributes": {
                        "org.gradle.usage": "java-runtime",
                        "org.gradle.category": "library"
                    },
                    "dependencies": [
                        {
                            "group": "com.google.guava",
                            "module": "guava",
                            "version": {
                                "requires": "31.1-jre"
                            }
                        },
                        {
                            "group": "org.slf4j",
                            "module": "slf4j-api",
                            "version": {
                                "requires": "1.7.36"
                            }
                        }
                    ]
                }
            ]
        }"#;

        let module = GradleModuleParser::parse(json).unwrap();
        assert_eq!(module.variants.len(), 2);

        let api = module.api_variant().unwrap();
        assert_eq!(api.name, "apiElements");
        assert_eq!(api.dependencies.len(), 1);
        assert_eq!(api.files.len(), 1);
        assert_eq!(api.files[0].name, "library-1.0.0.jar");
        assert_eq!(api.files[0].sha256, Some("abc123def456".to_string()));

        let runtime = module.runtime_variant().unwrap();
        assert_eq!(runtime.name, "runtimeElements");
        assert_eq!(runtime.dependencies.len(), 2);
    }

    #[test]
    fn test_parse_with_dependency_constraints() {
        let json = r#"{
            "formatVersion": "1.1",
            "component": {
                "group": "org.example",
                "module": "platform",
                "version": "1.0.0",
                "attributes": {
                    "org.gradle.category": "platform"
                }
            },
            "variants": [
                {
                    "name": "apiElements",
                    "attributes": {
                        "org.gradle.usage": "java-api",
                        "org.gradle.category": "platform"
                    },
                    "dependencyConstraints": [
                        {
                            "group": "com.google.guava",
                            "module": "guava",
                            "version": {
                                "requires": "31.1-jre"
                            }
                        }
                    ]
                }
            ]
        }"#;

        let module = GradleModuleParser::parse(json).unwrap();
        assert!(module.is_platform());

        let api = module.api_variant().unwrap();
        assert_eq!(api.dependency_constraints.len(), 1);
        assert_eq!(api.dependency_constraints[0].group, "com.google.guava");
    }

    #[test]
    fn test_parse_with_exclusions() {
        let json = r#"{
            "formatVersion": "1.1",
            "component": {
                "group": "org.example",
                "module": "library",
                "version": "1.0.0"
            },
            "variants": [
                {
                    "name": "runtimeElements",
                    "attributes": {
                        "org.gradle.usage": "java-runtime"
                    },
                    "dependencies": [
                        {
                            "group": "org.springframework",
                            "module": "spring-core",
                            "version": {
                                "requires": "5.3.20"
                            },
                            "excludes": [
                                {
                                    "group": "commons-logging",
                                    "module": "commons-logging"
                                }
                            ]
                        }
                    ]
                }
            ]
        }"#;

        let module = GradleModuleParser::parse(json).unwrap();
        let runtime = module.runtime_variant().unwrap();
        assert_eq!(runtime.dependencies[0].excludes.len(), 1);
        assert!(runtime.dependencies[0].excludes[0].matches("commons-logging", "commons-logging"));
    }

    #[test]
    fn test_parse_with_available_at() {
        let json = r#"{
            "formatVersion": "1.1",
            "component": {
                "group": "org.example",
                "module": "library",
                "version": "1.0.0"
            },
            "variants": [
                {
                    "name": "runtimeElements",
                    "attributes": {
                        "org.gradle.usage": "java-runtime"
                    },
                    "availableAt": {
                        "url": "../library-jvm/1.0.0/library-jvm-1.0.0.module",
                        "group": "org.example",
                        "module": "library-jvm",
                        "version": "1.0.0"
                    }
                }
            ]
        }"#;

        let module = GradleModuleParser::parse(json).unwrap();
        let variant = &module.variants[0];
        assert!(variant.is_redirect());

        let available_at = variant.available_at.as_ref().unwrap();
        assert_eq!(available_at.group, "org.example");
        assert_eq!(available_at.module, "library-jvm");
        assert_eq!(available_at.version, "1.0.0");
    }

    #[test]
    fn test_parse_with_capabilities() {
        let json = r#"{
            "formatVersion": "1.1",
            "component": {
                "group": "org.example",
                "module": "library",
                "version": "1.0.0"
            },
            "variants": [
                {
                    "name": "runtimeElements",
                    "attributes": {
                        "org.gradle.usage": "java-runtime"
                    },
                    "capabilities": [
                        {
                            "group": "org.example",
                            "name": "library",
                            "version": "1.0.0"
                        },
                        {
                            "group": "org.example",
                            "name": "library-feature"
                        }
                    ]
                }
            ]
        }"#;

        let module = GradleModuleParser::parse(json).unwrap();
        let variant = &module.variants[0];
        assert_eq!(variant.capabilities.len(), 2);
        assert_eq!(variant.capabilities[0].version, Some("1.0.0".to_string()));
        assert_eq!(variant.capabilities[1].version, None);
    }

    #[test]
    fn test_parse_with_strict_version() {
        let json = r#"{
            "formatVersion": "1.1",
            "component": {
                "group": "org.example",
                "module": "library",
                "version": "1.0.0"
            },
            "variants": [
                {
                    "name": "runtimeElements",
                    "attributes": {
                        "org.gradle.usage": "java-runtime"
                    },
                    "dependencies": [
                        {
                            "group": "com.google.guava",
                            "module": "guava",
                            "version": {
                                "strictly": "31.1-jre",
                                "requires": "31.1-jre"
                            }
                        }
                    ]
                }
            ]
        }"#;

        let module = GradleModuleParser::parse(json).unwrap();
        let runtime = module.runtime_variant().unwrap();
        let dep = &runtime.dependencies[0];
        assert!(dep.version.as_ref().unwrap().is_strict());
        assert_eq!(dep.version_string(), Some("31.1-jre"));
    }

    #[test]
    fn test_parse_version_10() {
        let json = r#"{
            "formatVersion": "1.0",
            "component": {
                "group": "org.example",
                "module": "library",
                "version": "1.0.0"
            }
        }"#;

        let module = GradleModuleParser::parse(json).unwrap();
        assert_eq!(module.format_version, "1.0");
    }

    #[test]
    fn test_parse_unsupported_version() {
        let json = r#"{
            "formatVersion": "2.0",
            "component": {
                "group": "org.example",
                "module": "library",
                "version": "1.0.0"
            }
        }"#;

        let result = GradleModuleParser::parse(json);
        assert!(matches!(result, Err(Error::UnsupportedVersion(_))));
    }

    #[test]
    fn test_parse_invalid_json() {
        let json = "not valid json";
        let result = GradleModuleParser::parse(json);
        assert!(matches!(result, Err(Error::Json(_))));
    }

    #[test]
    fn test_parse_created_by() {
        let json = r#"{
            "formatVersion": "1.1",
            "component": {
                "group": "org.example",
                "module": "library",
                "version": "1.0.0"
            },
            "createdBy": {
                "gradle": {
                    "version": "7.4.2",
                    "buildId": "abc123"
                }
            }
        }"#;

        let module = GradleModuleParser::parse(json).unwrap();
        let created_by = module.created_by.unwrap();
        let gradle = created_by.gradle.unwrap();
        assert_eq!(gradle.version, "7.4.2");
        assert_eq!(gradle.build_id, Some("abc123".to_string()));
    }

    #[test]
    fn test_parse_bytes() {
        let json = r#"{
            "formatVersion": "1.1",
            "component": {
                "group": "org.example",
                "module": "library",
                "version": "1.0.0"
            }
        }"#;

        let module = GradleModuleParser::parse_bytes(json.as_bytes()).unwrap();
        assert_eq!(module.component.group, "org.example");
    }
}
