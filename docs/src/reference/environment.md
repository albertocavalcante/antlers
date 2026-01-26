# Environment Variables

This page documents environment variables that affect Antlers behavior.

## Credential Variables

Reference environment variables in credentials using `{ env = "VAR_NAME" }`:

```toml
[repositories.credentials]
type = "basic"
username = { env = "MAVEN_USER" }
password = { env = "MAVEN_PASS" }
```

### Common Variables

| Variable           | Description                    |
| ------------------ | ------------------------------ |
| `GITHUB_TOKEN`     | GitHub Packages authentication |
| `GITLAB_TOKEN`     | GitLab Packages authentication |
| `ARTIFACTORY_USER` | Artifactory username           |
| `ARTIFACTORY_PASS` | Artifactory password           |
| `NEXUS_USER`       | Nexus username                 |
| `NEXUS_PASS`       | Nexus password                 |

## Allowed Variables

For hermetic builds, explicitly allow variables:

```toml
[env]
allow = ["GITHUB_TOKEN", "MAVEN_USER", "MAVEN_PASS"]
```

When `[env].allow` is set, only listed variables can be used in credentials.

## Proxy Configuration

Antlers respects standard proxy environment variables:

| Variable                      | Description                             |
| ----------------------------- | --------------------------------------- |
| `HTTP_PROXY` / `http_proxy`   | HTTP proxy URL                          |
| `HTTPS_PROXY` / `https_proxy` | HTTPS proxy URL                         |
| `NO_PROXY` / `no_proxy`       | Comma-separated list of hosts to bypass |

Example:

```bash
export HTTPS_PROXY=http://proxy.example.com:8080
export NO_PROXY=localhost,internal.example.com
antlers resolve com.google.guava:guava:33.0.0-jre
```

## Cache Directory

Override the default cache location:

| Variable            | Description                 |
| ------------------- | --------------------------- |
| `ANTLERS_CACHE_DIR` | Custom cache directory path |

Or configure in `antlers.toml`:

```toml
[cache]
path = "/custom/cache/path"
```

## Logging

Control log output:

| Variable   | Description                                       |
| ---------- | ------------------------------------------------- |
| `RUST_LOG` | Log level filter (e.g., `debug`, `antlers=debug`) |

Example:

```bash
RUST_LOG=debug antlers resolve com.google.guava:guava:33.0.0-jre
```

Or use the `--verbose` flag:

```bash
antlers --verbose resolve com.google.guava:guava:33.0.0-jre
```

## CI/CD Detection

Antlers automatically detects CI environments and adjusts behavior:

| Variable         | CI System            |
| ---------------- | -------------------- |
| `CI`             | Generic CI detection |
| `GITHUB_ACTIONS` | GitHub Actions       |
| `GITLAB_CI`      | GitLab CI            |
| `JENKINS_URL`    | Jenkins              |
| `CIRCLECI`       | CircleCI             |

In CI mode:

- Progress bars are disabled
- Output is optimized for logs

## Example: GitHub Actions

```yaml
- name: Resolve dependencies
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: |
    antlers resolve com.github.user:repo:v1.0.0 --preset jitpack
```

## Example: GitLab CI

```yaml
resolve:
  script:
    - antlers resolve com.google.guava:guava:33.0.0-jre
  variables:
    GITLAB_TOKEN: $CI_JOB_TOKEN
```
