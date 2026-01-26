//! Parser for npm .npmrc configuration files.
//!
//! Supports:
//! - registry=URL (default registry)
//! - @scope:registry=URL (scoped registries)
//! - //registry.npmjs.org/:_authToken=TOKEN (auth tokens)
//! - //registry/:username=USER (basic auth)
//! - //registry/:_password=BASE64PASS (basic auth password)

use gather::Ecosystem;

use crate::migrate::{MigratedCredentials, MigratedRepository, MigrationError, SourceParser};

/// Parser for .npmrc files.
pub struct NpmrcParser;

impl SourceParser for NpmrcParser {
    fn parse(&self, content: &str) -> Result<Vec<MigratedRepository>, MigrationError> {
        let mut repositories = Vec::new();
        let mut credentials_map: std::collections::HashMap<String, PartialCredentials> =
            std::collections::HashMap::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            // Parse key=value
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            let key = key.trim();
            let value = value.trim();

            // Default registry
            if key == "registry" {
                repositories.push(MigratedRepository {
                    id: "npm-default".to_string(),
                    name: Some("Default npm Registry".to_string()),
                    url: value.to_string(),
                    ecosystem: Ecosystem::Npm,
                    credentials: None,
                });
                continue;
            }

            // Scoped registry: @scope:registry=URL
            if let Some(scope) = key.strip_suffix(":registry")
                && scope.starts_with('@')
            {
                let id = format!("npm-{}", &scope[1..]);
                repositories.push(MigratedRepository {
                    id: id.clone(),
                    name: Some(format!("npm registry for {scope}")),
                    url: value.to_string(),
                    ecosystem: Ecosystem::Npm,
                    credentials: None,
                });
                continue;
            }

            // Auth token: //host/:_authToken=TOKEN or ${ENV_VAR}
            if key.starts_with("//") && key.contains(":_authToken") {
                let host = extract_host(key);
                let creds = if value.starts_with("${") && value.ends_with('}') {
                    let var = &value[2..value.len() - 1];
                    MigratedCredentials::BearerEnv {
                        token_var: var.to_string(),
                    }
                } else {
                    MigratedCredentials::BearerInline {
                        token: value.to_string(),
                    }
                };
                credentials_map.entry(host).or_default().auth_token = Some(creds);
                continue;
            }

            // Basic auth username: //host/:username=USER
            if key.starts_with("//") && key.contains(":username") {
                let host = extract_host(key);
                let username = if value.starts_with("${") && value.ends_with('}') {
                    AuthValue::Env(value[2..value.len() - 1].to_string())
                } else {
                    AuthValue::Inline(value.to_string())
                };
                credentials_map.entry(host).or_default().username = Some(username);
                continue;
            }

            // Basic auth password: //host/:_password=BASE64PASS
            if key.starts_with("//") && key.contains(":_password") {
                let host = extract_host(key);
                let password = if value.starts_with("${") && value.ends_with('}') {
                    AuthValue::Env(value[2..value.len() - 1].to_string())
                } else {
                    AuthValue::Inline(value.to_string())
                };
                credentials_map.entry(host).or_default().password = Some(password);
            }
        }

        // Attach credentials to matching repositories
        for repo in &mut repositories {
            let host = extract_host_from_url(&repo.url);
            if let Some(partial) = credentials_map.get(&host) {
                repo.credentials = partial.to_credentials();
            }
        }

        // Create repositories for hosts with credentials but no explicit registry entry
        for (host, partial) in credentials_map {
            if !repositories
                .iter()
                .any(|r| extract_host_from_url(&r.url) == host)
                && let Some(creds) = partial.to_credentials()
            {
                repositories.push(MigratedRepository {
                    id: format!("npm-{}", host.replace('.', "-")),
                    name: Some(format!("npm registry at {host}")),
                    url: format!("https://{host}/"),
                    ecosystem: Ecosystem::Npm,
                    credentials: Some(creds),
                });
            }
        }

        Ok(repositories)
    }

    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Npm
    }
}

/// Partial credentials being accumulated from multiple lines.
#[derive(Default)]
struct PartialCredentials {
    auth_token: Option<MigratedCredentials>,
    username: Option<AuthValue>,
    password: Option<AuthValue>,
}

