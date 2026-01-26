//! Parser for Apache Ivy ivysettings.xml configuration files.
//!
//! Supports:
//! - `<credentials>` - authentication configuration
//! - `<resolvers>` - repository definitions (ibiblio, chain, url)
//!
//! Example ivysettings.xml:
//! ```xml
//! <ivysettings>
//!     <settings defaultResolver="chain"/>
//!     <credentials host="example.com" realm="Realm" username="user" passwd="pass"/>
//!     <resolvers>
//!         <chain name="main">
//!             <ibiblio name="central" m2compatible="true" root="https://repo1.maven.org/maven2/"/>
//!             <ibiblio name="private" root="https://private.example.com/repo/"/>
//!         </chain>
//!     </resolvers>
//! </ivysettings>
//! ```

use gather::Ecosystem;
use serde::Deserialize;

use crate::migrate::{MigratedCredentials, MigratedRepository, MigrationError, SourceParser};

/// Maven Central default URL used when ibiblio resolver has no explicit root.
const MAVEN_CENTRAL_URL: &str = "https://repo1.maven.org/maven2/";

/// Parser for Ivy ivysettings.xml files.
pub struct IvyParser;

impl SourceParser for IvyParser {
    fn parse(&self, content: &str) -> Result<Vec<MigratedRepository>, MigrationError> {
        let settings: IvySettings =
            quick_xml::de::from_str(content).map_err(MigrationError::Xml)?;

        let mut repositories = Vec::new();

        // Flatten all resolvers (recursively extract from chains)
        let resolvers = flatten_resolvers(settings.resolvers.as_ref());

        for resolver in resolvers {
            let url = resolver.url();

            // Match credentials by host
            let credentials = settings.find_credentials_for_url(&url);

            repositories.push(MigratedRepository {
                id: resolver.name.clone(),
                name: Some(resolver.name.clone()),
                url,
                ecosystem: Ecosystem::Maven,
                credentials,
            });
        }

        Ok(repositories)
    }

    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Maven
    }
}

/// Flattens nested resolver structures (chains) into a flat list of ibiblio/url resolvers.
fn flatten_resolvers(resolvers: Option<&IvyResolvers>) -> Vec<FlatResolver> {
    let Some(resolvers) = resolvers else {
        return Vec::new();
    };

    let mut flat = Vec::new();
    collect_resolvers(&resolvers.items, &mut flat);
    flat
}

/// Recursively collects resolvers from chains.
fn collect_resolvers(items: &[ResolverItem], flat: &mut Vec<FlatResolver>) {
    for item in items {
        match item {
            ResolverItem::Ibiblio(ibiblio) => {
                flat.push(FlatResolver {
                    name: ibiblio.name.clone(),
                    root: ibiblio.root.clone(),
                });
            }
            ResolverItem::Chain(chain) => {
                // Recursively process chain's resolvers
                collect_resolvers(&chain.resolvers, flat);
            }
            ResolverItem::Url(url_resolver) => {
                // URL resolver with artifact patterns
                if let Some(artifact) = url_resolver.artifacts.first() {
                    // Extract base URL from artifact pattern
                    let base_url = extract_base_url(&artifact.url);
                    flat.push(FlatResolver {
                        name: url_resolver.name.clone(),
                        root: Some(base_url),
                    });
                }
            }
            ResolverItem::Unknown => {}
        }
    }
}

/// Extracts the base URL from an Ivy artifact pattern.
///
/// For example, given `https://example.com/repo/[organisation]/[module]/[artifact].[ext]`,
/// this returns `https://example.com/repo/`.
fn extract_base_url(pattern: &str) -> String {
    // Find the first Ivy pattern marker
    pattern.find('[').map_or_else(
        || pattern.to_string(),
        |idx| {
            let base = &pattern[..idx];
            // Ensure it ends with /
            if base.ends_with('/') {
                base.to_string()
            } else {
                format!("{base}/")
            }
        },
    )
}

