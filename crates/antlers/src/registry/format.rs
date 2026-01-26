//! Format handler registry for artifact metadata formats.

use std::sync::Arc;

use gav::Artifact;

/// Metadata about a supported artifact format.
#[derive(Debug, Clone)]
pub struct FormatInfo {
    /// Short identifier (e.g., "pom", "gmm").
    pub id: &'static str,
    /// Human-readable name.
    pub name: &'static str,
    /// Short description.
    pub description: &'static str,
}

/// A handler that can detect and parse a metadata format.
pub trait FormatHandler: Send + Sync {
    /// Returns information about this format.
    fn info(&self) -> FormatInfo;

    /// Returns true if the handler can process the given artifact.
    fn can_handle(&self, artifact: &Artifact, files: &[String]) -> bool;

    /// Returns the priority for this handler (higher wins).
    fn priority(&self) -> u32;
}

/// Registry of format handlers.
pub struct FormatRegistry {
    handlers: Vec<Arc<dyn FormatHandler>>,
}

impl FormatRegistry {
    /// Creates a registry with default handlers (POM + GMM).
    #[must_use]
    #[allow(unused_mut)] // mut needed when features are enabled
    pub fn with_defaults() -> Self {
        let mut registry = Self {
            handlers: Vec::new(),
        };

        #[cfg(feature = "gmm")]
        registry.register(Arc::new(GmmFormatHandler));
        #[cfg(feature = "pom")]
        registry.register(Arc::new(PomFormatHandler));

        registry
    }

    /// Registers a format handler.
    pub fn register(&mut self, handler: Arc<dyn FormatHandler>) {
        self.handlers.push(handler);
    }

    /// Detects the best handler for the given artifact and available files.
    #[must_use]
    pub fn detect(&self, artifact: &Artifact, files: &[String]) -> Option<&dyn FormatHandler> {
        self.handlers
            .iter()
            .filter(|handler| handler.can_handle(artifact, files))
            .max_by_key(|handler| handler.priority())
            .map(std::convert::AsRef::as_ref)
    }
}

#[cfg(feature = "pom")]
struct PomFormatHandler;

#[cfg(feature = "pom")]
impl FormatHandler for PomFormatHandler {
    fn info(&self) -> FormatInfo {
        FormatInfo {
            id: "pom",
            name: "Maven POM",
            description: "Maven POM metadata",
        }
    }

    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    fn can_handle(&self, _artifact: &Artifact, files: &[String]) -> bool {
        // Maven uses lowercase extensions by convention
        files
            .iter()
            .any(|f| f.ends_with(".pom") || f.ends_with(".pom.xml"))
    }

    fn priority(&self) -> u32 {
        10
    }
}

#[cfg(feature = "gmm")]
struct GmmFormatHandler;

#[cfg(feature = "gmm")]
impl FormatHandler for GmmFormatHandler {
    fn info(&self) -> FormatInfo {
        FormatInfo {
            id: "gmm",
            name: "Gradle Module Metadata",
            description: "Gradle module metadata (.module)",
        }
    }

    fn can_handle(&self, _artifact: &Artifact, files: &[String]) -> bool {
        files.iter().any(|f| f.ends_with(".module"))
    }

    fn priority(&self) -> u32 {
        100
    }
}
