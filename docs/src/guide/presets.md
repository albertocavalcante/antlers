# Repository Presets

Antlers includes built-in presets for common Maven repositories, so you don't
need to remember URLs.

## Listing Available Presets

```bash
antlers repos list
```

Filter by ecosystem:

```bash
antlers repos list --ecosystem maven
```

## Using Presets

### With Resolution

```bash
antlers resolve com.github.user:repo:v1.0.0 --preset jitpack
```

### Adding to Configuration

```bash
antlers add repo jitpack --preset
```

This adds the repository to your `antlers.toml`:

```toml
[[repositories]]
id = "jitpack"
name = "JitPack"
url = "https://jitpack.io/"
```

## Available Maven Presets

| Preset ID            | Name               | URL                                                        | Use Case            |
| -------------------- | ------------------ | ---------------------------------------------------------- | ------------------- |
| `central`            | Maven Central      | `https://repo1.maven.org/maven2/`                          | Default repository  |
| `jitpack`            | JitPack            | `https://jitpack.io/`                                      | GitHub projects     |
| `google`             | Google Maven       | `https://maven.google.com/`                                | Android, Firebase   |
| `gradle-plugins`     | Gradle Plugins     | `https://plugins.gradle.org/m2/`                           | Gradle plugins      |
| `jenkins`            | Jenkins            | `https://repo.jenkins-ci.org/releases/`                    | Jenkins plugins     |
| `spring-releases`    | Spring Releases    | `https://repo.spring.io/release/`                          | Spring framework    |
| `spring-milestones`  | Spring Milestones  | `https://repo.spring.io/milestone/`                        | Spring pre-releases |
| `atlassian`          | Atlassian          | `https://packages.atlassian.com/maven/public/`             | Jira, Confluence    |
| `clojars`            | Clojars            | `https://repo.clojars.org/`                                | Clojure libraries   |
| `confluent`          | Confluent          | `https://packages.confluent.io/maven/`                     | Kafka ecosystem     |
| `redhat-ga`          | Red Hat GA         | `https://maven.repository.redhat.com/ga/`                  | Red Hat/JBoss       |
| `sonatype-snapshots` | Sonatype Snapshots | `https://oss.sonatype.org/content/repositories/snapshots/` | OSS snapshots       |

## Preset Suggestions

When resolution fails, Antlers suggests relevant presets based on the group ID:

```bash
$ antlers resolve org.jenkins-ci.plugins:git:5.0.0

Error: Failed to resolve org.jenkins-ci.plugins:git:5.0.0

Hint: This artifact may be in the jenkins repository.
Try: antlers resolve org.jenkins-ci.plugins:git:5.0.0 --preset jenkins
```

### Auto-Suggested Presets

| Group ID Pattern                   | Suggested Preset |
| ---------------------------------- | ---------------- |
| `org.jenkins-ci.*`, `io.jenkins.*` | `jenkins`        |
| `org.gradle.*`, `com.gradle.*`     | `gradle-plugins` |
| `com.atlassian.*`                  | `atlassian`      |
| `com.github.*`, `io.github.*`      | `jitpack`        |
| `org.clojure.*`, `clojure`         | `clojars`        |
| `com.redhat.*`, `org.jboss.*`      | `redhat-ga`      |
| `io.confluent.*`                   | `confluent`      |

## Custom Presets

You can define your own "presets" by adding repositories to your project or
global configuration:

```toml
# In antlers.toml
[[repositories]]
id = "mycompany"
name = "My Company Artifactory"
url = "https://artifactory.mycompany.com/maven/"

[repositories.credentials]
type = "basic"
username = { env = "ARTIFACTORY_USER" }
password = { env = "ARTIFACTORY_PASS" }
```

## Next Steps

- [Configuration Guide](configuration.md) - Full configuration options
- [CLI Reference](../reference/cli.md) - All command options
