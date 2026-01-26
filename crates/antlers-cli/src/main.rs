//! antlers CLI - JVM dependency resolver
//!
//! A native Rust tool for resolving Maven/Gradle dependencies.

// format_push_string is fine for simple string building
#![allow(clippy::format_push_string)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand, ValueEnum};
#[cfg(feature = "color")]
use owo_colors::{OwoColorize, Style};
use tracing::info;
use tracing_subscriber::EnvFilter;

use antlers::{
    Antlers, AntlersToml, Artifact, Ecosystem, MavenRepository, MigrationSource,
    RepositoryRegistry, SourceFormat, TomlFormatter,
    config::ConfigEditor,
    registry::{FormatOptions, OutputRegistry},
};
#[cfg(not(feature = "color"))]
use color::{OwoColorize, Style};

// =============================================================================
// Progress helpers (TTY-aware)
// =============================================================================

// Template strings like "{spinner:.cyan}" are for indicatif, not format!
#[cfg(feature = "progress")]
#[allow(clippy::literal_string_with_formatting_args)]
mod progress {
    use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

    use super::OwoColorize;

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

#[cfg(not(feature = "color"))]
mod color {
    use std::fmt::Display;

    #[derive(Clone, Copy)]
    pub struct Style;

    impl Style {
        pub const fn new() -> Self {
            Self
        }

        pub const fn bold(self) -> Self {
            self
        }

        pub const fn dimmed(self) -> Self {
            self
        }

        pub const fn cyan(self) -> Self {
            self
        }

        pub fn style<T: Display>(&self, input: T) -> String {
            input.to_string()
        }
    }

    pub trait OwoColorize {
        fn green(&self) -> String;
        fn yellow(&self) -> String;
        fn red(&self) -> String;
        fn cyan(&self) -> String;
        fn bold(&self) -> String;
        fn dimmed(&self) -> String;
    }

    impl<T: Display + ?Sized> OwoColorize for T {
        fn green(&self) -> String {
            self.to_string()
        }

        fn yellow(&self) -> String {
            self.to_string()
        }

        fn red(&self) -> String {
            self.to_string()
        }

        fn cyan(&self) -> String {
            self.to_string()
        }

        fn bold(&self) -> String {
            self.to_string()
        }

        fn dimmed(&self) -> String {
            self.to_string()
        }
    }
}

#[cfg(not(feature = "progress"))]
mod progress {
    /// Check if stdout is a TTY.
    pub fn is_tty() -> bool {
        std::io::IsTerminal::is_terminal(&std::io::stdout())
    }

    /// A no-op spinner for non-progress builds.
    pub struct Spinner;

    impl Spinner {
        pub fn new(_msg: &str) -> Self {
            Self
        }

        pub fn finish(self, _interactive_msg: &str, plain_msg: &str) {
            println!("{plain_msg}");
        }

        pub fn finish_clear(self) {}
    }

    /// A no-op download progress bar for non-progress builds.
    pub struct DownloadProgress;

    impl DownloadProgress {
        pub fn new(_filename: &str, _total_size: Option<u64>) -> Self {
            Self
        }

        pub fn inc(&self, _n: u64) {}

        pub fn finish(self, filename: &str) {
            println!("Downloaded {filename}");
        }

        pub fn finish_clear(self) {}
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

        /// Use a preset repository (e.g., gradle-plugins, jenkins, jitpack)
        ///
        /// Use `antlers repos list` to see all available presets.
        /// Can be specified multiple times for multiple presets.
        #[arg(long, short = 'p')]
        preset: Vec<String>,

        /// Disable Gradle Module Metadata (use POM only)
        ///
        /// By default, antlers uses .module files when available, which provide
        /// richer dependency information. Use this flag to force POM-only mode,
        /// which may be useful for debugging or compatibility testing.
        #[arg(long)]
        pom_only: bool,
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

    /// List available repository presets
    Repos {
        #[command(subcommand)]
        command: Option<ReposCommand>,
    },

