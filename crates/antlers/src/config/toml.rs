//! TOML configuration file types for antlers.
//!
//! This module provides the schema for `antlers.toml` configuration files,
//! with support for hermetic builds, repository configuration, and dependency management.
//!
//! # Schema Overview
//!
//! ```toml
//! [project]
//! name = "my-project"
//! version = "0.1.0"
//!
//! [[repositories]]
//! id = "central"
//! name = "Maven Central"
//! url = "https://repo1.maven.org/maven2/"
//! ecosystem = "maven"
//!
//! [dependencies]
//! "com.google.guava:guava" = "33.0.0-jre"
//!
//! [resolver]
//! conflict-strategy = "highest-wins"
//! transitive = true
//! ```

use std::path::PathBuf;

use gather::{Ecosystem, StringOrEnvRef};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// The main antlers.toml configuration file structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct AntlersToml {
    /// Project metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectConfig>,

    /// Repository configurations (order matters for priority).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repositories: Vec<RepositoryToml>,

    /// Main dependencies.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub dependencies: IndexMap<String, DependencySpec>,

    /// Development dependencies.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub dev_dependencies: IndexMap<String, DependencySpec>,

    /// Build-time dependencies.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub build_dependencies: IndexMap<String, DependencySpec>,

    /// Version constraints (BOMs, platform constraints).
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub constraints: IndexMap<String, ConstraintSpec>,

    /// Global exclusions.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub exclusions: IndexMap<String, String>,

    /// Resolver configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolver: Option<ResolverConfig>,

    /// Cache configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache: Option<CacheToml>,

    /// Network configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkToml>,

    /// Environment variable access control.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<EnvToml>,

    /// Output configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputConfig>,
}

impl AntlersToml {
    /// Creates a new empty configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses a TOML string into a configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the TOML is invalid or doesn't match the schema.
    pub fn parse(content: &str) -> Result<Self, TomlError> {
        toml::from_str(content).map_err(TomlError::Parse)
    }

    /// Serializes the configuration to a TOML string.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn to_toml(&self) -> Result<String, TomlError> {
        toml::to_string_pretty(self).map_err(TomlError::Serialize)
    }

    /// Loads configuration from a file path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, TomlError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| TomlError::Io(path.as_ref().to_path_buf(), e))?;
        Self::parse(&content)
    }

    /// Saves the configuration to a file.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or writing fails.
    pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<(), TomlError> {
        let content = self.to_toml()?;
        std::fs::write(path.as_ref(), content)
            .map_err(|e| TomlError::Io(path.as_ref().to_path_buf(), e))
    }

    /// Returns all environment variables referenced in the config.
    #[must_use]
    pub fn referenced_env_vars(&self) -> Vec<&str> {
        let mut vars = Vec::new();

        // Collect from repository credentials
        for repo in &self.repositories {
            if let Some(creds) = &repo.credentials {
                creds.collect_env_refs(&mut vars);
            }
        }

        vars
    }

    /// Validates the configuration for hermetic mode.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration violates hermetic constraints.
    pub fn validate_hermetic(&self) -> Result<(), TomlError> {
        // Cache path must be explicit
        if let Some(cache) = &self.cache {
            if cache.path.is_none() {
                return Err(TomlError::HermeticViolation(
                    "cache.path must be explicit for hermetic builds".to_string(),
                ));
            }
        } else {
            return Err(TomlError::HermeticViolation(
                "cache section is required for hermetic builds".to_string(),
            ));
        }

        // All env refs must be in allowlist
        let env_refs = self.referenced_env_vars();
        if !env_refs.is_empty() {
            let allowed = self.env.as_ref().map_or(&[] as &[String], |e| &e.allow);
            for var in env_refs {
                if !allowed.iter().any(|a| a == var) {
                    return Err(TomlError::HermeticViolation(format!(
                        "environment variable '{var}' is referenced but not in env.allow list"
                    )));
                }
            }
        }

        Ok(())
    }
}

/// Project metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project name.
    pub name: String,
    /// Project version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Project description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Repository configuration in TOML format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryToml {
    /// Repository identifier.
    pub id: String,
    /// Human-readable name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Repository URL.
    pub url: String,
    /// Ecosystem type.
    #[serde(default)]
    pub ecosystem: Ecosystem,
    /// Credentials for authentication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credentials: Option<CredentialsToml>,
}

