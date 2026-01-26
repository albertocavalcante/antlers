//! Minimal lockfile resolver outputs for Bazel/Buck2 integration.

pub mod output;

use antlers_lock::Lockfile;

/// Supported output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// JSON output.
    Json,
    /// Bazel BUILD output.
    Bazel,
    /// Buck2 BUCK output.
    Buck,
    /// Starlark (.bzl) output.
    Starlark,
}

/// Options that affect output formatting.
#[derive(Debug, Clone)]
pub struct FormatOptions {
    /// Bazel workspace name for label generation.
    pub workspace_name: String,
    /// Fail on missing checksums.
    pub strict: bool,
}

/// Errors that can occur during formatting.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid artifact coordinates in the lockfile.
    #[error("invalid coordinates: {0}")]
    InvalidCoordinates(String),
    /// Required checksum is missing.
    #[error("missing checksum for {0}")]
    MissingChecksum(String),
    /// Lockfile parse error.
    #[error(transparent)]
    Lockfile(#[from] antlers_lock::Error),
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Error writing output file.
    #[error("failed to write output file '{path}': {source}")]
    WriteFile {
        /// The path that failed to write.
        path: std::path::PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Formats a lockfile using the selected format and options.
pub fn format_lockfile(
    format: Format,
    lockfile: &Lockfile,
    options: &FormatOptions,
) -> Result<String> {
    match format {
        Format::Json => output::json::format(lockfile),
        Format::Bazel => output::bazel::format(lockfile, options),
        Format::Buck => output::buck::format(lockfile, options),
        Format::Starlark => output::starlark::format(lockfile, options),
    }
}
