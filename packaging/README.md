# Packaging Templates

This directory contains packaging templates for building distribution packages.

## Structure

```
packaging/
├── deb/
│   └── control.in            # Debian/Ubuntu package control file template
├── rpm/
│   └── antlers.spec.in       # RPM spec file template
├── homebrew/
│   └── antlers-nightly.rb.in # Homebrew formula template (nightly)
└── README.md
```

## Template Variables

Templates use `${VAR}` syntax for `envsubst` substitution:

### DEB/RPM Variables

| Variable               | Description            | Example (nightly)                    | Example (release) |
| ---------------------- | ---------------------- | ------------------------------------ | ----------------- |
| `${BINARY_NAME}`       | Binary name            | `antlers`                            | `antlers`         |
| `${VERSION}`           | Package version        | `0.0.0~nightly.20240101.abc1234`     | `1.0.0`           |
| `${RELEASE}`           | RPM release number     | `0.nightly.20240101.abc1234`         | `1`               |
| `${ARCH}`              | Architecture (DEB)     | `amd64`                              | `amd64`           |
| `${SUMMARY_SUFFIX}`    | Suffix for summary     | ` (nightly)`                         | (empty)           |
| `${DESCRIPTION_EXTRA}` | Extra description line | `This is an unstable nightly build.` | (empty)           |

### Homebrew Variables

| Variable                 | Description              | Example                                       |
| ------------------------ | ------------------------ | --------------------------------------------- |
| `${SHORT_SHA}`           | Git short SHA            | `abc1234`                                     |
| `${SHA256_DARWIN_ARM64}` | Checksum for macOS ARM64 | `7a003ee177ce17d7b72be0e751e99f1fc903b24c...` |
| `${SHA256_DARWIN_AMD64}` | Checksum for macOS Intel | `0d3e99d77196992714d623aacf24bdde2be2400c...` |
| `${SHA256_LINUX_ARM64}`  | Checksum for Linux ARM64 | `515bd6246c86c65f712735cb11abc4e490b9ea3e...` |
| `${SHA256_LINUX_AMD64}`  | Checksum for Linux AMD64 | `1ade74fcfef9f453c9a56f38adaa1b3e1f7433c6...` |

## Usage

Templates are processed with `envsubst` during CI builds:

```bash
# DEB/RPM example
export BINARY_NAME="antlers"
export VERSION="1.0.0"
export ARCH="amd64"
export SUMMARY_SUFFIX=""
export DESCRIPTION_EXTRA=""
envsubst < packaging/deb/control.in > pkg/DEBIAN/control

# Homebrew example
export SHORT_SHA="abc1234"
export SHA256_DARWIN_ARM64="..."
# ... other checksums
envsubst < packaging/homebrew/antlers-nightly.rb.in > antlers-nightly.rb
```

## Why envsubst?

- Pre-installed on Ubuntu CI runners (part of `gettext`)
- Uses standard shell variable syntax
- No escaping issues (unlike `sed`)
- Clean and readable

## Homebrew Tap Updates

The nightly workflow automatically updates
[homebrew-tap](https://github.com/albertocavalcante/homebrew-tap) via GitHub
API:

- **Committer:** Eukia[bot] (GitHub App)
- **Author:** github-actions[bot]
- **Method:** `gh api` with contents API (no git clone needed)

Users can install via:

```bash
brew tap albertocavalcante/tap
brew install antlers-nightly
```
