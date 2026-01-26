//! Parser for Gradle settings.gradle and settings.gradle.kts files.
//!
//! Supports extracting repository URLs from:
//! - pluginManagement { repositories { ... } }
//! - dependencyResolutionManagement { repositories { ... } }
//! - maven { url = "..." } / maven("...")
//! - mavenCentral() / google() / gradlePluginPortal()

use gather::Ecosystem;

use crate::migrate::{MigratedCredentials, MigratedRepository, MigrationError, SourceParser};

/// Parser for Gradle settings files.
pub struct GradleParser;

impl SourceParser for GradleParser {
    fn parse(&self, content: &str) -> Result<Vec<MigratedRepository>, MigrationError> {
        let mut repositories = Vec::new();
        let mut counter = 0;

        // Find all repository blocks
        for line in content.lines() {
            let line = line.trim();

            // Skip comments
            if line.starts_with("//") || line.starts_with("/*") || line.starts_with("*") {
                continue;
            }

            // Detect well-known repositories
            if line.contains("mavenCentral()") {
                repositories.push(MigratedRepository {
                    id: "central".to_string(),
                    name: Some("Maven Central".to_string()),
                    url: "https://repo1.maven.org/maven2/".to_string(),
                    ecosystem: Ecosystem::Maven,
                    credentials: None,
                });
            }

            if line.contains("google()") {
                repositories.push(MigratedRepository {
                    id: "google".to_string(),
                    name: Some("Google Maven".to_string()),
                    url: "https://maven.google.com/".to_string(),
                    ecosystem: Ecosystem::Maven,
                    credentials: None,
                });
            }

            if line.contains("gradlePluginPortal()") {
                repositories.push(MigratedRepository {
                    id: "gradle-plugins".to_string(),
                    name: Some("Gradle Plugin Portal".to_string()),
                    url: "https://plugins.gradle.org/m2/".to_string(),
                    ecosystem: Ecosystem::Maven,
                    credentials: None,
                });
            }

            if line.contains("jcenter()") {
                repositories.push(MigratedRepository {
                    id: "jcenter".to_string(),
                    name: Some("JCenter (deprecated)".to_string()),
                    url: "https://jcenter.bintray.com/".to_string(),
                    ecosystem: Ecosystem::Maven,
                    credentials: None,
                });
            }

            if line.contains("mavenLocal()") {
                repositories.push(MigratedRepository {
                    id: "local".to_string(),
                    name: Some("Maven Local".to_string()),
                    url: "file://~/.m2/repository/".to_string(),
                    ecosystem: Ecosystem::Maven,
                    credentials: None,
                });
            }

            // Extract maven URLs from various patterns
            if let Some(url) = extract_maven_url(line) {
                counter += 1;
                let id = format!("maven-{counter}");
                let (clean_url, credentials) = parse_url_with_credentials(&url);

                repositories.push(MigratedRepository {
                    id,
                    name: None,
                    url: clean_url,
                    ecosystem: Ecosystem::Maven,
                    credentials,
                });
            }
        }

        // Look for credentials blocks
        let repositories = add_credentials_from_content(content, repositories);

        // Deduplicate by URL
        let mut seen_urls = std::collections::HashSet::new();
        let deduplicated = repositories
            .into_iter()
            .filter(|r| seen_urls.insert(r.url.clone()))
            .collect::<Vec<_>>();

        Ok(deduplicated)
    }

    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Maven
    }
}

/// Extracts a maven URL from a line.
fn extract_maven_url(line: &str) -> Option<String> {
    // Pattern: maven { url = "..." } or maven { url = '...' }
    if let Some(start) = line.find("url") {
        let rest = &line[start..];
        return extract_quoted_string(rest);
    }

    // Pattern: maven("...") or maven('...')
    if line.contains("maven(") || line.contains("maven (") {
        let start = line.find('(')? + 1;
        let end = line.rfind(')')?;
        let inner = line[start..end].trim();
        return extract_quoted_string(inner);
    }

    // Pattern: url("...") or url('...')
    if line.contains("url(") || line.contains("url (") {
        let start = line.find("url(")? + 4;
        let rest = &line[start..];
        return extract_quoted_string(rest);
    }

    // Pattern: setUrl("...")
    if line.contains("setUrl(") {
        let start = line.find("setUrl(")? + 7;
        let rest = &line[start..];
        return extract_quoted_string(rest);
    }

    None
}

