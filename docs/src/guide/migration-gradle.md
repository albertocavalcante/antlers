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
id = "maven-example-com"
name = "maven-example-com"
url = "https://maven.example.com/releases"
ecosystem = "maven"
```

### Well-Known Repositories

Gradle shortcuts are recognized:

| Gradle                 | Antlers Equivalent |
| ---------------------- | ------------------ |
| `mavenCentral()`       | Maven Central      |
| `google()`             | Google Maven       |
| `gradlePluginPortal()` | Gradle Plugins     |
| `mavenLocal()`         | (skipped)          |

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
id = "github"
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

The following are **not** migrated:

- `mavenLocal()` - Local Maven cache (not applicable)
- Plugin repositories - Only dependency repositories
- Version catalogs - Dependencies themselves

## Comparison

| Gradle Concept          | Antlers Equivalent           |
| ----------------------- | ---------------------------- |
| `maven { url = ... }`   | `[[repositories]]`           |
| `credentials { }`       | `[repositories.credentials]` |
| `System.getenv()`       | `{ env = "..." }`            |
| `settings.gradle(.kts)` | `antlers.toml`               |
