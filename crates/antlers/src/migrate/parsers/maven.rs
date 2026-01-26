//! Parser for Maven settings.xml configuration files.
//!
//! Supports:
//! - <servers> - credential configuration
//! - <mirrors> - repository mirrors
//! - <profiles><profile><repositories> - repository definitions

use gather::Ecosystem;
use serde::Deserialize;

use crate::migrate::{MigratedCredentials, MigratedRepository, MigrationError, SourceParser};

/// Parser for Maven settings.xml files.
pub struct MavenParser;

impl SourceParser for MavenParser {
    fn parse(&self, content: &str) -> Result<Vec<MigratedRepository>, MigrationError> {
        let settings: MavenSettings =
            quick_xml::de::from_str(content).map_err(MigrationError::Xml)?;

        let mut repositories = Vec::new();

        // Process mirrors
        for mirror in &settings.mirrors.mirrors {
            let credentials = settings.find_server_credentials(&mirror.id);
            repositories.push(MigratedRepository {
                id: mirror.id.clone(),
                name: mirror.name.clone(),
                url: mirror.url.clone(),
                ecosystem: Ecosystem::Maven,
                credentials,
            });
        }

        // Process repositories from profiles
        for profile in &settings.profiles.profiles {
            for repo in &profile.repositories.repositories {
                // Skip if we already have this URL from mirrors
                if repositories.iter().any(|r| r.url == repo.url) {
                    continue;
                }

                let credentials = settings.find_server_credentials(&repo.id);
                repositories.push(MigratedRepository {
                    id: repo.id.clone(),
                    name: repo.name.clone(),
                    url: repo.url.clone(),
                    ecosystem: Ecosystem::Maven,
                    credentials,
                });
            }
        }

        // Create entries for servers that don't have matching repositories
        // (useful for deploying)
        for server in &settings.servers.servers {
            if !repositories.iter().any(|r| r.id == server.id) {
                // We have credentials but no URL - create a placeholder
                let credentials = server_to_credentials(server);
                if credentials.is_some() {
                    repositories.push(MigratedRepository {
                        id: server.id.clone(),
                        name: Some(format!("Server: {}", server.id)),
                        url: format!("https://{}.example.com/", server.id),
                        ecosystem: Ecosystem::Maven,
                        credentials,
                    });
                }
            }
        }

        Ok(repositories)
    }

    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Maven
    }
}

impl MavenSettings {
    /// Finds credentials for a server ID.
    fn find_server_credentials(&self, id: &str) -> Option<MigratedCredentials> {
        self.servers
            .servers
            .iter()
            .find(|s| s.id == id)
            .and_then(server_to_credentials)
    }
}

/// Converts a Maven server to migrated credentials.
fn server_to_credentials(server: &Server) -> Option<MigratedCredentials> {
    // Check for password (basic auth)
    if let (Some(username), Some(password)) = (&server.username, &server.password) {
        // Check for env var references ${env.VAR}
        if username.starts_with("${env.") {
            let u_var = username[6..username.len() - 1].to_string();
            let p_var = if password.starts_with("${env.") {
                password[6..password.len() - 1].to_string()
            } else {
                format!("{u_var}_PASSWORD")
            };
            return Some(MigratedCredentials::BasicEnv {
                username_var: u_var,
                password_var: p_var,
            });
        }

        return Some(MigratedCredentials::BasicInline {
            username: username.clone(),
            password: password.clone(),
        });
    }

    None
}

/// Maven settings.xml structure.
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct MavenSettings {
    #[serde(default)]
    servers: Servers,
    #[serde(default)]
    mirrors: Mirrors,
    #[serde(default)]
    profiles: Profiles,
}

#[derive(Debug, Deserialize, Default)]
struct Servers {
    #[serde(rename = "server", default)]
    servers: Vec<Server>,
}

#[derive(Debug, Deserialize)]
struct Server {
    id: String,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    password: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct Mirrors {
    #[serde(rename = "mirror", default)]
    mirrors: Vec<Mirror>,
}

#[derive(Debug, Deserialize)]
struct Mirror {
    id: String,
    #[serde(default)]
    name: Option<String>,
    url: String,
    #[serde(default, rename = "mirrorOf")]
    mirror_of: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct Profiles {
    #[serde(rename = "profile", default)]
    profiles: Vec<Profile>,
}

#[derive(Debug, Deserialize)]
struct Profile {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    repositories: Repositories,
}

#[derive(Debug, Deserialize, Default)]
struct Repositories {
    #[serde(rename = "repository", default)]
    repositories: Vec<Repository>,
}

#[derive(Debug, Deserialize)]
struct Repository {
    id: String,
    #[serde(default)]
    name: Option<String>,
    url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_servers() {
        let content = r#"
<settings>
    <servers>
        <server>
            <id>github</id>
            <username>myuser</username>
            <password>mytoken</password>
        </server>
    </servers>
</settings>
"#;
        let parser = MavenParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].id, "github");

        if let Some(MigratedCredentials::BasicInline { username, password }) = &repos[0].credentials
        {
            assert_eq!(username, "myuser");
            assert_eq!(password, "mytoken");
        } else {
            panic!("Expected basic inline credentials");
        }
    }

    #[test]
    fn test_parse_mirrors() {
        let content = r#"
<settings>
    <servers>
        <server>
            <id>artifactory</id>
            <username>admin</username>
            <password>secret</password>
        </server>
    </servers>
    <mirrors>
        <mirror>
            <id>artifactory</id>
            <name>Artifactory Mirror</name>
            <url>https://artifactory.example.com/maven</url>
            <mirrorOf>central</mirrorOf>
        </mirror>
    </mirrors>
</settings>
"#;
        let parser = MavenParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].id, "artifactory");
        assert_eq!(repos[0].name, Some("Artifactory Mirror".to_string()));
        assert_eq!(repos[0].url, "https://artifactory.example.com/maven");
        assert!(repos[0].credentials.is_some());
    }

    #[test]
    fn test_parse_profiles_repositories() {
        let content = r#"
<settings>
    <profiles>
        <profile>
            <id>dev</id>
            <repositories>
                <repository>
                    <id>private-repo</id>
                    <name>Private Repository</name>
                    <url>https://repo.example.com/maven</url>
                </repository>
            </repositories>
        </profile>
    </profiles>
</settings>
"#;
        let parser = MavenParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].id, "private-repo");
        assert_eq!(repos[0].name, Some("Private Repository".to_string()));
    }

    #[test]
    fn test_parse_env_var_credentials() {
        let content = r#"
<settings>
    <servers>
        <server>
            <id>github</id>
            <username>${env.GITHUB_USER}</username>
            <password>${env.GITHUB_TOKEN}</password>
        </server>
    </servers>
</settings>
"#;
        let parser = MavenParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);

        if let Some(MigratedCredentials::BasicEnv {
            username_var,
            password_var,
        }) = &repos[0].credentials
        {
            assert_eq!(username_var, "GITHUB_USER");
            assert_eq!(password_var, "GITHUB_TOKEN");
        } else {
            panic!("Expected basic env credentials");
        }
    }

    #[test]
    fn test_empty_settings() {
        let content = "<settings></settings>";
        let parser = MavenParser;
        let repos = parser.parse(content).unwrap();

        assert!(repos.is_empty());
    }
}