enum AuthValue {
    Inline(String),
    Env(String),
}

impl PartialCredentials {
    fn to_credentials(&self) -> Option<MigratedCredentials> {
        // Prefer auth token if present
        if let Some(token) = &self.auth_token {
            return Some(token.clone());
        }

        // Otherwise try basic auth
        match (&self.username, &self.password) {
            (Some(AuthValue::Inline(u)), Some(AuthValue::Inline(p))) => {
                Some(MigratedCredentials::BasicInline {
                    username: u.clone(),
                    password: p.clone(),
                })
            }
            (Some(AuthValue::Env(u)), Some(AuthValue::Env(p))) => {
                Some(MigratedCredentials::BasicEnv {
                    username_var: u.clone(),
                    password_var: p.clone(),
                })
            }
            // Mixed inline/env - treat as env for safety
            (Some(AuthValue::Inline(u)), Some(AuthValue::Env(p))) => {
                Some(MigratedCredentials::BasicEnv {
                    username_var: format!("NPM_USER_{}", u.to_uppercase()),
                    password_var: p.clone(),
                })
            }
            (Some(AuthValue::Env(u)), Some(AuthValue::Inline(_))) => {
                Some(MigratedCredentials::BasicEnv {
                    username_var: u.clone(),
                    password_var: format!("{u}_PASSWORD"),
                })
            }
            _ => None,
        }
    }
}

/// Extracts host from an npmrc key like "//registry.npmjs.org/:_authToken".
#[allow(clippy::option_if_let_else)]
fn extract_host(key: &str) -> String {
    let without_slashes = key.trim_start_matches('/');
    if let Some(idx) = without_slashes.find('/') {
        without_slashes[..idx].to_string()
    } else if let Some(idx) = without_slashes.find(':') {
        without_slashes[..idx].to_string()
    } else {
        without_slashes.to_string()
    }
}

/// Extracts host from a URL.
fn extract_host_from_url(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
#[allow(clippy::needless_raw_string_hashes)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_default_registry() {
        let content = "registry=https://registry.npmjs.org/";
        let parser = NpmrcParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].id, "npm-default");
        assert_eq!(repos[0].url, "https://registry.npmjs.org/");
    }

    #[test]
    fn test_parse_scoped_registry() {
        let content = r#"
@myorg:registry=https://npm.pkg.github.com/
@otherorg:registry=https://registry.example.com/
"#;
        let parser = NpmrcParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].id, "npm-myorg");
        assert_eq!(repos[0].url, "https://npm.pkg.github.com/");
        assert_eq!(repos[1].id, "npm-otherorg");
    }

    #[test]
    fn test_parse_auth_token_inline() {
        let content = r#"
registry=https://registry.npmjs.org/
//registry.npmjs.org/:_authToken=my-secret-token
"#;
        let parser = NpmrcParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
        if let Some(MigratedCredentials::BearerInline { token }) = &repos[0].credentials {
            assert_eq!(token, "my-secret-token");
        } else {
            panic!("Expected bearer inline credentials");
        }
    }

    #[test]
    fn test_parse_auth_token_env() {
        let content = r#"
@myorg:registry=https://npm.pkg.github.com/
//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}
"#;
        let parser = NpmrcParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
        if let Some(MigratedCredentials::BearerEnv { token_var }) = &repos[0].credentials {
            assert_eq!(token_var, "GITHUB_TOKEN");
        } else {
            panic!("Expected bearer env credentials");
        }
    }

    #[test]
    fn test_parse_basic_auth() {
        let content = r#"
registry=https://registry.example.com/
//registry.example.com/:username=myuser
//registry.example.com/:_password=bXlwYXNz
"#;
        let parser = NpmrcParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
        if let Some(MigratedCredentials::BasicInline { username, password }) = &repos[0].credentials
        {
            assert_eq!(username, "myuser");
            assert_eq!(password, "bXlwYXNz");
        } else {
            panic!("Expected basic inline credentials");
        }
    }

    #[test]
    fn test_skip_comments() {
        let content = r#"
# This is a comment
; This is also a comment
registry=https://registry.npmjs.org/
"#;
        let parser = NpmrcParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
    }
}
