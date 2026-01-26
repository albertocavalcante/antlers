# Repository Presets

Antlers includes built-in presets for common repositories, so you don't need to
remember URLs.

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

## Available Presets

### Maven (JVM)

| Preset ID                | Name                   | URL                                                             | Description                                            |
| ------------------------ | ---------------------- | --------------------------------------------------------------- | ------------------------------------------------------ |
| `central`                | Maven Central          | `https://repo1.maven.org/maven2/`                               | The default Maven repository for open-source artifacts |
| `google`                 | Google Maven           | `https://maven.google.com/`                                     | Android and Google libraries                           |
| `gradle-plugins`         | Gradle Plugin Portal   | `https://plugins.gradle.org/m2/`                                | Gradle plugins                                         |
| `jcenter`                | JCenter                | `https://jcenter.bintray.com/`                                  | Deprecated, read-only mirror                           |
| `sonatype-snapshots`     | Sonatype Snapshots     | `https://oss.sonatype.org/content/repositories/snapshots/`      | OSS snapshot builds                                    |
| `sonatype-releases`      | Sonatype Releases      | `https://oss.sonatype.org/content/repositories/releases/`       | OSS releases                                           |
| `sonatype-s01-snapshots` | Sonatype S01 Snapshots | `https://s01.oss.sonatype.org/content/repositories/snapshots/`  | Newer OSS snapshots                                    |
| `jenkins`                | Jenkins                | `https://repo.jenkins-ci.org/public/`                           | Jenkins plugins and libraries                          |
| `jitpack`                | JitPack                | `https://jitpack.io/`                                           | Build JVM artifacts from GitHub                        |
| `spring-milestones`      | Spring Milestones      | `https://repo.spring.io/milestone/`                             | Spring milestone/RC builds                             |
| `spring-snapshots`       | Spring Snapshots       | `https://repo.spring.io/snapshot/`                              | Spring snapshot builds                                 |
| `atlassian`              | Atlassian              | `https://packages.atlassian.com/maven-public/`                  | Atlassian public repository                            |
| `redhat-ga`              | Red Hat GA             | `https://maven.repository.redhat.com/ga/`                       | Red Hat GA repository                                  |
| `clojars`                | Clojars                | `https://repo.clojars.org/`                                     | Clojure libraries                                      |
| `jboss`                  | JBoss                  | `https://repository.jboss.org/nexus/content/groups/public/`     | JBoss community repository                             |
| `apache-snapshots`       | Apache Snapshots       | `https://repository.apache.org/content/repositories/snapshots/` | Apache snapshot builds                                 |
| `hortonworks`            | Hortonworks            | `https://repo.hortonworks.com/content/repositories/releases/`   | Hortonworks artifacts                                  |
| `confluent`              | Confluent              | `https://packages.confluent.io/maven/`                          | Kafka ecosystem artifacts                              |
| `kotlin-dev`             | Kotlin Dev             | `https://maven.pkg.jetbrains.space/kotlin/p/kotlin/dev/`        | Kotlin development builds                              |
| `compose-dev`            | Compose Dev            | `https://maven.pkg.jetbrains.space/public/p/compose/dev/`       | Compose Multiplatform development builds               |

### npm

| Preset ID | Name          | URL                             | Description          |
| --------- | ------------- | ------------------------------- | -------------------- |
| `npmjs`   | npm Registry  | `https://registry.npmjs.org/`   | Default npm registry |
| `yarn`    | Yarn Registry | `https://registry.yarnpkg.com/` | Yarn registry mirror |

### PyPI

| Preset ID  | Name     | URL                             | Description          |
| ---------- | -------- | ------------------------------- | -------------------- |
| `pypi`     | PyPI     | `https://pypi.org/simple/`      | Python Package Index |
| `testpypi` | TestPyPI | `https://test.pypi.org/simple/` | Pre-release testing  |

### NuGet

| Preset ID | Name          | URL                                   | Description            |
| --------- | ------------- | ------------------------------------- | ---------------------- |
| `nuget`   | NuGet Gallery | `https://api.nuget.org/v3/index.json` | Default NuGet registry |

## Preset Suggestions

When resolution fails, Antlers suggests relevant presets based on the group ID:

```bash
$ antlers resolve org.jenkins-ci.plugins:git:5.0.0

Error: Failed to resolve org.jenkins-ci.plugins:git:5.0.0

Hint: This artifact may be in the jenkins repository.
Try: antlers resolve org.jenkins-ci.plugins:git:5.0.0 --preset jenkins
```

### Auto-Suggested Presets

| Group ID Pattern                                    | Suggested Preset   |
| --------------------------------------------------- | ------------------ |
| `org.jenkins-ci.*`, `io.jenkins.*`                  | `jenkins`          |
| `org.gradle.*`, `com.gradle.*`                      | `gradle-plugins`   |
| `org.springframework.*` with `snapshot` in group id | `spring-snapshots` |
| `com.atlassian.*`                                   | `atlassian`        |
| `com.github.*`, `io.github.*`                       | `jitpack`          |
| `org.clojure.*`, `clojure`                          | `clojars`          |
| `com.redhat.*`, `org.jboss.*`                       | `redhat-ga`        |
| `io.confluent.*`                                    | `confluent`        |

## Custom Presets

You can define your own "presets" by adding repositories to your project
configuration:

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
