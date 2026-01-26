//! Configuration migration from other package managers.
//!
//! This module provides functionality to import repository and credential
//! configurations from various package manager configuration files:
//!
//! - `.npmrc` - npm registry configuration
//! - `pip.conf` / `pip.ini` - Python package index configuration
//! - `settings.xml` - Maven settings (servers, mirrors, proxies)
//! - `settings.gradle` / `settings.gradle.kts` - Gradle repository configuration
//! - `ivysettings.xml` - Apache Ivy resolver configuration
//!
//! # Example
//!
//! ```no_run
//! use antlers::migrate::{MigrationSource, migrate_to_antlers};
//!
//! // Detect and migrate from a config file
//! let source = MigrationSource::detect("~/.m2/settings.xml").unwrap();
//! let config = migrate_to_antlers(&source).unwrap();
//!
//! // Save to antlers.toml
//! config.save("antlers.toml").unwrap();
//! ```

pub mod parsers;

use std::path::{Path, PathBuf};

use gather::{Ecosystem, StringOrEnvRef};

use crate::config::{AntlersToml, CredentialsToml, EnvToml, ProjectConfig, RepositoryToml};

pub use parsers::{
    gradle::GradleParser, ivy::IvyParser, maven::MavenParser, npmrc::NpmrcParser, pip::PipParser,
};

/// A source configuration that can be migrated to antlers.toml.
#[derive(Debug, Clone)]
pub struct MigrationSource {
    /// Source file path.
    pub path: PathBuf,
    /// Detected format type.
    pub format: SourceFormat,
    /// Parsed repositories.
    pub repositories: Vec<MigratedRepository>,
    /// Environment variables that need to be allowed.
    pub env_vars: Vec<String>,
}

/// Supported source configuration formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    /// npm .npmrc file.
    Npmrc,
    /// Python pip.conf / pip.ini.
    Pip,
    /// Maven settings.xml.
    Maven,
    /// Gradle settings.gradle(.kts).
    Gradle,
    /// Apache Ivy ivysettings.xml.
    Ivy,
}

impl SourceFormat {
    /// Returns the format name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Npmrc => "npmrc",
            Self::Pip => "pip",
            Self::Maven => "maven",
            Self::Gradle => "gradle",
            Self::Ivy => "ivy",
        }
    }

    /// Detects the format from a file path.
    #[must_use]
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    pub fn detect(path: &Path) -> Option<Self> {
        let file_name = path.file_name()?.to_str()?;
        let file_name_lower = file_name.to_lowercase();

        // Note: We manually lowercase above, so case-sensitivity is handled
        if file_name == ".npmrc" || file_name_lower.ends_with(".npmrc") {
            return Some(Self::Npmrc);
        }
        if file_name_lower == "pip.conf" || file_name_lower == "pip.ini" {
            return Some(Self::Pip);
        }
        if file_name_lower == "settings.xml" {
            return Some(Self::Maven);
        }
        if file_name_lower.starts_with("settings.gradle") {
            return Some(Self::Gradle);
        }
        if file_name_lower == "ivysettings.xml" {
            return Some(Self::Ivy);
        }

        None
    }
}

/// A repository extracted from a migration source.
#[derive(Debug, Clone)]
pub struct MigratedRepository {
    /// Repository identifier.
    pub id: String,
    /// Repository name (may be inferred).
    pub name: Option<String>,
    /// Repository URL.
    pub url: String,
    /// Ecosystem type.
    pub ecosystem: Ecosystem,
    /// Credentials if present.
    pub credentials: Option<MigratedCredentials>,
}

