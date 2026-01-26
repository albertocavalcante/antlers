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

For hermetic workflows, explicitly allow variables:

```toml
[env]
allow = ["GITHUB_TOKEN", "MAVEN_USER", "MAVEN_PASS"]
```

When `[env].allow` is set, only listed variables can be referenced in
credentials. This is enforced by `AntlersToml::validate_hermetic()` or by
integrations that choose to enforce hermetic rules; the CLI does not enforce it
yet.

## Proxy Configuration

Antlers uses `reqwest`, which enables system proxy settings by default. The
following environment variables are honored by the HTTP client:

| Variable                      | Description                             |
| ----------------------------- | --------------------------------------- |
| `HTTP_PROXY` / `http_proxy`   | HTTP proxy URL                          |
| `HTTPS_PROXY` / `https_proxy` | HTTPS proxy URL                         |
| `ALL_PROXY` / `all_proxy`     | Proxy for both HTTP and HTTPS           |
| `NO_PROXY` / `no_proxy`       | Comma-separated list of hosts to bypass |

Example:

```bash
export HTTPS_PROXY=http://proxy.example.com:8080
export NO_PROXY=localhost,internal.example.com
antlers resolve com.google.guava:guava:33.0.0-jre
```

## Netrc

When using `type = "netrc"` credentials, the netrc file path can be overridden
with:

| Variable | Description        |
| -------- | ------------------ |
| `NETRC`  | Path to netrc file |
