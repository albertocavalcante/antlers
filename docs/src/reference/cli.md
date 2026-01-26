# CLI Reference

Complete reference for all `antlers` commands.

## Global Options

```
-v, --verbose    Enable verbose logging
-h, --help       Print help
-V, --version    Print version
```

---

## antlers init

Initialize a new `antlers.toml` configuration file.

```bash
antlers init [OPTIONS]
```

### Options

| Option                | Description                                                   |
| --------------------- | ------------------------------------------------------------- |
| `--from <FILE>`       | Import from a specific config file                            |
| `--detect`            | Auto-detect and import from all found package manager configs |
| `-o, --output <PATH>` | Output path (default: `antlers.toml`)                         |
| `--force`             | Overwrite existing file                                       |

### Examples

```bash
# Create minimal config
antlers init

# Import from Maven settings
antlers init --from ~/.m2/settings.xml

# Auto-detect all configs
antlers init --detect
```

---

## antlers resolve

Resolve dependencies for one or more artifacts.

```bash
antlers resolve [OPTIONS] <ARTIFACTS>...
```

### Arguments

| Argument      | Description                                           |
| ------------- | ----------------------------------------------------- |
| `<ARTIFACTS>` | Artifact coordinates (e.g., `group:artifact:version`) |

### Options

| Option                  | Description                                           |
| ----------------------- | ----------------------------------------------------- |
| `-t, --transitive`      | Resolve transitive dependencies (default: true)       |
| `-f, --format <FORMAT>` | Output format: `text`, `json`, `tree`, `buck` (Buck2) |
| `-o, --output <FILE>`   | Write output to file                                  |
| `--repo <URL>`          | Additional repository URL                             |
| `-p, --preset <NAME>`   | Use a preset repository                               |
| `--pom-only`            | Disable Gradle Module Metadata                        |

### Examples

```bash
# Basic resolution
antlers resolve org.jetbrains.kotlin:kotlin-stdlib:2.0.0

# Multiple artifacts
antlers resolve com.google.guava:guava:33.0.0-jre com.fasterxml.jackson.core:jackson-databind:2.17.0

# With preset repository
antlers resolve com.github.user:repo:v1.0.0 --preset jitpack

# JSON output to file
antlers resolve com.google.guava:guava:33.0.0-jre -f json -o deps.json

# Dependency tree
antlers resolve org.springframework.boot:spring-boot-starter:3.2.0 -f tree
```

---

## antlers add repo

Add a repository to `antlers.toml`.

```bash
antlers add repo [OPTIONS] <ID> [URL]
```

### Arguments

| Argument | Description                                 |
| -------- | ------------------------------------------- |
| `<ID>`   | Repository ID (used as key in config)       |
| `[URL]`  | Repository URL (not needed with `--preset`) |

### Options

| Option                | Description                                                 |
| --------------------- | ----------------------------------------------------------- |
| `-p, --preset`        | Use a preset repository (ID becomes preset name)            |
| `--name <NAME>`       | Display name for the repository                             |
| `--ecosystem <ECO>`   | Ecosystem: `maven`, `npm`, `pypi`, `nuget` (default: maven) |
| `--token-env <VAR>`   | Environment variable for bearer token auth                  |
| `-c, --config <PATH>` | Path to antlers.toml (default: `antlers.toml`)              |

### Examples

```bash
# Add a preset
antlers add repo jitpack --preset

# Add custom repository
antlers add repo mycompany https://maven.mycompany.com/releases/

# With authentication
antlers add repo github https://maven.pkg.github.com/org/repo --token-env GITHUB_TOKEN

# Different ecosystem
antlers add repo npm-private https://npm.mycompany.com/ --ecosystem npm
```

---

## antlers fetch

Download an artifact (verifies checksums when available).

```bash
antlers fetch [OPTIONS] <ARTIFACT>
```

### Options

| Option               | Description                     |
| -------------------- | ------------------------------- |
| `-o, --output <DIR>` | Output directory (default: `.`) |
| `--sources`          | Also fetch sources JAR          |
| `--javadoc`          | Also fetch javadoc JAR          |

### Examples

```bash
# Download artifact
antlers fetch org.jetbrains.kotlin:kotlin-stdlib:2.0.0

# With sources and javadoc
antlers fetch org.jetbrains.kotlin:kotlin-stdlib:2.0.0 --sources --javadoc

# To specific directory
antlers fetch org.jetbrains.kotlin:kotlin-stdlib:2.0.0 -o ./libs
```

---

## antlers info

Display information about an artifact coordinate.

```bash
antlers info <ARTIFACT>
```

### Examples

```bash
antlers info org.jetbrains.kotlin:kotlin-stdlib:2.0.0
```

Output:

```
Artifact Information
==================================================
Group ID:    org.jetbrains.kotlin
Artifact ID: kotlin-stdlib
Version:     2.0.0
Extension:   jar

Repository path: org/jetbrains/kotlin/kotlin-stdlib/2.0.0/kotlin-stdlib-2.0.0.jar
POM path:        org/jetbrains/kotlin/kotlin-stdlib/2.0.0/kotlin-stdlib-2.0.0.pom
```

---

## antlers show

Display the current configuration.

```bash
antlers show [OPTIONS] [PATH]
```

### Options

| Option   | Description    |
| -------- | -------------- |
| `--json` | Output as JSON |

### Examples

```bash
# Show config
antlers show

# JSON output
antlers show --json

# Specific file
antlers show /path/to/antlers.toml
```

---

## antlers fmt

Format `antlers.toml` with canonical ordering.

```bash
antlers fmt [OPTIONS] [PATH]
```

### Options

| Option    | Description                        |
| --------- | ---------------------------------- |
| `--check` | Check if formatted (exit 1 if not) |
| `--diff`  | Show diff without writing          |

### Examples

```bash
# Format in place
antlers fmt

# Check only (for CI)
antlers fmt --check

# Show diff
antlers fmt --diff
```

---

## antlers repos [list]

List available repository presets. `antlers repos` is equivalent to
`antlers repos list`.

```bash
antlers repos [list] [OPTIONS]
```

### Options

| Option                  | Description         |
| ----------------------- | ------------------- |
| `-e, --ecosystem <ECO>` | Filter by ecosystem |

### Examples

```bash
# List all presets
antlers repos list

# Maven only
antlers repos list --ecosystem maven
```
