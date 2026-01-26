# Migrating from Maven

This guide covers migrating from Maven's `settings.xml` to Antlers.

## Quick Start

```bash
antlers init --from ~/.m2/settings.xml
```

## What Gets Migrated

### Servers

Maven servers with credentials become Antlers repositories:

**Maven `settings.xml`:**

```xml
<settings>
    <servers>
        <server>
            <id>github</id>
            <username>myuser</username>
            <password>mytoken</password>
        </server>
    </servers>
</settings>
```

**Antlers `antlers.toml`:**

```toml
[[repositories]]
id = "github"
name = "Server: github"
url = "https://github.example.com/"  # Placeholder - update manually

[repositories.credentials]
type = "basic"
username = "myuser"
password = "mytoken"
```

### Mirrors

Maven mirrors are imported as repositories:

**Maven:**

```xml
<mirrors>
    <mirror>
        <id>artifactory</id>
        <name>Artifactory Mirror</name>
        <url>https://artifactory.example.com/maven</url>
        <mirrorOf>central</mirrorOf>
    </mirror>
</mirrors>
```

**Antlers:**

```toml
[[repositories]]
id = "artifactory"
name = "Artifactory Mirror"
url = "https://artifactory.example.com/maven"
ecosystem = "maven"
```

### Profile Repositories

Repositories defined in profiles are imported:

**Maven:**

```xml
<profiles>
    <profile>
        <id>dev</id>
        <repositories>
            <repository>
                <id>company-releases</id>
                <name>Company Releases</name>
                <url>https://nexus.company.com/releases</url>
            </repository>
        </repositories>
    </profile>
</profiles>
```

**Antlers:**

```toml
[[repositories]]
id = "company-releases"
name = "Company Releases"
url = "https://nexus.company.com/releases"
ecosystem = "maven"
```

### Environment Variables

Maven's `${env.VAR}` syntax is preserved:

**Maven:**

```xml
<server>
    <id>github</id>
    <username>${env.GITHUB_USER}</username>
    <password>${env.GITHUB_TOKEN}</password>
</server>
```

**Antlers:**

```toml
[[repositories]]
id = "github"

[repositories.credentials]
type = "basic"
username = { env = "GITHUB_USER" }
password = { env = "GITHUB_TOKEN" }

[env]
allow = ["GITHUB_USER", "GITHUB_TOKEN"]
```

## Manual Adjustments

After migration, you may need to:

1. **Add repository URLs** - Servers without matching mirrors/repositories get
   placeholder URLs
2. **Remove duplicates** - If the same repository appears in multiple places
3. **Update credentials** - Replace inline passwords with environment variables

## Comparison

| Maven Concept  | Antlers Equivalent           |
| -------------- | ---------------------------- |
| `<server>`     | `[repositories.credentials]` |
| `<mirror>`     | `[[repositories]]`           |
| `<repository>` | `[[repositories]]`           |
| `${env.VAR}`   | `{ env = "VAR" }`            |
| `settings.xml` | `antlers.toml`               |
