//! Output formatter registry for resolution results.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;

use dendro::Resolution;

/// Options that affect output formatting.
#[derive(Debug, Clone, Default)]
pub struct FormatOptions {
    /// Whether the output should include ANSI color codes.
    pub use_color: bool,
}

/// Metadata about an output format.
#[derive(Debug, Clone)]
pub struct OutputInfo {
    /// Short identifier (e.g., "text", "json").
    pub id: &'static str,
    /// Human-readable name.
    pub name: &'static str,
    /// Short description.
    pub description: &'static str,
}

/// Formats a resolution into an output string.
pub trait OutputFormatter: Send + Sync {
    /// Returns information about this output format.
    fn info(&self) -> OutputInfo;

    /// Formats the resolutions into a string.
    fn format(&self, resolutions: &[Resolution], options: &FormatOptions) -> String;
}

/// Registry of output formatters.
pub struct OutputRegistry {
    formatters: HashMap<&'static str, Arc<dyn OutputFormatter>>,
}

impl OutputRegistry {
    /// Creates a registry with default formatters.
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut registry = Self {
            formatters: HashMap::new(),
        };

        registry.register(Arc::new(TextFormatter));
        registry.register(Arc::new(JsonFormatter));
        registry.register(Arc::new(TreeFormatter));
        registry.register(Arc::new(BuckFormatter));

        registry
    }

    /// Registers an output formatter.
    pub fn register(&mut self, formatter: Arc<dyn OutputFormatter>) {
        let info = formatter.info();
        self.formatters.insert(info.id, formatter);
    }

    /// Returns the formatter for the given ID, if present.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&dyn OutputFormatter> {
        self.formatters.get(id).map(std::convert::AsRef::as_ref)
    }
}

struct TextFormatter;

impl OutputFormatter for TextFormatter {
    fn info(&self) -> OutputInfo {
        OutputInfo {
            id: "text",
            name: "Text",
            description: "Human-readable text output",
        }
    }

    fn format(&self, resolutions: &[Resolution], _options: &FormatOptions) -> String {
        let mut output = String::new();

        for resolution in resolutions {
            let _ = writeln!(output, "# {}", resolution.root);
            for artifact in resolution.artifacts() {
                let sha = artifact
                    .sha1
                    .as_deref()
                    .or(artifact.sha256.as_deref())
                    .unwrap_or("unknown");
                let repo_info = artifact
                    .repository
                    .as_deref()
                    .map(|r| format!(" [{r}]"))
                    .unwrap_or_default();
                let _ = writeln!(output, "  {} ({}){}", artifact.artifact, sha, repo_info);
            }
            output.push('\n');
        }

        output
    }
}

struct JsonFormatter;

impl OutputFormatter for JsonFormatter {
    fn info(&self) -> OutputInfo {
        OutputInfo {
            id: "json",
            name: "JSON",
            description: "Machine-readable JSON output",
        }
    }

    fn format(&self, resolutions: &[Resolution], _options: &FormatOptions) -> String {
        serde_json::to_string_pretty(resolutions).unwrap_or_else(|_| "[]".to_string())
    }
}

struct TreeFormatter;

impl OutputFormatter for TreeFormatter {
    fn info(&self) -> OutputInfo {
        OutputInfo {
            id: "tree",
            name: "Tree",
            description: "Dependency tree output",
        }
    }

    fn format(&self, resolutions: &[Resolution], _options: &FormatOptions) -> String {
        let mut output = String::new();

        for resolution in resolutions {
            let mut ga_to_info: HashMap<String, String> = HashMap::new();
            let mut children: HashMap<Option<String>, Vec<String>> = HashMap::new();

            for artifact in resolution.artifacts() {
                let ga = format!(
                    "{}:{}",
                    artifact.artifact.coordinates.group_id,
                    artifact.artifact.coordinates.artifact_id
                );
                let coord = artifact.coordinate();
                let display = artifact
                    .repository
                    .as_deref()
                    .map(|r| format!("{coord} [{r}]"))
                    .unwrap_or(coord);
                ga_to_info.insert(ga.clone(), display);
                children
                    .entry(artifact.parent.clone())
                    .or_default()
                    .push(ga);
            }

            if let Some(roots) = children.get(&None) {
                let mut roots = roots.clone();
                roots.sort();
                for (idx, root_ga) in roots.iter().enumerate() {
                    let is_last = idx + 1 == roots.len();
                    render_tree_node(
                        root_ga,
                        &children,
                        &ga_to_info,
                        "",
                        is_last,
                        true,
                        &mut output,
                    );
                }
            }
            output.push('\n');
        }

        output
    }
}

