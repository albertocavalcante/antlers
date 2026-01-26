//! Authentication support for Maven repositories.
//!
//! This module provides credential handling for private repositories,
//! including Basic Auth, Bearer tokens, and `.netrc` file support.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use base64::prelude::*;
use serde::{Deserialize, Serialize};

/// A string value that can either be inline or referenced from an environment variable.
///
/// This enables TOML configs to use `{ env = "VAR_NAME" }` for credential values,
/// supporting hermetic builds where env var access is explicit.
///
/// # Examples
///
/// ```toml
/// # Inline value
/// token = "my-token"
///
/// # Environment variable reference
/// token = { env = "GITHUB_TOKEN" }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum StringOrEnvRef {
    /// An inline string value.
    Inline(String),
    /// A reference to an environment variable.
    Env {
        /// The name of the environment variable.
        env: String,
    },
}

impl StringOrEnvRef {
    /// Creates an inline string value.
    #[must_use]
    pub fn inline(value: impl Into<String>) -> Self {
        Self::Inline(value.into())
    }

    /// Creates an environment variable reference.
    #[must_use]
    pub fn env(var_name: impl Into<String>) -> Self {
        Self::Env {
            env: var_name.into(),
        }
    }

    /// Resolves the value, reading from the environment if needed.
    ///
    /// # Errors
    ///
    /// Returns `None` if this is an env ref and the variable is not set.
    #[must_use]
    pub fn resolve(&self) -> Option<String> {
        match self {
            Self::Inline(s) => Some(s.clone()),
            Self::Env { env } => std::env::var(env).ok(),
        }
    }

    /// Resolves the value using a custom environment lookup function.
    ///
    /// This is useful for hermetic builds where env access is controlled.
    #[must_use]
    pub fn resolve_with<F>(&self, env_lookup: F) -> Option<String>
    where
        F: Fn(&str) -> Option<String>,
    {
        match self {
            Self::Inline(s) => Some(s.clone()),
            Self::Env { env } => env_lookup(env),
        }
    }

    /// Returns the env var name if this is an env reference.
    #[must_use]
    pub fn env_var_name(&self) -> Option<&str> {
        match self {
            Self::Inline(_) => None,
            Self::Env { env } => Some(env),
        }
    }

    /// Returns true if this is an env reference.
    #[must_use]
    pub const fn is_env_ref(&self) -> bool {
        matches!(self, Self::Env { .. })
    }
}

/// Credentials for authenticating to a Maven repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Credentials {
    /// HTTP Basic authentication (username + password/token).
    Basic {
        /// Username for authentication.
        username: String,
        /// Password or token for authentication.
        password: String,
    },
    /// Bearer token authentication (`OAuth2`, GitHub, etc.).
    Bearer {
        /// The bearer token.
        token: String,
    },
    /// Use credentials from `~/.netrc` file.
    Netrc,
}

impl Credentials {
    /// Creates Basic auth credentials.
    #[must_use]
    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self::Basic {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Creates Bearer token credentials.
    #[must_use]
    pub fn bearer(token: impl Into<String>) -> Self {
        Self::Bearer {
            token: token.into(),
        }
    }

    /// Creates credentials that will be loaded from `~/.netrc`.
    #[must_use]
    pub const fn netrc() -> Self {
        Self::Netrc
    }

    /// Returns the Authorization header value for this credential.
    ///
    /// For `Netrc` credentials, the host is used to look up the credentials.
    #[must_use]
    pub fn authorization_header(&self, host: &str) -> Option<String> {
        match self {
            Self::Basic { username, password } => {
                let encoded = BASE64_STANDARD.encode(format!("{username}:{password}"));
                Some(format!("Basic {encoded}"))
            }
            Self::Bearer { token } => Some(format!("Bearer {token}")),
            Self::Netrc => {
                let netrc = Netrc::load();
                netrc.credentials_for(host).map(|(user, pass)| {
                    let encoded = BASE64_STANDARD.encode(format!("{user}:{pass}"));
                    format!("Basic {encoded}")
                })
            }
        }
    }
}

/// Parsed `.netrc` file for credential lookup.
#[derive(Debug, Default)]
pub struct Netrc {
    /// Machine-specific credentials.
    machines: HashMap<String, (String, String)>,
    /// Default credentials (for `default` entry).
    default: Option<(String, String)>,
}

/// Global netrc cache - loaded once on first access.
static NETRC_CACHE: OnceLock<Netrc> = OnceLock::new();

impl Netrc {
    /// Loads and caches the netrc file.
    ///
    /// The file is only read once; subsequent calls return the cached version.
    #[must_use]
    pub fn load() -> &'static Self {
        NETRC_CACHE.get_or_init(|| Self::parse_file().unwrap_or_default())
    }

