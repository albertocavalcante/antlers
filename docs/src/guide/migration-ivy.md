# Migrating from Ivy

This guide covers migrating from Apache Ivy's `ivysettings.xml` to Antlers.

## Quick Start

```bash
antlers init --from ~/.ivy2/ivysettings.xml
```

## What Gets Migrated

### Ibiblio Resolvers

Maven-compatible resolvers are imported:

**Ivy:**

```xml
<ivysettings>
    <resolvers>
        <ibiblio name="central" m2compatible="true" root="https://repo1.maven.org/maven2/"/>
        <ibiblio name="jitpack" root="https://jitpack.io/"/>
    </resolvers>
</ivysettings>
```

**Antlers:**

```toml
[[repositories]]
id = "central"
name = "central"
url = "https://repo1.maven.org/maven2/"
ecosystem = "maven"

[[repositories]]
id = "jitpack"
name = "jitpack"
url = "https://jitpack.io/"
ecosystem = "maven"
```

### Chain Resolvers

Resolvers inside chains are flattened:

**Ivy:**

```xml
<resolvers>
    <chain name="main">
        <ibiblio name="central" root="https://repo1.maven.org/maven2/"/>
        <ibiblio name="private" root="https://maven.example.com/"/>
    </chain>
</resolvers>
```

**Antlers:**

```toml
[[repositories]]
id = "central"
url = "https://repo1.maven.org/maven2/"

[[repositories]]
id = "private"
url = "https://maven.example.com/"
```

### Credentials

Ivy credentials are matched to resolvers by host:

**Ivy:**

```xml
<ivysettings>
    <credentials host="maven.example.com" username="myuser" passwd="mytoken"/>
    <resolvers>
        <ibiblio name="private" root="https://maven.example.com/repo/"/>
    </resolvers>
</ivysettings>
```

**Antlers:**

```toml
[[repositories]]
id = "private"
url = "https://maven.example.com/repo/"

[repositories.credentials]
type = "basic"
username = "myuser"
password = "mytoken"
```

### Environment Variables

Ivy's `${env.VAR}` pattern is preserved:

**Ivy:**

```xml
<credentials host="maven.example.com"
             username="${env.MAVEN_USER}"
             passwd="${env.MAVEN_PASS}"/>
```

**Antlers:**

```toml
[repositories.credentials]
type = "basic"
username = { env = "MAVEN_USER" }
password = { env = "MAVEN_PASS" }

[env]
allow = ["MAVEN_USER", "MAVEN_PASS"]
```

### Default Repository

If an ibiblio resolver has no `root` attribute, Maven Central is used:

**Ivy:**

```xml
<ibiblio name="central" m2compatible="true"/>
```

**Antlers:**

```toml
[[repositories]]
id = "central"
url = "https://repo1.maven.org/maven2/"
```

## Unsupported Features

The following Ivy features are **not** migrated:

- `<url>` resolvers with custom patterns (partially supported)
- `<filesystem>` resolvers
- `<sftp>` and other transport resolvers
- Ivy-specific artifact patterns

## Comparison

| Ivy Concept       | Antlers Equivalent              |
| ----------------- | ------------------------------- |
| `<ibiblio>`       | `[[repositories]]`              |
| `<chain>`         | (flattened to individual repos) |
| `<credentials>`   | `[repositories.credentials]`    |
| `${env.VAR}`      | `{ env = "VAR" }`               |
| `ivysettings.xml` | `antlers.toml`                  |
