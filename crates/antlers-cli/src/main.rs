//! antlers CLI - JVM dependency resolver
//!
//! A native Rust tool for resolving Maven/Gradle dependencies.

// format_push_string is fine for simple string building
#![allow(clippy::format_push_string)]

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use tracing::info;
use tracing_subscriber::EnvFilter;

use antlers::{Antlers, Artifact, MavenRepository, Resolution};

#[derive(Parser)]
#[command(name = "antlers")]
#[command(author, version, about = "Native JVM dependency resolver", long_about = None)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Resolve dependencies for an artifact
    Resolve {
        /// Artifact coordinate (e.g., org.jetbrains.kotlin:kotlin-stdlib:2.3.0)
        #[arg(required = true)]
        artifacts: Vec<String>,

        /// Resolve transitive dependencies
        #[arg(short, long, default_value = "true")]
        transitive: bool,

        /// Output format
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,

        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Additional Maven repository URLs
        #[arg(long)]
        repo: Vec<String>,
    },

    /// Fetch an artifact and its checksums
    Fetch {
        /// Artifact coordinate
        artifact: String,

        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: PathBuf,

        /// Also fetch sources JAR
        #[arg(long)]
        sources: bool,

        /// Also fetch javadoc JAR
        #[arg(long)]
        javadoc: bool,
    },

    /// Show information about an artifact
    Info {
        /// Artifact coordinate
        artifact: String,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    /// Human-readable text
    Text,
    /// JSON output
    Json,
    /// Generate Buck2 BUCK file
    Buck,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    match cli.command {
        Commands::Resolve {
            artifacts,
            transitive,
            format,
            output,
            repo,
        } => {
            resolve_command(artifacts, transitive, format, output, repo).await?;
        }
        Commands::Fetch {
            artifact,
            output,
            sources,
            javadoc,
        } => {
            fetch_command(&artifact, output, sources, javadoc).await?;
        }
        Commands::Info { artifact } => {
            info_command(&artifact)?;
        }
    }

    Ok(())
}

async fn resolve_command(
    artifacts: Vec<String>,
    transitive: bool,
    format: OutputFormat,
    output: Option<PathBuf>,
    extra_repos: Vec<String>,
) -> Result<()> {
    // Build Antler resolver with default repositories
    let mut antler = Antlers::with_defaults().transitive(transitive);

    // Add extra repositories
    for (i, url) in extra_repos.iter().enumerate() {
        antler = antler.with_repository(MavenRepository::new(
            format!("custom-{i}"),
            format!("Custom Repository {i}"),
            url,
        ));
    }

    let mut all_resolutions = Vec::new();

    for coord in &artifacts {
        let artifact =
            Artifact::parse(coord).with_context(|| format!("Failed to parse artifact: {coord}"))?;

        info!("Resolving {}...", artifact);
        let resolution = antler.resolve(&artifact).await?;
        all_resolutions.push(resolution);
    }

    // Output results
    let output_str = match format {
        OutputFormat::Text => {
            let mut output = String::new();
            for resolution in &all_resolutions {
                output.push_str(&format!("# {}\n", resolution.root));
                for artifact in resolution.artifacts() {
                    let sha = artifact
                        .sha1
                        .as_deref()
                        .or(artifact.sha256.as_deref())
                        .unwrap_or("unknown");
                    output.push_str(&format!("  {} ({})\n", artifact.artifact, sha));
                }
                output.push('\n');
            }
            output
        }
        OutputFormat::Json => serde_json::to_string_pretty(&all_resolutions)?,
        OutputFormat::Buck => generate_buck_output(&all_resolutions),
    };

    if let Some(path) = output {
        std::fs::write(&path, &output_str)
            .with_context(|| format!("Failed to write to {}", path.display()))?;
        info!("Wrote output to {}", path.display());
    } else {
        println!("{output_str}");
    }

    Ok(())
}

