# antlers.toml Reference

Complete reference for the `antlers.toml` configuration file format.

## Overview

```toml
[project]
name = "my-project"
version = "1.0.0"
description = "My JVM project"

[[repositories]]
id = "central"
name = "Maven Central"
url = "https://repo1.maven.org/maven2/"
ecosystem = "maven"

[[repositories]]
id = "github"
name = "GitHub Packages"
url = "https://maven.pkg.github.com/org/repo"

[repositories.credentials]
type = "bearer"
token = { env = "GITHUB_TOKEN" }

[dependencies]
"com.google.guava:guava" = "33.0.0-jre"
"org.jetbrains.kotlin:kotlin-stdlib" = "2.0.0"

[resolver]
conflict-strategy = "highest"
transitive = true

[cache]
path = ".antlers/cache"
mode = "read-write"

[env]
allow = ["GITHUB_TOKEN", "MAVEN_USER", "MAVEN_PASS"]
```

---

## [project]

Project metadata. Optional but recommended.

| Field         | Type   | Description         |
| ------------- | ------ | ------------------- |
| `name`        | string | Project name        |
| `version`     | string | Project version     |
| `description` | string | Project description |

```toml
[project]
name = "my-project"
version = "1.0.0"
description = "A sample JVM project"
```

---

## [[repositories]]

Repository configuration. Can have multiple entries.

| Field         | Type   | Required | Description                                        |
| ------------- | ------ | -------- | -------------------------------------------------- |
| `id`          | string | Yes      | Unique identifier                                  |
| `url`         | string | Yes      | Repository URL                                     |
| `name`        | string | No       | Display name                                       |
| `ecosystem`   | string | No       | `maven`, `npm`, `pypi`, `nuget` (default: `maven`) |
| `credentials` | table  | No       | Authentication configuration                       |

### Basic Repository

```toml
[[repositories]]
id = "central"
name = "Maven Central"
url = "https://repo1.maven.org/maven2/"
ecosystem = "maven"
```

### With Bearer Token

```toml
[[repositories]]
id = "github"
url = "https://maven.pkg.github.com/org/repo"

[repositories.credentials]
type = "bearer"
token = { env = "GITHUB_TOKEN" }
```

### With Basic Auth

```toml
[[repositories]]
id = "artifactory"
url = "https://artifactory.example.com/maven"

[repositories.credentials]
type = "basic"
username = { env = "ARTIFACTORY_USER" }
password = { env = "ARTIFACTORY_PASS" }
```

### Inline Values (Not Recommended)

```toml
[repositories.credentials]
type = "basic"
username = "myuser"
password = "mypassword"  # Avoid committing secrets!
```

### Netrc Authentication

```toml
[repositories.credentials]
type = "netrc"
```

---

## [dependencies]

Dependencies to resolve. Keys are `group:artifact` coordinates.

### Simple Version

```toml
[dependencies]
"com.google.guava:guava" = "33.0.0-jre"
"org.jetbrains.kotlin:kotlin-stdlib" = "2.0.0"
```

### Detailed Specification

```toml
[dependencies]
"com.google.guava:guava" = { version = "33.0.0-jre", classifier = "jdk8" }
"org.apache.logging.log4j:log4j-core" = { version = "2.23.0", exclusions = ["org.apache.logging.log4j:log4j-api"] }
```

---

## [dev-dependencies]

Development-only dependencies. Same format as `[dependencies]`.

```toml
[dev-dependencies]
"org.junit.jupiter:junit-jupiter" = "5.10.0"
"org.mockito:mockito-core" = "5.10.0"
```

---

## [resolver]

Resolution behavior configuration.

| Field               | Type   | Default     | Description                     |
| ------------------- | ------ | ----------- | ------------------------------- |
| `conflict-strategy` | string | `"highest"` | `highest`, `nearest`, `strict`  |
| `transitive`        | bool   | `true`      | Resolve transitive dependencies |

```toml
[resolver]
conflict-strategy = "highest"
transitive = true
```

### Conflict Strategies

- **`highest`** - Use highest version when conflicts occur (Maven default)
- **`nearest`** - Use version from nearest dependency in tree (Gradle-like)
- **`strict`** - Fail on any version conflict

---

## [cache]

Caching configuration.

| Field  | Type   | Default        | Description                           |
| ------ | ------ | -------------- | ------------------------------------- |
| `path` | string | OS default     | Cache directory path                  |
| `mode` | string | `"read-write"` | `read-write`, `read-only`, `disabled` |

```toml
[cache]
path = ".antlers/cache"
mode = "read-write"
```

### Cache Modes

- **`read-write`** - Normal caching (default)
- **`read-only`** - Use cache but don't update it
- **`disabled`** - No caching

---

## [env]

Environment variable configuration for hermetic builds.

| Field   | Type  | Description                            |
| ------- | ----- | -------------------------------------- |
| `allow` | array | Environment variables that can be used |

```toml
[env]
allow = ["GITHUB_TOKEN", "MAVEN_USER", "MAVEN_PASS"]
```

When `allow` is set, only listed variables can be referenced in credentials.

---

## [hermetic]

Hermetic build configuration for reproducibility.

| Field   | Type   | Description                          |
| ------- | ------ | ------------------------------------ |
| `level` | string | `disabled`, `reproducible`, `strict` |

```toml
[hermetic]
level = "strict"
```

### Hermetic Levels

- **`disabled`** - No hermetic enforcement (default)
- **`reproducible`** - Require lockfile, allow network
- **`strict`** - Require lockfile, no network (fully offline)

---

## [exclusions]

Global exclusions applied to all dependencies.

```toml
[exclusions]
patterns = [
    "commons-logging:commons-logging",
    "log4j:log4j",
]
```

---

## [constraints]

Version constraints applied during resolution.

```toml
[constraints]
"org.slf4j:slf4j-api" = "2.0.0"
"com.fasterxml.jackson.core:jackson-databind" = "[2.15,2.18)"
```

---

## Complete Example

```toml
[project]
name = "my-application"
version = "1.0.0"
description = "A sample JVM application"

[[repositories]]
id = "central"
name = "Maven Central"
url = "https://repo1.maven.org/maven2/"

[[repositories]]
id = "jitpack"
name = "JitPack"
url = "https://jitpack.io/"

[[repositories]]
id = "company"
name = "Company Artifactory"
url = "https://artifactory.example.com/maven"

[repositories.credentials]
type = "basic"
username = { env = "ARTIFACTORY_USER" }
password = { env = "ARTIFACTORY_PASS" }

[dependencies]
"org.jetbrains.kotlin:kotlin-stdlib" = "2.0.0"
"com.google.guava:guava" = "33.0.0-jre"
"com.squareup.okhttp3:okhttp" = "4.12.0"

[dev-dependencies]
"org.junit.jupiter:junit-jupiter" = "5.10.0"

[resolver]
conflict-strategy = "highest"
transitive = true

[cache]
path = ".antlers/cache"
mode = "read-write"

[env]
allow = ["ARTIFACTORY_USER", "ARTIFACTORY_PASS"]

[exclusions]
patterns = ["commons-logging:commons-logging"]

[constraints]
"org.slf4j:slf4j-api" = "2.0.0"
```
