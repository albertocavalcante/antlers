//! Parser for Python pip.conf / pip.ini configuration files.
//!
//! Supports:
//! - [global] index-url = URL (primary index)
//! - [global] extra-index-url = URL (additional indices)
//! - URLs with embedded credentials: <https://user:pass@host>/

use gather::Ecosystem;

use crate::migrate::{MigratedCredentials, MigratedRepository, MigrationError, SourceParser};

/// Parser for pip.conf / pip.ini files.
pub struct PipParser;

impl SourceParser for PipParser {
    fn parse(&self, content: &str) -> Result<Vec<MigratedRepository>, MigrationError> {
        let mut repositories = Vec::new();
        let mut in_global = false;

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            // Track section
            if line.starts_with('[') && line.ends_with(']') {
                in_global = line.to_lowercase() == "[global]";
                continue;
            }

            // Only process global section
            if !in_global {
                continue;
            }

            // Parse key = value
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            let key = key.trim().to_lowercase();
            let value = value.trim();

            // Primary index
            if key == "index-url" {
                if let Some(repo) = parse_pip_url(value, "pypi-default", "Default PyPI Index") {
                    repositories.push(repo);
                }
                continue;
            }

            // Extra indices (can be multiple lines or space/newline separated)
            if key == "extra-index-url" {
                for (i, url) in value.split_whitespace().enumerate() {
                    let id = format!("pypi-extra-{}", i + 1);
                    let name = format!("Extra PyPI Index {}", i + 1);
                    if let Some(repo) = parse_pip_url(url, &id, &name) {
                        repositories.push(repo);
                    }
                }
            }
        }

        Ok(repositories)
    }

    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Pypi
    }
}

/// Parses a pip URL, extracting any embedded credentials.
#[allow(clippy::unnecessary_wraps, clippy::option_if_let_else)]
fn parse_pip_url(url: &str, id: &str, name: &str) -> Option<MigratedRepository> {
    // Check for embedded credentials: https://user:pass@host/
    if let Some(at_pos) = url.find('@') {
        let protocol_end = url.find("://").map_or(0, |p| p + 3);
        if at_pos > protocol_end {
            // Has embedded credentials
            let creds_str = &url[protocol_end..at_pos];
            let clean_url = format!("{}{}", &url[..protocol_end], &url[at_pos + 1..]);

            let credentials = if let Some(colon_pos) = creds_str.find(':') {
                let username = &creds_str[..colon_pos];
                let password = &creds_str[colon_pos + 1..];

                // Check for env var syntax ${VAR}
                let (username, password, is_env) = if username.starts_with("${") {
                    let u = username[2..username.len() - 1].to_string();
                    let p = if password.starts_with("${") {
                        password[2..password.len() - 1].to_string()
                    } else {
                        format!("{u}_PASSWORD")
                    };
                    (u, p, true)
                } else if password.starts_with("${") {
                    let p = password[2..password.len() - 1].to_string();
                    (format!("PYPI_USER_{}", id.to_uppercase()), p, true)
                } else {
                    (username.to_string(), password.to_string(), false)
                };

                if is_env {
                    Some(MigratedCredentials::BasicEnv {
                        username_var: username,
                        password_var: password,
                    })
                } else {
                    Some(MigratedCredentials::BasicInline { username, password })
                }
            } else {
                None
            };

            return Some(MigratedRepository {
                id: id.to_string(),
                name: Some(name.to_string()),
                url: clean_url,
                ecosystem: Ecosystem::Pypi,
                credentials,
            });
        }
    }

    // No embedded credentials
    Some(MigratedRepository {
        id: id.to_string(),
        name: Some(name.to_string()),
        url: url.to_string(),
        ecosystem: Ecosystem::Pypi,
        credentials: None,
    })
}

#[cfg(test)]
#[allow(clippy::needless_raw_string_hashes)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_index_url() {
        let content = r#"
[global]
index-url = https://pypi.org/simple/
"#;
        let parser = PipParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].id, "pypi-default");
        assert_eq!(repos[0].url, "https://pypi.org/simple/");
        assert!(repos[0].credentials.is_none());
    }

    #[test]
    fn test_parse_extra_index_url() {
        let content = r#"
[global]
index-url = https://pypi.org/simple/
extra-index-url = https://private.example.com/simple/
"#;
        let parser = PipParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].id, "pypi-default");
        assert_eq!(repos[1].id, "pypi-extra-1");
    }

    #[test]
    fn test_parse_embedded_credentials() {
        let content = r#"
[global]
index-url = https://myuser:mypassword@private.pypi.com/simple/
"#;
        let parser = PipParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].url, "https://private.pypi.com/simple/");

        if let Some(MigratedCredentials::BasicInline { username, password }) = &repos[0].credentials
        {
            assert_eq!(username, "myuser");
            assert_eq!(password, "mypassword");
        } else {
            panic!("Expected basic inline credentials");
        }
    }

    #[test]
    fn test_skip_non_global_section() {
        let content = r#"
[install]
index-url = https://should-be-ignored.com/

[global]
index-url = https://pypi.org/simple/
"#;
        let parser = PipParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].url, "https://pypi.org/simple/");
    }

    #[test]
    fn test_skip_comments() {
        let content = r#"
# This is a comment
[global]
; Another comment
index-url = https://pypi.org/simple/
"#;
        let parser = PipParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
    }

    #[test]
    fn test_case_insensitive_section() {
        let content = r#"
[GLOBAL]
index-url = https://pypi.org/simple/
"#;
        let parser = PipParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
    }
}
