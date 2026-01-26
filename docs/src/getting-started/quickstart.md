# Quick Start

This guide will get you resolving JVM dependencies in under 5 minutes.

## Initialize a Project

Create a new `antlers.toml` configuration file:

```bash
antlers init
```

This creates a minimal configuration with Maven Central:

```toml
[project]
name = "my-project"
version = "0.1.0"

[[repositories]]
id = "central"
name = "Maven Central"
url = "https://repo1.maven.org/maven2/"
ecosystem = "maven"
```

## Resolve a Dependency

Resolve a single artifact and its transitive dependencies:

```bash
antlers resolve org.jetbrains.kotlin:kotlin-stdlib:2.0.0
```

Output:

```
✓ org.jetbrains.kotlin:kotlin-stdlib:2.0.0 (3 artifacts)
# org.jetbrains.kotlin:kotlin-stdlib:2.0.0
  org.jetbrains.kotlin:kotlin-stdlib:2.0.0 (sha256:abc123...)
  org.jetbrains:annotations:13.0 (sha256:def456...)
  org.jetbrains.kotlin:kotlin-stdlib-common:2.0.0 (sha256:789...)
```

## Add a Repository

Need artifacts from JitPack, Jenkins, or another repository?

```bash
# Add a preset repository
antlers add repo jitpack --preset

# Add a custom repository
antlers add repo mycompany https://maven.mycompany.com/releases/

# Add with authentication
antlers add repo github https://maven.pkg.github.com/org/repo --token-env GITHUB_TOKEN
```

## Different Output Formats

```bash
# JSON output
antlers resolve com.google.guava:guava:33.0.0-jre --format json

# Dependency tree
antlers resolve com.google.guava:guava:33.0.0-jre --format tree

# Buck2 build file
antlers resolve com.google.guava:guava:33.0.0-jre --format buck
```

## Import Existing Configuration

Already have Maven or Gradle configured? Import your settings:

```bash
# Auto-detect and import from all found configs
antlers init --detect

# Import from a specific file
antlers init --from ~/.m2/settings.xml
```

This imports repositories and credentials from:

- Maven `settings.xml`
- Gradle `settings.gradle` / `settings.gradle.kts`
- Ivy `ivysettings.xml`
- npm `.npmrc`
- pip `pip.conf`

## Next Steps

- [Configuration Guide](../guide/configuration.md) - Learn about `antlers.toml`
- [Repository Presets](../guide/presets.md) - Available preset repositories
- [CLI Reference](../reference/cli.md) - All available commands
