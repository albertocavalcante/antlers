# antler

Native Rust resolver for JVM dependencies.

> Like [Coursier](https://get-coursier.io/), but focused purely on resolution - no installer, no environment setup, just fast dependency resolution.

## Features

- **POM Resolution** - Parse and resolve Maven POM files
- **Gradle Module Metadata** - Support for `.module` files with variant-aware resolution
- **Native Performance** - No JVM startup overhead
- **Embeddable** - Use as a library or standalone CLI
- **Focused** - Just resolution and fetching, nothing else

## Status

**Early Development** - Not ready for production use.

## Installation

### From source

```bash
cargo install --git https://github.com/albertocavalcante/antler antler-cli
```

### As a library

Add to your `Cargo.toml`:

```toml
[dependencies]
antler-core = { git = "https://github.com/albertocavalcante/antler" }
```

## Architecture

```
antler/
├── antler-core/    # Library: resolution logic, POM/GMM parsing
└── antler-cli/     # Binary: standalone CLI tool
```

## Usage

### As a CLI

```bash
# Resolve a single artifact
antler resolve org.jetbrains.kotlin:kotlin-stdlib:2.3.0

# Resolve with transitive dependencies
antler resolve org.codehaus.groovy:groovy:4.0.24 --transitive

# Output as JSON
antler resolve org.codehaus.groovy:groovy:4.0.24 --format json
```

### As a Library

```rust
use antler_core::{Artifact, Resolver, Repository};

let resolver = Resolver::new()
    .with_repository(Repository::maven_central())
    .with_repository(Repository::google());

let artifact = Artifact::parse("org.jetbrains.kotlin:kotlin-stdlib:2.3.0")?;
let resolution = resolver.resolve(&artifact).await?;

for dep in resolution.transitive_dependencies() {
    println!("{} -> {}", dep.coordinate(), dep.sha256());
}
```

## Supported Formats

| Format | Status |
|--------|--------|
| Maven POM | In Progress |
| Gradle Module Metadata (.module) | Planned |
| Bill of Materials (BOM) | Planned |
| Version Catalogs | Planned |

## Resolution Features

| Feature | Status |
|---------|--------|
| Transitive resolution | In Progress |
| Version mediation (nearest-wins) | Planned |
| Exclusions | Planned |
| Optional dependencies | Planned |
| Classifiers (sources, javadoc) | Planned |
| Version ranges | Planned |
| Relocations | Planned |
| Parent POM inheritance | Planned |

## Related Projects

- **[jvmdeer](https://github.com/albertocavalcante/jvmdeer)** - Buck2 integration that uses antler
- **[Reindeer](https://github.com/facebookincubator/reindeer)** - Similar tool for Rust/Cargo deps in Buck2
- **[Coursier](https://get-coursier.io/)** - The JVM-based resolver that inspired this project

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
