# Configuration

Antlers uses a TOML configuration file called `antlers.toml`. This guide covers
common configuration scenarios.

Note: the CLI currently uses `antlers.toml` for `init`, `add`, `show`, and
`fmt`. `resolve` and `fetch` are driven by CLI arguments and do not read the
file yet.

## Creating a Configuration

The easiest way to create a configuration is with `antlers init`:

```bash
antlers init
```

This creates a minimal `antlers.toml` with Maven Central configured.

## Adding Repositories

### Using the CLI

The easiest way to add repositories:

```bash
# Add a preset repository
antlers add repo jitpack --preset

# Add a custom repository
antlers add repo mycompany https://maven.mycompany.com/releases/
```

### Manual Configuration

Edit `antlers.toml` directly:

```toml
[[repositories]]
id = "central"
name = "Maven Central"
url = "https://repo1.maven.org/maven2/"
ecosystem = "maven"

[[repositories]]
id = "jitpack"
name = "JitPack"
url = "https://jitpack.io/"
```

## Authentication

### Bearer Token (GitHub Packages, etc.)

```bash
antlers add repo github https://maven.pkg.github.com/org/repo --token-env GITHUB_TOKEN
```

Or in `antlers.toml`:

```toml
[[repositories]]
id = "github"
url = "https://maven.pkg.github.com/org/repo"

[repositories.credentials]
type = "bearer"
token = { env = "GITHUB_TOKEN" }
```

### Basic Auth (Artifactory, Nexus, etc.)

```toml
[[repositories]]
id = "artifactory"
url = "https://artifactory.example.com/maven"

[repositories.credentials]
type = "basic"
username = { env = "ARTIFACTORY_USER" }
password = { env = "ARTIFACTORY_PASS" }
```

### Environment Variables

Use `{ env = "VAR_NAME" }` to reference environment variables:

```toml
[repositories.credentials]
type = "basic"
username = { env = "MAVEN_USER" }
password = { env = "MAVEN_PASS" }
```

For hermetic builds, explicitly allow the variables:

```toml
[env]
allow = ["MAVEN_USER", "MAVEN_PASS"]
```

## Importing from Existing Tools

### Auto-Detection

Scan for all existing package manager configurations:

```bash
antlers init --detect
```

This searches for:

- `~/.m2/settings.xml` (Maven)
- `settings.gradle` / `settings.gradle.kts` (Gradle)
- `~/.ivy2/ivysettings.xml` (Ivy)
- `~/.npmrc`, `./.npmrc` (npm)
- `~/.config/pip/pip.conf`, `~/.pip/pip.conf` (pip)
- `~/Library/Application Support/pip/pip.conf` (pip, macOS)

### Specific File

Import from a specific configuration:

```bash
antlers init --from ~/.m2/settings.xml
antlers init --from settings.gradle.kts
antlers init --from ~/.ivy2/ivysettings.xml
```

## Formatting

Keep your configuration tidy:

```bash
# Format in place
antlers fmt

# Check formatting (useful for CI)
antlers fmt --check

# Preview changes
antlers fmt --diff
```

## Viewing Configuration

```bash
# Pretty print
antlers show

# JSON output
antlers show --json
```

## Next Steps

- [antlers.toml Reference](../reference/antlers-toml.md) - Complete
  configuration reference
- [Repository Presets](presets.md) - Available preset repositories
- [Hermetic Builds](../advanced/hermetic.md) - Reproducible build configuration
