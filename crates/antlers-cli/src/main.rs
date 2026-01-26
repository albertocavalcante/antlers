//! antlers CLI - JVM dependency resolver
//!
//! A native Rust tool for resolving Maven/Gradle dependencies.

// format_push_string is fine for simple string building
#![allow(clippy::format_push_string)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use owo_colors::OwoColorize;
use tracing::info;
use tracing_subscriber::EnvFilter;

use antlers::{
    Antlers, AntlersToml, Artifact, MavenRepository, MigrationSource, Resolution, SourceFormat,
    TomlFormatter,
};

// =============================================================================
// Progress helpers (TTY-aware)
// =============================================================================

// Template strings like "{spinner:.cyan}" are for indicatif, not format!
#[allow(clippy::literal_string_with_formatting_args)]
mod progress {
    use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
    use owo_colors::OwoColorize;

    /// Check if stdout is a TTY.
    pub fn is_tty() -> bool {
        std::io::IsTerminal::is_terminal(&std::io::stdout())
    }

    /// A TTY-aware spinner.
    pub struct Spinner {
        pb: ProgressBar,
        interactive: bool,
    }

    impl Spinner {
        pub fn new(msg: &str) -> Self {
            let interactive = is_tty();
            let pb = ProgressBar::new_spinner();

            if interactive {
                pb.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner:.cyan} {msg}")
                        .expect("valid template"),
                );
                pb.set_message(msg.to_string());
                pb.enable_steady_tick(std::time::Duration::from_millis(80));
            } else {
                pb.set_draw_target(ProgressDrawTarget::hidden());
            }

            Self { pb, interactive }
        }

        pub fn finish(self, interactive_msg: &str, plain_msg: &str) {
            if self.interactive {
                self.pb
                    .finish_with_message(format!("{} {}", "✓".green(), interactive_msg));
            } else {
                self.pb.finish_and_clear();
                println!("{plain_msg}");
            }
        }

        pub fn finish_clear(self) {
            self.pb.finish_and_clear();
        }
    }

    /// A TTY-aware download progress bar.
    pub struct DownloadProgress {
        pb: ProgressBar,
        interactive: bool,
    }

    impl DownloadProgress {
        #[allow(clippy::option_if_let_else)]
        pub fn new(filename: &str, total_size: Option<u64>) -> Self {
            let interactive = is_tty();

            let pb = if interactive {
                if let Some(size) = total_size {
                    let pb = ProgressBar::new(size);
                    pb.set_style(
                        ProgressStyle::default_bar()
                            .template("{spinner:.cyan} {msg}\n  [{bar:40.cyan/dim}] {bytes}/{total_bytes} ({bytes_per_sec})")
                            .expect("valid template")
                            .progress_chars("━╸─"),
                    );
                    pb
                } else {
                    let pb = ProgressBar::new_spinner();
                    pb.set_style(
                        ProgressStyle::default_spinner()
                            .template("{spinner:.cyan} {msg} ({bytes})")
                            .expect("valid template"),
                    );
                    pb
                }
            } else {
                let pb = ProgressBar::new_spinner();
                pb.set_draw_target(ProgressDrawTarget::hidden());
                pb
            };

            pb.set_message(filename.to_string());
            if interactive {
                pb.enable_steady_tick(std::time::Duration::from_millis(100));
            }

            Self { pb, interactive }
        }

        pub fn inc(&self, n: u64) {
            self.pb.inc(n);
        }

        pub fn finish(self, filename: &str) {
            if self.interactive {
                self.pb
                    .finish_with_message(format!("{} {}", "✓".green(), filename));
            } else {
                self.pb.finish_and_clear();
                println!("Downloaded {filename}");
            }
        }

        pub fn finish_clear(self) {
            self.pb.finish_and_clear();
        }
    }
}

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
    /// Initialize a new antlers.toml configuration
    Init {
        /// Import from a specific config file (.npmrc, pip.conf, settings.xml, settings.gradle)
        #[arg(long, value_name = "FILE")]
        from: Option<PathBuf>,

        /// Auto-detect and import from all found package manager configs
        #[arg(long)]
        detect: bool,

        /// Output path for antlers.toml (default: ./antlers.toml)
        #[arg(short, long, default_value = "antlers.toml")]
        output: PathBuf,

        /// Overwrite existing antlers.toml
        #[arg(long)]
        force: bool,
    },

    /// Format antlers.toml with canonical ordering
    #[command(name = "fmt")]
    Format {
        /// Path to antlers.toml (default: ./antlers.toml)
        #[arg(default_value = "antlers.toml")]
        path: PathBuf,

        /// Check if file is formatted (exit 1 if not)
        #[arg(long)]
        check: bool,

        /// Show diff of changes without writing
        #[arg(long)]
        diff: bool,
    },

    /// Display the current configuration
    Show {
        /// Path to antlers.toml (default: ./antlers.toml)
        #[arg(default_value = "antlers.toml")]
        path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

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
        EnvFilter::new("warn")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    match cli.command {
        Commands::Init {
            from,
            detect,
            output,
            force,
        } => {
            init_command(from.as_deref(), detect, &output, force)?;
        }
        Commands::Format { path, check, diff } => {
            fmt_command(&path, check, diff)?;
        }
        Commands::Show { path, json } => {
            show_command(&path, json)?;
        }
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
            fetch_command(&artifact, &output, sources, javadoc).await?;
        }
        Commands::Info { artifact } => {
            info_command(&artifact)?;
        }
    }

    Ok(())
}

