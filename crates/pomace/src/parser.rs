//! XML parsing for Maven POM files.
//!
//! This module provides the [`PomParser`] for parsing Maven POM XML files.

use crate::{Error, Pom, Result};

/// Parser for Maven POM XML files.
///
/// # Examples
///
/// ```
/// use pomace::PomParser;
///
/// let xml = r#"
///     <project>
///         <groupId>com.example</groupId>
///         <artifactId>my-lib</artifactId>
///         <version>1.0.0</version>
///     </project>
/// "#;
///
/// let pom = PomParser::parse(xml).unwrap();
/// assert_eq!(pom.group_id, Some("com.example".to_string()));
/// ```
pub struct PomParser;

impl PomParser {
    /// Parse a POM from XML content.
    ///
    /// # Errors
    ///
    /// Returns an error if the XML cannot be parsed or doesn't represent a valid POM.
    pub fn parse(xml: &str) -> Result<Pom> {
        let cleaned = Self::clean_xml(xml);
        quick_xml::de::from_str(&cleaned).map_err(|e| Error::Parse {
            artifact: String::new(),
            details: e.to_string(),
        })
    }

    /// Parse a POM from XML content with artifact context for better error messages.
    ///
    /// # Errors
    ///
    /// Returns an error if the XML cannot be parsed, with the artifact coordinate
    /// included in the error message for easier debugging.
    pub fn parse_with_context(xml: &str, artifact: &str) -> Result<Pom> {
        let cleaned = Self::clean_xml(xml);
        quick_xml::de::from_str(&cleaned).map_err(|e| Error::Parse {
            artifact: artifact.to_string(),
            details: e.to_string(),
        })
    }