/// A flattened resolver entry.
struct FlatResolver {
    name: String,
    root: Option<String>,
}

impl FlatResolver {
    /// Returns the URL for this resolver, defaulting to Maven Central for ibiblio.
    fn url(&self) -> String {
        self.root
            .clone()
            .unwrap_or_else(|| MAVEN_CENTRAL_URL.to_string())
    }
}

impl IvySettings {
    /// Finds credentials matching a URL by host.
    fn find_credentials_for_url(&self, url: &str) -> Option<MigratedCredentials> {
        let host = extract_host(url)?;

        self.credentials
            .iter()
            .find(|c| c.host == host)
            .and_then(credentials_to_migrated)
    }
}

/// Extracts the host from a URL.
fn extract_host(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(String::from))
}

/// Converts Ivy credentials to migrated credentials.
fn credentials_to_migrated(creds: &IvyCredentials) -> Option<MigratedCredentials> {
    let username = creds.username.as_ref()?;
    let password = creds.password.as_ref()?;

    // Check for env var references ${env.VAR}
    if username.starts_with("${env.") && username.ends_with('}') {
        let u_var = username[6..username.len() - 1].to_string();
        let p_var = if password.starts_with("${env.") && password.ends_with('}') {
            password[6..password.len() - 1].to_string()
        } else {
            format!("{u_var}_PASSWORD")
        };
        return Some(MigratedCredentials::BasicEnv {
            username_var: u_var,
            password_var: p_var,
        });
    }

    Some(MigratedCredentials::BasicInline {
        username: username.clone(),
        password: password.clone(),
    })
}

/// Ivy settings.xml root structure.
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct IvySettings {
    #[serde(rename = "credentials", default)]
    credentials: Vec<IvyCredentials>,
    #[serde(default)]
    resolvers: Option<IvyResolvers>,
}

/// Ivy credentials configuration.
#[derive(Debug, Deserialize)]
struct IvyCredentials {
    #[serde(rename = "@host")]
    host: String,
    #[serde(rename = "@username", default)]
    username: Option<String>,
    #[serde(rename = "@passwd", alias = "@password", default)]
    password: Option<String>,
}

/// Container for resolver definitions.
#[derive(Debug, Deserialize, Default)]
struct IvyResolvers {
    #[serde(rename = "$value", default)]
    items: Vec<ResolverItem>,
}

/// Different types of Ivy resolvers.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ResolverItem {
    Ibiblio(IbiblioDef),
    Chain(ChainDef),
    Url(UrlDef),
    #[serde(other)]
    Unknown,
}

/// Ibiblio resolver (Maven-compatible repository).
#[derive(Debug, Deserialize)]
struct IbiblioDef {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@root", default)]
    root: Option<String>,
    /// Indicates Maven 2 compatibility (parsed but not currently used).
    #[allow(dead_code)]
    #[serde(rename = "@m2compatible", default)]
    m2compatible: Option<bool>,
}

/// Chain resolver containing multiple child resolvers.
#[derive(Debug, Deserialize)]
struct ChainDef {
    #[allow(dead_code)]
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "$value", default)]
    resolvers: Vec<ResolverItem>,
}

/// URL resolver with custom artifact patterns.
#[derive(Debug, Deserialize)]
struct UrlDef {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "artifact", default)]
    artifacts: Vec<ArtifactPattern>,
}

/// Artifact pattern definition.
#[derive(Debug, Deserialize)]
struct ArtifactPattern {
    #[serde(rename = "@url", default)]
    url: String,
}

#[cfg(test)]
#[allow(clippy::needless_raw_string_hashes)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_ibiblio() {
        let content = r#"
<ivysettings>
    <resolvers>
        <ibiblio name="central" m2compatible="true" root="https://repo1.maven.org/maven2/"/>
    </resolvers>