// =============================================================================
// Init command
// =============================================================================

/// Well-known config file locations to scan.
fn get_config_search_paths() -> Vec<(PathBuf, SourceFormat)> {
    let mut paths = Vec::new();

    // Current directory
    let cwd_paths = [
        (".npmrc", SourceFormat::Npmrc),
        ("settings.gradle", SourceFormat::Gradle),
        ("settings.gradle.kts", SourceFormat::Gradle),
    ];

    for (name, format) in cwd_paths {
        paths.push((PathBuf::from(name), format));
    }

    // User home directory
    if let Some(home) = dirs::home_dir() {
        paths.push((home.join(".npmrc"), SourceFormat::Npmrc));
        paths.push((home.join(".m2/settings.xml"), SourceFormat::Maven));

        // pip config locations
        paths.push((home.join(".config/pip/pip.conf"), SourceFormat::Pip));
        paths.push((home.join(".pip/pip.conf"), SourceFormat::Pip));

        #[cfg(target_os = "macos")]
        paths.push((
            home.join("Library/Application Support/pip/pip.conf"),
            SourceFormat::Pip,
        ));
    }

    paths
}

fn init_command(from: Option<&Path>, detect: bool, output: &Path, force: bool) -> Result<()> {
    // Check if output already exists
    if output.exists() && !force {
        anyhow::bail!(
            "{} already exists. Use --force to overwrite.",
            output.display()
        );
    }

    let mut sources: Vec<MigrationSource> = Vec::new();

    if let Some(path) = from {
        // Import from specific file
        println!("Importing from {}...", path.display().to_string().cyan());
        let source = MigrationSource::detect(path)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        sources.push(source);
    } else if detect {
        // Auto-detect all configs
        println!("Scanning for package manager configs...\n");

        for (path, format) in get_config_search_paths() {
            if path.exists() {
                match MigrationSource::parse_with_format(&path, format) {
                    Ok(source) if !source.repositories.is_empty() => {
                        print_detected_source(&source);
                        sources.push(source);
                    }
                    Ok(_) => {
                        // Empty, skip
                    }
                    Err(e) => {
                        eprintln!("  {} {} ({})", "!".yellow(), path.display(), e);
                    }
                }
            }
        }

        if sources.is_empty() {
            println!("No package manager configs found.\n");
        }
    }

    // Build the config
    let config = if sources.is_empty() {
        // Create minimal config
        create_minimal_config()
    } else {
        // Merge all sources
        merge_sources(&sources)
    };

    // Format and write
    let toml_str = config.to_toml()?;
    let toml_fmt = TomlFormatter::new();
    let output_toml = toml_fmt.format(&toml_str)?;

    std::fs::write(output, &output_toml)
        .with_context(|| format!("Failed to write {}", output.display()))?;

    println!();
    println!(
        "{} Created {}",
        "✓".green(),
        output.display().to_string().cyan()
    );

    // Summary
    if !config.repositories.is_empty() {
        println!(
            "  {} {} repositories",
            "•".dimmed(),
            config.repositories.len()
        );
    }
    if let Some(env) = &config.env
        && !env.allow.is_empty()
    {
        println!(
            "  {} {} env vars ({})",
            "•".dimmed(),
            env.allow.len(),
            env.allow.join(", ")
        );
    }

    println!();
    println!("Next steps:");
    println!("  Review:  {}", "antlers show".to_string().dimmed());
    println!(
        "  Edit:    {}",
        format!("$EDITOR {}", output.display()).dimmed()
    );
    println!("  Format:  {}", "antlers fmt".to_string().dimmed());

    Ok(())
}