    /// Gets credentials for a specific host.
    ///
    /// Returns `(username, password)` if found, or the default credentials.
    #[must_use]
    pub fn credentials_for(&self, host: &str) -> Option<(String, String)> {
        self.machines
            .get(host)
            .cloned()
            .or_else(|| self.default.clone())
    }

    /// Parses the netrc file from the default location.
    fn parse_file() -> Option<Self> {
        let path = Self::default_path()?;
        let content = fs::read_to_string(&path).ok()?;
        Some(Self::parse(&content))
    }

    /// Returns the default netrc file path.
    fn default_path() -> Option<PathBuf> {
        // Try $NETRC environment variable first
        if let Ok(path) = std::env::var("NETRC") {
            let path = PathBuf::from(path);
            if path.exists() {
                return Some(path);
            }
        }

        // Then try ~/.netrc (Unix) or ~/_netrc (Windows)
        let home = dirs::home_dir()?;

        #[cfg(windows)]
        let netrc_path = home.join("_netrc");
        #[cfg(not(windows))]
        let netrc_path = home.join(".netrc");

        if netrc_path.exists() {
            Some(netrc_path)
        } else {
            None
        }
    }

    /// Parses netrc content.
    ///
    /// Format:
    /// ```text
    /// machine hostname
    ///   login username
    ///   password secret
    ///
    /// default
    ///   login username
    ///   password secret
    /// ```
    fn parse(content: &str) -> Self {
        let mut netrc = Self::default();
        let mut current_machine: Option<String> = None;
        let mut current_login: Option<String> = None;
        let mut current_password: Option<String> = None;
        let mut is_default = false;

        let tokens: Vec<&str> = content.split_whitespace().collect();
        let mut i = 0;

        while i < tokens.len() {
            match tokens[i] {
                "machine" => {
                    // Save previous entry
                    Self::save_entry(
                        &mut netrc,
                        current_machine.as_ref(),
                        current_login.as_ref(),
                        current_password.as_ref(),
                        is_default,
                    );

                    is_default = false;
                    current_machine = tokens.get(i + 1).map(|s| (*s).to_string());
                    current_login = None;
                    current_password = None;
                    i += 2;
                }
                "default" => {
                    // Save previous entry
                    Self::save_entry(
                        &mut netrc,
                        current_machine.as_ref(),
                        current_login.as_ref(),
                        current_password.as_ref(),
                        is_default,
                    );

                    is_default = true;
                    current_machine = None;
                    current_login = None;
                    current_password = None;
                    i += 1;
                }
                "login" => {
                    current_login = tokens.get(i + 1).map(|s| (*s).to_string());
                    i += 2;
                }
                "password" => {
                    current_password = tokens.get(i + 1).map(|s| (*s).to_string());
                    i += 2;
                }
                "account" | "macdef" => {
                    // Skip these tokens and their values
                    i += 2;
                }
                _ => {
                    i += 1;
                }
            }
        }

        // Save last entry
        Self::save_entry(
            &mut netrc,
            current_machine.as_ref(),
            current_login.as_ref(),
            current_password.as_ref(),
            is_default,
        );

        netrc
    }