</ivysettings>
"#;
        let parser = IvyParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].id, "central");
        assert_eq!(repos[0].url, "https://repo1.maven.org/maven2/");
        assert!(repos[0].credentials.is_none());
    }

    #[test]
    fn test_parse_ibiblio_default_root() {
        let content = r#"
<ivysettings>
    <resolvers>
        <ibiblio name="central" m2compatible="true"/>
    </resolvers>
</ivysettings>
"#;
        let parser = IvyParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].url, MAVEN_CENTRAL_URL);
    }

    #[test]
    fn test_parse_chain_resolver() {
        let content = r#"
<ivysettings>
    <resolvers>
        <chain name="main">
            <ibiblio name="central" root="https://repo1.maven.org/maven2/"/>
            <ibiblio name="private" root="https://private.example.com/repo/"/>
        </chain>
    </resolvers>
</ivysettings>
"#;
        let parser = IvyParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].id, "central");
        assert_eq!(repos[1].id, "private");
        assert_eq!(repos[1].url, "https://private.example.com/repo/");
    }

    #[test]
    fn test_parse_with_credentials() {
        let content = r#"
<ivysettings>
    <credentials host="private.example.com" username="myuser" passwd="mytoken"/>
    <resolvers>
        <ibiblio name="private" root="https://private.example.com/repo/"/>
    </resolvers>
</ivysettings>
"#;
        let parser = IvyParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].id, "private");

        if let Some(MigratedCredentials::BasicInline { username, password }) = &repos[0].credentials
        {
            assert_eq!(username, "myuser");
            assert_eq!(password, "mytoken");
        } else {
            panic!("Expected basic inline credentials");
        }
    }

    #[test]
    fn test_parse_env_var_credentials() {
        let content = r#"
<ivysettings>
    <credentials host="private.example.com" username="${env.REPO_USER}" passwd="${env.REPO_PASS}"/>
    <resolvers>
        <ibiblio name="private" root="https://private.example.com/repo/"/>
    </resolvers>
</ivysettings>
"#;
        let parser = IvyParser;
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
    fn test_parse_multiple_credentials() {
        let content = r#"
<ivysettings>
    <credentials host="repo1.example.com" username="user1" passwd="pass1"/>
    <credentials host="repo2.example.com" username="user2" passwd="pass2"/>
    <resolvers>
        <ibiblio name="repo1" root="https://repo1.example.com/maven/"/>
        <ibiblio name="repo2" root="https://repo2.example.com/maven/"/>
    </resolvers>
</ivysettings>
"#;
        let parser = IvyParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 2);

        // First repo should have first credentials
        if let Some(MigratedCredentials::BasicInline { username, .. }) = &repos[0].credentials {
            assert_eq!(username, "user1");
        } else {
            panic!("Expected credentials for repo1");
        }

        // Second repo should have second credentials
        if let Some(MigratedCredentials::BasicInline { username, .. }) = &repos[1].credentials {
            assert_eq!(username, "user2");
        } else {
            panic!("Expected credentials for repo2");
        }
    }

    #[test]
    fn test_empty_settings() {
        let content = "<ivysettings></ivysettings>";
        let parser = IvyParser;
        let repos = parser.parse(content).unwrap();

        assert!(repos.is_empty());
    }

    #[test]
    fn test_nested_chain() {
        let content = r#"
<ivysettings>
    <resolvers>
        <chain name="outer">
            <chain name="inner">
                <ibiblio name="nested" root="https://nested.example.com/"/>
            </chain>
            <ibiblio name="sibling" root="https://sibling.example.com/"/>
        </chain>
    </resolvers>
</ivysettings>
"#;
        let parser = IvyParser;
        let repos = parser.parse(content).unwrap();

        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].id, "nested");
        assert_eq!(repos[1].id, "sibling");
    }

    #[test]
    fn test_extract_base_url() {
        assert_eq!(
            extract_base_url(
                "https://example.com/repo/[organisation]/[module]/[revision]/[artifact].[ext]"
            ),
            "https://example.com/repo/"
        );
        assert_eq!(
            extract_base_url("https://example.com/maven2"),
            "https://example.com/maven2"
        );
    }
}
