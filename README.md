# antlers

**Polyglot dependency resolution for the modern era.**

A native Rust library for resolving dependencies across multiple package ecosystems. Currently focused on JVM (Maven/Gradle), with plans to support npm, Python (uv/pip), and more.

> **Vision:** A single, fast, embeddable dependency resolver that speaks every package manager's language.

## Why antlers?

- **Polyglot by design** - One resolver for JVM, npm, Python, and beyond
- **Native performance** - No JVM/Node/Python runtime overhead
- **Embeddable** - Use as a library in your tools, or as a standalone CLI
- **Modular** - Use only what you need via feature flags
- **Reproducible** - Universal lockfile format with strong checksums

## Current Status

| Ecosystem | Status | Crates |
|-----------|--------|--------|
| **JVM (Maven/Gradle)** | ✅ Working | `gav`, `pomace`, `grale` |
| **npm** | 🗓️ Planned | - |
| **Python (uv/pip)** | 🗓️ Planned | - |
| **Cargo** | 🗓️ Planned | - |
| **Go modules** | 🗓️ Planned | - |

## Installation

**Homebrew:**

```bash
brew install albertocavalcante/tap/antlers

# or tap first, then install by name
brew tap albertocavalcante/tap
brew install antlers
```

This installs both the `antlers` command and the `atl` shorthand alias.

**Cargo:**

```bash
cargo install --git https://github.com/albertocavalcante/antlers antlers-cli
```

### Library

```toml
[dependencies]
# High-level API (includes everything)
antlers = { git = "https://github.com/albertocavalcante/antlers" }

# Or pick individual crates:
gav = { git = "https://github.com/albertocavalcante/antlers" }      # Core artifact types
pomace = { git = "https://github.com/albertocavalcante/antlers" }   # Maven POM parsing
grale = { git = "https://github.com/albertocavalcante/antlers" }    # Gradle metadata parsing
```

## Architecture

```
crates/
├── gav/              # Core: Artifact coordinates, versions, constraints
├── pomace/           # Maven POM parsing with property substitution
├── grale/            # Gradle Module Metadata parsing
├── gather/           # HTTP fetching, caching, checksum verification
├── dendro/           # Dependency resolution algorithms
├── antlers/          # High-level unified API
├── antlers-lock/      # Universal lockfile format
└── antlers-cli/       # CLI tool
```

### Crate Overview

| Crate | Purpose | Standalone? |
|-------|---------|-------------|
| `gav` | Artifact coordinates (GAV), versions, version constraints, exclusions | ✅ Yes |
| `pomace` | Parse Maven POM XML files | ✅ Yes |
| `grale` | Parse Gradle Module Metadata JSON | ✅ Yes |
| `gather` | Fetch artifacts with caching and checksums | ✅ Yes |
| `dendro` | Resolution algorithms (nearest-wins, highest-wins) | ✅ Yes |
| `antlers` | High-level API combining all crates | ✅ Yes |
| `antlers-lock` | Universal lockfile (reads rules_jvm_external v1/v2) | ✅ Yes |

## Usage

### CLI

Use `antlers` or the `atl` shorthand:

```bash
# Resolve a single artifact
antlers resolve org.jetbrains.kotlin:kotlin-stdlib:2.0.0
atl resolve org.jetbrains.kotlin:kotlin-stdlib:2.0.0  # same thing

# Resolve with transitive dependencies
atl resolve com.google.guava:guava:33.0.0-jre --transitive

# Output as JSON
atl resolve org.slf4j:slf4j-api:2.0.9 --format json

# Generate Buck2 BUCK file
atl resolve com.squareup.okhttp3:okhttp:4.12.0 --format buck

# Fetch artifact with checksum verification
atl fetch org.jetbrains.kotlin:kotlin-stdlib:2.0.0 --output ./libs/
```

### Library