fn generate_buck_output(resolutions: &[Resolution]) -> String {
    let mut output = String::new();
    output.push_str("# Generated by antler\n");
    output.push_str("# https://github.com/albertocavalcante/antler\n\n");

    let mut skipped = 0;

    for resolution in resolutions {
        for artifact in resolution.artifacts() {
            let name = artifact.artifact.artifact_id().replace(['-', '.'], "_");

            // Security: Prefer SHA256, fall back to SHA1, skip if neither available
            let (checksum_field, checksum_value) = if let Some(sha256) = artifact.sha256.as_deref()
            {
                ("sha256", sha256)
            } else if let Some(sha1) = artifact.sha1.as_deref() {
                ("sha1", sha1)
            } else {
                // Security: Never use placeholder checksums
                skipped += 1;
                output.push_str(&format!(
                    "# SKIPPED: {} - no checksum available\n\n",
                    artifact.artifact.coordinate()
                ));
                continue;
            };

            output.push_str(&format!(
                r#"# {}
remote_file(
    name = "{}_jar",
    out = "{}",
    {} = "{}",
    url = "mvn:{}:{}:jar:{}",
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
                artifact.artifact.group_id(),
                artifact.artifact.artifact_id(),
                artifact.artifact.version,
                name,
                name,
            ));
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

async fn fetch_command(coord: &str, output: PathBuf, sources: bool, javadoc: bool) -> Result<()> {
    let artifact = Artifact::parse(coord)?;
    info!("Fetching {}...", artifact);

    // Create output directory if needed
    std::fs::create_dir_all(&output)
        .with_context(|| format!("Failed to create output directory: {}", output.display()))?;

    // Use Antler to get checksums and find the artifact
    let antler = Antlers::with_defaults();
    let resolution = antler.resolve(&artifact).await?;

    // Get the root artifact info
    let resolved = resolution
        .artifacts()
        .first()
        .context("No artifacts resolved")?;

    // Download the main JAR
    let jar_url = format!(
        "https://repo1.maven.org/maven2/{}",
        artifact.repository_path()
    );
    let jar_path = output.join(artifact.filename());

    info!("Downloading {} to {}", jar_url, jar_path.display());
    download_and_verify(
        &jar_url,
        &jar_path,
        resolved.sha256.as_deref(),
        resolved.sha1.as_deref(),
    )
    .await?;
    info!("Downloaded {}", jar_path.display());

    // Optionally download sources
    if sources {
        let sources_artifact = artifact.clone().with_classifier("sources");
        let sources_url = format!(
            "https://repo1.maven.org/maven2/{}",
            sources_artifact.repository_path()
        );
        let sources_path = output.join(sources_artifact.filename());

        info!("Downloading sources...");
        match download_file(&sources_url, &sources_path).await {
            Ok(()) => info!("Downloaded {}", sources_path.display()),
            Err(e) => tracing::warn!("Sources not available: {}", e),
        }
    }

    // Optionally download javadoc
    if javadoc {
        let javadoc_artifact = artifact.clone().with_classifier("javadoc");
        let javadoc_url = format!(
            "https://repo1.maven.org/maven2/{}",
            javadoc_artifact.repository_path()
        );
        let javadoc_path = output.join(javadoc_artifact.filename());

        info!("Downloading javadoc...");
        match download_file(&javadoc_url, &javadoc_path).await {
            Ok(()) => info!("Downloaded {}", javadoc_path.display()),
            Err(e) => tracing::warn!("Javadoc not available: {}", e),
        }
    }

    Ok(())
}

/// Download a file and verify its checksum.
async fn download_and_verify(
    url: &str,
    path: &std::path::Path,
    expected_sha256: Option<&str>,
    expected_sha1: Option<&str>,
) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch {url}"))?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to download {}: HTTP {}", url, response.status());
    }

    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("Failed to read response from {url}"))?;

    // Verify checksum (prefer SHA256)
    if let Some(expected) = expected_sha256 {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let actual = format!("{:x}", hasher.finalize());
        if actual != expected {
            anyhow::bail!("SHA256 mismatch for {url}:\n  expected: {expected}\n  actual: {actual}");
        }
        info!("SHA256 verified: {}", &actual[..16]);
    } else if let Some(expected) = expected_sha1 {
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(&bytes);
        let actual = format!("{:x}", hasher.finalize());
        if actual != expected {
            anyhow::bail!("SHA1 mismatch for {url}:\n  expected: {expected}\n  actual: {actual}");
        }
        info!("SHA1 verified: {}", &actual[..16]);
    } else {
        tracing::warn!("No checksum available for verification - proceeding without verification");
    }

    std::fs::write(path, &bytes).with_context(|| format!("Failed to write {}", path.display()))?;

    Ok(())
}

/// Download a file without checksum verification (for sources/javadoc).
async fn download_file(url: &str, path: &std::path::Path) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch {url}"))?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to download {}: HTTP {}", url, response.status());
    }

    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("Failed to read response from {url}"))?;

    std::fs::write(path, &bytes).with_context(|| format!("Failed to write {}", path.display()))?;

    Ok(())
}

fn info_command(coord: &str) -> Result<()> {
    let artifact = Artifact::parse(coord)?;

    println!("Artifact Information");
    println!("====================");
    println!("Group ID:    {}", artifact.group_id());
    println!("Artifact ID: {}", artifact.artifact_id());
    println!("Version:     {}", artifact.version);
    if let Some(ref classifier) = artifact.classifier {
        println!("Classifier:  {classifier}");
    }
    println!("Extension:   {}", artifact.extension);
    println!();
    println!("Repository path: {}", artifact.repository_path());
    println!("POM path:        {}", artifact.pom_path());

    Ok(())
}