    /// Add a repository to antlers.toml
    Add {
        #[command(subcommand)]
        command: AddCommand,
    },
}

#[derive(Subcommand)]
enum ReposCommand {
    /// List all available repository presets
    List {
        /// Filter by ecosystem (maven, npm, pypi, nuget)
        #[arg(long, short)]
        ecosystem: Option<String>,
    },
}

#[derive(Subcommand)]
enum AddCommand {
    /// Add a repository to the configuration
    ///
    /// Examples:
    /// - `antlers add repo jitpack --preset`
    /// - `antlers add repo private https://maven.example.com/`
    /// - `antlers add repo github https://maven.pkg.github.com/org/repo --token-env GITHUB_TOKEN`
    Repo {
        /// Repository ID (used as key in config, e.g., 'central', 'jitpack')
        id: String,

        /// Repository URL (not needed with --preset)
        url: Option<String>,

        /// Use a preset repository (id becomes the preset name)
        #[arg(long, short)]
        preset: bool,

        /// Display name for the repository
        #[arg(long)]
        name: Option<String>,

        /// Ecosystem type
        #[arg(long, default_value = "maven")]
        ecosystem: EcosystemArg,

        /// Environment variable containing bearer token for authentication
        #[arg(long, value_name = "VAR")]
        token_env: Option<String>,

        /// Path to antlers.toml
        #[arg(long, short, default_value = "antlers.toml")]
        config: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum EcosystemArg {
    #[default]
    Maven,
    Npm,
    Pypi,
    Nuget,
}

impl From<EcosystemArg> for Ecosystem {
    fn from(arg: EcosystemArg) -> Self {
        match arg {
            EcosystemArg::Maven => Self::Maven,
            EcosystemArg::Npm => Self::Npm,
            EcosystemArg::Pypi => Self::Pypi,
            EcosystemArg::Nuget => Self::Nuget,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    /// Human-readable text
    Text,
    /// JSON output
    Json,
    /// Dependency tree (like `cs resolve -t`)
    Tree,
    /// Generate Buck2 BUCK file
    Buck,
}

impl OutputFormat {
    const fn id(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::Tree => "tree",
            Self::Buck => "buck",
        }
    }
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
        .without_time()
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
            preset,
            pom_only,
        } => {
            resolve_command(
                artifacts, transitive, format, output, repo, preset, pom_only,
            )
            .await?;
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
        Commands::Repos { command } => {
            repos_command(command.as_ref())?;
        }
        Commands::Add { command } => {
            add_command(&command)?;
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

        // Ivy settings
        paths.push((home.join(".ivy2/ivysettings.xml"), SourceFormat::Ivy));
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
        SourceFormat::Ivy => "ivysettings.xml",
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
    presets: Vec<String>,
    pom_only: bool,
) -> Result<()> {
    // Build Antler resolver with default repositories
    let mut antler = Antlers::with_defaults()
        .transitive(transitive)
        .with_gmm(!pom_only);

    // Add preset repositories
    for preset_name in &presets {
        if let Some(preset) = RepositoryRegistry::get(preset_name) {
            antler = antler.with_repository(preset.to_repository());
            info!("Added preset repository: {} ({})", preset.name, preset.url);
        } else {
            // Show helpful error with available presets
            let maven_presets: Vec<_> =
                RepositoryRegistry::ids_by_ecosystem(Ecosystem::Maven).collect();
            anyhow::bail!(
                "Unknown repository preset: '{}'\n\nAvailable Maven presets:\n  {}\n\nRun 'antlers repos list' for all options.",
                preset_name,
                maven_presets.join(", ")
            );
        }
    }

    // Add extra repositories (by URL)
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
        let resolution = match antler.resolve(&artifact).await {
            Ok(r) => r,
            Err(e) => {
                spinner.finish_clear();
                // Suggest preset if group ID matches a known pattern
                if let Some(suggestion) = suggest_preset_for_group(artifact.group_id()) {
                    anyhow::bail!(
                        "Failed to resolve {artifact}: {e}\n\n\
                         Hint: This artifact may be in the {suggestion} repository.\n\
                         Try: antlers resolve {coord} --preset {suggestion}"
                    );
                }
                return Err(e.into());
            }
        };

        let n = resolution.artifacts().len();
        let s = if n == 1 { "" } else { "s" };
        spinner.finish(
            &format!("{artifact} ({n} artifact{s})"),
            &format!("Resolved {artifact} ({n} artifact{s})"),
        );

        all_resolutions.push(resolution);
    }

    // Output results
    let mut registry = OutputRegistry::with_defaults();
    #[cfg(feature = "tree")]
    registry.register(std::sync::Arc::new(tree_output::TreeFormatter));
    let formatter = registry
        .get(format.id())
        .ok_or_else(|| anyhow!("Unknown output format: {}", format.id()))?;
    let options = FormatOptions {
        use_color: progress::is_tty(),
    };
    let output_str = formatter.format(&all_resolutions, &options);

    if let Some(path) = output {
        std::fs::write(&path, &output_str)
            .with_context(|| format!("Failed to write to {}", path.display()))?;
        info!("Wrote output to {}", path.display());
    } else {
        println!("{output_str}");
    }

    Ok(())
}

// =============================================================================
// Tree output (termtree)
// =============================================================================

#[cfg(feature = "tree")]
mod tree_output {
    use std::collections::HashMap;

    use antlers::Resolution;
    use antlers::registry::{FormatOptions, OutputFormatter, OutputInfo};

    use super::OwoColorize;

    pub struct TreeFormatter;

    impl OutputFormatter for TreeFormatter {
        fn info(&self) -> OutputInfo {
            OutputInfo {
                id: "tree",
                name: "Tree",
                description: "Dependency tree output (termtree)",
            }
        }

        fn format(&self, resolutions: &[Resolution], options: &FormatOptions) -> String {
            let mut output = String::new();

            for resolution in resolutions {
                let ga_to_info: HashMap<String, (String, usize, Option<String>)> = resolution
                    .artifacts()
                    .iter()
                    .map(|a| {
                        let ga = format!(
                            "{}:{}",
                            a.artifact.coordinates.group_id, a.artifact.coordinates.artifact_id
                        );
                        (ga, (a.coordinate(), a.depth, a.repository.clone()))
                    })
                    .collect();

                let mut children: HashMap<Option<String>, Vec<String>> = HashMap::new();
                for artifact in resolution.artifacts() {
                    let ga = format!(
                        "{}:{}",
                        artifact.artifact.coordinates.group_id,
                        artifact.artifact.coordinates.artifact_id
                    );
                    children
                        .entry(artifact.parent.clone())
                        .or_default()
                        .push(ga);
                }

                if let Some(roots) = children.get(&None) {
                    for root_ga in roots {
                        let tree = build_tree_node(root_ga, &children, &ga_to_info, options);
                        output.push_str(&tree.to_string());
                    }
                }
                output.push('\n');
            }

            output
        }
    }

    fn build_tree_node(
        ga: &str,
        children: &HashMap<Option<String>, Vec<String>>,
        ga_to_info: &HashMap<String, (String, usize, Option<String>)>,
        options: &FormatOptions,
    ) -> termtree::Tree<String> {
        let (coord, depth, repo) = ga_to_info
            .get(ga)
            .cloned()
            .unwrap_or_else(|| (ga.to_string(), 0, None));

        let coord_with_repo = if let Some(ref r) = repo {
            format!("{coord} [{r}]")
        } else {
            coord
        };

        let display = if options.use_color {
            match depth {
                0 => coord_with_repo.cyan().bold().to_string(),
                1 => coord_with_repo.green().to_string(),
                _ => coord_with_repo.dimmed().to_string(),
            }
        } else {
            coord_with_repo
        };

        let mut tree = termtree::Tree::new(display);
        let key = Some(ga.to_string());
        if let Some(child_gas) = children.get(&key) {
            for child_ga in child_gas {
                tree.push(build_tree_node(child_ga, children, ga_to_info, options));
            }
        }

        tree
    }
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

    let repo_url = resolved
        .repository
        .as_deref()
        .and_then(|name| antler.repository_url(name))
        .unwrap_or("https://repo1.maven.org/maven2/");
    let repo_base = repo_url.trim_end_matches('/');

    // Download the main JAR
    let jar_url = format!("{repo_base}/{}", artifact.repository_path());
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
        let sources_url = format!("{repo_base}/{}", sources_artifact.repository_path());
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
        let javadoc_url = format!("{repo_base}/{}", javadoc_artifact.repository_path());
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

// =============================================================================
// Repos command
// =============================================================================

fn repos_command(command: Option<&ReposCommand>) -> Result<()> {
    let eco_filter = match command {
        Some(ReposCommand::List { ecosystem }) => match ecosystem.as_deref() {
            Some("maven" | "jvm") => Some(Ecosystem::Maven),
            Some("npm" | "node" | "js") => Some(Ecosystem::Npm),
            Some("pypi" | "python" | "pip") => Some(Ecosystem::Pypi),
            Some("nuget" | "dotnet" | "csharp") => Some(Ecosystem::Nuget),
            Some(other) => {
                anyhow::bail!(
                    "Unknown ecosystem: '{other}'\n\nSupported ecosystems: maven, npm, pypi, nuget"
                );
            }
            None => None,
        },
        None => None,
    };

    // Styles for colored output (no-op styles when not TTY)
    let styles = RepoStyles::new(progress::is_tty());

    println!("{}", styles.bold.style("Available Repository Presets"));
    println!("{}", styles.dimmed.style("=".repeat(50)));
    println!();

    // Get presets, optionally filtered by ecosystem
    let presets: Vec<_> = eco_filter.map_or_else(
        || RepositoryRegistry::all().iter().collect(),
        |eco| RepositoryRegistry::by_ecosystem(eco).collect(),
    );

    // Group by ecosystem and print with colors
    let mut current_ecosystem: Option<Ecosystem> = None;

    for preset in presets {
        if current_ecosystem != Some(preset.ecosystem) {
            if current_ecosystem.is_some() {
                println!();
            }
            let header = format!("{}:", preset.ecosystem.as_str().to_uppercase());
            println!("{}", styles.bold.style(&header));
            current_ecosystem = Some(preset.ecosystem);
        }

        println!(
            "  {:<18} {:<45} {}",
            styles.cyan.style(preset.id),
            styles.dimmed.style(preset.url),
            preset.description
        );
    }

    println!();
    println!("Use with: {} <id>", styles.cyan.style("--preset"));
    println!(
        "Example:  {}",
        styles
            .dimmed
            .style("antlers resolve com.example:lib:1.0 --preset jenkins")
    );

    Ok(())
}

/// Conditional styles for repos command output.
struct RepoStyles {
    bold: Style,
    dimmed: Style,
    cyan: Style,
}

impl RepoStyles {
    const fn new(use_color: bool) -> Self {
        if use_color {
            Self {
                bold: Style::new().bold(),
                dimmed: Style::new().dimmed(),
                cyan: Style::new().cyan(),
            }
        } else {
            Self {
                bold: Style::new(),
                dimmed: Style::new(),
                cyan: Style::new(),
            }
        }
    }
}

// =============================================================================
// Add command
// =============================================================================

fn add_command(command: &AddCommand) -> Result<()> {
    match command {
        AddCommand::Repo {
            id,
            url,
            preset,
            name,
            ecosystem,
            token_env,
            config,
        } => add_repo_command(
            id,
            url.as_deref(),
            *preset,
            name.as_deref(),
            *ecosystem,
            token_env.as_deref(),
            config,
        ),
    }
}

fn add_repo_command(
    id: &str,
    url: Option<&str>,
    preset: bool,
    name: Option<&str>,
    ecosystem: EcosystemArg,
    token_env: Option<&str>,
    config_path: &Path,
) -> Result<()> {
    // Resolve URL: from preset or from argument
    let (resolved_url, resolved_name, resolved_ecosystem) = if preset {
        let preset_info = RepositoryRegistry::get(id).ok_or_else(|| {
            let maven_presets: Vec<_> =
                RepositoryRegistry::ids_by_ecosystem(Ecosystem::Maven).collect();
            anyhow::anyhow!(
                "Unknown preset: '{}'\n\nAvailable presets:\n  {}\n\nRun 'antlers repos list' for all options.",
                id,
                maven_presets.join(", ")
            )
        })?;
        (
            preset_info.url.to_string(),
            name.map_or_else(|| preset_info.name.to_string(), String::from),
            preset_info.ecosystem,
        )
    } else {
        let url = url.ok_or_else(|| {
            anyhow::anyhow!("URL is required (or use --preset to add a preset repository)")
        })?;
        (
            url.to_string(),
            name.map_or_else(|| id.to_string(), String::from),
            Ecosystem::from(ecosystem),
        )
    };

    // Check if config file exists
    if !config_path.exists() {
        anyhow::bail!(
            "{} not found. Run 'antlers init' first, or specify --config path.",
            config_path.display()
        );
    }

    // Open and edit the config
    let mut editor = ConfigEditor::open(config_path)
        .with_context(|| format!("Failed to open {}", config_path.display()))?;

    // Add repository (with or without credentials)
    if let Some(token_var) = token_env {
        editor.add_repository_with_bearer(id, &resolved_name, &resolved_url, token_var);
    } else {
        editor.add_repository(id, &resolved_name, &resolved_url, resolved_ecosystem);
    }

    // Save
    editor
        .save()
        .with_context(|| format!("Failed to save {}", config_path.display()))?;

    // Output
    println!("{} Added repository '{}'", "✓".green(), id.cyan());
    println!("    {} {}", "url:".dimmed(), resolved_url);
    println!(
        "    {} {}",
        "ecosystem:".dimmed(),
        resolved_ecosystem.as_str()
    );
    if token_env.is_some() {
        println!(
            "    {} bearer (from ${})",
            "auth:".dimmed(),
            token_env.unwrap()
        );
    }

    Ok(())
}

// =============================================================================
// Preset suggestions
// =============================================================================

/// Suggests a repository preset based on the artifact's group ID.
///
/// Returns `Some(preset_id)` if the group ID matches a known pattern.
fn suggest_preset_for_group(group_id: &str) -> Option<&'static str> {
    // Jenkins plugins and libraries
    if group_id.starts_with("org.jenkins-ci.") || group_id.starts_with("io.jenkins.") {
        return Some("jenkins");
    }

    // Gradle plugins
    if group_id.starts_with("org.gradle.") || group_id.starts_with("com.gradle.") {
        return Some("gradle-plugins");
    }

    // Spring (milestones/snapshots)
    if group_id.starts_with("org.springframework.") && group_id.contains("snapshot") {
        return Some("spring-snapshots");
    }

    // Atlassian (Jira, Confluence, etc.)
    if group_id.starts_with("com.atlassian.") {
        return Some("atlassian");
    }

    // JitPack (GitHub-based builds)
    if group_id.starts_with("com.github.") || group_id.starts_with("io.github.") {
        return Some("jitpack");
    }

    // Clojure libraries
    if group_id.starts_with("org.clojure.") || group_id == "clojure" {
        return Some("clojars");
    }

    // Red Hat
    if group_id.starts_with("com.redhat.") || group_id.starts_with("org.jboss.") {
        return Some("redhat-ga");
    }

    // Confluent (Kafka ecosystem)
    if group_id.starts_with("io.confluent.") {
        return Some("confluent");
    }

    None
}