fn render_tree_node(
    ga: &str,
    children: &HashMap<Option<String>, Vec<String>>,
    ga_to_info: &HashMap<String, String>,
    prefix: &str,
    is_last: bool,
    is_root: bool,
    output: &mut String,
) {
    let line = ga_to_info.get(ga).map_or(ga, String::as_str);
    if !is_root {
        let connector = if is_last { "\\-- " } else { "|-- " };
        output.push_str(prefix);
        output.push_str(connector);
    }
    output.push_str(line);
    output.push('\n');

    let key = Some(ga.to_string());
    if let Some(child_gas) = children.get(&key) {
        let mut child_gas = child_gas.clone();
        child_gas.sort();
        let next_prefix = if is_root {
            String::new()
        } else if is_last {
            format!("{prefix}    ")
        } else {
            format!("{prefix}|   ")
        };

        for (idx, child_ga) in child_gas.iter().enumerate() {
            let is_last_child = idx + 1 == child_gas.len();
            render_tree_node(
                child_ga,
                children,
                ga_to_info,
                &next_prefix,
                is_last_child,
                false,
                output,
            );
        }
    }
}

struct BuckFormatter;

impl OutputFormatter for BuckFormatter {
    fn info(&self) -> OutputInfo {
        OutputInfo {
            id: "buck",
            name: "Buck2",
            description: "Buck2 BUCK file output",
        }
    }

    fn format(&self, resolutions: &[Resolution], _options: &FormatOptions) -> String {
        let mut output = String::new();
        output.push_str("# Generated by antlers\n");
        output.push_str("# https://github.com/albertocavalcante/antler\n\n");

        let mut skipped = 0;

        for resolution in resolutions {
            for artifact in resolution.artifacts() {
                let name = artifact.artifact.artifact_id().replace(['-', '.'], "_");

                let (checksum_field, checksum_value) =
                    if let Some(sha256) = artifact.sha256.as_deref() {
                        ("sha256", sha256)
                    } else if let Some(sha1) = artifact.sha1.as_deref() {
                        ("sha1", sha1)
                    } else {
                        skipped += 1;
                        let _ = writeln!(
                            output,
                            "# SKIPPED: {} - no checksum available\n",
                            artifact.artifact.coordinate()
                        );
                        continue;
                    };

                let mvn_url = buck_mvn_url(&artifact.artifact);
                let _ = write!(
                    output,
                    r#"# {}
remote_file(
    name = "{}_jar",
    out = "{}",
    {} = "{}",
    url = "{}",
)

prebuilt_jar(
    name = "{}",
    binary_jar = ":{}_jar",
    visibility = ["PUBLIC"],
)

"#,
                    artifact.artifact.coordinate(),
                    name,
                    artifact.artifact.filename(),
                    checksum_field,
                    checksum_value,
                    mvn_url,
                    name,
                    name,
                );
            }
        }

        if skipped > 0 {
            tracing::warn!(
                "{} artifact(s) skipped due to missing checksums (security requirement)",
                skipped
            );
        }

        output
    }
}

/// Formats a Buck2 mvn: URL for an artifact.
fn buck_mvn_url(artifact: &gav::Artifact) -> String {
    let extension = artifact.extension.as_str();
    artifact.classifier.as_ref().map_or_else(
        || {
            format!(
                "mvn:{}:{}:{}:{}",
                artifact.group_id(),
                artifact.artifact_id(),
                extension,
                artifact.version
            )
        },
        |classifier| {
            format!(
                "mvn:{}:{}:{}:{}:{}",
                artifact.group_id(),
                artifact.artifact_id(),
                extension,
                artifact.version,
                classifier.0
            )
        },
    )
}