```rust
use antlers::{Antlers, Artifact};

#[tokio::main]
async fn main() -> antlers::Result<()> {
    let resolution = Antlers::with_defaults()
        .resolve(&Artifact::parse("com.google.guava:guava:33.0.0-jre")?)
        .await?;

    println!("Resolved {} artifacts:", resolution.len());
    for artifact in resolution.artifacts() {
        println!("  {} (sha256: {})",
            artifact.coordinate(),
            artifact.sha256.as_deref().unwrap_or("unknown")
        );
    }

    Ok(())
}
```

### Using Individual Crates

```rust
// Just parse a POM file
use pomace::PomParser;

let pom = PomParser::parse(pom_xml)?;
println!("Group: {}", pom.effective_group_id().unwrap());
for dep in pom.direct_dependencies() {
    println!("  {} (scope: {:?})", dep.coordinate(), dep.scope);
}
```

```rust
// Just parse Gradle Module Metadata
use grale::GradleModuleParser;

let module = GradleModuleParser::parse(module_json)?;
if let Some(variant) = module.runtime_variant() {
    for dep in &variant.dependencies {
        println!("  {}:{}", dep.group, dep.module);
    }
}
```

## JVM Features

### Supported Formats

| Format | Status |
|--------|--------|
| Maven POM | ✅ Full support |
| Gradle Module Metadata (.module) | ✅ Full support |
| Bill of Materials (BOM) imports | ✅ Supported |
| Parent POM inheritance | ✅ Supported |
| Property substitution | ✅ Supported |
| Maven profiles | ⚠️ Parsed, not activated |

### Resolution Features

| Feature | Status |
|---------|--------|
| Transitive resolution | ✅ Working |
| Version conflict detection | ✅ Working |
| Nearest-wins strategy | ✅ Working |
| Highest-wins strategy | ✅ Working |
| Dependency exclusions | ✅ Working |
| Optional dependencies | ✅ Working |
| Classifiers (sources, javadoc) | ✅ Working |
| Checksum verification (SHA-256) | ✅ Working |
| Version ranges | ⚠️ Partial |

## Lockfile Format

antlers-lock provides a universal lockfile format that:

- **Reads** rules_jvm_external v1 and v2 lockfiles
- **Writes** our format or rules_jvm_external v2 (for Bazel compatibility)
- **Preserves** unknown fields for forward/backward compatibility
- **Uses** SHA-256 for integrity verification

```json
{
  "version": "1",
  "format": "antlers-lock",
  "artifacts": {
    "com.google.guava:guava": {
      "version": "33.0.0-jre",
      "sha256": "abc123...",
      "dependencies": ["com.google.guava:failureaccess"]
    }
  }
}
```

## Roadmap

### Near-term
- [ ] Full recursive transitive resolution
- [ ] Version range resolution
- [ ] CLI `lock` command for lockfile generation
- [ ] Better error messages

### Future Ecosystems
- [ ] **npm** - package.json, package-lock.json
- [ ] **Python** - pyproject.toml, uv.lock, requirements.txt
- [ ] **Cargo** - Cargo.toml (meta: resolve Rust deps in Rust!)
- [ ] **Go** - go.mod

### Integration
- [ ] Buck2 rules integration
- [ ] Bazel rules compatibility
- [ ] GitHub Actions

## Related Projects

- **[jvmdeer](https://github.com/albertocavalcante/jvmdeer)** - Buck2 integration using antlers
- **[Reindeer](https://github.com/facebookincubator/reindeer)** - Rust/Cargo deps for Buck2
- **[Coursier](https://get-coursier.io/)** - JVM-based resolver (inspiration)
- **[uv](https://github.com/astral-sh/uv)** - Fast Python package manager (inspiration for speed)
- **[rules_jvm_external](https://github.com/bazelbuild/rules_jvm_external)** - Bazel JVM rules (lockfile compatibility)

## Contributing

Contributions welcome! See the [GitHub Issues](https://github.com/albertocavalcante/antlers/issues) for planned work.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