fn print_detected_source(source: &MigrationSource) {
    let format_name = match source.format {
        SourceFormat::Npmrc => ".npmrc",
        SourceFormat::Pip => "pip.conf",
        SourceFormat::Maven => "settings.xml",
        SourceFormat::Gradle => "settings.gradle",
    };

    println!(
        "  {} {}",
        "✓".green(),
        source.path.display().to_string().cyan()
    );

    let repo_count = source.repositories.len();
    let cred_count = source
        .repositories
        .iter()
        .filter(|r| r.credentials.is_some())
        .count();

    print!(
        "    → {} {}",
        repo_count,
        if repo_count == 1 {
            "repository"
        } else {
            "repositories"
        }
    );

    if cred_count > 0 {
        print!(", {cred_count} with credentials");
    }

    if !source.env_vars.is_empty() {
        print!(" (env: {})", source.env_vars.join(", "));
    }

    println!(" [{}]", format_name.dimmed());
}

fn create_minimal_config() -> AntlersToml {
    use antlers::Ecosystem;
    use antlers::config::{ProjectConfig, RepositoryToml};

    AntlersToml {
        project: Some(ProjectConfig {
            name: std::env::current_dir()
                .ok()
                .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
                .unwrap_or_else(|| "my-project".to_string()),
            version: Some("0.1.0".to_string()),
            description: None,
        }),
        repositories: vec![RepositoryToml {
            id: "central".to_string(),
            name: Some("Maven Central".to_string()),
            url: "https://repo1.maven.org/maven2/".to_string(),
            ecosystem: Ecosystem::Maven,
            credentials: None,
        }],
        ..Default::default()
    }
}

fn merge_sources(sources: &[MigrationSource]) -> AntlersToml {
    use antlers::config::{EnvToml, ProjectConfig, RepositoryToml};
    use std::collections::HashSet;

    let mut repositories: Vec<RepositoryToml> = Vec::new();
    let mut seen_urls: HashSet<String> = HashSet::new();
    let mut env_vars: Vec<String> = Vec::new();

    for source in sources {
        for repo in &source.repositories {
            // Skip duplicates by URL
            if seen_urls.contains(&repo.url) {
                continue;
            }
            seen_urls.insert(repo.url.clone());

            repositories.push(RepositoryToml {
                id: repo.id.clone(),
                name: repo.name.clone(),
                url: repo.url.clone(),
                ecosystem: repo.ecosystem,
                credentials: repo
                    .credentials
                    .as_ref()
                    .map(antlers::migrate::MigratedCredentials::to_toml),
            });
        }

        for var in &source.env_vars {
            if !env_vars.contains(var) {
                env_vars.push(var.clone());
            }
        }
    }

    let env = if env_vars.is_empty() {
        None
    } else {
        Some(EnvToml { allow: env_vars })
    };

    AntlersToml {
        project: Some(ProjectConfig {
            name: std::env::current_dir()
                .ok()
                .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
                .unwrap_or_else(|| "my-project".to_string()),
            version: Some("0.1.0".to_string()),
            description: Some(format!("Imported from {} source(s)", sources.len())),
        }),
        repositories,
        env,
        ..Default::default()
    }
}

