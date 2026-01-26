# Packaging Templates

This directory contains packaging templates for building distribution packages.

## Structure

```
packaging/
├── deb/
│   └── control.in      # Debian/Ubuntu package control file template
├── rpm/
│   └── antlers.spec.in # RPM spec file template
└── README.md
```

## Template Variables

Templates use `${VAR}` syntax for `envsubst` substitution:

| Variable               | Description            | Example (nightly)                    | Example (release) |
| ---------------------- | ---------------------- | ------------------------------------ | ----------------- |
| `${BINARY_NAME}`       | Binary name            | `antlers`                            | `antlers`         |
| `${VERSION}`           | Package version        | `0.0.0~nightly.20240101.abc1234`     | `1.0.0`           |
| `${RELEASE}`           | RPM release number     | `0.nightly.20240101.abc1234`         | `1`               |
| `${ARCH}`              | Architecture (DEB)     | `amd64`                              | `amd64`           |
| `${SUMMARY_SUFFIX}`    | Suffix for summary     | ` (nightly)`                         | (empty)           |
| `${DESCRIPTION_EXTRA}` | Extra description line | `This is an unstable nightly build.` | (empty)           |

## Usage

Templates are processed with `envsubst` during CI builds:

```bash
# Set variables
export BINARY_NAME="antlers"
export VERSION="1.0.0"
export ARCH="amd64"
export SUMMARY_SUFFIX=""
export DESCRIPTION_EXTRA=""

# Generate from template
envsubst < packaging/deb/control.in > pkg/DEBIAN/control
```

## Why envsubst?

- Pre-installed on Ubuntu CI runners (part of `gettext`)
- Uses standard shell variable syntax
- No escaping issues (unlike `sed`)
- Clean and readable

## Adding New Package Formats

To add support for new package formats (e.g., Homebrew, Alpine APK):

1. Create a new directory: `packaging/<format>/`
2. Add template files with `.in` extension using `${VAR}` syntax
3. Update the relevant workflow to export variables and run `envsubst`
