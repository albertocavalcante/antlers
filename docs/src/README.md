# Antlers

> **Fast, native JVM dependency resolution without the JVM.**

**Antlers** is a native JVM dependency resolver written in Rust. It resolves
Maven and Gradle dependencies without requiring Java, Maven, or Gradle to be
installed.

## Why Antlers?

- **Fast** - Native Rust implementation with parallel downloads
- **No JVM required** - Resolve Java dependencies without Java installed
- **Hermetic builds** - Reproducible, lockfile-based resolution
- **Multi-ecosystem** - Supports Maven Central, Gradle plugins, and custom
  repositories
- **Migration-friendly** - Import settings from Maven, Gradle, npm, pip, and Ivy

## Quick Example

```bash
# Initialize a new project
antlers init

# Resolve a dependency
antlers resolve org.jetbrains.kotlin:kotlin-stdlib:2.0.0

# Add a repository
antlers add repo jitpack --preset
```

## Features

| Feature                 | Description                                                 |
| ----------------------- | ----------------------------------------------------------- |
| Dependency resolution   | Transitive resolution with conflict strategies              |
| Gradle Module Metadata  | Full .module file support for rich variants                 |
| Repository presets      | Built-in support for JitPack, Jenkins, Gradle Plugins, etc. |
| Configuration migration | Import from settings.xml, settings.gradle, ivysettings.xml  |
| Hermetic mode           | Fully reproducible builds with lockfiles                    |
| Credential management   | Bearer tokens, basic auth, environment variables            |

## Getting Started

Head to the [Installation](getting-started/installation.md) page to get started,
or jump straight to the [Quick Start](getting-started/quickstart.md) guide.

## License

Antlers is dual-licensed under Apache 2.0 and MIT.
