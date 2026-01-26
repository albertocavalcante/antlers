//! Error types for maven-pom parsing.

use thiserror::Error;

/// A specialized Result type for POM operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur when parsing Maven POM files.
#[derive(Error, Debug)]
pub enum Error {
    /// Failed to parse a POM file.
    #[error("failed to parse POM for {artifact}: {details}")]
    Parse {
        /// The artifact coordinate (if known).
        artifact: String,
        /// Details about the parsing failure.
        details: String,
    },

    /// XML parsing error.
    #[error("XML error: {0}")]
    Xml(#[from] quick_xml::Error),

    /// XML deserialization error.
    #[error("XML deserialization error: {0}")]
    XmlDe(#[from] quick_xml::DeError),
}