/// Credentials configuration for TOML.
///
/// Supports inline values or environment variable references.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CredentialsToml {
    /// HTTP Basic authentication.
    Basic {
        /// Username (inline or env ref).
        username: StringOrEnvRef,
        /// Password (inline or env ref).
        password: StringOrEnvRef,
    },
    /// Bearer token authentication.
    Bearer {
        /// Token (inline or env ref).
        token: StringOrEnvRef,
    },
    /// Use credentials from ~/.netrc.
    Netrc,
}

impl CredentialsToml {
    /// Collects all environment variable references.
    pub fn collect_env_refs<'a>(&'a self, vars: &mut Vec<&'a str>) {
        match self {
            Self::Basic { username, password } => {
                if let Some(var) = username.env_var_name() {
                    vars.push(var);
                }
                if let Some(var) = password.env_var_name() {
                    vars.push(var);
                }
            }
            Self::Bearer { token } => {
                if let Some(var) = token.env_var_name() {
                    vars.push(var);
                }
            }
            Self::Netrc => {}
        }
    }
}

/// Dependency specification.
///
/// Can be a simple version string or a table with additional options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencySpec {
    /// Simple version string (e.g., "1.0.0").
    Simple(String),
    /// Detailed specification with options.
    Detailed(DetailedDependency),
}

impl DependencySpec {
    /// Returns the version string.
    #[must_use]
    pub fn version(&self) -> &str {
        match self {
            Self::Simple(v) => v,
            Self::Detailed(d) => &d.version,
        }
    }

    /// Returns the scope, defaulting to "compile".
    #[must_use]
    pub fn scope(&self) -> &str {
        match self {
            Self::Simple(_) => "compile",
            Self::Detailed(d) => d.scope.as_deref().unwrap_or("compile"),
        }
    }
}

/// Detailed dependency specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedDependency {
    /// Version requirement.
    pub version: String,
    /// Dependency scope (compile, runtime, test, provided).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Classifier (e.g., "sources", "javadoc").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classifier: Option<String>,
    /// Artifact type/extension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// Transitive dependency exclusions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclusions: Vec<String>,
    /// Whether to exclude transitive dependencies.
    #[serde(default)]
    pub transitive: Option<bool>,
}

/// Constraint specification (for BOMs and platforms).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConstraintSpec {
    /// Simple version string.
    Simple(String),
    /// Detailed constraint.
    Detailed(DetailedConstraint),
}

impl ConstraintSpec {
    /// Returns the version string.
    #[must_use]
    pub fn version(&self) -> &str {
        match self {
            Self::Simple(v) => v,
            Self::Detailed(c) => &c.version,
        }
    }
}

/// Detailed constraint specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedConstraint {
    /// Version requirement.
    pub version: String,
    /// Constraint type (bom, platform).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ConstraintType>,
}

/// Type of version constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConstraintType {
    /// Bill of Materials - imports managed dependencies.
    Bom,
    /// Platform constraint - aligns versions.
    Platform,
}

/// Resolver configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct ResolverConfig {
    /// Conflict resolution strategy.
    #[serde(default)]
    pub conflict_strategy: ConflictStrategy,
    /// Whether to resolve transitive dependencies.
    #[serde(default = "default_true")]
    pub transitive: bool,
}

fn default_true() -> bool {
    true
}

/// Conflict resolution strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConflictStrategy {
    /// Always select the highest version.
    #[default]
    HighestWins,
    /// Select the version closest to the root.
    NearestWins,
    /// Fail on any version conflict.
    Strict,
}

impl ConflictStrategy {
    /// Returns the strategy name as a string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::HighestWins => "highest-wins",
            Self::NearestWins => "nearest-wins",
            Self::Strict => "strict",
        }
    }
}

/// Cache configuration in TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CacheToml {
    /// Cache directory path (must be explicit for hermetic builds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    /// Cache access mode.
    #[serde(default)]
    pub mode: CacheAccessMode,
}

/// Cache access mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CacheAccessMode {
    /// Read and write to cache.
    #[default]
    ReadWrite,
    /// Read-only cache access.
    ReadOnly,
    /// Cache is disabled.
    Disabled,
}

/// Network configuration in TOML.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct NetworkToml {
    /// Whether to operate offline.
    #[serde(default)]
    pub offline: bool,
    /// Connection timeout in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connect_timeout: Option<u64>,
    /// Maximum concurrent connections.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_connections: Option<u16>,
}

/// Environment variable access configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvToml {
    /// Explicit allowlist of environment variables.
    #[serde(default)]
    pub allow: Vec<String>,
}

