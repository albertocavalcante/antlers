# Migrating from Gradle

This guide covers migrating from Gradle's `settings.gradle` or
`settings.gradle.kts` to Antlers.

## Quick Start

```bash
antlers init --from settings.gradle.kts
```

## What Gets Migrated

### Maven Repositories

Gradle `maven { }` blocks are imported:

**Gradle (Kotlin DSL):**

```kotlin
dependencyResolutionManagement {
    repositories {
        mavenCentral()
        maven {
            url = uri("https://maven.example.com/releases")
        }
    }
}
```

**Antlers:**

```toml
[[repositories]]
id = "central"
name = "Maven Central"
url = "https://repo1.maven.org/maven2/"
ecosystem = "maven"

[[repositories]]
id = "maven-1"
url = "https://maven.example.com/releases"
ecosystem = "maven"
```

### Well-Known Repositories

Gradle shortcuts are recognized:

| Gradle                 | Antlers Equivalent                       |
| ---------------------- | ---------------------------------------- |
| `mavenCentral()`       | Maven Central                            |
| `google()`             | Google Maven                             |
| `gradlePluginPortal()` | Gradle Plugin Portal                     |
| `jcenter()`            | JCenter (deprecated, read-only)          |
| `mavenLocal()`         | Maven Local (`file://~/.m2/repository/`) |

### Authenticated Repositories

**Gradle:**

```kotlin
maven {
    url = uri("https://maven.pkg.github.com/org/repo")
    credentials {
        username = System.getenv("GITHUB_USER")
        password = System.getenv("GITHUB_TOKEN")
    }
}
```

**Antlers:**

```toml
[[repositories]]
id = "maven-1"
url = "https://maven.pkg.github.com/org/repo"

[repositories.credentials]
type = "basic"
username = { env = "GITHUB_USER" }
password = { env = "GITHUB_TOKEN" }

[env]
allow = ["GITHUB_USER", "GITHUB_TOKEN"]
```

## Groovy DSL

Both Kotlin and Groovy DSL are supported:

**Gradle (Groovy):**

```groovy
dependencyResolutionManagement {
    repositories {
        mavenCentral()
        maven {
            url 'https://maven.example.com/releases'
        }
    }
}
```

## Limitations

The parser is line-based and does not evaluate Gradle logic. Review the output,
especially when repositories are built dynamically or conditionally. Credentials
are attached to the most recent repository block, and pluginManagement and
dependencyResolutionManagement repositories are both scanned. Dependencies and
version catalogs are not migrated.

## Comparison

| Gradle Concept          | Antlers Equivalent           |
| ----------------------- | ---------------------------- |
| `maven { url = ... }`   | `[[repositories]]`           |
| `credentials { }`       | `[repositories.credentials]` |
| `System.getenv()`       | `{ env = "..." }`            |
| `settings.gradle(.kts)` | `antlers.toml`               |