    /// Clean XML content for parsing.
    ///
    /// This removes XML declarations and namespace attributes that can interfere
    /// with deserialization.
    fn clean_xml(xml: &str) -> String {
        // Remove XML declaration
        xml.lines()
            .filter(|line| !line.trim().starts_with("<?xml"))
            .collect::<Vec<_>>()
            .join("\n")
            // Remove namespace declarations
            .replace("xmlns=\"http://maven.apache.org/POM/4.0.0\"", "")
            .replace(
                "xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"",
                "",
            )
            .replace(
                r#"xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd""#,
                "",
            )
            .replace(
                r#"xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 https://maven.apache.org/xsd/maven-4.0.0.xsd""#,
                "",
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_pom() {
        let xml = r"
            <project>
                <modelVersion>4.0.0</modelVersion>
                <groupId>com.example</groupId>
                <artifactId>my-lib</artifactId>
                <version>1.0.0</version>
            </project>
        ";

        let pom = PomParser::parse(xml).unwrap();
        assert_eq!(pom.group_id, Some("com.example".to_string()));
        assert_eq!(pom.artifact_id, Some("my-lib".to_string()));
        assert_eq!(pom.version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_parse_pom_with_namespace() {
        let xml = r#"
            <?xml version="1.0" encoding="UTF-8"?>
            <project xmlns="http://maven.apache.org/POM/4.0.0"
                     xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
                     xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd">
                <modelVersion>4.0.0</modelVersion>
                <groupId>com.example</groupId>
                <artifactId>my-lib</artifactId>
                <version>1.0.0</version>
            </project>
        "#;

        let pom = PomParser::parse(xml).unwrap();
        assert_eq!(pom.group_id, Some("com.example".to_string()));
    }

    #[test]
    fn test_parse_pom_with_parent() {
        let xml = r"
            <project>
                <modelVersion>4.0.0</modelVersion>
                <parent>
                    <groupId>com.example</groupId>
                    <artifactId>parent</artifactId>
                    <version>1.0.0</version>
                </parent>
                <artifactId>my-lib</artifactId>
            </project>
        ";

        let pom = PomParser::parse(xml).unwrap();
        assert!(pom.parent.is_some());
        let parent = pom.parent.as_ref().unwrap();
        assert_eq!(parent.group_id, "com.example");
        assert_eq!(parent.artifact_id, "parent");
        assert_eq!(parent.version, "1.0.0");
        assert_eq!(pom.effective_group_id(), Some("com.example"));
    }

    #[test]
    fn test_parse_pom_with_dependencies() {
        let xml = r"
            <project>
                <modelVersion>4.0.0</modelVersion>
                <groupId>com.example</groupId>
                <artifactId>my-lib</artifactId>
                <version>1.0.0</version>
                <dependencies>
                    <dependency>
                        <groupId>org.slf4j</groupId>
                        <artifactId>slf4j-api</artifactId>
                        <version>2.0.0</version>
                    </dependency>
                    <dependency>
                        <groupId>junit</groupId>
                        <artifactId>junit</artifactId>
                        <version>4.13.2</version>
                        <scope>test</scope>
                    </dependency>
                </dependencies>
            </project>
        ";

        let pom = PomParser::parse(xml).unwrap();
        let deps = pom.direct_dependencies();
        assert_eq!(deps.len(), 2);

        assert_eq!(deps[0].group_id, "org.slf4j");
        assert_eq!(deps[0].artifact_id, "slf4j-api");
        assert!(deps[0].should_include());

        assert_eq!(deps[1].group_id, "junit");
        assert_eq!(deps[1].scope, Some("test".to_string()));
        assert!(!deps[1].should_include());
    }

    #[test]
    fn test_parse_pom_with_dependency_management() {
        let xml = r"
            <project>
                <modelVersion>4.0.0</modelVersion>
                <groupId>com.example</groupId>
                <artifactId>my-lib</artifactId>
                <version>1.0.0</version>
                <dependencyManagement>
                    <dependencies>
                        <dependency>
                            <groupId>org.slf4j</groupId>
                            <artifactId>slf4j-api</artifactId>
                            <version>2.0.0</version>
                        </dependency>
                    </dependencies>
                </dependencyManagement>
            </project>
        ";

        let pom = PomParser::parse(xml).unwrap();
        let managed = pom.managed_dependencies();
        assert_eq!(managed.len(), 1);
        assert_eq!(managed[0].group_id, "org.slf4j");
        assert_eq!(managed[0].version, Some("2.0.0".to_string()));
    }

    #[test]
    fn test_parse_pom_with_bom_import() {
        let xml = r"
            <project>
                <modelVersion>4.0.0</modelVersion>
                <groupId>com.example</groupId>
                <artifactId>my-lib</artifactId>
                <version>1.0.0</version>
                <dependencyManagement>
                    <dependencies>
                        <dependency>
                            <groupId>org.springframework.boot</groupId>
                            <artifactId>spring-boot-dependencies</artifactId>
                            <version>3.0.0</version>
                            <type>pom</type>
                            <scope>import</scope>
                        </dependency>
                    </dependencies>
                </dependencyManagement>
            </project>
        ";

        let pom = PomParser::parse(xml).unwrap();
        let boms = pom.bom_imports();
        assert_eq!(boms.len(), 1);
        assert!(boms[0].is_bom_import());
    }

    #[test]
    fn test_parse_pom_with_exclusions() {
        let xml = r"
            <project>
                <modelVersion>4.0.0</modelVersion>
                <groupId>com.example</groupId>
                <artifactId>my-lib</artifactId>
                <version>1.0.0</version>
                <dependencies>
                    <dependency>
                        <groupId>org.example</groupId>
                        <artifactId>lib</artifactId>
                        <version>1.0.0</version>
                        <exclusions>
                            <exclusion>
                                <groupId>commons-logging</groupId>
                                <artifactId>commons-logging</artifactId>
                            </exclusion>
                        </exclusions>
                    </dependency>
                </dependencies>
            </project>
        ";

        let pom = PomParser::parse(xml).unwrap();
        let deps = pom.direct_dependencies();
        assert_eq!(deps.len(), 1);

        let exclusions = deps[0].exclusions.as_ref().unwrap();
        assert_eq!(exclusions.exclusions.len(), 1);
        assert!(exclusions.exclusions[0].matches("commons-logging", "commons-logging"));
    }

    #[test]
    fn test_parse_with_context_error() {
        let xml = "not valid xml";
        let result = PomParser::parse_with_context(xml, "com.example:my-lib:1.0.0");
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(err_str.contains("com.example:my-lib:1.0.0"));
    }

    #[test]
    fn test_parse_pom_with_profiles() {
        let xml = r"
            <project>
                <modelVersion>4.0.0</modelVersion>
                <groupId>com.example</groupId>
                <artifactId>my-lib</artifactId>
                <version>1.0.0</version>
                <profiles>
                    <profile>
                        <id>dev</id>
                        <activation>
                            <activeByDefault>true</activeByDefault>
                        </activation>
                    </profile>
                </profiles>
            </project>
        ";

        let pom = PomParser::parse(xml).unwrap();
        let profiles = pom.profiles();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id, "dev");
        assert!(
            profiles[0]
                .activation
                .as_ref()
                .is_some_and(|a| a.active_by_default)
        );
    }
}