// =============================================================================
// Format command
// =============================================================================

fn fmt_command(path: &Path, check: bool, diff: bool) -> Result<()> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let toml_fmt = TomlFormatter::new();
    let output = toml_fmt
        .format(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;

    if check {
        if content == output {
            println!("{} {} is formatted", "✓".green(), path.display());
            return Ok(());
        }
        eprintln!("{} {} needs formatting", "✗".red(), path.display());
        std::process::exit(1);
    }

    if diff {
        if content == output {
            println!("{} {} is already formatted", "✓".green(), path.display());
        } else {
            let diff_output = toml_fmt.diff(&content, &output);
            print_colored_diff(&diff_output);
        }
        return Ok(());
    }

    // Write in place
    if content == output {
        println!("{} {} is already formatted", "✓".green(), path.display());
    } else {
        std::fs::write(path, &output)
            .with_context(|| format!("Failed to write {}", path.display()))?;
        println!("{} Formatted {}", "✓".green(), path.display());
    }

    Ok(())
}

fn print_colored_diff(diff: &str) {
    for line in diff.lines() {
        if line.starts_with('+') {
            println!("{}", line.green());
        } else if line.starts_with('-') {
            println!("{}", line.red());
        } else {
            println!("{line}");
        }
    }
}

// =============================================================================
// Show command
// =============================================================================

fn show_command(path: &Path, json: bool) -> Result<()> {
    let config =
        AntlersToml::load(path).with_context(|| format!("Failed to load {}", path.display()))?;

    if json {
        let json_str = serde_json::to_string_pretty(&config)?;
        println!("{json_str}");
        return Ok(());
    }

    // Pretty print
    println!("{}", "Configuration".bold());
    println!("{}", "=".repeat(50).dimmed());

    if let Some(project) = &config.project {
        println!();
        println!("{}", "[project]".cyan());
        println!("  name = \"{}\"", project.name);
        if let Some(v) = &project.version {
            println!("  version = \"{v}\"");
        }
        if let Some(d) = &project.description {
            println!("  description = \"{d}\"");
        }
    }

    if !config.repositories.is_empty() {
        println!();
        println!("{}", "[[repositories]]".cyan());
        for repo in &config.repositories {
            println!();
            println!(
                "  {} {}",
                repo.id.bold(),
                format!("({})", repo.ecosystem).dimmed()
            );
            println!("    url = \"{}\"", repo.url);
            if let Some(name) = &repo.name {
                println!("    name = \"{name}\"");
            }
            if repo.credentials.is_some() {
                println!("    credentials = {}", "[configured]".yellow());
            }
        }
    }

    if !config.dependencies.is_empty() {
        println!();
        println!("{}", "[dependencies]".cyan());
        for (coord, spec) in &config.dependencies {
            println!("  \"{}\" = \"{}\"", coord, spec.version());
        }
    }

    if !config.dev_dependencies.is_empty() {
        println!();
        println!("{}", "[dev-dependencies]".cyan());
        for (coord, spec) in &config.dev_dependencies {
            println!("  \"{}\" = \"{}\"", coord, spec.version());
        }
    }

    if let Some(resolver) = &config.resolver {
        println!();
        println!("{}", "[resolver]".cyan());
        println!(
            "  conflict-strategy = \"{}\"",
            resolver.conflict_strategy.as_str()
        );
        println!("  transitive = {}", resolver.transitive);
    }

    if let Some(cache) = &config.cache {
        println!();
        println!("{}", "[cache]".cyan());
        if let Some(p) = &cache.path {
            println!("  path = \"{}\"", p.display());
        }
        println!("  mode = \"{:?}\"", cache.mode);
    }

    if let Some(env) = &config.env
        && !env.allow.is_empty()
    {
        println!();
        println!("{}", "[env]".cyan());
        println!("  allow = {:?}", env.allow);
    }

    println!();

    Ok(())
}