/// Extracts a quoted string from text.
fn extract_quoted_string(text: &str) -> Option<String> {
    // Try double quotes
    if let Some(start) = text.find('"') {
        let rest = &text[start + 1..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }

    // Try single quotes
    if let Some(start) = text.find('\'') {
        let rest = &text[start + 1..];
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }

    None
}

/// Parses a URL that might contain embedded credentials.
fn parse_url_with_credentials(url: &str) -> (String, Option<MigratedCredentials>) {
    if let Some(at_pos) = url.find('@') {
        let protocol_end = url.find("://").map(|p| p + 3).unwrap_or(0);
        if at_pos > protocol_end {
            let creds_str = &url[protocol_end..at_pos];
            let clean_url = format!("{}{}", &url[..protocol_end], &url[at_pos + 1..]);

            if let Some(colon_pos) = creds_str.find(':') {
                let username = creds_str[..colon_pos].to_string();
                let password = creds_str[colon_pos + 1..].to_string();

                return (
                    clean_url,
                    Some(MigratedCredentials::BasicInline { username, password }),
                );
            }
        }
    }

    (url.to_string(), None)
}

/// Looks for credentials blocks and associates them with repositories.
fn add_credentials_from_content(
    content: &str,
    mut repositories: Vec<MigratedRepository>,
) -> Vec<MigratedRepository> {
    // Look for patterns like:
    // credentials {
    //     username = "..."
    //     password = "..."
    // }
    // or
    // credentials {
    //     username = project.findProperty("...") ?: System.getenv("...")
    // }

    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let line = line.trim();

        // Look for credentials blocks
        if line.contains("credentials") && line.contains('{') {
            let mut username: Option<String> = None;
            let mut password: Option<String> = None;
            let mut is_env = false;

            // Scan following lines for username/password
            for j in i + 1..lines.len().min(i + 10) {
                let cred_line = lines[j].trim();

                if cred_line.contains('}') {
                    break;
                }

                if cred_line.contains("username") {
                    if cred_line.contains("System.getenv") || cred_line.contains("getenv(") {
                        is_env = true;
                        if let Some(var) = extract_env_var_name(cred_line) {
                            username = Some(var);
                        }
                    } else if let Some(val) = extract_quoted_string(cred_line) {
                        username = Some(val);
                    }
                }

                if cred_line.contains("password") {
                    if cred_line.contains("System.getenv") || cred_line.contains("getenv(") {
                        is_env = true;
                        if let Some(var) = extract_env_var_name(cred_line) {
                            password = Some(var);
                        }
                    } else if let Some(val) = extract_quoted_string(cred_line) {
                        password = Some(val);
                    }
                }
            }

            // Find the most recent repository without credentials
            if let (Some(u), Some(p)) = (username, password) {
                if let Some(repo) = repositories
                    .iter_mut()
                    .rev()
                    .find(|r| r.credentials.is_none())
                {
                    repo.credentials = Some(if is_env {
                        MigratedCredentials::BasicEnv {
                            username_var: u,
                            password_var: p,
                        }
                    } else {
                        MigratedCredentials::BasicInline {
                            username: u,
                            password: p,
                        }
                    });
                }
            }
        }
    }

    repositories
}

/// Extracts environment variable name from System.getenv("VAR") or similar.
fn extract_env_var_name(line: &str) -> Option<String> {
    // Pattern: System.getenv("VAR") or getenv("VAR")
    if let Some(start) = line.find("getenv(") {
        let rest = &line[start + 7..];
        return extract_quoted_string(rest);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_well_known_repositories() {
        let content = r#"
dependencyResolutionManagement {
    repositories {
        mavenCentral()
        google()
        gradlePluginPortal()
    }
}
"#;
        let parser = GradleParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 3);
        assert!(repos.iter().any(|r| r.id == "central"));
        assert!(repos.iter().any(|r| r.id == "google"));
        assert!(repos.iter().any(|r| r.id == "gradle-plugins"));
    }

    #[test]
    fn test_parse_maven_url() {
        let content = r#"
repositories {
    maven { url = "https://repo.example.com/maven" }
}
"#;
        let parser = GradleParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].url, "https://repo.example.com/maven");
    }

    #[test]
    fn test_parse_maven_kotlin_style() {
        let content = r#"
repositories {
    maven("https://repo.example.com/maven")
}
"#;
        let parser = GradleParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].url, "https://repo.example.com/maven");
    }

    #[test]
    fn test_parse_with_credentials() {
        let content = r#"
repositories {
    maven {
        url = "https://repo.example.com/maven"
        credentials {
            username = "myuser"
            password = "mypass"
        }
    }
}
"#;
        let parser = GradleParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
        if let Some(MigratedCredentials::BasicInline { username, password }) = &repos[0].credentials
        {
            assert_eq!(username, "myuser");
            assert_eq!(password, "mypass");
        } else {
            panic!("Expected basic inline credentials");
        }
    }

    #[test]
    fn test_parse_env_var_credentials() {
        let content = r#"
repositories {
    maven {
        url = "https://repo.example.com/maven"
        credentials {
            username = System.getenv("REPO_USER")
            password = System.getenv("REPO_PASS")
        }
    }
}
"#;
        let parser = GradleParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
        if let Some(MigratedCredentials::BasicEnv {
            username_var,
            password_var,
        }) = &repos[0].credentials
        {
            assert_eq!(username_var, "REPO_USER");
            assert_eq!(password_var, "REPO_PASS");
        } else {
            panic!("Expected basic env credentials");
        }
    }

    #[test]
    fn test_skip_comments() {
        let content = r#"
// mavenCentral()
/* google() */
repositories {
    mavenCentral()
}
"#;
        let parser = GradleParser;
        let repos = parser.parse(content).unwrap();

        // Should only find 1, not 3
        assert_eq!(repos.len(), 1);
    }

    #[test]
    fn test_extract_quoted_string() {
        assert_eq!(
            extract_quoted_string(r#"url = "https://example.com""#),
            Some("https://example.com".to_string())
        );
        assert_eq!(
            extract_quoted_string(r#"url = 'https://example.com'"#),
            Some("https://example.com".to_string())
        );
    }
}