/// Output configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct OutputConfig {
    /// Lockfile path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lockfile: Option<PathBuf>,
    /// Whether to require SHA-256 checksums.
    #[serde(default)]
    pub require_sha256: bool,
}

/// TOML configuration errors.
#[derive(Debug, thiserror::Error)]
pub enum TomlError {
    /// TOML parse error.
    #[error("failed to parse TOML: {0}")]
    Parse(#[from] toml::de::Error),

    /// TOML serialization error.
    #[error("failed to serialize TOML: {0}")]
    Serialize(#[from] toml::ser::Error),

    /// I/O error.
    #[error("I/O error at {0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),

    /// Hermetic constraint violation.
    #[error("hermetic violation: {0}")]
    HermeticViolation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal() {
        let toml = r#"
[project]
name = "my-project"
"#;
        let config = AntlersToml::parse(toml).unwrap();
        assert_eq!(config.project.as_ref().unwrap().name, "my-project");
    }

    #[test]
    fn test_parse_full() {
        let toml = r#"
[project]
name = "my-project"
version = "0.1.0"

[[repositories]]
id = "central"
name = "Maven Central"
url = "https://repo1.maven.org/maven2/"
ecosystem = "maven"

[[repositories]]
id = "github"
name = "GitHub Packages"
url = "https://maven.pkg.github.com/owner/repo"
ecosystem = "maven"
[repositories.credentials]
type = "bearer"
token = { env = "GITHUB_TOKEN" }

[dependencies]
"com.google.guava:guava" = "33.0.0-jre"
"org.jetbrains.kotlin:kotlin-stdlib" = { version = "2.0.0", scope = "compile" }

[dev-dependencies]
"junit:junit" = "4.13.2"

[constraints]
"com.fasterxml.jackson:jackson-bom" = { version = "2.16.0", type = "bom" }

[exclusions]
"commons-logging:commons-logging" = "*"

[resolver]
conflict-strategy = "highest-wins"
transitive = true

[cache]
path = ".antlers/cache"
mode = "read-write"

[network]
offline = false
connect-timeout = 30
max-connections = 16

[env]
allow = ["GITHUB_TOKEN", "HTTPS_PROXY"]

[output]
lockfile = "antlers.lock.json"
require-sha256 = true
"#;
        let config = AntlersToml::parse(toml).unwrap();

        // Verify project
        let project = config.project.unwrap();
        assert_eq!(project.name, "my-project");
        assert_eq!(project.version.as_deref(), Some("0.1.0"));

        // Verify repositories
        assert_eq!(config.repositories.len(), 2);
        assert_eq!(config.repositories[0].id, "central");
        assert_eq!(config.repositories[1].id, "github");

        // Verify credentials
        if let Some(CredentialsToml::Bearer { token }) = &config.repositories[1].credentials {
            assert!(token.is_env_ref());
            assert_eq!(token.env_var_name(), Some("GITHUB_TOKEN"));
        } else {
            panic!("Expected bearer credentials");
        }

        // Verify dependencies
        assert_eq!(config.dependencies.len(), 2);
        assert_eq!(
            config
                .dependencies
                .get("com.google.guava:guava")
                .unwrap()
                .version(),
            "33.0.0-jre"
        );

        // Verify resolver
        let resolver = config.resolver.unwrap();
        assert_eq!(resolver.conflict_strategy, ConflictStrategy::HighestWins);
        assert!(resolver.transitive);

        // Verify cache
        let cache = config.cache.unwrap();
        assert_eq!(cache.path, Some(PathBuf::from(".antlers/cache")));
        assert_eq!(cache.mode, CacheAccessMode::ReadWrite);

        // Verify env
        let env = config.env.unwrap();
        assert_eq!(env.allow, vec!["GITHUB_TOKEN", "HTTPS_PROXY"]);

        // Verify output
        let output = config.output.unwrap();
        assert_eq!(output.lockfile, Some(PathBuf::from("antlers.lock.json")));
        assert!(output.require_sha256);
    }

    #[test]
    fn test_roundtrip() {
        let original = AntlersToml {
            project: Some(ProjectConfig {
                name: "test".to_string(),
                version: Some("1.0.0".to_string()),
                description: None,
            }),
            repositories: vec![RepositoryToml {
                id: "central".to_string(),
                name: Some("Maven Central".to_string()),
                url: "https://repo1.maven.org/maven2/".to_string(),
                ecosystem: Ecosystem::Maven,
                credentials: None,
            }],
            dependencies: {
                let mut deps = IndexMap::new();
                deps.insert(
                    "junit:junit".to_string(),
                    DependencySpec::Simple("4.13.2".to_string()),
                );
                deps
            },
            ..Default::default()
        };

        let toml_str = original.to_toml().unwrap();
        let parsed = AntlersToml::parse(&toml_str).unwrap();

        assert_eq!(parsed.project.as_ref().unwrap().name, "test");
        assert_eq!(parsed.repositories.len(), 1);
        assert_eq!(parsed.dependencies.len(), 1);
    }

    #[test]
    fn test_referenced_env_vars() {
        let config = AntlersToml {
            repositories: vec![
                RepositoryToml {
                    id: "github".to_string(),
                    name: None,
                    url: "https://maven.pkg.github.com".to_string(),
                    ecosystem: Ecosystem::Maven,
                    credentials: Some(CredentialsToml::Bearer {
                        token: StringOrEnvRef::env("GITHUB_TOKEN"),
                    }),
                },
                RepositoryToml {
                    id: "private".to_string(),
                    name: None,
                    url: "https://private.example.com".to_string(),
                    ecosystem: Ecosystem::Maven,
                    credentials: Some(CredentialsToml::Basic {
                        username: StringOrEnvRef::env("REPO_USER"),
                        password: StringOrEnvRef::env("REPO_PASS"),
                    }),
                },
            ],
            ..Default::default()
        };

        let vars = config.referenced_env_vars();
        assert_eq!(vars.len(), 3);
        assert!(vars.contains(&"GITHUB_TOKEN"));
        assert!(vars.contains(&"REPO_USER"));
        assert!(vars.contains(&"REPO_PASS"));
    }

    #[test]
    fn test_validate_hermetic_missing_cache() {
        let config = AntlersToml::default();
        let result = config.validate_hermetic();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_hermetic_missing_cache_path() {
        let config = AntlersToml {
            cache: Some(CacheToml {
                path: None,
                mode: CacheAccessMode::ReadWrite,
            }),
            ..Default::default()
        };
        let result = config.validate_hermetic();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_hermetic_env_not_allowed() {
        let config = AntlersToml {
            cache: Some(CacheToml {
                path: Some(PathBuf::from("/cache")),
                mode: CacheAccessMode::ReadWrite,
            }),
            repositories: vec![RepositoryToml {
                id: "test".to_string(),
                name: None,
                url: "https://example.com".to_string(),
                ecosystem: Ecosystem::Maven,
                credentials: Some(CredentialsToml::Bearer {
                    token: StringOrEnvRef::env("SECRET_TOKEN"),
                }),
            }],
            ..Default::default()
        };
        let result = config.validate_hermetic();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_hermetic_success() {
        let config = AntlersToml {
            cache: Some(CacheToml {
                path: Some(PathBuf::from("/cache")),
                mode: CacheAccessMode::ReadWrite,
            }),
            repositories: vec![RepositoryToml {
                id: "test".to_string(),
                name: None,
                url: "https://example.com".to_string(),
                ecosystem: Ecosystem::Maven,
                credentials: Some(CredentialsToml::Bearer {
                    token: StringOrEnvRef::env("SECRET_TOKEN"),
                }),
            }],
            env: Some(EnvToml {
                allow: vec!["SECRET_TOKEN".to_string()],
            }),
            ..Default::default()
        };
        let result = config.validate_hermetic();
        assert!(result.is_ok());
    }

    #[test]
    fn test_conflict_strategy_serde() {
        assert_eq!(
            serde_json::to_string(&ConflictStrategy::HighestWins).unwrap(),
            r#""highest-wins""#
        );
        assert_eq!(
            serde_json::from_str::<ConflictStrategy>(r#""nearest-wins""#).unwrap(),
            ConflictStrategy::NearestWins
        );
    }

    #[test]
    fn test_dependency_spec_simple() {
        let spec = DependencySpec::Simple("1.0.0".to_string());
        assert_eq!(spec.version(), "1.0.0");
        assert_eq!(spec.scope(), "compile");
    }

    #[test]
    fn test_dependency_spec_detailed() {
        let spec = DependencySpec::Detailed(DetailedDependency {
            version: "1.0.0".to_string(),
            scope: Some("test".to_string()),
            classifier: None,
            r#type: None,
            exclusions: vec![],
            transitive: None,
        });
        assert_eq!(spec.version(), "1.0.0");
        assert_eq!(spec.scope(), "test");
    }
}