    fn save_entry(
        netrc: &mut Self,
        machine: Option<&String>,
        login: Option<&String>,
        password: Option<&String>,
        is_default: bool,
    ) {
        if let (Some(login), Some(password)) = (login, password) {
            if is_default {
                netrc.default = Some((login.clone(), password.clone()));
            } else if let Some(machine) = machine {
                netrc
                    .machines
                    .insert(machine.clone(), (login.clone(), password.clone()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_auth_header() {
        let creds = Credentials::basic("user", "pass");
        let header = creds.authorization_header("example.com").unwrap();
        assert!(header.starts_with("Basic "));
        // Base64 of "user:pass"
        assert!(header.contains("dXNlcjpwYXNz"));
    }

    #[test]
    fn test_bearer_auth_header() {
        let creds = Credentials::bearer("my-token-123");
        let header = creds.authorization_header("example.com").unwrap();
        assert_eq!(header, "Bearer my-token-123");
    }

    #[test]
    fn test_netrc_parse() {
        let content = r"
            machine github.com
              login myuser
              password ghp_token123

            machine maven.pkg.github.com
              login myuser
              password ghp_token123

            default
              login anonymous
              password anonymous
        ";

        let netrc = Netrc::parse(content);

        let (user, pass) = netrc.credentials_for("github.com").unwrap();
        assert_eq!(user, "myuser");
        assert_eq!(pass, "ghp_token123");

        let (user, pass) = netrc.credentials_for("maven.pkg.github.com").unwrap();
        assert_eq!(user, "myuser");
        assert_eq!(pass, "ghp_token123");

        // Unknown host falls back to default
        let (user, pass) = netrc.credentials_for("unknown.com").unwrap();
        assert_eq!(user, "anonymous");
        assert_eq!(pass, "anonymous");
    }

    #[test]
    fn test_netrc_parse_compact() {
        let content = "machine example.com login user password secret";
        let netrc = Netrc::parse(content);

        let (user, pass) = netrc.credentials_for("example.com").unwrap();
        assert_eq!(user, "user");
        assert_eq!(pass, "secret");
    }

    #[test]
    fn test_netrc_no_default() {
        let content = "machine example.com login user password secret";
        let netrc = Netrc::parse(content);

        assert!(netrc.credentials_for("unknown.com").is_none());
    }

    #[test]
    fn test_netrc_empty() {
        let netrc = Netrc::parse("");
        assert!(netrc.credentials_for("example.com").is_none());
    }

    #[test]
    fn test_netrc_with_comments() {
        // Comments aren't standard in netrc, but we should handle garbage gracefully
        let content = r"
            machine example.com
            login user
            password secret
            # this is not a valid token
            somegarbage
        ";
        let netrc = Netrc::parse(content);
        let (user, pass) = netrc.credentials_for("example.com").unwrap();
        assert_eq!(user, "user");
        assert_eq!(pass, "secret");
    }

    #[test]
    fn test_netrc_multiple_machines() {
        let content = r"
            machine repo1.example.com login user1 password pass1
            machine repo2.example.com login user2 password pass2
            machine repo3.example.com login user3 password pass3
        ";
        let netrc = Netrc::parse(content);

        let (u, p) = netrc.credentials_for("repo1.example.com").unwrap();
        assert_eq!((u.as_str(), p.as_str()), ("user1", "pass1"));

        let (u, p) = netrc.credentials_for("repo2.example.com").unwrap();
        assert_eq!((u.as_str(), p.as_str()), ("user2", "pass2"));

        let (u, p) = netrc.credentials_for("repo3.example.com").unwrap();
        assert_eq!((u.as_str(), p.as_str()), ("user3", "pass3"));
    }

    #[test]
    fn test_netrc_missing_password() {
        let content = "machine example.com login user";
        let netrc = Netrc::parse(content);
        // Should not match - both login and password required
        assert!(netrc.credentials_for("example.com").is_none());
    }

    #[test]
    fn test_netrc_missing_login() {
        let content = "machine example.com password secret";
        let netrc = Netrc::parse(content);
        // Should not match - both login and password required
        assert!(netrc.credentials_for("example.com").is_none());
    }

    #[test]
    fn test_credentials_basic_special_chars() {
        // Test with special characters in password
        let creds = Credentials::basic("user", "p@ss:word/123");
        let header = creds.authorization_header("example.com").unwrap();
        assert!(header.starts_with("Basic "));
        // Verify it decodes correctly
        let encoded = header.strip_prefix("Basic ").unwrap();
        let decoded = String::from_utf8(BASE64_STANDARD.decode(encoded).unwrap()).unwrap();
        assert_eq!(decoded, "user:p@ss:word/123");
    }

    #[test]
    fn test_string_or_env_ref_inline() {
        let value = StringOrEnvRef::inline("my-token");
        assert_eq!(value.resolve(), Some("my-token".to_string()));
        assert!(!value.is_env_ref());
        assert_eq!(value.env_var_name(), None);
    }

    #[test]
    fn test_string_or_env_ref_env() {
        // Use resolve_with to simulate environment lookup safely
        let value = StringOrEnvRef::env("TEST_TOKEN_12345");
        assert!(value.is_env_ref());
        assert_eq!(value.env_var_name(), Some("TEST_TOKEN_12345"));

        // Simulate env lookup with custom function
        let result = value.resolve_with(|name| {
            if name == "TEST_TOKEN_12345" {
                Some("secret-value".to_string())
            } else {
                None
            }
        });
        assert_eq!(result, Some("secret-value".to_string()));
    }

    #[test]
    fn test_string_or_env_ref_env_not_set() {
        let value = StringOrEnvRef::env("DEFINITELY_NOT_SET_VAR_12345");
        assert_eq!(value.resolve(), None);
    }

    #[test]
    fn test_string_or_env_ref_resolve_with() {
        let value = StringOrEnvRef::env("MY_VAR");
        let result = value.resolve_with(|name| {
            if name == "MY_VAR" {
                Some("custom-value".to_string())
            } else {
                None
            }
        });
        assert_eq!(result, Some("custom-value".to_string()));
    }

    #[test]
    fn test_string_or_env_ref_serde_inline() {
        let value = StringOrEnvRef::inline("my-token");
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, r#""my-token""#);

        let parsed: StringOrEnvRef = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, value);
    }

    #[test]
    fn test_string_or_env_ref_serde_env() {
        let value = StringOrEnvRef::env("GITHUB_TOKEN");
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, r#"{"env":"GITHUB_TOKEN"}"#);

        let parsed: StringOrEnvRef = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, value);
    }
}
