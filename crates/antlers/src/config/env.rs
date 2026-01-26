//! Environment variable access control for hermetic builds.
//!
//! This module provides fine-grained control over which environment
//! variables antler can read. For hermetic builds, environment access
//! must be explicitly allowed.

use std::collections::HashSet;
use std::env;

/// Environment variable with its source.
#[derive(Debug, Clone)]
pub struct EnvVar {
    /// Variable name.
    pub name: String,
    /// Variable value.
    pub value: String,
    /// Whether this was explicitly provided (vs read from environment).
    pub explicit: bool,
}

/// Environment variable access control.
#[derive(Debug, Clone)]
pub struct EnvConfig {
    /// Allowlist of environment variable names that can be read.
    /// Empty means all are blocked.
    allowed: HashSet<String>,

    /// Explicitly provided environment variables.
    /// These override any environment reads.
    explicit: Vec<EnvVar>,

    /// Whether to allow all environment variables (non-hermetic).
    allow_all: bool,

    /// Whether to inherit proxy settings (`HTTP_PROXY`, `HTTPS_PROXY`, etc).
    inherit_proxy: bool,
}

impl Default for EnvConfig {
    fn default() -> Self {
        Self::interactive()
    }
}

impl EnvConfig {
    /// Creates a configuration that allows common environment variables.
    ///
    /// This is the default for interactive use. Allows:
    /// - Proxy settings (`HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`)
    /// - XDG directories (`XDG_CACHE_HOME`, `XDG_CONFIG_HOME`)
    /// - Home directory (`HOME`, `USERPROFILE`)
    #[must_use]
    pub fn interactive() -> Self {
        let mut allowed = HashSet::new();

        // Proxy settings
        allowed.insert("HTTP_PROXY".to_string());
        allowed.insert("HTTPS_PROXY".to_string());
        allowed.insert("http_proxy".to_string());
        allowed.insert("https_proxy".to_string());
        allowed.insert("NO_PROXY".to_string());
        allowed.insert("no_proxy".to_string());

        // XDG directories
        allowed.insert("XDG_CACHE_HOME".to_string());
        allowed.insert("XDG_CONFIG_HOME".to_string());
        allowed.insert("XDG_DATA_HOME".to_string());

        // Home directory
        allowed.insert("HOME".to_string());
        allowed.insert("USERPROFILE".to_string());

        Self {
            allowed,
            explicit: Vec::new(),
            allow_all: false,
            inherit_proxy: true,
        }
    }

    /// Creates a configuration that blocks all environment access.
    ///
    /// This is required for strict hermetic builds.
    #[must_use]
    pub fn blocked() -> Self {
        Self {
            allowed: HashSet::new(),
            explicit: Vec::new(),
            allow_all: false,
            inherit_proxy: false,
        }
    }

    /// Creates a configuration suitable for CI environments.
    ///
    /// Allows proxy settings but blocks other environment access.
    #[must_use]
    pub fn ci_defaults() -> Self {
        let mut config = Self::blocked();
        config.inherit_proxy = true;

        // Allow proxy settings
        config.allow("HTTP_PROXY");
        config.allow("HTTPS_PROXY");
        config.allow("http_proxy");
        config.allow("https_proxy");
        config.allow("NO_PROXY");
        config.allow("no_proxy");

        config
    }

    /// Creates a configuration that allows all environment variables.
    ///
    /// **Warning**: This is not hermetic.
    #[must_use]
    pub fn allow_all() -> Self {
        Self {
            allowed: HashSet::new(),
            explicit: Vec::new(),
            allow_all: true,
            inherit_proxy: true,
        }
    }

    /// Allows access to a specific environment variable.
    pub fn allow(&mut self, name: &str) {
        self.allowed.insert(name.to_string());
    }

    /// Denies access to a specific environment variable.
    pub fn deny(&mut self, name: &str) {
        self.allowed.remove(name);
    }

