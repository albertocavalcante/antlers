# antlers.toml Reference

Complete reference for the `antlers.toml` configuration file format.

## Status

The CLI currently uses `antlers.toml` for `init`, `add`, `show`, and `fmt`.
`resolve` and `fetch` are driven by CLI arguments and do not read the file yet.
The schema below is intended for tooling and library integrations.

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

[dev-dependencies]
"org.junit.jupiter:junit-jupiter" = "5.10.0"

[build-dependencies]
"org.jetbrains.kotlin:kotlin-gradle-plugin" = "2.0.0"

[constraints]
"com.fasterxml.jackson:jackson-bom" = { version = "2.16.0", type = "bom" }

[exclusions]
"commons-logging:commons-logging" = "*"

[resolver]
conflict-strategy = "highest-wins"
transitive = true

[cache]
path = ".antlers/cache"
mode = "read-write"

[network]
offline = false
connect-timeout = 30
max-connections = 16

[env]
allow = ["GITHUB_TOKEN", "MAVEN_USER", "MAVEN_PASS"]

[output]
lockfile = "antlers.lock.json"
require-sha256 = true
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

Repository configuration. Order matters for priority.

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

Detailed fields:

| Field        | Type   | Description                                        |
| ------------ | ------ | -------------------------------------------------- |
| `version`    | string | Required version requirement                       |
| `scope`      | string | Maven scope (compile, runtime, test, provided)     |
| `classifier` | string | Classifier (e.g., sources, javadoc)                |
| `type`       | string | Artifact type/extension (e.g., jar, pom)           |
| `exclusions` | array  | Transitive exclusions (`group:artifact`)           |
| `transitive` | bool   | Whether to include transitive deps (default: true) |

---

## [dev-dependencies]

Development-only dependencies. Same format as `[dependencies]`.

```toml
[dev-dependencies]
"org.junit.jupiter:junit-jupiter" = "5.10.0"
"org.mockito:mockito-core" = "5.10.0"
```

---

## [build-dependencies]

Build-time dependencies. Same format as `[dependencies]`.

```toml
[build-dependencies]
"org.jetbrains.kotlin:kotlin-gradle-plugin" = "2.0.0"
```

---

## [constraints]

Version constraints (BOMs, platform constraints). Keys are `group:artifact`.

```toml
[constraints]
"com.fasterxml.jackson:jackson-bom" = { version = "2.16.0", type = "bom" }
"org.springframework:spring-framework-bom" = { version = "6.1.5", type = "platform" }
```

Constraint fields:

| Field     | Type   | Description         |
| --------- | ------ | ------------------- |
| `version` | string | Required version    |
| `type`    | string | `bom` or `platform` |

---

## [exclusions]

Global exclusions applied to all dependencies.

```toml
[exclusions]
"commons-logging:commons-logging" = "*"
"log4j:log4j" = "2.0.0"
```

---

## [resolver]

Resolution behavior configuration.

| Field               | Type   | Default        | Description                              |
| ------------------- | ------ | -------------- | ---------------------------------------- |
| `conflict-strategy` | string | `highest-wins` | `highest-wins`, `nearest-wins`, `strict` |
| `transitive`        | bool   | `true`         | Resolve transitive dependencies          |

```toml
[resolver]
conflict-strategy = "highest-wins"
transitive = true
```

### Conflict Strategies

- **`highest-wins`** - Use highest version when conflicts occur
  (Gradle/Coursier-like)
- **`nearest-wins`** - Use version from nearest dependency in tree (Maven-like)
- **`strict`** - Fail on any version conflict

---

## [cache]

Caching configuration.

| Field  | Type   | Default      | Description                           |
| ------ | ------ | ------------ | ------------------------------------- |
| `path` | string | OS default   | Cache directory path                  |
| `mode` | string | `read-write` | `read-write`, `read-only`, `disabled` |

```toml
[cache]
path = ".antlers/cache"
mode = "read-write"
```

---

## [network]

Network behavior configuration.

| Field             | Type | Description                    |
| ----------------- | ---- | ------------------------------ |
| `offline`         | bool | Disable network access         |
| `connect-timeout` | int  | Connection timeout in seconds  |
| `max-connections` | int  | Maximum concurrent connections |

```toml
[network]
offline = false
connect-timeout = 30
max-connections = 16
```

---

## [env]

Environment variable configuration for hermetic workflows.

| Field   | Type  | Description                            |
| ------- | ----- | -------------------------------------- |
| `allow` | array | Environment variables that can be used |

```toml
[env]
allow = ["GITHUB_TOKEN", "MAVEN_USER", "MAVEN_PASS"]
```

---

## [output]

Output configuration used by tooling/integrations.

| Field            | Type   | Description                          |
| ---------------- | ------ | ------------------------------------ |
| `lockfile`       | string | Lockfile path                        |
| `require-sha256` | bool   | Require SHA-256 checksums in outputs |

```toml
[output]
lockfile = "antlers.lock.json"
require-sha256 = true
```
