//! Minimal lockfile resolver for Bazel/Buck2 integration.

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use antlers_lock::Lockfile;
use antlers_resolve::{Format, FormatOptions, Result, format_lockfile};

#[derive(Debug, Parser)]
#[command(name = "antlers-resolve", version, about)]
struct Cli {
    /// Lockfile path (antlers-lock or `rules_jvm_external` formats).
    lockfile: PathBuf,

    /// Output format.
    #[arg(short, long, value_enum, default_value = "json")]
    format: OutputFormat,

    /// Output file (stdout if omitted).
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Bazel workspace name.
    #[arg(long, default_value = "maven")]
    workspace_name: String,

    /// Fail on missing checksums.
    #[arg(long)]
    strict: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Json,
    Bazel,
    /// Buck2 BUCK output.
    Buck,
    Starlark,
}

impl From<OutputFormat> for Format {
    fn from(value: OutputFormat) -> Self {
        match value {
            OutputFormat::Json => Self::Json,
            OutputFormat::Bazel => Self::Bazel,
            OutputFormat::Buck => Self::Buck,
            OutputFormat::Starlark => Self::Starlark,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let lockfile = Lockfile::read_file(&cli.lockfile)?;
    let options = FormatOptions {
        workspace_name: cli.workspace_name,
        strict: cli.strict,
    };

    let output = format_lockfile(cli.format.into(), &lockfile, &options)?;

    if let Some(path) = cli.output {
        std::fs::write(&path, &output).map_err(|e| antlers_resolve::Error::WriteFile {
            path: path.clone(),
            source: e,
        })?;
    } else {
        println!("{output}");
    }

    Ok(())
}