// =============================================================================
// Resolve command
// =============================================================================

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

        let spinner = progress::Spinner::new(&format!("Resolving {artifact}"));

        info!("Resolving {}...", artifact);
        let resolution = antler.resolve(&artifact).await?;

        let n = resolution.artifacts().len();
        let s = if n == 1 { "" } else { "s" };
        spinner.finish(
            &format!("{artifact} ({n} artifact{s})"),
            &format!("Resolved {artifact} ({n} artifact{s})"),
        );

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
    output.push_str("# Generated by antlers\n");
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

// =============================================================================
// Fetch command
// =============================================================================

async fn fetch_command(coord: &str, output: &Path, sources: bool, javadoc: bool) -> Result<()> {
    let artifact = Artifact::parse(coord)?;

    // Create output directory if needed
    std::fs::create_dir_all(output)
        .with_context(|| format!("Failed to create output directory: {}", output.display()))?;

    // Resolve first to get checksums
    let spinner = progress::Spinner::new(&format!("Resolving {artifact}"));
    let antler = Antlers::with_defaults();
    let resolution = antler.resolve(&artifact).await?;
    spinner.finish_clear();

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

    download_with_progress(
        &jar_url,
        &jar_path,
        &artifact.filename(),
        resolved.sha256.as_deref(),
        resolved.sha1.as_deref(),
    )
    .await?;

    // Optionally download sources
    if sources {
        let sources_artifact = artifact.clone().with_classifier("sources");
        let sources_url = format!(
            "https://repo1.maven.org/maven2/{}",
            sources_artifact.repository_path()
        );
        let sources_path = output.join(sources_artifact.filename());

        match download_with_progress(
            &sources_url,
            &sources_path,
            &sources_artifact.filename(),
            None,
            None,
        )
        .await
        {
            Ok(()) => {}
            Err(_) => eprintln!("  {} sources not available", "!".yellow()),
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

        match download_with_progress(
            &javadoc_url,
            &javadoc_path,
            &javadoc_artifact.filename(),
            None,
            None,
        )
        .await
        {
            Ok(()) => {}
            Err(_) => eprintln!("  {} javadoc not available", "!".yellow()),
        }
    }

    Ok(())
}

/// Download a file with progress bar and optional checksum verification.
async fn download_with_progress(
    url: &str,
    path: &std::path::Path,
    filename: &str,
    expected_sha256: Option<&str>,
    expected_sha1: Option<&str>,
) -> Result<()> {
    use futures_util::StreamExt;

    let client = reqwest::Client::new();

    // Get content length for progress bar (only if interactive)
    let total_size = if progress::is_tty() {
        client.head(url).send().await.ok().and_then(|r| {
            r.headers()
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
        })
    } else {
        None
    };

    let pb = progress::DownloadProgress::new(filename, total_size);

    // Download with streaming
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch {url}"))?;

    if !response.status().is_success() {
        pb.finish_clear();
        anyhow::bail!("Failed to download {}: HTTP {}", url, response.status());
    }

    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.with_context(|| format!("Failed to read chunk from {url}"))?;
        pb.inc(chunk.len() as u64);
        bytes.extend_from_slice(&chunk);
    }

    // Verify checksum
    if let Some(expected) = expected_sha256 {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let actual = format!("{:x}", hasher.finalize());
        if actual != expected {
            pb.finish_clear();
            anyhow::bail!("SHA256 mismatch for {url}:\n  expected: {expected}\n  actual: {actual}");
        }
    } else if let Some(expected) = expected_sha1 {
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(&bytes);
        let actual = format!("{:x}", hasher.finalize());
        if actual != expected {
            pb.finish_clear();
            anyhow::bail!("SHA1 mismatch for {url}:\n  expected: {expected}\n  actual: {actual}");
        }
    }

    std::fs::write(path, &bytes).with_context(|| format!("Failed to write {}", path.display()))?;
    pb.finish(filename);

    Ok(())
}

// =============================================================================
// Info command
// =============================================================================

fn info_command(coord: &str) -> Result<()> {
    let artifact = Artifact::parse(coord)?;

    println!("{}", "Artifact Information".bold());
    println!("{}", "=".repeat(50).dimmed());
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