/// Credentials extracted from a migration source.
#[derive(Debug, Clone)]
pub enum MigratedCredentials {
    /// Basic auth with inline values.
    BasicInline {
        /// Username.
        username: String,
        /// Password/token.
        password: String,
    },
    /// Basic auth with environment variable references.
    BasicEnv {
        /// Environment variable for username.
        username_var: String,
        /// Environment variable for password.
        password_var: String,
    },
    /// Bearer token inline.
    BearerInline {
        /// The token value.
        token: String,
    },
    /// Bearer token from environment.
    BearerEnv {
        /// Environment variable for token.
        token_var: String,
    },
    /// Use netrc file.
    Netrc,
}

impl MigratedCredentials {
    /// Converts to TOML credentials format.
    #[must_use]
    pub fn to_toml(&self) -> CredentialsToml {
        match self {
            Self::BasicInline { username, password } => CredentialsToml::Basic {
                username: StringOrEnvRef::inline(username),
                password: StringOrEnvRef::inline(password),
            },
            Self::BasicEnv {
                username_var,
                password_var,
            } => CredentialsToml::Basic {
                username: StringOrEnvRef::env(username_var),
                password: StringOrEnvRef::env(password_var),
            },
            Self::BearerInline { token } => CredentialsToml::Bearer {
                token: StringOrEnvRef::inline(token),
            },
            Self::BearerEnv { token_var } => CredentialsToml::Bearer {
                token: StringOrEnvRef::env(token_var),
            },
            Self::Netrc => CredentialsToml::Netrc,
        }
    }

    /// Returns any environment variables referenced.
    #[must_use]
    pub fn env_vars(&self) -> Vec<&str> {
        match self {
            Self::BasicEnv {
                username_var,
                password_var,
            } => vec![username_var, password_var],
            Self::BearerEnv { token_var } => vec![token_var],
            _ => vec![],
        }
    }
}

/// Trait for source format parsers.
pub trait SourceParser {
    /// Parses a configuration file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be parsed.
    fn parse(&self, content: &str) -> Result<Vec<MigratedRepository>, MigrationError>;

    /// Returns the ecosystem type this parser produces.
    fn ecosystem(&self) -> Ecosystem;
}

impl MigrationSource {
    /// Detects and parses a configuration file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn detect(path: impl AsRef<Path>) -> Result<Self, MigrationError> {
        let path = path.as_ref();
        let format = SourceFormat::detect(path)
            .ok_or_else(|| MigrationError::UnknownFormat(path.to_path_buf()))?;

        Self::parse_with_format(path, format)
    }

    /// Parses a file with a specified format.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn parse_with_format(
        path: impl AsRef<Path>,
        format: SourceFormat,
    ) -> Result<Self, MigrationError> {
        let path = path.as_ref();
        let content =
            std::fs::read_to_string(path).map_err(|e| MigrationError::Io(path.to_path_buf(), e))?;

        let repositories = match format {
            SourceFormat::Npmrc => NpmrcParser.parse(&content)?,
            SourceFormat::Pip => PipParser.parse(&content)?,
            SourceFormat::Maven => MavenParser.parse(&content)?,
            SourceFormat::Gradle => GradleParser.parse(&content)?,
            SourceFormat::Ivy => IvyParser.parse(&content)?,
        };

        // Collect all env vars
        let mut env_vars = Vec::new();
        for repo in &repositories {
            if let Some(creds) = &repo.credentials {
                for var in creds.env_vars() {
                    if !env_vars.contains(&var.to_string()) {
                        env_vars.push(var.to_string());
                    }
                }
            }
        }

        Ok(Self {
            path: path.to_path_buf(),
            format,
            repositories,
            env_vars,
        })
    }

    /// Converts to an `AntlersToml` configuration.
    #[must_use]
    pub fn to_antlers_toml(&self) -> AntlersToml {
        let repositories: Vec<RepositoryToml> = self
            .repositories
            .iter()
            .map(|repo| RepositoryToml {
                id: repo.id.clone(),
                name: repo.name.clone(),
                url: repo.url.clone(),
                ecosystem: repo.ecosystem,
                credentials: repo.credentials.as_ref().map(MigratedCredentials::to_toml),
            })
            .collect();

        let env = if self.env_vars.is_empty() {
            None
        } else {
            Some(EnvToml {
                allow: self.env_vars.clone(),
            })
        };

        AntlersToml {
            project: Some(ProjectConfig {
                name: "migrated-project".to_string(),
                version: Some("0.1.0".to_string()),
                description: Some(format!("Migrated from {}", self.format.name())),
            }),
            repositories,
            env,
            ..Default::default()
        }
    }
}