    /// Sets an explicit environment variable value.
    ///
    /// Explicit values are used instead of reading from the environment,
    /// making the build hermetic for that variable.
    pub fn set(&mut self, name: &str, value: &str) {
        // Remove any existing explicit value
        self.explicit.retain(|v| v.name != name);
        self.explicit.push(EnvVar {
            name: name.to_string(),
            value: value.to_string(),
            explicit: true,
        });
    }

    /// Gets an environment variable value.
    ///
    /// Returns explicit values first, then checks allowed environment variables.
    /// Returns `None` if the variable is not allowed or not set.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<EnvVar> {
        // Check explicit values first
        if let Some(var) = self.explicit.iter().find(|v| v.name == name) {
            return Some(var.clone());
        }

        // Check if allowed to read from environment
        if !self.allow_all && !self.allowed.contains(name) {
            return None;
        }

        // Read from environment
        env::var(name).ok().map(|value| EnvVar {
            name: name.to_string(),
            value,
            explicit: false,
        })
    }

    /// Gets a required environment variable.
    ///
    /// # Errors
    ///
    /// Returns an error if the variable is not set or not allowed.
    pub fn require(&self, name: &str) -> Result<EnvVar, EnvError> {
        self.get(name).ok_or_else(|| {
            if !self.allow_all && !self.allowed.contains(name) {
                EnvError::NotAllowed(name.to_string())
            } else {
                EnvError::NotSet(name.to_string())
            }
        })
    }

    /// Returns whether the configuration is hermetic.
    ///
    /// A configuration is hermetic if it doesn't allow reading arbitrary
    /// environment variables.
    #[must_use]
    pub const fn is_hermetic(&self) -> bool {
        !self.allow_all
    }

    /// Returns all explicitly set variables.
    #[must_use]
    pub fn explicit_vars(&self) -> &[EnvVar] {
        &self.explicit
    }

    /// Returns all allowed variable names.
    #[must_use]
    pub fn allowed_vars(&self) -> Vec<&str> {
        self.allowed.iter().map(String::as_str).collect()
    }
}

/// Environment access error.
#[derive(Debug, Clone, thiserror::Error)]
pub enum EnvError {
    /// Environment variable is not in the allowlist.
    #[error("Environment variable '{0}' is not allowed (hermetic mode)")]
    NotAllowed(String),

    /// Environment variable is not set.
    #[error("Environment variable '{0}' is not set")]
    NotSet(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocked_config() {
        let config = EnvConfig::blocked();
        assert!(config.get("HOME").is_none());
        assert!(config.is_hermetic());
    }

    #[test]
    fn test_explicit_value() {
        let mut config = EnvConfig::blocked();
        config.set("MY_VAR", "my_value");

        let var = config.get("MY_VAR").unwrap();
        assert_eq!(var.value, "my_value");
        assert!(var.explicit);
    }

    #[test]
    fn test_explicit_overrides_env() {
        let mut config = EnvConfig::allow_all();
        config.set("PATH", "/explicit/path");

        let var = config.get("PATH").unwrap();
        assert_eq!(var.value, "/explicit/path");
        assert!(var.explicit);
    }

    #[test]
    fn test_allowed_var() {
        let mut config = EnvConfig::blocked();
        config.allow("TEST_VAR");

        // Can't test actual env read without setting, but we can verify allow works
        assert!(config.allowed.contains("TEST_VAR"));
    }

    #[test]
    fn test_require_not_allowed() {
        let config = EnvConfig::blocked();
        let result = config.require("BLOCKED_VAR");
        assert!(matches!(result, Err(EnvError::NotAllowed(_))));
    }

    #[test]
    fn test_interactive_allows_proxy() {
        let config = EnvConfig::interactive();
        assert!(config.allowed.contains("HTTP_PROXY"));
        assert!(config.allowed.contains("HTTPS_PROXY"));
    }

    #[test]
    fn test_ci_defaults() {
        let config = EnvConfig::ci_defaults();
        assert!(config.is_hermetic());
        assert!(config.allowed.contains("HTTP_PROXY"));
        assert!(!config.allowed.contains("HOME"));
    }
}
