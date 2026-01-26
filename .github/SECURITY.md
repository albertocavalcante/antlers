# Security Policy

## Scope of Security Vulnerabilities

Antlers is a dependency resolver for JVM (and eventually other) package ecosystems. Due to the nature of dependency resolution and package management, there are cases where antlers may facilitate execution of code that is outside our control:

- **Package build scripts**: When resolving dependencies, upstream packages may contain build logic
- **Repository responses**: Antlers fetches metadata from configured Maven repositories
- **Transitive dependencies**: Resolved dependency trees include packages authored by third parties

These are inherent to package management and are **not** considered vulnerabilities in antlers itself. However, we welcome suggestions for hardening antlers' security posture in these areas as feature requests.

### What IS in scope

- Vulnerabilities in antlers' own code (buffer overflows, injection flaws, etc.)
- Checksum verification bypasses
- Path traversal or arbitrary file write vulnerabilities
- Dependency confusion attack vectors that antlers could mitigate
- Information disclosure (credentials, tokens, sensitive paths)

### What is NOT in scope

- Malicious packages in upstream repositories
- Vulnerabilities in resolved/downloaded dependencies
- Issues requiring physical access to the machine
- Social engineering attacks

## Reporting a Vulnerability

If you believe you have found a security vulnerability in antlers, please report it through **GitHub's private vulnerability reporting**:

1. Go to the [Security tab](https://github.com/albertocavalcante/antlers/security) of this repository
2. Click "Report a vulnerability"
3. Provide a detailed description including:
   - Type of vulnerability
   - Steps to reproduce
   - Potential impact
   - Affected versions
   - Suggested fix (if any)

Alternatively, you can email the maintainer directly. **Please do not report security vulnerabilities through public GitHub issues.**

## Response Process

This is a community-maintained project with limited resources. While we take security seriously and prioritize vulnerability reports, we cannot commit to specific response timelines. We will:

1. Acknowledge receipt of your report
2. Investigate and validate the issue
3. Work on a fix and coordinate disclosure
4. Credit reporters in the security advisory (unless anonymity is requested)

For critical vulnerabilities, we will make best efforts to respond promptly.

## Vulnerability Disclosure

Security issues will be disclosed via:

- [GitHub Security Advisories](https://github.com/albertocavalcante/antlers/security/advisories)
- [RustSec Advisory Database](https://rustsec.org/) (for `cargo-audit` integration)
- Release notes for the patched version

## Security Practices

This project implements several security measures:

- **Checksum verification**: Downloaded artifacts are verified against SHA-256 checksums
- **Pinned CI dependencies**: GitHub Actions are pinned to commit SHAs to prevent supply chain attacks
- **Automated updates**: Dependabot monitors for security updates in dependencies
- **No credential storage**: The CLI does not store or cache authentication credentials
- **Minimal permissions**: CI workflows use minimal required permissions

## Bug Bounty

This project does not currently have a bug bounty program.