/// Migrates a source configuration to `AntlersToml`.
///
/// # Errors
///
/// Returns an error if the source cannot be parsed.
pub fn migrate_to_antlers(source: &MigrationSource) -> Result<AntlersToml, MigrationError> {
    Ok(source.to_antlers_toml())
}

/// Migration errors.
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    /// Unknown configuration format.
    #[error("unknown configuration format: {0}")]
    UnknownFormat(PathBuf),

    /// I/O error.
    #[error("I/O error reading {0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),

    /// Parse error.
    #[error("parse error: {0}")]
    Parse(String),

    /// XML parse error.
    #[error("XML parse error: {0}")]
    Xml(#[from] quick_xml::DeError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_detection() {
        assert_eq!(
            SourceFormat::detect(Path::new(".npmrc")),
            Some(SourceFormat::Npmrc)
        );
        assert_eq!(
            SourceFormat::detect(Path::new("/home/user/.npmrc")),
            Some(SourceFormat::Npmrc)
        );
        assert_eq!(
            SourceFormat::detect(Path::new("pip.conf")),
            Some(SourceFormat::Pip)
        );
        assert_eq!(
            SourceFormat::detect(Path::new("pip.ini")),
            Some(SourceFormat::Pip)
        );
        assert_eq!(
            SourceFormat::detect(Path::new("settings.xml")),
            Some(SourceFormat::Maven)
        );
        assert_eq!(
            SourceFormat::detect(Path::new("settings.gradle")),
            Some(SourceFormat::Gradle)
        );
        assert_eq!(
            SourceFormat::detect(Path::new("settings.gradle.kts")),
            Some(SourceFormat::Gradle)
        );
        assert_eq!(
            SourceFormat::detect(Path::new("ivysettings.xml")),
            Some(SourceFormat::Ivy)
        );
        assert_eq!(
            SourceFormat::detect(Path::new("/home/user/.ivy2/ivysettings.xml")),
            Some(SourceFormat::Ivy)
        );
        assert_eq!(SourceFormat::detect(Path::new("unknown.txt")), None);
    }

    #[test]
    fn test_migrated_credentials_to_toml() {
        let inline = MigratedCredentials::BasicInline {
            username: "user".to_string(),
            password: "pass".to_string(),
        };
        if let CredentialsToml::Basic { username, password } = inline.to_toml() {
            assert!(!username.is_env_ref());
            assert!(!password.is_env_ref());
        } else {
            panic!("Expected basic credentials");
        }

        let env = MigratedCredentials::BearerEnv {
            token_var: "GITHUB_TOKEN".to_string(),
        };
        if let CredentialsToml::Bearer { token } = env.to_toml() {
            assert!(token.is_env_ref());
            assert_eq!(token.env_var_name(), Some("GITHUB_TOKEN"));
        } else {
            panic!("Expected bearer credentials");
        }
    }

    #[test]
    fn test_env_vars_collection() {
        let creds = MigratedCredentials::BasicEnv {
            username_var: "USER".to_string(),
            password_var: "PASS".to_string(),
        };
        assert_eq!(creds.env_vars(), vec!["USER", "PASS"]);

        let creds = MigratedCredentials::BearerEnv {
            token_var: "TOKEN".to_string(),
        };
        assert_eq!(creds.env_vars(), vec!["TOKEN"]);

        let creds = MigratedCredentials::Netrc;
        assert!(creds.env_vars().is_empty());
    }
}
