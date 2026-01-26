# Hermetic Builds

Hermetic builds ensure reproducibility by eliminating external dependencies and
non-determinism. In Antlers today, hermetic controls are available in the Rust
API and in configuration validation. The CLI does not enforce hermetic behavior
yet.

## Hermetic Levels (Rust API)

The `antlers::config::HermeticConfig` supports three levels:

- **Disabled** - No hermetic guarantees
- **Reproducible** - Deterministic output, but network/env access allowed
- **Strict** - No network, explicit cache path, strict environment controls

## Using the Rust API

```rust
use antlers::config::{AntlerConfig, CacheConfig};

// Strict hermetic configuration for build systems
let config = AntlerConfig::hermetic()
    .with_cache(CacheConfig::read_only("/sandbox/cache"));

config.validate()?;
```

## Validating antlers.toml

The TOML schema does not include a `[hermetic]` section. If you want to enforce
hermetic rules in tooling, use `AntlersToml::validate_hermetic()` to check for:

- An explicit cache path
- An environment allowlist that covers any referenced env vars

```rust
use antlers::AntlersToml;

let config = AntlersToml::load("antlers.toml")?;
config.validate_hermetic()?;
```

## Lockfiles

Antlers provides the `antlers-lock` crate for lockfile generation and
compatibility with `rules_jvm_external`:

```rust
use antlers_lock::from_resolution;

let lockfile = from_resolution(&resolution);
lockfile.write_file("antlers.lock.json")?;
```

CLI lockfile commands are not available yet; see the Roadmap.
