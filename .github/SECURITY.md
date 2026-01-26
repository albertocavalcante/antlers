# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in antlers, please report it by:

1. **DO NOT** open a public issue
2. Email the maintainer directly or use GitHub's private vulnerability reporting feature
3. Include as much detail as possible:
   - Type of vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

We will respond within 48 hours and work with you to understand and address the issue.

## Security Measures

This project implements several security best practices:

- **Checksum verification**: All downloaded artifacts are verified against SHA-256 checksums
- **Pinned dependencies**: GitHub Actions are pinned to commit SHAs
- **Automated updates**: Dependabot monitors for security updates
- **No credential storage**: The CLI does not store or cache credentials
